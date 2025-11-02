import argparse
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from slugify import slugify
from pathlib import Path

from common import *

def create_calibration_plot(df, output_path, title, bin_width=0.05):
    """
    Create a calibration plot showing predicted vs actual values.
    """
    # Create bins
    bins = np.arange(0, 1 + bin_width, bin_width)
    bin_centers = (bins[:-1] + bins[1:]) / 2

    # Digitize predictions into bins
    bin_indices = np.digitize(df['predicted'], bins) - 1

    # Calculate statistics for each bin
    bin_stats = []
    for i in range(len(bins) - 1):
        mask = bin_indices == i
        if mask.sum() > 0:
            bin_data = df[mask]
            mean_predicted = bin_data['predicted'].mean()
            mean_actual = bin_data['actual'].mean()
            count = len(bin_data)

            # Calculate confidence intervals using binomial distribution
            if count > 0:
                std_err = np.sqrt(mean_actual * (1 - mean_actual) / count)
                ci_lower = max(0, mean_actual - 1.96 * std_err)
                ci_upper = min(1, mean_actual + 1.96 * std_err)
            else:
                ci_lower = ci_upper = mean_actual

            bin_stats.append({
                'bin_center': bin_centers[i],
                'mean_predicted': mean_predicted,
                'mean_actual': mean_actual,
                'count': count,
                'ci_lower': ci_lower,
                'ci_upper': ci_upper
            })

    if not bin_stats:
        raise ValueError("No data points found in any bins")

    stats_df = pd.DataFrame(bin_stats)

    # Create the plot
    fig, ax = plt.subplots(1, 1, figsize=(10, 8))

    # Plot perfect calibration line
    ax.plot([0, 1], [0, 1], 'k--', alpha=0.5, label='Perfect Calibration', linewidth=2)

    # Plot calibration curve with error bars
    ax.errorbar(stats_df['mean_predicted'], stats_df['mean_actual'],
                yerr=[stats_df['mean_actual'] - stats_df['ci_lower'],
                      stats_df['ci_upper'] - stats_df['mean_actual']],
                fmt='o:', color='tab:blue', capsize=2, capthick=1,
                label='Observed Calibration', linewidth=1, markersize=8)

    # Add count annotations
    for _, row in stats_df.iterrows():
        ax.annotate(f"n={int(row['count'])}",
                   (row['mean_predicted'], row['mean_actual']),
                   xytext=(10, -10), textcoords='offset points',
                   fontsize=9, alpha=0.7)

    # Formatting
    ax.set_xlabel('Predicted Probability', fontsize=12)
    ax.set_ylabel('Actual Resolution', fontsize=12)
    ax.set_title(f'{title} (Bin Width: {bin_width:.1%})', fontsize=14, fontweight='bold')
    ax.set_xlim([0, 1])
    ax.set_ylim([0, 1])
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=10)

    # Add summary statistics
    total_samples = len(df)
    mean_predicted = df['predicted'].mean()
    mean_actual = df['actual'].mean()
    brier_score = ((df['predicted'] - df['actual']) ** 2).mean()
    weights = stats_df['count'] / total_samples
    calibration_error = (weights * np.abs(stats_df['mean_predicted'] - stats_df['mean_actual'])).sum()

    stats_text = f"""Summary Statistics:
Total Samples: {total_samples:,}
Mean Predicted: {mean_predicted:.3f}
Mean Actual: {mean_actual:.3f}
Brier Score: {brier_score:.3f}
ECE: {calibration_error:.3f}"""

    ax.text(0.02, 0.98, stats_text, transform=ax.transAxes, fontsize=10,
            verticalalignment='top', bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))

    # Save and return filename
    plt.tight_layout()
    filename = Path(f"{output_path}/{slugify(title)}.png")
    fig.savefig(filename, dpi=300, bbox_inches='tight')
    plt.close(fig)
    print(f"Plot saved to {filename}")
    return filename

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

    # Charlie analysis
    charlie = pd.DataFrame([
        {
            "predicted": i["charlie"]["predicted_outcome"],
            "actual": i["market"]["resolution"],
            "confidence": 1 - i["charlie"]["uncertainty"],
            "high_volume": i["market"]["high_volume"],
        } for i in data
    ])
    create_calibration_plot(charlie, args.output_dir, "Charlie Calibration")
    charlie_confident = charlie[charlie["confidence"] > 0.75]
    create_calibration_plot(charlie_confident, args.output_dir, "Charlie Calibration (Confident)")
    charlie_important = charlie[charlie["high_volume"]]
    create_calibration_plot(charlie_important, args.output_dir, "Charlie Calibration (Important)")

    # Sally analysis
    sally = pd.DataFrame([
        {
            "predicted": i["sally"]["resolution"],
            "actual": i["market"]["resolution"],
            "high_volume": i["market"]["high_volume"],
        } for i in data if i["sally"].get("resolution", None) is not None
    ])
    create_calibration_plot(sally, args.output_dir, "Sally Calibration")
    sally_important = sally[sally["high_volume"]]
    create_calibration_plot(sally_important, args.output_dir, "Sally Calibration (Important)")
    sally_volume = pd.DataFrame([
        {
            "predicted": i["sally"]["high_volume"],
            "actual": 1 if i["market"]["high_volume"] else 0,
        } for i in data if i["sally"].get("high_volume", None) is not None
    ])
    create_calibration_plot(sally_volume, args.output_dir, "Sally Volume")
    sally_traders = pd.DataFrame([
        {
            "predicted": i["sally"]["high_traders"],
            "actual": 1 if i["market"]["high_traders"] else 0,
        } for i in data if i["sally"].get("high_traders", None) is not None
    ])
    create_calibration_plot(sally_traders, args.output_dir, "Sally Traders")

    # Linus analysis
    linus = pd.DataFrame([
        {
            "predicted": i["linus"]["prob_resolution_1"],
            "actual": i["market"]["resolution"],
            "confidence": i["linus"]["confidence"],
            "high_volume": i["market"]["high_volume"],
        } for i in data if i["linus"].get("prob_resolution_1", None) is not None
    ])
    create_calibration_plot(linus, args.output_dir, "Linus Calibration")
    linus_confident = linus[linus["confidence"] > 0.75]
    create_calibration_plot(linus_confident, args.output_dir, "Linus Calibration (Confident)")
    linus_important = linus[linus["high_volume"]]
    create_calibration_plot(linus_important, args.output_dir, "Linus Calibration (Important)")

    return 0

if __name__ == '__main__':
    exit(main())
