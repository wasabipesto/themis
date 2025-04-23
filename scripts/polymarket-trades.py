# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "argparse",
#     "matplotlib",
#     "requests",
# ]
# ///

import requests
import argparse
import json
import statistics
from datetime import datetime
import matplotlib.pyplot as plt


def fetch_all_trades(market):
    all_items = []
    limit = 1000
    offset = 0

    while True:
        response = requests.get(
            "https://data-api.polymarket.com/trades",
            params={
                "market": market,
                "limit": limit,
                "offset": offset,
            },
        )
        if response.status_code != 200:
            raise Exception(f"Failed to fetch trades: {response.text}")
        batch = response.json()
        if not batch:
            break
        all_items.extend(batch)
        if len(batch) < limit:
            break
        offset += limit

    return all_items


def save_to_file(data, filename):
    with open(filename, "w") as f:
        json.dump(data, f, indent=2)


def load_from_file(filename):
    with open(filename, "r") as f:
        data = json.load(f)
    return data


def analyze_trades(trades):
    # Extract unique traders
    pseudonyms = {trade["pseudonym"] for trade in trades}
    print(f"Unique Traders Count: {len(pseudonyms)}")
    print(f"Empty Pseudonyms: {len([p for p in pseudonyms if p is None or p == ''])}")

    # Collect bet sizes
    bet_sizes = [trade["size"] for trade in trades]
    print(f"Min Bet Size: {min(bet_sizes)}")
    print(f"Median Bet Size: {max(bet_sizes)}")
    print(f"Max Bet Size: {statistics.median(bet_sizes)}")

    # Check outcomes
    outcome_issues_yes = [
        trade
        for trade in trades
        if trade["outcome"] == "Yes" and trade["outcomeIndex"] != 0
    ]
    print(f"Number of Outcome Issues (Yes): {len(outcome_issues_yes)}")
    outcome_issues_no = [
        trade
        for trade in trades
        if trade["outcome"] == "No" and trade["outcomeIndex"] != 1
    ]
    print(f"Number of Outcome Issues (No): {len(outcome_issues_no)}")

    # Find the earliest/latest timestamps
    timestamps = [trade["timestamp"] for trade in trades]
    earliest_timestamp = min(timestamps)
    earliest_datetime = datetime.fromtimestamp(earliest_timestamp).strftime(
        "%Y-%m-%d %H:%M:%S"
    )
    print(f"Earliest Timestamp (Datetime): {earliest_datetime}")
    latest_timestamp = max(timestamps)
    latest_datetime = datetime.fromtimestamp(latest_timestamp).strftime(
        "%Y-%m-%d %H:%M:%S"
    )
    print(f"Latest Timestamp (Datetime): {latest_datetime}")

    # Check consistency of slugs and eventSlugs
    all_slugs = {trade["slug"] for trade in trades}
    print(f"All Slugs Same: {'Yes' if len(all_slugs) == 1 else 'No'}")
    all_event_slugs = {trade["eventSlug"] for trade in trades}
    print(f"All EventSlugs Same: {'Yes' if len(all_event_slugs) == 1 else 'No'}")


def plot_price_over_time(trades, filename):
    # Extract timestamps and prices
    timestamps = [trade["timestamp"] for trade in trades]
    prices = [
        trade["price"] if trade["outcome"] == "Yes" else 1 - trade["price"]
        for trade in trades
    ]

    # Convert timestamps to datetime objects for plotting
    dates = [datetime.fromtimestamp(ts) for ts in timestamps]

    # Plotting the data
    plt.figure(figsize=(12, 6))
    plt.plot(dates, prices, marker=".", linestyle="-")

    plt.title("Price Over Time")
    plt.xlabel("Date and Time")
    plt.ylabel("Price")
    plt.grid(True)
    plt.xticks(rotation=45)
    plt.tight_layout()

    plt.savefig(filename, format="png", bbox_inches="tight")


def main():
    parser = argparse.ArgumentParser(description="Fetch or Load Polymarket Trade Data.")

    # Add arguments to the parser
    parser.add_argument(
        "--id",
        type=str,
        help="Condition ID to download trades for. If empty, loads from data-file.",
    )
    parser.add_argument(
        "--data-file",
        type=str,
        help="Path to save and/or load JSON data to/from.",
        default="cache/all_trades.json",
    )
    parser.add_argument(
        "--plot-file",
        type=str,
        help="Path to save the plot image.",
        default="cache/price_over_time.png",
    )

    args = parser.parse_args()

    if args.id:
        all_items = fetch_all_trades(args.id)
        print("Downloaded", len(all_items), "trades from API.")
        save_to_file(all_items, args.data_file)
        print("All items saved to", args.data_file)
    elif args.data_file:
        all_items = load_from_file(args.data_file)
        print(f"Loaded {len(all_items)} items from", args.data_file)
    else:
        parser.print_help()

    analyze_trades(all_items)
    plot_price_over_time(all_items, args.plot_file)
    print("Price plot saved to", args.plot_file)


if __name__ == "__main__":
    main()
