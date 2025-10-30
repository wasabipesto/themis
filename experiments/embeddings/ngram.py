import json
from collections import Counter
from nltk import ngrams, word_tokenize
from nltk.corpus import stopwords
from tqdm import tqdm

# Download stopwords if not already present
import nltk
nltk.download('punkt')
nltk.download('punkt_tab')
nltk.download('stopwords')

# --- Parameters ---
input_file = "cache/markets.jsonl"
max_n = 3          # unigrams, bigrams, trigrams
min_freq = 5       # ignore extremely rare n-grams
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
            if platform not in ["manifold"]:
                continue  # only look at manifold for now

            text = item.get("title", "").lower()
            tokens = [t for t in word_tokenize(text) if t.isalpha() and t not in stop_words]

            for n in range(1, max_n + 1):
                ngram_counts[res][n].update(ngrams(tokens, n))

        except json.JSONDecodeError:
            continue

# --- Print Top N-Grams by Resolution ---
for res in (0, 1):
    print(f"\n--- Top n-grams for resolution={res} ---")
    for n in range(1, max_n + 1):
        print(f"\nTop {n}-grams:")
        for gram, count in ngram_counts[res][n].most_common(50):
            opp = 1 - res
            opp_count = ngram_counts[opp][n].get(tuple(gram), 0)
            total_count = count + opp_count
            percent = count / total_count * 100
            print(f"{' '.join(gram):<30} {count}/{total_count} ({percent:.2f}%)")
