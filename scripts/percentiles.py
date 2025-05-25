# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "numpy",
# ]
# ///

import argparse
import json
import numpy as np


def filter_and_percentile(markets, platform, attribute, percentile):
    if platform is None:
        sample = [m[attribute] for m in markets if m[attribute]]
        platform_text = "all platforms"
    else:
        sample = [
            m[attribute]
            for m in markets
            if m[attribute] and m["platform_slug"] == platform
        ]
        platform_text = f"platform {platform}"
    if len(sample) > 0:
        result = np.percentile(
            sample,
            percentile,
        )
        print(
            f"{percentile}th percentile of {attribute} for {platform_text} is {result}"
        )
    else:
        print(f"No data available for {attribute} on {platform_text}")


def main():
    parser = argparse.ArgumentParser(
        description="Calculate percentiles of market statistics."
    )
    parser.add_argument(
        "--file_path",
        type=str,
        default="cache/migration/markets.json",
        help="Path to the exported JSON file. Default: cache/migration/markets.json",
    )
    parser.add_argument(
        "--platforms",
        "-plt",
        nargs="*",
        default=["kalshi", "manifold", "metaculus", "polymarket", "all"],
        help="Platform(s) to analyze. Use 'all' for all platforms. Default: kalshi manifold metaculus polymarket all",
    )
    parser.add_argument(
        "--attributes",
        "-att",
        nargs="*",
        default=["traders_count", "volume_usd", "duration_days"],
        help="Attribute(s) to analyze. Default: traders_count volume_usd duration_days",
    )
    parser.add_argument(
        "--percentiles",
        "-pct",
        nargs="*",
        type=int,
        default=[10, 25, 50, 75, 90],
        help="Percentile(s) to calculate. Default: 10 25 50 75 90",
    )
    args = parser.parse_args()

    with open(args.file_path, "r") as f:
        markets = json.load(f)

    # Convert 'all' to None for the filter function
    platforms = [None if p == "all" else p for p in args.platforms]
    attributes = args.attributes
    percentiles = args.percentiles

    for platform in platforms:
        for attribute in attributes:
            for percentile in percentiles:
                filter_and_percentile(markets, platform, attribute, percentile)


if __name__ == "__main__":
    main()
