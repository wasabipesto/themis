# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "matplotlib",
#     "requests",
#     "numpy",
# ]
# ///

import os
import sys
import argparse
import requests
import numpy as np
from dotenv import load_dotenv
import matplotlib.pyplot as plt
from collections import defaultdict


def fetch_scores(postgrest_url, score_type=None, linked_only=False, min_traders=None, min_volume=None, min_duration=None):
    """Fetch scores from the API.

    Args:
        postgrest_url: The base URL for the PostgREST API
        score_type: Optional filter for score_type using ILIKE pattern
        linked_only: If True, only fetch scores where question_id is not null
        min_traders: Minimum number of traders
        min_volume: Minimum volume in USD
        min_duration: Minimum market duration in days
    """
    try:
        url = f"{postgrest_url}/market_scores_details"
        params = []

        # Add score_type filter if provided
        if score_type:
            params.append(f"score_type=ilike.*{score_type}*")

        # Add linked_only filter if enabled
        if linked_only:
            params.append("question_id=not.is.null")

        # Handle min_traders and min_volume parameters
        if min_traders is not None or min_volume is not None:
            # If only one is provided, calculate the other based on the 1:10 ratio
            if min_traders is not None and min_volume is None:
                min_volume = min_traders * 10
            elif min_volume is not None and min_traders is None:
                min_traders = int(min_volume / 10)

            # Add OR filter for traders count or volume
            params.append(f"or=(traders_count.gte.{min_traders},volume_usd.gte.{min_volume})")

        # Add min_duration filter if provided
        if min_duration is not None:
            params.append(f"duration_days=gte.{min_duration}")

        # Add parameters to URL if any exist
        if params:
            url += "?" + "&".join(params)

        print(f"Fetching scores from: {url}")
        response = requests.get(url)
        response.raise_for_status()  # Raise exception for HTTP errors

        try:
            all_market_scores = response.json()

            # Check if the response is empty
            if not all_market_scores:
                print(f"No scores found with the current filters (score_type: {score_type}, linked_only: {linked_only})")
                print("Try different filter options or check that the API is returning data")
                sys.exit(1)

            # Check if the response is a list (as expected)
            if not isinstance(all_market_scores, list):
                print(f"API returned unexpected data format: {type(all_market_scores).__name__}")
                print(f"Response content (truncated): {str(all_market_scores)[:200]}...")
                sys.exit(1)

            print(f"Found {len(all_market_scores)} scores to analyze")
            return all_market_scores

        except ValueError:
            print(f"Error parsing API response as JSON. Raw response:")
            print(f"{response.text[:500]}..." if len(response.text) > 500 else response.text)
            sys.exit(1)

    except requests.exceptions.RequestException as e:
        print(f"Error fetching scores: {e}")
        sys.exit(1)


def group_scores_by_type(scores):
    """Group scores by score_type and filter out invalid scores."""
    scores_by_type = defaultdict(list)
    for s in scores:
        if not s['score'] is None and np.isfinite(float(s['score'])):
            scores_by_type[s['score_type']].append(float(s['score']))
        else:
            print("Invalid score:", s)
    return scores_by_type


def custom_grade_sort_key(grade):
    """Custom sorting function for grades where S comes first and X+ comes before X."""
    # Special case for 'S' grade - should come first
    if grade == 'S':
        return (0, '')  # Tuple for sorting priority: (position, secondary sort)

    # Handle grades with + suffix (should come before the base grade)
    if grade.endswith('+'):
        base_grade = grade[:-1]
        return (1, base_grade, 0)  # Position 1, then by base grade, then suffix priority 0 for +

    # Regular grades
    return (1, grade, 1)  # Position 1, then by grade name, then suffix priority 1 (after +)


def group_grades_by_type(scores):
    """Group grade values by score_type."""
    grades_by_type = defaultdict(lambda: defaultdict(int))
    for s in scores:
        # Check if the score has a valid grade value
        if 'grade' in s and s['grade'] is not None:
            # Increment the count for this grade value in the appropriate score_type
            grades_by_type[s['score_type']][s['grade']] += 1
    return grades_by_type


def plot_score_histograms(scores_by_type, clip_range=(-10, 10)):
    """Create a figure with histograms for each score type."""
    # Calculate grid dimensions
    num_score_types = len(scores_by_type)
    num_cols = np.clip(num_score_types, 1, 3)
    num_rows = int(np.ceil(num_score_types / num_cols))

    # Create the figure and subplots
    fig = plt.figure(figsize=(6 * num_cols, 4 * num_rows))

    # Sort score types alphabetically
    sorted_score_types = sorted(scores_by_type.keys())

    # Create a histogram for each score_type
    for i, score_type in enumerate(sorted_score_types):
        scores_list = scores_by_type[score_type]
        plt.subplot(num_rows, num_cols, i + 1)

        # Clip scores to specified range for the histogram
        min_clip, max_clip = clip_range
        clipped_scores = np.clip(scores_list, min_clip, max_clip)

        # Create histogram
        n, bins, patches = plt.hist(clipped_scores, bins=50, alpha=0.7, color='skyblue', edgecolor='black')

        # Calculate percentiles (every 10th percentile)
        percentiles = np.percentile(clipped_scores, np.arange(0, 101, 10))
        colors = plt.cm.viridis(np.linspace(0, 1, len(percentiles)))

        # Add percentile lines
        legend_entries = []
        for j, (percentile_value, color) in enumerate(zip(percentiles, colors)):
            percentile_line = plt.axvline(percentile_value, color=color, linestyle='solid', linewidth=1)
            legend_entries.append((percentile_line, f'{j*10}th: {percentile_value:.5f}'))

        # Set title and labels
        plt.title(f'Distribution of {score_type} Scores', fontsize=12)
        plt.xlabel('Score', fontsize=10)
        plt.ylabel('Frequency', fontsize=10)
        plt.grid(axis='y', alpha=0.75)

        # Add legend with percentiles
        plt.legend(handles=[line for line, _ in legend_entries],
                  labels=[label for _, label in legend_entries],
                  loc='upper right', fontsize='x-small', ncol=2)

    plt.tight_layout()
    return fig


def plot_grade_bar_charts(grades_by_type):
    """Create a figure with bar charts for grade frequencies by score type."""
    # Calculate grid dimensions
    num_score_types = len(grades_by_type)
    num_cols = np.clip(num_score_types, 1, 3)
    num_rows = int(np.ceil(num_score_types / num_cols))

    # Create the figure and subplots
    fig = plt.figure(figsize=(6 * num_cols, 4 * num_rows))

    # Sort score types alphabetically
    sorted_score_types = sorted(grades_by_type.keys())

    # Create a bar chart for each score_type
    for i, score_type in enumerate(sorted_score_types):
        grade_counts = grades_by_type[score_type]
        plt.subplot(num_rows, num_cols, i + 1)

        if not grade_counts:  # Skip if no grades for this score type
            plt.title(f'No Grades for {score_type}', fontsize=12)
            continue

        # Sort grade values with custom sorting (S first, X+ before X)
        grades = sorted(grade_counts.keys(), key=custom_grade_sort_key)
        counts = [grade_counts[grade] for grade in grades]

        # Create bar chart with pleasant colors
        bars = plt.bar(grades, counts, alpha=0.7, color=plt.cm.Paired(np.linspace(0, 1, len(grades))))

        # Add count labels on top of bars
        for bar, count in zip(bars, counts):
            plt.text(bar.get_x() + bar.get_width()/2., bar.get_height() + 0.1,
                    str(count), ha='center', va='bottom', fontsize=8)

        # Set title and labels
        plt.title(f'Grade Distribution for {score_type}', fontsize=12)
        plt.xlabel('Grade', fontsize=10)
        plt.ylabel('Frequency', fontsize=10)
        plt.xticks(rotation=45)
        plt.grid(axis='y', alpha=0.3)

        # Filter info will be added at the figure level in the main function

    plt.tight_layout()
    return fig


def save_or_show_plot(fig, output_path=None):
    """Save the figure to a file or show it interactively."""
    if output_path:
        # Make sure the directory exists
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        fig.savefig(output_path)
        print(f"Plot saved to {output_path}")
    else:
        # Check if we're in an interactive environment
        import matplotlib
        if matplotlib.get_backend().lower() in ['agg', 'cairo', 'pdf', 'ps', 'svg', 'template']:
            print("Warning: Running in a non-interactive environment. Use --output to save the plot instead.")
            # Save to a default location as fallback
            default_path = 'cache/all_score_types_histogram.png'
            os.makedirs(os.path.dirname(default_path), exist_ok=True)
            fig.savefig(default_path)
            print(f"Plot saved to {default_path}")
        else:
            plt.show()


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description='Generate histograms of scores from Themis.')
    parser.add_argument('--output', '-o', type=str, help='Output file path. If not provided, displays plot interactively.')
    parser.add_argument('--clip-min', type=float, default=-10, help='Minimum value for score clipping (default: -10)')
    parser.add_argument('--clip-max', type=float, default=10, help='Maximum value for score clipping (default: 10)')
    parser.add_argument('--score-type', type=str, help='Filter scores by score_type (case insensitive, partial match)')
    parser.add_argument('--linked-only', action='store_true', help='Only include scores that are linked to questions')
    parser.add_argument('--min-traders', type=int, help='Minimum number of traders (if min-volume not provided, uses min-traders * 10 for volume)')
    parser.add_argument('--min-volume', type=float, help='Minimum volume in USD (if min-traders not provided, uses min-volume / 10 for traders)')
    parser.add_argument('--min-duration', type=int, help='Minimum market duration in days')
    # No longer needed as we now plot both score histograms and grade bar charts
    return parser.parse_args()


def main():
    """Main function to run the script."""
    args = parse_arguments()

    # Load environment variables
    load_dotenv()
    postgrest_url = os.environ.get('PGRST_URL')
    if not postgrest_url:
        print("Error: PGRST_URL not found in environment variables")
        sys.exit(1)

    # Fetch scores from API (only need to do this once)
    scores = fetch_scores(postgrest_url, args.score_type, args.linked_only, args.min_traders, args.min_volume, args.min_duration)

    # Build common filename components based on filters applied
    filename_parts = []
    if args.score_type:
        filename_parts.append(args.score_type)
    if args.linked_only:
        filename_parts.append('linked')
    if args.min_traders:
        filename_parts.append(f'min{args.min_traders}traders')
    if args.min_volume:
        filename_parts.append(f'min{int(args.min_volume)}vol')
    if args.min_duration:
        filename_parts.append(f'min{args.min_duration}days')

    filter_text = []
    if args.score_type:
        filter_text.append(f"Score type: {args.score_type}")
    if args.linked_only:
        filter_text.append("Linked only")
    if args.min_traders:
        filter_text.append(f"Min traders: {args.min_traders}")
    if args.min_volume:
        filter_text.append(f"Min volume: ${args.min_volume}")
    if args.min_duration:
        filter_text.append(f"Min duration: {args.min_duration} days")

    default_dir = 'cache'
    os.makedirs(default_dir, exist_ok=True)
    filter_str = '_'.join(filename_parts) if filename_parts else 'all'

    # 1. Process and plot score histograms
    scores_by_type = group_scores_by_type(scores)
    if scores_by_type:
        # Create histograms with percentiles
        fig_scores = plot_score_histograms(scores_by_type, clip_range=(args.clip_min, args.clip_max))

        # Add filter information to the figure
        if filter_text:
            fig_scores.text(0.01, 0.01, "Filters: " + ", ".join(filter_text), fontsize=8, ha='left', va='bottom')

        # Determine output path for scores plot
        if args.output:
            # If explicit output provided, add a suffix for scores
            output_base = os.path.splitext(args.output)
            scores_output = f"{output_base[0]}_scores{output_base[1]}"
        else:
            # Use default path with filter string
            scores_output = f'{default_dir}/{filter_str}_scores_plot.png'

        save_or_show_plot(fig_scores, scores_output)
        plt.close(fig_scores)  # Close figure to free memory
    else:
        print("No valid scores found to plot histograms.")

    # 2. Process and plot grade bar charts
    grades_by_type = group_grades_by_type(scores)
    if grades_by_type:
        # Create grade frequency bar charts
        fig_grades = plot_grade_bar_charts(grades_by_type)

        # Add filter information to the figure
        local_filter_text = filter_text.copy()
        if local_filter_text:
            fig_grades.text(0.01, 0.01, "Filters: " + ", ".join(local_filter_text), fontsize=8, ha='left', va='bottom')

        # Determine output path for grades plot
        if args.output:
            # If explicit output provided, add a suffix for grades
            output_base = os.path.splitext(args.output)
            grades_output = f"{output_base[0]}_grades{output_base[1]}"
        else:
            # Use default path with filter string
            grades_output = f'{default_dir}/{filter_str}_grades_plot.png'

        save_or_show_plot(fig_grades, grades_output)
        plt.close(fig_grades)  # Close figure to free memory
    else:
        print("No valid grade data found to plot bar charts.")


if __name__ == "__main__":
    main()
