# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "numpy",
#     "tqdm",
# ]
# ///

import json
from collections import defaultdict
import numpy as np
from tqdm import tqdm


def get_prediction(market, criterion):
    return market["probs"].get(criterion)


def get_brier_score(market, criterion):
    prediction = get_prediction(market, criterion)
    if prediction is None:
        return None
    resolution = market["resolution"]
    return (resolution - prediction) ** 2


def main():
    print("Opening caches...")
    with open("cache/migration/market_details.json", "r") as f:
        markets = json.load(f)
    with open("cache/migration/criterion_probabilities.json", "r") as f:
        criterion_probabilities = json.load(f)

    print("Collecting probabilities & scores...")
    prob_index = defaultdict(list)
    for cp in criterion_probabilities:
        prob_index[cp["market_id"]].append((cp["criterion_type"], cp["prob"]))
    for market in markets:
        market["probs"] = dict(prob_index[market["id"]])

    basic_cali_chart = defaultdict(int)
    for market in markets:
        prediction_val = get_prediction(market, "before-close-days-30")
        if prediction_val is None:
            continue
        if prediction_val < 0.3:
            prediction_bin = "Pred NO"
        elif prediction_val < 0.7:
            prediction_bin = "Pred MAYBE"
        else:
            prediction_bin = "Pred YES"
        resolution_val = market["resolution"]
        if resolution_val == 0:
            resolution_bin = "Res NO"
        elif resolution_val == 1:
            resolution_bin = "Res YES"
        else:
            continue
        basic_cali_chart[(prediction_bin, resolution_bin)] += 1
    print(basic_cali_chart)

    # Get average brier scores for markets for each platform and whether or not question_id is set
    brier_scores_by_group = defaultdict(list)
    print("\nCalculating Brier scores for platform/linked combinations")
    for market in markets:
        brier_score = get_brier_score(market, "midpoint")
        if brier_score is None:
            continue

        platform = market["platform_name"]
        question_id_status = (
            "Linked" if market.get("question_id") is not None else "Unlinked"
        )

        # Group by platform and question_id status
        group_key = (platform, question_id_status)
        brier_scores_by_group[group_key].append(brier_score)

    # Calculate and display average Brier scores
    print("\nAverage Midpoint Brier Scores by Platform and Question Linked Status:")
    print("=" * 60)
    for (platform, question_id_status), scores in brier_scores_by_group.items():
        if scores:
            avg_brier = np.mean(scores)
            count = len(scores)
            std_brier = np.std(scores)
            print(
                f"Platform: {platform:15} | Linked: {question_id_status:15} | "
                f"Count: {count:6} | Avg Brier: {avg_brier:.4f} | StDev: {std_brier:.4f}"
            )

    # Summary statistics
    print("\nSummary by Platform:")
    print("-" * 30)
    platform_stats = defaultdict(list)
    for (platform, question_id_status), scores in brier_scores_by_group.items():
        platform_stats[platform].extend(scores)

    for platform, all_scores in platform_stats.items():
        if all_scores:
            avg_brier = np.mean(all_scores)
            count = len(all_scores)
            print(f"{platform:15}: {count:4} markets, Avg Brier: {avg_brier:.4f}")

    print("\nSummary by Question ID Status:")
    print("-" * 35)
    qid_stats = defaultdict(list)
    for (platform, question_id_status), scores in brier_scores_by_group.items():
        qid_stats[question_id_status].extend(scores)

    for question_id_status, all_scores in qid_stats.items():
        if all_scores:
            avg_brier = np.mean(all_scores)
            count = len(all_scores)
            print(
                f"{question_id_status:15}: {count:4} markets, Avg Brier: {avg_brier:.4f}"
            )


if __name__ == "__main__":
    main()
