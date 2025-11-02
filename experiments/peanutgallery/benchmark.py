import argparse
from dotenv import load_dotenv

from common import *
from utils.position_predictor import *
from utils.ngram_predictor import NGramPredictor
from utils.embedding_predictor import EmbeddingPredictor

def main():
    parser = argparse.ArgumentParser(description='Benchmark various prediction models.')
    parser.add_argument("--cache-dir", "-cd", default="./cache",
                       help="Cache directory (default: ./cache)")
    parser.add_argument("--model-dir", "-md", default="./models",
                       help="Model directory (default: ./models)")
    parser.add_argument("--output-dir", "-od", default="./output",
                       help="Output directory for results (default: ./output)")
    parser.add_argument("--ignore-cache", action="store_true",
                       help="Ignore cache and re-download all data")
    parser.add_argument("--sample-platform", "-sp", type=str, default="manifold",
                       help="Sample markets from specific platform slug")
    parser.add_argument("--sample-size", "-ss", type=int,
                       help="Random sample size of markets to use")
    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Start timer
    start_time = time.time()

    # Create directories
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)

    # File names and paths
    markets_cache = f"{args.cache_dir}/markets.jsonl"
    embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"

    # Load markets
    print("Loading markets...")
    markets_df = load_dataframe_from_cache(markets_cache)
    if markets_df is None or args.ignore_cache:
        markets_df = get_data_as_dataframe(f"{postgrest_base}/markets", params={"order": "id"})
        save_dataframe_to_cache(markets_cache, markets_df)

    # Load market embeddings
    print("Loading market embeddings...")
    market_embeddings_df = load_dataframe_from_cache(embeddings_cache)
    if market_embeddings_df is None or args.ignore_cache:
        market_embeddings_df = get_data_as_dataframe(f"{postgrest_base}/market_embeddings", params={"order": "market_id"})
        # Parse embeddings from JSON strings
        market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)
        save_dataframe_to_cache(embeddings_cache, market_embeddings_df)
    else:
        # If loaded from cache, embeddings might already be lists or need parsing
        if isinstance(market_embeddings_df['embedding'].iloc[0], str):
            market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)

    # Create mapping for efficient lookup
    market_embeddings_mapped = dict(zip(market_embeddings_df['market_id'], market_embeddings_df['embedding']))

    # Sample down markets if requested
    if args.sample_platform:
        print(f"Filtering markets by platform: {args.sample_platform}")
        markets_df = markets_df[markets_df['platform_slug'] == args.sample_platform]
        print(f"Found {len(markets_df)} markets for platform {args.sample_platform}")

    if args.sample_size and len(markets_df) > args.sample_size:
        print(f"Randomly sampling {args.sample_size} markets from {len(markets_df)} total")
        markets_df = markets_df.sample(n=args.sample_size, random_state=42)
        print(f"Using {len(markets_df)} sampled markets")

    # Calculate market scores and some indicators
    markets_df['score'] = calculate_market_scores(markets_df)
    markets_df['high_score'] = markets_df['score'] > markets_df['score'].quantile(0.5)
    markets_df['high_volume'] = markets_df['volume_usd'] > markets_df['volume_usd'].quantile(0.5)
    markets_df['high_traders'] = markets_df['traders_count'] > markets_df['traders_count'].quantile(0.5)
    markets_df['high_duration'] = markets_df['duration_days'] > markets_df['duration_days'].quantile(0.5)
    markets_df['resolution_bool'] = markets_df['resolution'] == 1.0

    # Prepare predictors
    print("\nLoading predictors...")
    position_predictor = PositionPredictor(args.cache_dir)
    embedding_predictor = EmbeddingPredictor(f"{args.model_dir}/embeddings/")
    ngram_predictor = NGramPredictor(f"{args.model_dir}/ngram_counts.pkl")
    print(f"Everything loaded in {time.time() - start_time:.2f} seconds.")

    print("\nGenerating predictions...")
    for _, market in markets_df.iterrows():
        # Get market info
        market_id = market['id']
        market_title = market['title']
        market_slug = market['url'].split('/')[-1]
        title_and_description = market['title'] + " \n " + market['description']
        embeddings = market_embeddings_mapped.get(market_id, None)

        # Make predictions
        try:
            row = {}
            row["market"] = market.copy()
            row["charlie"] = position_predictor.predict_outcome(market_slug)
            row["sally"] = embedding_predictor.predict_all(title_and_description, embeddings=embeddings)
            row["linus"] = ngram_predictor.predict_resolution(market_title)

            with open(f"{args.output_dir}/predictions.jsonl", "a") as f:
                json.dump(row, f)
                f.write("\n")
            print(f"Predictions complete for market {market_id}")
        except Exception as e:
            print(f"Warning: Could not load positions predictor for market {market_id}: {e}")

if __name__ == "__main__":
    main()
