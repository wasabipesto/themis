import json
import argparse
from datetime import datetime
import numpy as np
from tabulate import tabulate

def parse_datetime(dt_str):
    try:
        return datetime.strptime(dt_str, "%Y-%m-%dT%H:%M:%S.%fZ")
    except ValueError:
        return datetime.strptime(dt_str, "%Y-%m-%dT%H:%M:%SZ")

def collect_durations(file_path):
    durations = []

    with open(file_path, 'r') as file:
        for line in file:
            try:
                data = json.loads(line)
                history = list(reversed(data.get('history', [])))

                if len(history) < 2:
                    continue

                for i in range(len(history) - 1):
                    current_time = parse_datetime(history[i]['created_time'])
                    next_time = parse_datetime(history[i + 1]['created_time'])
                    duration = (next_time - current_time).total_seconds()
                    durations.append(duration)

            except (json.JSONDecodeError, KeyError):
                print("Could not decode JSON for item.")
                continue

    return durations

def generate_statistics(durations):
    if not durations:
        return []

    durations = np.array(durations)
    negative_count = np.sum(durations < 0)
    zero_count = np.sum(durations == 0)

    stats = []
    stats.append(['Total count', int(len(durations))])
    stats.append(['Negative duration count', int(negative_count)])
    stats.append(['Zero duration count', int(zero_count)])
    stats.append(['Minimum', durations.min()])

    # Calculate percentiles
    for p in range(5, 100, 5):
        stats.append([f'{p}th percentile', np.percentile(durations, p)])

    stats.append(['Maximum', durations.max()])

    return stats

def main():
    parser = argparse.ArgumentParser(description="Analyze Kalshi event duration distributions")
    parser.add_argument("file_path", help="Path to the JSONL file")
    args = parser.parse_args()

    durations = collect_durations(args.file_path)

    if not durations:
        print("No duration data found")
        return

    stats = generate_statistics(durations)

    # Format the statistics for display
    formatted_stats = [[label, f"{value:.2f}" if isinstance(value, float) else value]
                      for label, value in stats]

    print(tabulate(formatted_stats, headers=['Metric', 'Value'], tablefmt="github"))

if __name__ == "__main__":
    main()
