# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "tqdm",
# ]
# ///

import json
from collections import defaultdict
from datetime import datetime
from tqdm import tqdm


def main():
    file = "cache/052525-full-download/manifold-data.jsonl"
    cutoff = datetime(2025, 1, 1)
    limit_orders_by_market = defaultdict(int)
    limit_orders_by_year = defaultdict(int)

    with open(file, "r", encoding="utf-8") as f:
        total_lines = sum(1 for _ in f)

    with open(file, "r", encoding="utf-8") as f:
        for i, line in enumerate(tqdm(f, total=total_lines, desc="Processing markets")):
            # deserialize
            data = json.loads(line)
            market_id = data["id"]
            bets = data["bets"]

            for bet in bets:
                if bet.get("limitProb") is not None:
                    created_time = datetime.fromtimestamp(bet["createdTime"] / 1000)
                    if created_time < cutoff:
                        limit_orders_by_market[market_id] += 1
                    limit_orders_by_year[created_time.year] += 1

    total_markets = i

    # Get values for statistics calculations
    market_values = list(limit_orders_by_market.values())
    year_values = list(limit_orders_by_year.values())

    # Print summary statistics for limit orders by market
    print("=== Limit Orders by Market ===")
    print(f"Only counting those placed before {cutoff}")
    print(f"Total markets: {total_markets}")
    print(f"Total markets with limit orders: {len(market_values)}")
    print(f"Total limit orders: {sum(year_values)}")
    print(f"Total limit orders before {cutoff}: {sum(market_values)}")
    print(f"Average limit orders per market: {sum(market_values) / total_markets}")

    # Print year breakdown
    print("\nYear breakdown:")
    for year in sorted(limit_orders_by_year.keys()):
        print(f"  {year}: {limit_orders_by_year[year]} limit orders")


if __name__ == "__main__":
    main()
