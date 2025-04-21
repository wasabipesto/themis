# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
#     "tabulate",
# ]
# ///

import os
import requests
from dotenv import load_dotenv
from tabulate import tabulate


# Download all markets from PostgREST API
def download_markets():
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")
    if not postgrest_url:
        raise ValueError("Missing required environment variable PGRST_URL")

    limit = 100_000
    offset = 0
    all_markets = []

    while True:
        response = requests.get(
            f"{postgrest_url}/markets", params={"limit": limit, "offset": offset}
        )
        if response.status_code != 200:
            raise Exception(f"Failed to fetch markets: {response.text}")
        markets_batch = response.json()
        if not markets_batch:
            break
        all_markets.extend(markets_batch)
        if len(markets_batch) < limit:
            break
        offset += limit

    return all_markets


def main():
    markets = download_markets()
    platform_category_map = {}

    for market in markets:
        category_slug = market.get("category_slug")
        platform_slug = market.get("platform_slug")
        if platform_slug:
            if platform_slug not in platform_category_map:
                platform_category_map[platform_slug] = {"none": 0}
            if category_slug:
                if category_slug not in platform_category_map[platform_slug]:
                    platform_category_map[platform_slug][category_slug] = 0
                platform_category_map[platform_slug][category_slug] += 1
            else:
                platform_category_map[platform_slug]["none"] += 1

    # Prepare data for tabulate
    table_data = []
    headers = ["Platform", "Category", "Count", "Percentage", "Bar"]

    for platform, categories in platform_category_map.items():
        total_categories = sum(categories.values())

        for category, count in sorted(categories.items()):
            percentage = (count / total_categories) * 100
            table_data.append(
                [
                    platform,
                    category,
                    count,
                    f"{percentage:.2f}%",
                    "#" * int(percentage / 2),
                ]
            )

    # Print the organized output using tabulate with GitHub style
    print(tabulate(table_data, headers=headers, tablefmt="github"))


if __name__ == "__main__":
    main()
