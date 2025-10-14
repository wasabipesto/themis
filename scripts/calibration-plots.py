# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "argparse",
#     "matplotlib",
#     "psycopg2-binary",
#     "numpy",
#     "python-dotenv",
# ]
# ///

import argparse
import os
import sys
from pathlib import Path
from datetime import datetime
import psycopg2
import matplotlib.pyplot as plt
import numpy as np
from dotenv import load_dotenv


def connect_to_database():
    """Connect to PostgreSQL database using environment variables"""
    # Get connection parameters
    host = os.getenv('POSTGRES_HOST', 'localhost')
    port = os.getenv('POSTGRES_PORT', 5432)
    database = os.getenv('POSTGRES_DB', 'themis')
    user = os.getenv('POSTGRES_USER', 'themis')
    password = os.getenv('POSTGRES_PASSWORD')

    # Validate required parameters
    if not password:
        print("Error: POSTGRES_PASSWORD environment variable is required")
        sys.exit(1)

    print(f"Connecting to database: {user}@{host}:{port}/{database}")

    try:
        conn = psycopg2.connect(
            host=host,
            port=port,
            database=database,
            user=user,
            password=password
        )
        print("Successfully connected to database")
        return conn
    except psycopg2.Error as e:
        print(f"Error connecting to database: {e}")
        print("Make sure PostgreSQL is running and environment variables are set correctly")
        sys.exit(1)

def fetch_market_data(conn):
    """Fetch market data with midpoint criterion probabilities"""
    query = """
    SELECT
        m.id,
        m.title,
        m.resolution,
        cp.prob as criterion_prob
    FROM markets m
    INNER JOIN criterion_probabilities cp ON m.id = cp.market_id
    WHERE cp.criterion_type = 'midpoint'
    AND m.resolution IS NOT NULL
    AND cp.prob IS NOT NULL;
    """

    try:
        with conn.cursor() as cur:
            cur.execute(query)
            results = cur.fetchall()
            return results
    except psycopg2.Error as e:
        print(f"Error executing query: {e}")
        sys.exit(1)

def bin_data(data, bin_size=0.05):
    """Bin market data by criterion probability"""
    bins = {}

    for market_id, title, resolution, criterion_prob in data:
        # Convert to float for calculations
        prob = float(criterion_prob)
        res = float(resolution)

        # Determine which bin this probability falls into
        bin_index = int(prob // bin_size)
        bin_start = bin_index * bin_size
        bin_end = bin_start + bin_size

        # Create bin key
        bin_key = f"{bin_start:.3f}-{bin_end:.3f}"

        if bin_key not in bins:
            bins[bin_key] = {
                'probs': [],
                'resolutions': [],
                'bin_center': bin_start + bin_size / 2,
                'count': 0
            }

        bins[bin_key]['probs'].append(prob)
        bins[bin_key]['resolutions'].append(res)
        bins[bin_key]['count'] += 1

    return bins

def calculate_averages(bins):
    """Calculate average resolution for each bin"""
    plot_data = []

    for bin_key, bin_data in sorted(bins.items()):
        if bin_data['count'] > 0:
            avg_resolution = np.mean(bin_data['resolutions'])
            bin_center = bin_data['bin_center']
            count = bin_data['count']

            plot_data.append({
                'bin_key': bin_key,
                'bin_center': bin_center,
                'avg_resolution': avg_resolution,
                'count': count
            })

    return plot_data

def calculate_slopes(plot_data):
    """Calculate the local slope at each data point."""

    for i, item in enumerate(plot_data):
        if i == 0 or i == len(plot_data) - 1:
            item["slope"] = None
            item["reciprocal"] = None
        else:
            x1, y1 = plot_data[i - 1]['bin_center'], plot_data[i - 1]['avg_resolution']
            x2, y2 = plot_data[i + 1]['bin_center'], plot_data[i + 1]['avg_resolution']
            slope = (y2 - y1) / (x2 - x1)
            item["slope"] = slope
            item["reciprocal"] = 0.01 / abs(slope)

    return plot_data

def create_calibration_plot(plot_data, output_path=None):
    """Create and save calibration plot"""
    if not plot_data:
        print("No data to plot")
        return

    # Determine bin width
    bin_width = plot_data[1]['bin_center'] - plot_data[0]['bin_center']

    # Extract data for plotting
    x_values = [item['bin_center'] for item in plot_data]
    y_values = [item['avg_resolution'] for item in plot_data]
    counts = [item['count'] for item in plot_data]
    slopes = [item['slope'] or 0.0 for item in plot_data]
    reciprocals = [item['reciprocal'] or 0.0 for item in plot_data]

    # Create the plot
    plt.figure(figsize=(10, 8))

    # Main calibration plot
    plt.scatter(x_values, y_values, s=[c**0.5 for c in counts], alpha=0.7, label='Actual Calibration')
    plt.errorbar(x_values, y_values, [r*0.5 for r in reciprocals], label='Slope Reciprocal')

    # Perfect calibration line
    plt.plot([0, 1], [0, 1], '-', alpha=0.5, linewidth=1, label='Perfect Calibration')

    # Formatting
    plt.xlabel('Criterion Probability (Midpoint)', fontsize=12)
    plt.ylabel('Average Market Resolution', fontsize=12)
    plt.title(f'Market Calibration Plot, bin width {(bin_width*100):.1f}%', fontsize=14)
    plt.grid(True, alpha=0.3)
    plt.legend()

    # Set axis limits
    plt.xlim(0, 1)
    plt.ylim(0, 1)

    # Add text annotations for bin counts
    #for item in plot_data:
    #    plt.annotate(f"n={item['count']}",
    #                (item['bin_center'], item['avg_resolution']),
    #                xytext=(5, 5), textcoords='offset points',
    #                fontsize=8, alpha=0.7)

    plt.tight_layout()

    # Save the plot
    if output_path:
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        print(f"Plot saved to {output_path}")
    else:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_path = f"calibration_plot_{timestamp}.png"
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        print(f"Plot saved to {output_path}")

    return output_path

def print_summary(data, plot_data):
    """Print summary statistics"""
    print(f"\nSummary:")
    print(f"Total markets: {len(data)}")
    print(f"Number of bins with data: {len(plot_data)}")
    print(f"\nBin breakdown:")

    for item in sorted(plot_data, key=lambda x: x['bin_center']):
        print(f"  {item['bin_key']}: {item['count']} markets, "
              f"avg resolution: {item['avg_resolution']:.3f}, "
              f"slope: {(item['slope'] or 0):.3f}, "
              f"reciprocal: {(item['reciprocal'] or 0):.3f}")

def main():
    parser = argparse.ArgumentParser(
        description="Generate calibration plot of prediction markets.\n"
                   "Connects to PostgreSQL database and creates a plot showing how well "
                   "market probabilities are calibrated against actual outcomes.\n\n"
                   "Examples:\n"
                   "  %(prog)s                    # Generate plot with default 5%% bins\n"
                   "  %(prog)s -b 0.1 -o plot.png # Use 10%% bins, save to plot.png\n"
                   "  %(prog)s --bin-size 0.02    # Use 2%% bins for finer resolution",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        help="Output file path for the plot (default: calibration_plot_YYYYMMDD_HHMMSS.png)"
    )
    parser.add_argument(
        "--bin-size", "-b",
        type=float,
        default=0.05,
        help="Bin size for probability grouping (default: 0.05 for 5%% bins). "
             "Smaller values create more bins with fewer markets each."
    )

    args = parser.parse_args()

    # Load environment variables
    load_dotenv()

    # Connect to database
    print("Connecting to database...")
    conn = connect_to_database()

    try:
        # Fetch data
        print("Fetching market data...")
        data = fetch_market_data(conn)

        if not data:
            print("No market data found with midpoint criterion probabilities")
            print("Make sure the database contains markets with criterion_type='midpoint'")
            return

        print(f"Found {len(data)} markets with midpoint probabilities")

        # Validate we have meaningful data
        valid_data = [(mid, title, res, prob) for mid, title, res, prob in data
                     if res is not None and prob is not None]
        if len(valid_data) < len(data):
            print(f"Warning: {len(data) - len(valid_data)} markets have null resolution or probability")
        data = valid_data

        # Bin the data
        print(f"Binning data with {args.bin_size*100}% bins...")
        bins = bin_data(data, bin_size=args.bin_size)

        # Calculate averages
        plot_data = calculate_averages(bins)

        # Calculate slopes
        plot_data_adv = calculate_slopes(plot_data)

        # Create plot
        print("Creating calibration plot...")
        output_path = create_calibration_plot(plot_data, args.output)

        # Print summary
        print_summary(data, plot_data)

        print(f"\nCalibration plot completed successfully!")
        print(f"Output saved to: {output_path}")

    finally:
        conn.close()

if __name__ == "__main__":
    main()
