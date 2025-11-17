import os
import sys
import json
import time
import requests
import argparse
from tqdm import tqdm
from pathlib import Path

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from utils.position_predictor import PositionPredictor
from utils.ngram_predictor import NGramPredictor
from utils.embedding_predictor import EmbeddingPredictor


def main():
    parser = argparse.ArgumentParser(
        description="Run various prediction models on live markets."
    )
    parser.add_argument(
        "--cache-dir",
        "-cd",
        default="../cache",
        help="Cache directory (default: ./cache)",
    )
    parser.add_argument(
        "--model-dir",
        "-md",
        default="../models",
        help="Model directory (default: ../models)",
    )
    parser.add_argument(
        "--output-dir",
        "-od",
        default="./output",
        help="Output directory for results (default: ../output)",
    )
    args = parser.parse_args()

    # Create directories
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)
    results_file = f"{args.output_dir}/predictions-{int(time.time())}.jsonl"
    Path(results_file).touch()

    # Prepare predictors
    print("\nLoading predictors...")
    position_predictor = PositionPredictor(args.cache_dir)
    embedding_predictor = EmbeddingPredictor(f"{args.model_dir}/embeddings/")
    ngram_predictor = NGramPredictor(f"{args.model_dir}/ngram_counts.pkl")

    # Start getting markets
    last_market_id = None
    while True:
        # Get market batch with retry
        try:
            response = requests.get(
                "https://api.manifold.markets/v0/markets",
                params={"limit": 1000, "before": last_market_id},
            )
            response.raise_for_status()
            markets = response.json()
        except Exception as e:
            print(f"Failed to get market batch: {e}")
            try:
                print("Retrying in 3 seconds...")
                time.sleep(3)
                response = requests.get(
                    "https://api.manifold.markets/v0/markets",
                    params={"limit": 1000, "before": last_market_id},
                )
                response.raise_for_status()
                markets = response.json()
            except Exception as retry_e:
                print(f"Retry failed: {retry_e}. Exiting...")
                break

        last_market_id = markets[-1]["id"]

        for market_lite in tqdm(markets):
            # Get extended market details with retry
            try:
                response = requests.get(
                    f"https://api.manifold.markets/v0/market/{market_lite['id']}",
                    timeout=10
                )
                response.raise_for_status()
                market = response.json()
            except Exception as e:
                print(f"Failed to get market {market_lite['id']}: {e}")
                try:
                    print(f"Retrying market {market_lite['id']} in 2 seconds...")
                    time.sleep(2)
                    response = requests.get(
                        f"https://api.manifold.markets/v0/market/{market_lite['id']}",
                        timeout=30
                    )
                    response.raise_for_status()
                    market = response.json()
                except Exception as retry_e:
                    print(f"Retry failed for market {market_lite['id']}: {retry_e}. Skipping...")
                    continue

            # Get market info
            market_id = market["id"]
            market_title = market["question"]
            market_slug = market["url"].split("/")[-1]
            title_and_description = market["question"] + " \n " + market["textDescription"]

            if market.get("outcomeType") != "BINARY":
                continue
            if market.get("isResolved", True) is True:
                continue

            # Make predictions
            try:
                row = {}
                row["market"] = market
                row["charlie"] = position_predictor.predict_outcome(
                    market_slug
                ).__dict__
                row["sally"] = embedding_predictor.predict_all(title_and_description)
                row["linus"] = ngram_predictor.predict_resolution(market_title)
            except Exception as e:
                print(f"Warning: Could not predict market {market_id}: {e}")

            # Save to disk
            try:
                with open(results_file, "a") as f:
                    json.dump(row, f)
                    f.write("\n")
            except OSError as e:
                print(f"Warning: Failed to append row to cache file ({e}).")

if __name__ == "__main__":
    main()
