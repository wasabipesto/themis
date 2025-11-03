import os
import sys
import argparse
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from slugify import slugify
from pathlib import Path

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from common import *

def main():
    """Main function to handle command line arguments and create the plot."""
    parser = argparse.ArgumentParser(description='Create calibration plot for predictions')
    parser.add_argument('--input', '-i', type=str, default='output/predictions.jsonl',
                       help='Path to input CSV file (default: output/predictions.jsonl)')
    parser.add_argument('--output-dir', '-o', type=str, default="output/calibration-plots",
                       help='Output file path for plot (default: output/calibration-plots)')
    args = parser.parse_args()
    os.makedirs(args.output_dir, exist_ok=True)

    # Load data
    with open(args.input) as f:
        data = [json.loads(line) for line in f]

    # Skim data and collect stats
    market_stats = []
    for m in data:
        if m["market"]["outcomeType"] != "BINARY":
            continue
        if m["market"].get("isResolved", False) is False:
            continue
        if m["market"].get("probability", None) is None:
            continue
        if m["sally"].get("resolution", None) is None:
            continue
        if m["linus"].get("prob_resolution_1", None) is None:
            continue

        stats = {
            "id": m["market"]["id"],
            "title": m["market"]["question"],
            "url": m["market"]["url"],
            "current_prob": m["market"]["probability"],
            "volume": m["market"]["volume"],
            "liquidity": m["market"]["totalLiquidity"],
        }
        stats["charlie_pred"] = m["charlie"]["predicted_outcome"]
        stats["charlie_pred_delta"] = abs(stats["charlie_pred"] - stats["current_prob"])
        stats["sally_pred"] = m["sally"]["resolution"]
        stats["sally_pred_delta"] = abs(stats["sally_pred"] - stats["current_prob"])
        stats["linus_pred"] = m["linus"]["prob_resolution_1"]
        stats["linus_pred_delta"] = abs(stats["linus_pred"] - stats["current_prob"])
        stats["mean_pred"] = (stats["charlie_pred"] + stats["sally_pred"] + stats["linus_pred"]) / 3.0
        stats["mean_pred_delta"] = abs(stats["mean_pred"] - stats["current_prob"])
        market_stats.append(stats)
    used = []

    # Show top results
    print("Furthest by Consensus:")
    market_stats.sort(key=lambda x: x["mean_pred_delta"], reverse=True)
    top = market_stats[:10]
    for stat in top:
        print(f"{stat['mean_pred_delta']*100:.2f}% delta ({stat['mean_pred']*100:.1f}% vs {stat['current_prob']*100:.1f}% actual)")
        print(f"       {stat['title']}")
        print(f"       {stat['url']}")
    used.append(top)

    print("\nFurthest by Charlie:")
    market_stats.sort(key=lambda x: x["charlie_pred_delta"], reverse=True)
    top = [i for i in market_stats if not i in used][:10]
    for stat in top:
        print(f"{stat['charlie_pred_delta']*100:.2f}% delta ({stat['charlie_pred']*100:.1f}% vs {stat['current_prob']*100:.1f}% actual), M${stat['volume']} volume")
        print(f"       {stat['title']}")
        print(f"       {stat['url']}")
    used.append(top)

    print("\nFurthest by Sally:")
    market_stats.sort(key=lambda x: x["sally_pred_delta"], reverse=True)
    top = [i for i in market_stats if not i in used][:10]
    for stat in top:
        print(f"{stat['sally_pred_delta']*100:.2f}% delta ({stat['sally_pred']*100:.1f}% vs {stat['current_prob']*100:.1f}% actual)")
        print(f"       {stat['title']}")
        print(f"       {stat['url']}")
    used.append(top)

    print("\nFurthest by Linus:")
    market_stats.sort(key=lambda x: x["linus_pred_delta"], reverse=True)
    top = [i for i in market_stats if not i in used][:10]
    for stat in top:
        print(f"{stat['linus_pred_delta']*100:.2f}% delta ({stat['linus_pred']*100:.1f}% vs {stat['current_prob']*100:.1f}% actual)")
        print(f"       {stat['title']}")
        print(f"       {stat['url']}")
    used.append(top)

if __name__ == '__main__':
    exit(main())
