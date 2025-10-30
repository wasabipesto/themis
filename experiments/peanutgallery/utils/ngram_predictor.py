import pickle
import numpy as np
from nltk import ngrams, word_tokenize
from nltk.corpus import stopwords
import os

# Download stopwords if not already present
import nltk
nltk.download('punkt', quiet=True)
nltk.download('punkt_tab', quiet=True)
nltk.download('stopwords', quiet=True)

class NGramPredictor:
    def __init__(self, ngram_data_path="models/ngram_counts.pkl", exclude_stop_words=False):
        """
        Initialize the N-gram predictor.

        Args:
            ngram_data_path: Path to the pickled n-gram counts file
        """
        self.stop_words = set(stopwords.words("english")) if exclude_stop_words else set()
        self.ngram_counts = self.load_ngram_data(ngram_data_path)
        self.max_n = max(max(self.ngram_counts[res].keys()) for res in self.ngram_counts)

    def load_ngram_data(self, data_path):
        """Load the n-gram counts from the saved file."""
        if not os.path.exists(data_path):
            raise FileNotFoundError(f"N-gram data file not found: {data_path}")

        with open(data_path, "rb") as f:
            return pickle.load(f)

    def get_stats(self):
        """
        Get statistics about the loaded n-gram data.

        Returns:
            dict: Statistics including total ngrams, counts per resolution, etc.
        """
        stats = {
            'total_ngrams': 0,
            'resolutions': {},
            'ngram_sizes': {},
            'max_n': self.max_n,
            'exclude_stop_words': len(self.stop_words) > 0
        }

        # Initialize counters
        for res in self.ngram_counts:
            stats['resolutions'][res] = {'total': 0, 'by_size': {}}

        for n in range(1, self.max_n + 1):
            stats['ngram_sizes'][n] = {'total': 0, 'by_resolution': {}}
            for res in self.ngram_counts:
                stats['ngram_sizes'][n]['by_resolution'][res] = 0

        # Count ngrams
        for res in self.ngram_counts:
            for n in range(1, self.max_n + 1):
                if n in self.ngram_counts[res]:
                    count = len(self.ngram_counts[res][n])
                    stats['resolutions'][res]['total'] += count
                    stats['resolutions'][res]['by_size'][n] = count
                    stats['ngram_sizes'][n]['total'] += count
                    stats['ngram_sizes'][n]['by_resolution'][res] = count
                    stats['total_ngrams'] += count

        return stats

    def preprocess_title(self, title):
        """Preprocess a title to extract tokens."""
        text = title.lower()
        tokens = [t for t in word_tokenize(text) if t.isalpha() and t not in self.stop_words]
        return tokens

    def extract_ngrams(self, tokens):
        """Extract all n-grams from tokens."""
        all_ngrams = {}
        for n in range(1, self.max_n + 1):
            all_ngrams[n] = list(ngrams(tokens, n))
        return all_ngrams

    def calculate_ngram_scores(self, title_ngrams):
        """
        Calculate resolution scores based on n-gram prevalence.

        Returns:
            dict: Scores for each resolution and n-gram size
        """
        scores = {
            0: {n: [] for n in range(1, self.max_n + 1)},
            1: {n: [] for n in range(1, self.max_n + 1)}
        }

        ngram_evidence = {
            0: {n: [] for n in range(1, self.max_n + 1)},
            1: {n: [] for n in range(1, self.max_n + 1)}
        }

        for n in range(1, self.max_n + 1):
            for gram in title_ngrams[n]:
                # Get counts for this n-gram in both resolutions
                count_0 = self.ngram_counts[0][n].get(gram, 0)
                count_1 = self.ngram_counts[1][n].get(gram, 0)
                total_count = count_0 + count_1

                if total_count > 0:  # Only consider n-grams we've seen before
                    # Calculate the probability of each resolution given this n-gram
                    prob_0 = count_0 / total_count
                    prob_1 = count_1 / total_count

                    scores[0][n].append(prob_0)
                    scores[1][n].append(prob_1)

                    # Store evidence for analysis
                    ngram_evidence[0][n].append({
                        'ngram': ' '.join(gram),
                        'prob': prob_0,
                        'count': count_0,
                        'total': total_count
                    })
                    ngram_evidence[1][n].append({
                        'ngram': ' '.join(gram),
                        'prob': prob_1,
                        'count': count_1,
                        'total': total_count
                    })

        return scores, ngram_evidence

    def find_strongest_evidence(self, evidence, predicted_resolution):
        """
        Find the single strongest piece of evidence for the predicted resolution.
        Takes into account the weighting scheme used in predictions.
        """
        strongest = None
        max_weighted_score = 0

        # Use the same weighting scheme as in predict_resolution
        weights = {n: n**2 for n in range(1, self.max_n + 1)}

        for n in range(1, self.max_n + 1):
            for ev in evidence[predicted_resolution][n]:
                # Calculate weighted score: probability * weight
                weighted_score = ev['prob'] * weights[n]

                if weighted_score > max_weighted_score:
                    max_weighted_score = weighted_score
                    strongest = ev.copy()
                    strongest['ngram_size'] = n
                    strongest['weight'] = weights[n]
                    strongest['weighted_score'] = weighted_score

        return strongest

    def predict_resolution(self, title, verbose=False):
        """
        Predict the resolution of a given title.

        Args:
            title: The title to predict
            verbose: Whether to show detailed analysis

        Returns:
            dict: Prediction results including resolution, confidence, and uncertainty
        """
        tokens = self.preprocess_title(title)

        if not tokens:
            return {
                'prediction': None,
                'confidence': 0.0,
                'uncertainty': 1.0,
                'error': 'No valid tokens found in title'
            }

        title_ngrams = self.extract_ngrams(tokens)
        scores, evidence = self.calculate_ngram_scores(title_ngrams)

        # Calculate weighted average scores for each resolution
        final_scores = {0: 0.0, 1: 0.0}
        total_weight = 0
        evidence_count = 0

        # Weight higher n-grams more heavily (they're more specific)
        weights = {n: n**2 for n in range(1, self.max_n + 1)}

        for res in [0, 1]:
            weighted_sum = 0
            weight_sum = 0

            for n in range(1, self.max_n + 1):
                if scores[res][n]:  # If we have scores for this n-gram size
                    avg_score = np.mean(scores[res][n])
                    weighted_sum += avg_score * weights[n] * len(scores[res][n])
                    weight_sum += weights[n] * len(scores[res][n])
                    evidence_count += len(scores[res][n])

            if weight_sum > 0:
                final_scores[res] = weighted_sum / weight_sum

            total_weight += weight_sum

        # Normalize scores to probabilities
        total_score = final_scores[0] + final_scores[1]
        if total_score > 0:
            prob_0 = final_scores[0] / total_score
            prob_1 = final_scores[1] / total_score
        else:
            prob_0 = prob_1 = 0.5  # Default to 50/50 if no evidence

        # Determine prediction
        predicted_resolution = 1 if prob_1 > prob_0 else 0
        confidence = max(prob_0, prob_1)

        # Calculate uncertainty based on:
        # 1. How close the probabilities are (entropy-like measure)
        # 2. How much evidence we have
        prob_uncertainty = 1 - abs(prob_1 - prob_0)  # Higher when probabilities are close
        evidence_uncertainty = max(0, 1 - evidence_count / 10)  # Less uncertain with more evidence
        uncertainty = (prob_uncertainty + evidence_uncertainty) / 2

        # Find strongest evidence
        strongest_evidence = self.find_strongest_evidence(evidence, predicted_resolution)

        result = {
            'title': title,
            'tokens': tokens,
            'prediction': predicted_resolution,
            'prob_resolution_0': prob_0,
            'prob_resolution_1': prob_1,
            'confidence': confidence,
            'uncertainty': uncertainty,
            'evidence_count': evidence_count,
            'total_weight': total_weight,
            'strongest_evidence': strongest_evidence
        }

        if verbose:
            result['detailed_evidence'] = evidence
            result['ngram_breakdown'] = {
                f'{n}-grams': {
                    'resolution_0_avg': np.mean(scores[0][n]) if scores[0][n] else 0,
                    'resolution_1_avg': np.mean(scores[1][n]) if scores[1][n] else 0,
                    'count': len(scores[0][n]) + len(scores[1][n])
                }
                for n in range(1, self.max_n + 1)
            }

        return result

def main():
    import argparse
    parser = argparse.ArgumentParser(description='Predict market resolution based on title n-grams')
    parser.add_argument('title', nargs='?', help='Title to predict resolution for')
    parser.add_argument('--data-path', default='models/ngram_counts.pkl',
                       help='Path to n-gram data file')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Show detailed analysis')
    parser.add_argument('--stats', '-s', action='store_true',
                       help='Show statistics about loaded n-gram data')

    args = parser.parse_args()

    try:
        predictor = NGramPredictor(args.data_path)

        # Show stats if requested
        if args.stats:
            stats = predictor.get_stats()
            print("=== N-gram Data Statistics ===")
            print(f"Total N-grams: {stats['total_ngrams']:,}")
            print(f"Max N-gram size: {stats['max_n']}")
            print(f"Stop words excluded: {stats['exclude_stop_words']}")
            print()

            print("By Resolution:")
            for res in sorted(stats['resolutions'].keys()):
                res_stats = stats['resolutions'][res]
                print(f"  Resolution {res}: {res_stats['total']:,} n-grams")
                for n in sorted(res_stats['by_size'].keys()):
                    count = res_stats['by_size'][n]
                    print(f"    {n}-grams: {count:,}")
            print()

            print("By N-gram Size:")
            for n in sorted(stats['ngram_sizes'].keys()):
                size_stats = stats['ngram_sizes'][n]
                print(f"  {n}-grams: {size_stats['total']:,} total")
                for res in sorted(size_stats['by_resolution'].keys()):
                    count = size_stats['by_resolution'][res]
                    print(f"    Resolution {res}: {count:,}")
            print()

        # If no title provided and only stats requested, exit here
        if not args.title and args.stats:
            return

        # Require title for prediction
        if not args.title:
            parser.error("Title is required for prediction (or use --stats to see data statistics)")

        result = predictor.predict_resolution(args.title, verbose=args.verbose)

        if 'error' in result:
            print(f"Error: {result['error']}")
            return

        print(f"Title: {result['title']}")
        print(f"Tokens: {result['tokens']}")
        print(f"Predicted Resolution: {result['prediction']}")
        print(f"Probability Resolution=0: {result['prob_resolution_0']:.3f}")
        print(f"Probability Resolution=1: {result['prob_resolution_1']:.3f}")
        print(f"Confidence: {result['confidence']:.3f}")
        print(f"Uncertainty: {result['uncertainty']:.3f}")
        print(f"Evidence Count: {result['evidence_count']} n-grams")

        # Show strongest evidence in non-verbose mode
        if not args.verbose and result['strongest_evidence']:
            ev = result['strongest_evidence']
            print(f"Strongest Evidence: '{ev['ngram']}' ({ev['ngram_size']}-gram) - P={ev['prob']:.3f} ({ev['count']}/{ev['total']}) [weighted score: {ev['weighted_score']:.3f}]")

        if args.verbose and 'detailed_evidence' in result:
            print("\n--- Detailed Evidence ---")
            for res in [0, 1]:
                print(f"\nResolution {res} Evidence:")
                for n in range(1, predictor.max_n + 1):
                    if result['detailed_evidence'][res][n]:
                        print(f"  {n}-grams:")
                        # Sort by probability descending
                        sorted_evidence = sorted(result['detailed_evidence'][res][n],
                                               key=lambda x: x['prob'], reverse=True)
                        for ev in sorted_evidence[:10]:  # Show top 10
                            print(f"    {ev['ngram']:<20} P={ev['prob']:.3f} ({ev['count']}/{ev['total']})")

            print("\n--- N-gram Breakdown ---")
            for ngram_type, stats in result['ngram_breakdown'].items():
                print(f"{ngram_type}: Res0={stats['resolution_0_avg']:.3f}, "
                      f"Res1={stats['resolution_1_avg']:.3f}, Count={stats['count']}")

    except FileNotFoundError as e:
        print(f"Error: {e}")
        print("Make sure to run ngram.py first to generate the n-gram data.")
    except Exception as e:
        print(f"Unexpected error: {e}")

if __name__ == "__main__":
    main()
