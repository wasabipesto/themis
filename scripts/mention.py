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
    """Fetch market data"""
    query = """
    SELECT
        m.id,
        m.platform_name,
        m.title,
        m.resolution,
        m.volume_usd,
        m.open_datetime,
        m.close_datetime
    FROM market_details m
    WHERE m.resolution IS NOT NULL;
    """

    try:
        with conn.cursor() as cur:
            cur.execute(query)
            results = cur.fetchall()
            return results
    except psycopg2.Error as e:
        print(f"Error executing query: {e}")
        sys.exit(1)

def main():
    # Load environment variables
    load_dotenv()

    # Connect to database
    print("Connecting to database...")
    conn = connect_to_database()

    # Fetch data
    print("Fetching market data...")
    markets = fetch_market_data(conn)

    # Filter for mention markets
    mention_markets = []
    for market in markets:
        title = market[2].lower() if market[2] else ""
        if "mention" in title or "utter" in title or "say" in title:
            mention_markets.append(market)

    print(f"Found {len(mention_markets)} mention markets")

    if not mention_markets:
        print("No mention markets found")
        conn.close()
        return

    # Group by month and platform
    from collections import defaultdict
    data = defaultdict(lambda: defaultdict(int))

    for market in mention_markets:
        platform = market[1]
        close_date = market[6]
        if close_date:
            month_key = close_date.strftime("%Y-%m")
            data[month_key][platform] += 1

    # Sort months
    sorted_months = sorted(data.keys())

    # Get unique platforms
    platforms = sorted(set(platform for month_data in data.values() for platform in month_data.keys()))

    # Prepare data for plotting
    x = np.arange(len(sorted_months))
    bottom = np.zeros(len(sorted_months))

    # Create figure
    fig, ax = plt.subplots(figsize=(14, 8))

    # Color map for platforms
    colors = plt.cm.tab10(np.linspace(0, 1, len(platforms)))

    # Plot stacked bars
    for i, platform in enumerate(platforms):
        values = [data[month].get(platform, 0) for month in sorted_months]
        ax.bar(x, values, bottom=bottom, label=platform, color=colors[i])
        bottom += values

    # Customize plot
    ax.set_xlabel('Month')
    ax.set_ylabel('Number of Markets')
    ax.set_title('Mention Markets Over Time by Platform')
    ax.set_xticks(x)
    ax.set_xticklabels(sorted_months, rotation=45, ha='right')
    ax.legend(title='Platform')

    plt.tight_layout()

    # Save plot
    output_path = 'mention_markets.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Plot saved to {output_path}")

    # Show plot
    plt.show()

    # Close database connection
    conn.close()

    # List the first ten markets per platform (sorted by close date)
    print("\nFirst 10 markets per platform (by close date):")
    print("=" * 80)

    from collections import defaultdict
    platform_markets = defaultdict(list)

    for market in mention_markets:
        platform = market[1]
        title = market[2]
        market_id = market[0]
        close_date = market[6]
        platform_markets[platform].append((market_id, title, close_date))

    for platform in sorted(platform_markets.keys()):
        # Sort by close_date (ascending), None values at the end
        markets = sorted(
            platform_markets[platform],
            key=lambda x: x[2] if x[2] else datetime.max
        )
        print(f"\n{platform} ({len(markets)} mention markets):")
        print("-" * 40)
        for i, (market_id, title, close_date) in enumerate(markets[:10], 1):
            date_str = close_date.strftime("%Y-%m-%d") if close_date else "N/A"
            print(f"  {i}. [{market_id}] ({date_str}) {title[:80]}{'...' if len(title) > 80 else ''}")


if __name__ == "__main__":
    main()
