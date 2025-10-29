"""
Prediction Calibration Plot Script

This script creates a calibration plot comparing predicted probabilities to actual outcomes.
It bins the predictions and shows the mean predicted probability vs the actual rate within each bin.
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

def create_calibration_plot(df, bin_width=0.1):
    """
    Create a calibration plot showing predicted vs actual values.

    Args:
        df (pd.DataFrame): DataFrame with 'predicted' and 'actual' columns
        bin_width (float): Width of bins as a fraction (e.g., 0.1 for 10%)

    Returns:
        tuple: (fig, ax) matplotlib figure and axis objects
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
    ax.set_title(f'Calibration Plot (Bin Width: {bin_width:.1%})', fontsize=14, fontweight='bold')
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

    plt.tight_layout()
    return fig, ax

def main():
    """Main function to handle command line arguments and create the plot."""
    parser = argparse.ArgumentParser(description='Create calibration plot for predictions')
    parser.add_argument('--input', '-i', type=str, default='output/latest-predictions.csv',
                       help='Path to input CSV file (default: output/latest-predictions.csv)')
    parser.add_argument('--output', '-o', type=str, default="output/calibration-plot.png",
                       help='Output file path for plot (default: output/calibration-plot.png)')
    parser.add_argument('--bin-width', '-b', type=float, default=0.1,
                       help='Bin width as a fraction (default: 0.1 for 10%%)')
    args = parser.parse_args()

    # Validate bin width
    if not 0 < args.bin_width <= 1:
        raise ValueError("Bin width must be between 0 and 1")

    # Load data
    try:
        input_path = Path(args.input)
        if not input_path.is_absolute():
            # Look for file relative to script location or current directory
            script_dir = Path(__file__).parent.parent
            possible_paths = [
                Path(args.input),  # Current directory
                script_dir / args.input,  # Relative to project root
            ]

            input_path = None
            for path in possible_paths:
                if path.exists():
                    input_path = path
                    break

            if input_path is None:
                raise FileNotFoundError(f"Could not find input file: {args.input}")

        print(f"Loading data from: {input_path}")
        df = pd.read_csv(input_path)

    except Exception as e:
        print(f"Error loading data: {e}")
        return 1

    # Validate required columns
    required_columns = ['predicted', 'actual']
    missing_columns = [col for col in required_columns if col not in df.columns]
    if missing_columns:
        print(f"Error: Missing required columns: {missing_columns}")
        print(f"Available columns: {list(df.columns)}")
        return 1

    # Validate and clamp data ranges
    pred_min, pred_max = df['predicted'].min(), df['predicted'].max()
    if not (pred_min >= 0 and pred_max <= 1):
        print(f"Clamping predicted values from range [{pred_min:.3f}, {pred_max:.3f}] to [0, 1]")
        df['predicted'] = df['predicted'].clip(0, 1)

    actual_min, actual_max = df['actual'].min(), df['actual'].max()
    if not (actual_min >= 0 and actual_max <= 1):
        print(f"Clamping actual values from range [{actual_min:.3f}, {actual_max:.3f}] to [0, 1]")
        df['actual'] = df['actual'].clip(0, 1)

    print(f"Loaded {len(df)} predictions")
    print(f"Predicted range: [{df['predicted'].min():.3f}, {df['predicted'].max():.3f}]")
    actual_unique = df['actual'].unique()
    print(f"Actual values: {sorted(actual_unique)}")

    # Create calibration plot
    try:
        fig, ax = create_calibration_plot(df, args.bin_width)

        # Save or show plot
        if args.output:
            output_path = Path(args.output)
            print(f"Saving plot to: {output_path}")
            fig.savefig(output_path, dpi=300, bbox_inches='tight')
            plt.close(fig)
        else:
            print("Showing plot...")
            plt.show()

    except Exception as e:
        print(f"Error creating plot: {e}")
        return 1

    return 0

if __name__ == '__main__':
    exit(main())
