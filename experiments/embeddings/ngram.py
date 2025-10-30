import json
import pickle
from collections import Counter
from nltk import ngrams, word_tokenize
from nltk.corpus import stopwords
from tqdm import tqdm
import os

# Download stopwords if not already present
import nltk
nltk.download('punkt')
nltk.download('punkt_tab')
nltk.download('stopwords')

def load_and_process_data(input_file, max_n=3, min_freq=5, platforms=["manifold"]):
    """
    Load and process market data to generate n-grams by resolution.

    Args:
        input_file: Path to the JSONL file containing market data
        max_n: Maximum n-gram size (e.g., 3 for trigrams)
        min_freq: Minimum frequency threshold for n-grams
        platforms: List of platforms to include

    Returns:
        Dictionary of n-gram counts by resolution
    """
    stop_words = set(stopwords.words("english"))

    # Count total lines first for tqdm
    with open(input_file, "r", encoding="utf-8") as f:
        total_lines = sum(1 for _ in f)

    # Initialize counters for resolution=0 and resolution=1
    ngram_counts = {
        0: {n: Counter() for n in range(1, max_n + 1)},
        1: {n: Counter() for n in range(1, max_n + 1)}
    }

    with open(input_file, "r", encoding="utf-8") as f:
        for line in tqdm(f, total=total_lines, desc="Processing markets"):
            try:
                item = json.loads(line)
                platform = item.get("platform_slug")
                res = item.get("resolution")
                if res not in (0, 1):
                    continue  # skip if resolution is not 0 or 1
                if platform not in platforms:
                    continue  # only look at specified platforms

                text = item.get("title", "").lower()
                tokens = [t for t in word_tokenize(text) if t.isalpha() and t not in stop_words]

                for n in range(1, max_n + 1):
                    ngram_counts[res][n].update(ngrams(tokens, n))

            except json.JSONDecodeError:
                continue

    # Filter by minimum frequency
    filtered_counts = {
        0: {n: Counter() for n in range(1, max_n + 1)},
        1: {n: Counter() for n in range(1, max_n + 1)}
    }

    for res in (0, 1):
        for n in range(1, max_n + 1):
            for gram, count in ngram_counts[res][n].items():
                if count >= min_freq:
                    filtered_counts[res][n][gram] = count

    return filtered_counts

def print_top_ngrams(ngram_counts, top_k=50):
    """Print top n-grams by resolution for analysis."""
    for res in (0, 1):
        print(f"\n--- Top n-grams for resolution={res} ---")
        for n in range(1, len(ngram_counts[res]) + 1):
            print(f"\nTop {n}-grams:")
            for gram, count in ngram_counts[res][n].most_common(top_k):
                opp = 1 - res
                opp_count = ngram_counts[opp][n].get(tuple(gram), 0)
                total_count = count + opp_count
                percent = count / total_count * 100 if total_count > 0 else 0
                print(f"{' '.join(gram):<30} {count}/{total_count} ({percent:.2f}%)")

def save_ngrams(ngram_counts, output_dir="cache"):
    """
    Save n-gram counts to files for later use in prediction.

    Args:
        ngram_counts: Dictionary of n-gram counts by resolution
        output_dir: Directory to save the files
    """
    os.makedirs(output_dir, exist_ok=True)

    # Save raw counts
    with open(f"{output_dir}/ngram_counts.pkl", "wb") as f:
        pickle.dump(ngram_counts, f)

    # Also save as JSON for human readability
    json_data = {}
    for res in ngram_counts:
        json_data[res] = {}
        for n in ngram_counts[res]:
            json_data[res][n] = {
                ' '.join(gram): count
                for gram, count in ngram_counts[res][n].items()
            }

    with open(f"{output_dir}/ngram_counts.json", "w", encoding="utf-8") as f:
        json.dump(json_data, f, indent=2, ensure_ascii=False)

    print(f"N-gram data saved to {output_dir}/")

def main():
    """Main function to process market data and generate n-grams."""
    # Parameters
    input_file = "cache/markets.jsonl"
    max_n = 3          # unigrams, bigrams, trigrams
    min_freq = 5       # ignore extremely rare n-grams
    platforms = ["manifold"]  # platforms to analyze

    print("Loading and processing market data...")
    ngram_counts = load_and_process_data(
        input_file=input_file,
        max_n=max_n,
        min_freq=min_freq,
        platforms=platforms
    )

    print("Printing top n-grams for analysis...")
    print_top_ngrams(ngram_counts, top_k=50)

    print("\nSaving n-gram data...")
    save_ngrams(ngram_counts)

    print("Done!")

if __name__ == "__main__":
    main()
