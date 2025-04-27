# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "matplotlib",
#     "requests",
#     "numpy",
#     "tabulate",
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
from tabulate import tabulate


def fetch_scores(
    postgrest_url,
    score_type=None,
    linked_only=False,
    min_traders=None,
    min_volume=None,
    min_duration=None,
    batch_size=100_000,
):
    """Fetch scores from the API using pagination.

    Args:
        postgrest_url: The base URL for the PostgREST API
        score_type: Optional filter for score_type using ILIKE pattern
        linked_only: If True, only fetch scores where question_id is not null
        min_traders: Minimum number of traders
        min_volume: Minimum volume in USD
        min_duration: Minimum market duration in days
        batch_size: Number of records to fetch per request
    """
    try:
        all_market_scores = []
        offset = 0
        total_fetched = 0

        while True:
            url = f"{postgrest_url}/market_scores_details"
            params = []

            # Sort scores to keep stable ordering and avoid duplicates
            params.append("order=market_id,score_type")

            # Add pagination parameters
            params.append(f"limit={batch_size}")
            params.append(f"offset={offset}")

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
                params.append(
                    f"or=(traders_count.gte.{min_traders},volume_usd.gte.{min_volume})"
                )

            # Add min_duration filter if provided
            if min_duration is not None:
                params.append(f"duration_days=gte.{min_duration}")

            # Add parameters to URL
            query_url = f"{url}?{'&'.join(params)}"

            response = requests.get(query_url)
            response.raise_for_status()  # Raise exception for HTTP errors

            try:
                batch_scores = response.json()

                # Check if the response has the expected format
                if not isinstance(batch_scores, list):
                    print(
                        f"API returned unexpected data format: {type(batch_scores).__name__}"
                    )
                    print(f"Response content (truncated): {str(batch_scores)[:200]}...")
                    sys.exit(1)

                # If no more records, break the loop
                if not batch_scores:
                    break

                all_market_scores.extend(batch_scores)
                total_fetched += len(batch_scores)
                print(f"Fetched {len(batch_scores)} scores (total: {total_fetched})")

                # If we got fewer records than the batch size, we've reached the end
                if len(batch_scores) < batch_size:
                    break

                # Increment offset for next batch
                offset += batch_size

            except ValueError:
                print("Error parsing API response as JSON. Raw response:")
                print(
                    f"{response.text[:500]}..."
                    if len(response.text) > 500
                    else response.text
                )
                sys.exit(1)

        # Check if we found any scores
        if not all_market_scores:
            print(
                f"No scores found with the current filters (score_type: {score_type}, linked_only: {linked_only})"
            )
            print(
                "Try different filter options or check that the API is returning data"
            )
            sys.exit(1)

        print(f"Successfully fetched {len(all_market_scores)} total scores")
        return all_market_scores

    except requests.exceptions.RequestException as e:
        print(f"Error fetching scores: {e}")
        sys.exit(1)


def group_scores_by_type(scores):
    """Group scores by score_type and filter out invalid scores."""
    scores_by_type = defaultdict(list)
    for s in scores:
        if s["score"] is not None and np.isfinite(float(s["score"])):
            scores_by_type[s["score_type"]].append(float(s["score"]))
        else:
            print("Invalid score:", s)
    return scores_by_type


def custom_grade_sort_key(grade):
    """Custom sorting function for grades where S comes first and X+ comes before X."""
    # Special case for 'S' grade - should come first
    if grade == "S":
        return (0, "")  # Tuple for sorting priority: (position, secondary sort)

    # Handle grades with + suffix (should come before the base grade)
    if grade.endswith("+"):
        base_grade = grade[:-1]
        return (
            1,
            base_grade,
            0,
        )  # Position 1, then by base grade, then suffix priority 0 for +

    # Regular grades
    return (
        1,
        grade,
        1,
    )  # Position 1, then by grade name, then suffix priority 1 (after +)


def group_grades_by_type(scores):
    """Group grade values by score_type."""
    grades_by_type = defaultdict(lambda: defaultdict(int))
    for s in scores:
        # Check if the score has a valid grade value
        if "grade" in s and s["grade"] is not None:
            # Increment the count for this grade value in the appropriate score_type
            grades_by_type[s["score_type"]][s["grade"]] += 1
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
        n, bins, patches = plt.hist(
            clipped_scores, bins=50, alpha=0.7, color="skyblue", edgecolor="black"
        )

        # Calculate percentiles (every 10th percentile)
        percentiles = np.percentile(clipped_scores, np.arange(0, 101, 10))
        colors = plt.cm.viridis(np.linspace(0, 1, len(percentiles)))

        # Add percentile lines
        legend_entries = []
        for j, (percentile_value, color) in enumerate(zip(percentiles, colors)):
            percentile_line = plt.axvline(
                percentile_value, color=color, linestyle="solid", linewidth=1
            )
            legend_entries.append(
                (percentile_line, f"{j * 10}th: {percentile_value:.5f}")
            )

        # Set title and labels
        plt.title(f"Distribution of {score_type} Scores", fontsize=12)
        plt.xlabel("Score", fontsize=10)
        plt.ylabel("Frequency", fontsize=10)
        plt.grid(axis="y", alpha=0.75)

        # Add legend with percentiles
        plt.legend(
            handles=[line for line, _ in legend_entries],
            labels=[label for _, label in legend_entries],
            loc="upper right",
            fontsize="x-small",
            ncol=2,
        )

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
            plt.title(f"No Grades for {score_type}", fontsize=12)
            continue

        # Sort grade values with custom sorting (S first, X+ before X)
        grades = sorted(grade_counts.keys(), key=custom_grade_sort_key)
        counts = [grade_counts[grade] for grade in grades]

        # Create bar chart with pleasant colors
        bars = plt.bar(
            grades,
            counts,
            alpha=0.7,
            color=plt.cm.Paired(np.linspace(0, 1, len(grades))),
        )

        # Add count labels on top of bars
        for bar, count in zip(bars, counts):
            plt.text(
                bar.get_x() + bar.get_width() / 2.0,
                bar.get_height() + 0.1,
                str(count),
                ha="center",
                va="bottom",
                fontsize=8,
            )

        # Set title and labels
        plt.title(f"Grade Distribution for {score_type}", fontsize=12)
        plt.xlabel("Grade", fontsize=10)
        plt.ylabel("Frequency", fontsize=10)
        plt.xticks(rotation=45)
        plt.grid(axis="y", alpha=0.3)

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

        if matplotlib.get_backend().lower() in [
            "agg",
            "cairo",
            "pdf",
            "ps",
            "svg",
            "template",
        ]:
            print(
                "Warning: Running in a non-interactive environment. Use --output to save the plot instead."
            )
            # Save to a default location as fallback
            default_path = "cache/all_score_types_histogram.png"
            os.makedirs(os.path.dirname(default_path), exist_ok=True)
            fig.savefig(default_path)
            print(f"Plot saved to {default_path}")
        else:
            plt.show()


def calculate_tier_percentiles(tier_names, multiplier=2):
    """
    Calculate percentile cutoffs for tiers where each lower tier has
    multiplier times as many items as the tier above it.

    Args:
        tier_names: A list of tier names (highest to lowest)
        multiplier: How many times larger each subsequent tier should be

    Returns:
        A dictionary mapping tier names to their percentile cutoffs (upper bounds)
    """
    num_tiers = len(tier_names)

    # Calculate the first tier's percentage based on the geometric series
    # If x is the first tier's percentage, then:
    # x * (1 + multiplier + multiplier^2 + ... + multiplier^(n-1)) = 100%
    # This is a geometric series with sum: x * (multiplier^n - 1)/(multiplier - 1) = 100
    first_tier_percentage = 100 * (multiplier - 1) / (multiplier**num_tiers - 1)

    # Calculate percentages for each tier
    tier_percentages = [
        first_tier_percentage * (multiplier**i) for i in range(num_tiers)
    ]

    # Calculate cumulative percentages (these are the cutoffs)
    cumulative_percentages = []
    current_sum = 0
    for percentage in tier_percentages:
        current_sum += percentage
        cumulative_percentages.append(min(current_sum, 100))  # Cap at 100%

    # Create the result dictionary with tier names and their percentile cutoffs
    result = {}
    for i, tier_name in enumerate(tier_names):
        result[tier_name] = cumulative_percentages[i]

    return result


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Generate histograms of scores from Themis."
    )
    parser.add_argument(
        "--output",
        "-o",
        type=str,
        help="Output file path. If not provided, displays plot interactively.",
    )
    parser.add_argument(
        "--clip-min",
        type=float,
        default=-10,
        help="Minimum value for score clipping (default: -10)",
    )
    parser.add_argument(
        "--clip-max",
        type=float,
        default=10,
        help="Maximum value for score clipping (default: 10)",
    )
    parser.add_argument(
        "--score-type",
        type=str,
        help="Filter scores by score_type (case insensitive, partial match)",
    )
    parser.add_argument(
        "--linked-only",
        action="store_true",
        help="Only include scores that are linked to questions",
    )
    parser.add_argument(
        "--min-traders",
        type=int,
        help="Minimum number of traders (if min-volume not provided, uses min-traders * 10 for volume)",
    )
    parser.add_argument(
        "--min-volume",
        type=float,
        help="Minimum volume in USD (if min-traders not provided, uses min-volume / 10 for traders)",
    )
    parser.add_argument(
        "--min-duration", type=int, help="Minimum market duration in days"
    )
    # No longer needed as we now plot both score histograms and grade bar charts
    return parser.parse_args()


def main():
    """Main function to run the script."""
    args = parse_arguments()

    # Load environment variables
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")
    if not postgrest_url:
        print("Error: PGRST_URL not found in environment variables")
        sys.exit(1)

    # Fetch scores from API (only need to do this once)
    scores = fetch_scores(
        postgrest_url,
        args.score_type,
        args.linked_only,
        args.min_traders,
        args.min_volume,
        args.min_duration,
    )

    # Build common filename components based on filters applied
    filename_parts = []
    if args.score_type:
        filename_parts.append(args.score_type)
    if args.linked_only:
        filename_parts.append("linked")
    if args.min_traders:
        filename_parts.append(f"min{args.min_traders}traders")
    if args.min_volume:
        filename_parts.append(f"min{int(args.min_volume)}vol")
    if args.min_duration:
        filename_parts.append(f"min{args.min_duration}days")

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

    default_dir = "cache"
    os.makedirs(default_dir, exist_ok=True)
    filter_str = "_".join(filename_parts) if filename_parts else "all"

    # 1. Process and plot score histograms
    scores_by_type = group_scores_by_type(scores)
    if scores_by_type:
        # Create histograms with percentiles
        fig_scores = plot_score_histograms(
            scores_by_type, clip_range=(args.clip_min, args.clip_max)
        )

        # Add filter information to the figure
        if filter_text:
            fig_scores.text(
                0.01,
                0.01,
                "Filters: " + ", ".join(filter_text),
                fontsize=8,
                ha="left",
                va="bottom",
            )

        # Determine output path for scores plot
        if args.output:
            # If explicit output provided, add a suffix for scores
            output_base = os.path.splitext(args.output)
            scores_output = f"{output_base[0]}_scores{output_base[1]}"
        else:
            # Use default path with filter string
            scores_output = f"{default_dir}/{filter_str}_scores_plot.png"

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
            fig_grades.text(
                0.01,
                0.01,
                "Filters: " + ", ".join(local_filter_text),
                fontsize=8,
                ha="left",
                va="bottom",
            )

        # Determine output path for grades plot
        if args.output:
            # If explicit output provided, add a suffix for grades
            output_base = os.path.splitext(args.output)
            grades_output = f"{output_base[0]}_grades{output_base[1]}"
        else:
            # Use default path with filter string
            grades_output = f"{default_dir}/{filter_str}_grades_plot.png"

        save_or_show_plot(fig_grades, grades_output)
        plt.close(fig_grades)  # Close figure to free memory
    else:
        print("No valid grade data found to plot bar charts.")

    # 3. Show suggested grade cutoffs and score thresholds for each score type
    grades = [
        "S",
        "A+",
        "A",
        "A-",
        "B+",
        "B",
        "B-",
        "C+",
        "C",
        "C-",
        "D+",
        "D",
        "D-",
        "F",
    ]
    grade_cutoffs = calculate_tier_percentiles(grades, multiplier=1.25)

    # Calculate score thresholds for each score type based on percentiles
    score_thresholds = {}
    for score_type, scores_list in scores_by_type.items():
        if scores_list:  # Skip if no scores for this type
            # For each grade, calculate the actual score value at its percentile cutoff
            thresholds = {}
            for grade, percentile in grade_cutoffs.items():
                # For Brier scores, lower is better, so we flip the percentile calculation
                if score_type.lower().startswith("brier"):
                    thresholds[grade] = np.percentile(scores_list, percentile)
                else:
                    # For other score types, higher is better
                    thresholds[grade] = np.percentile(scores_list, 100 - percentile)
            score_thresholds[score_type] = thresholds

    # Create table header with basic columns plus one column per score type
    headers = ["Tier", "Percentile Start", "Percentile End", "Percentile Width"]
    for score_type in sorted(score_thresholds.keys()):
        headers.append(f"{score_type}")

    # Build table data with percentiles and score thresholds
    table_data = []
    prev_cutoff = 0
    for grade in grades:
        # Start with basic percentile data
        cutoff = grade_cutoffs[grade]
        range_width = cutoff - prev_cutoff
        row = [grade, f"{prev_cutoff:.2f}%", f"{cutoff:.2f}%", f"{range_width:.2f}%"]

        # Add threshold value for each score type
        for score_type in sorted(score_thresholds.keys()):
            if grade in score_thresholds[score_type]:
                row.append(f"{score_thresholds[score_type][grade]:.5f}")
            else:
                row.append("N/A")

        table_data.append(row)
        prev_cutoff = cutoff

    # Print the table with all details
    print("\nSuggested Grade Cutoffs and Score Thresholds:")
    print(tabulate(table_data, headers=headers, tablefmt="github"))

    # Save the table to a file in CSV format
    if filename_parts:
        cutoffs_file = f"{default_dir}/{filter_str}_grade_cutoffs.csv"
        with open(cutoffs_file, "w") as f:
            f.write(",".join(headers) + "\n")
            for row in table_data:
                f.write(",".join(str(item).rstrip("%") for item in row) + "\n")
        print(f"\nGrade cutoffs saved to {cutoffs_file}")


if __name__ == "__main__":
    main()
