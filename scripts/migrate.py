# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
#     "argparse",
#     "tqdm",
# ]
# ///

import argparse
import json
import itertools
import os
from pathlib import Path
import requests
from dotenv import load_dotenv
from tqdm import tqdm, trange
import math


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Import or export data to/from databases via the PostgREST API."
    )
    parser.add_argument(
        "--mode",
        "-m",
        type=str,
        choices=["import", "export"],
        required=True,
        help="Operation mode: import data to the database or export data to disk",
    )
    parser.add_argument(
        "--cache-dir",
        "-c",
        type=str,
        default="cache/migration",
        help='Directory containing/for JSON files (default: "cache/migration")',
    )
    parser.add_argument(
        "--url",
        type=str,
        default=None,
        help="URL of the PostgREST API endpoint (default: uses PGRST_URL environment variable)",
    )
    parser.add_argument(
        "--apikey",
        type=str,
        default=None,
        help="API key to use for the PostgREST API (default: uses PGRST_APIKEY environment variable)",
    )
    parser.add_argument(
        "--table",
        "-t",
        type=str,
        default=None,
        help="Only import/export a specific table (default: all tables)",
    )
    return parser.parse_args()


def get_data(endpoint, headers=None, params=None, batch_size=100_000):
    """Get data from a PostgREST endpoint and handle the response."""

    count_response = requests.get(endpoint, headers=headers, params="select=count")
    total_count = count_response.json()[0]["count"]
    if total_count == 0:
        raise ValueError(f"No data available at {endpoint}")

    result = []
    num_batches = math.ceil(total_count / batch_size)
    for i in trange(num_batches):
        params["limit"] = batch_size
        params["offset"] = len(result)
        response = requests.get(endpoint, headers=headers, params=params)
        if response.ok:
            data = response.json()
            result += data
        else:
            print(f"Download returned code {response.status_code} for {endpoint}")
            try:
                error_data = response.json()
                print(json.dumps(error_data, indent=2), "\n")
            except Exception as e:
                print("Could not parse JSON response:", e)
                print("Raw response:", response.text, "\n")
            raise ValueError()

    if total_count != len(result):
        raise ValueError(
            f"Data missing at {endpoint}: {total_count} expected, {len(result)} received"
        )

    return result


def post_data(endpoint, data, headers=None, params=None, batch_size=10_000):
    """Post data to a PostgREST endpoint and handle the response."""

    for batch in itertools.batched(tqdm(data), batch_size):
        response = requests.post(endpoint, headers=headers, json=batch, params=params)
        if not response.ok:
            print(
                f"Upload returned code {response.status_code} for {endpoint.split('/')[-1]}"
            )
            try:
                error_data = response.json()
                print(json.dumps(error_data, indent=2), "\n")
            except Exception as e:
                print("Could not parse JSON response:", e)
                print("Raw response:", response.text, "\n")
            return False


def load_json_file(filename):
    """Load and return data from a JSON file."""
    with open(filename, "r") as f:
        return json.load(f)


def import_simple(postgrest_base, headers, cache_dir, table):
    print(f"Importing table: {table}...")
    filename = cache_dir / f"{table}.json"
    if filename.exists():
        items = load_json_file(filename)
        post_data(
            f"{postgrest_base}/{table}",
            items,
            headers=headers,
        )
        print(
            f"Imported table {table} with {len(items)} items to {postgrest_base}/{table}."
        )
        print()
    else:
        print(f"Warning: {filename} not found")


def export_simple(postgrest_base, headers, cache_dir, table, order):
    print(f"Exporting table: {table}...")
    filename = cache_dir / f"{table}.json"
    items = get_data(
        f"{postgrest_base}/{table}",
        params={
            "order": order,
        },
        headers=headers,
    )
    with open(filename, "w") as f:
        json.dump(items, f, indent=2)
    print(f"Exported table {table} with {len(items)} items to {filename}")
    print()


def import_questions(postgrest_base, headers, cache_dir):
    """Import questions table and return necessary mapping data."""
    print("Importing table: questions...")
    questions_file = cache_dir / "questions.json"
    if not questions_file.exists():
        print(f"Warning: {questions_file} not found")
        return {}, {}

    questions_raw = load_json_file(questions_file)

    # Upload question data
    items = [
        {
            "title": question["title"],
            "slug": question["slug"],
            "description": question["description"],
            "category_slug": question["category_slug"],
            "start_date_override": question["start_date_override"],
            "end_date_override": question["end_date_override"],
        }
        for question in questions_raw
    ]
    post_data(
        f"{postgrest_base}/questions",
        items,
        headers=headers,
        # Merge if the slug already exists
        params={"on_conflict": "slug"},
    )
    print(
        f"Imported table questions with {len(items)} items to {postgrest_base}/questions."
    )
    print()

    # Get updated question map
    questions_new = requests.get(
        f"{postgrest_base}/questions", params={"select": "id,slug"}
    ).json()
    question_slug_to_id_map = {
        question["slug"]: question["id"] for question in questions_new
    }
    question_id_to_id_map = {
        question["id"]: question_slug_to_id_map[question["slug"]]
        for question in questions_raw
    }

    return question_slug_to_id_map, question_id_to_id_map

def import_markets(postgrest_base, headers, cache_dir, question_slug_to_id_map):
    """Import markets and related tables."""
    print("Importing table: markets...")
    markets_file = cache_dir / "market_details.json"
    if not markets_file.exists():
        print(f"Warning: {markets_file} not found")
        return

    markets_raw = load_json_file(markets_file)

    # Upload raw market data
    items = [
        {
            "id": market["id"],
            "title": market["title"],
            "url": market["url"],
            "description": market["description"],
            "platform_slug": market["platform_slug"],
            "category_slug": market["category_slug"],
            "open_datetime": market["open_datetime"],
            "close_datetime": market["close_datetime"],
            "traders_count": market["traders_count"],
            "volume_usd": market["volume_usd"],
            "duration_days": market["duration_days"],
            "resolution": market["resolution"],
        }
        for market in markets_raw
    ]
    if len(items) == 0:
        raise ValueError("No markets to import")
    post_data(
        f"{postgrest_base}/markets",
        items,
        headers=headers,
    )
    print(
        f"Imported table markets with {len(items)} items to {postgrest_base}/markets."
    )
    print()

    # Upload dismissals
    print("Importing table: market_dismissals...")
    items = [
        {
            "market_id": market["id"],
            "dismissed_status": market["question_dismissed"],
        }
        for market in markets_raw
        if market.get("question_dismissed") and market["question_dismissed"] > 0
    ]
    if len(items) > 0:
        post_data(
            f"{postgrest_base}/market_dismissals",
            items,
            headers=headers,
        )
        print(
            f"Imported table market_dismissals with {len(items)} items to {postgrest_base}/market_dismissals."
        )
    else:
        print("No markets with question_dismissed values to import")
    print()

    # Upload market links, replacing question slugs with IDs first
    print("Importing table: market_questions...")
    items = [
        {
            "market_id": market["id"],
            "question_invert": market["question_invert"],
            "question_id": question_slug_to_id_map[market["question_slug"]],
        }
        for market in markets_raw
        if market.get("question_slug")
    ]
    if len(items) > 0:
        post_data(f"{postgrest_base}/market_questions", items, headers=headers)
        print(
            f"Imported table market_questions with {len(items)} items to {postgrest_base}/market_questions."
        )
    else:
        print("No markets with question_slug values to import")
    print()

def import_question_embeddings(postgrest_base, headers, cache_dir, question_id_to_id_map):
    """Import question embeddings."""
    table = "question_embeddings"
    print(f"Importing table: {table}...")
    filename = cache_dir / f"{table}.json"
    if not filename.exists():
        print(f"Warning: {filename} not found")
        return
        
    data = load_json_file(filename)
    items = [
        {
            "question_id": question_id_to_id_map[qemb["question_id"]],
            "embedding": qemb["embedding"],
        }
        for qemb in data
    ]
    post_data(
        f"{postgrest_base}/{table}",
        items,
        headers=headers,
    )
    print(
        f"Imported table {table} with {len(items)} items to {postgrest_base}/{table}."
    )
    print()

def import_to_db(cache_dir, postgrest_base, postgrest_apikey, table=None):
    """Import data from JSON files to PostgREST."""
    # Ensure cache directory exists for import
    if not cache_dir.exists() or not cache_dir.is_dir():
        raise ValueError(
            f"Cache directory {cache_dir} does not exist, nothing to import!"
        )

    # Common headers for all requests
    headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Prefer": "resolution=merge-duplicates",
        "Content-Type": "application/json",
    }
    
    # Handle single table import
    if table:
        if table == "questions":
            import_questions(postgrest_base, headers, cache_dir)
        elif table == "markets" or table == "market_details":
            question_slug_to_id_map, _ = import_questions(postgrest_base, headers, cache_dir)
            import_markets(postgrest_base, headers, cache_dir, question_slug_to_id_map)
        elif table == "market_dismissals" or table == "market_questions":
            question_slug_to_id_map, _ = import_questions(postgrest_base, headers, cache_dir)
            import_markets(postgrest_base, headers, cache_dir, question_slug_to_id_map)
        elif table == "question_embeddings":
            question_slug_to_id_map, question_id_to_id_map = import_questions(postgrest_base, headers, cache_dir)
            import_question_embeddings(postgrest_base, headers, cache_dir, question_id_to_id_map)
        else:
            import_simple(postgrest_base, headers, cache_dir, table)
        return

    # Import all tables
    question_slug_to_id_map, question_id_to_id_map = import_questions(postgrest_base, headers, cache_dir)
    import_markets(postgrest_base, headers, cache_dir, question_slug_to_id_map)
    import_question_embeddings(postgrest_base, headers, cache_dir, question_id_to_id_map)

    # Upload other simple tables
    import_simple(postgrest_base, headers, cache_dir, "market_embeddings")
    import_simple(postgrest_base, headers, cache_dir, "daily_probabilities")
    import_simple(postgrest_base, headers, cache_dir, "criterion_probabilities")
    import_simple(postgrest_base, headers, cache_dir, "newsletter_signups")
    import_simple(postgrest_base, headers, cache_dir, "general_feedback")


def get_table_order(table):
    """Get the order parameter for a table for export."""
    table_orders = {
        "questions": "id",
        "market_details": "id",
        "market_embeddings": "market_id",
        "question_embeddings": "question_id",
        "daily_probabilities": "market_id,date",
        "criterion_probabilities": "market_id,criterion_type",
        "newsletter_signups": "date",
        "general_feedback": "date",
        "markets": "id",
        "market_dismissals": "market_id",
        "market_questions": "market_id,question_id",
    }
    return table_orders.get(table, "id")

def export_to_disk(cache_dir, postgrest_base, postgrest_apikey, table=None):
    """Export data from PostgREST to JSON files."""
    # Ensure cache directory exists
    cache_dir.mkdir(exist_ok=True)

    # Common headers for all requests
    headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Content-Type": "application/json",
    }

    # Handle single table export
    if table:
        export_simple(postgrest_base, headers, cache_dir, table, get_table_order(table))
        return

    # Export all tables
    export_simple(postgrest_base, headers, cache_dir, "questions", "id")
    export_simple(postgrest_base, headers, cache_dir, "market_details", "id")
    export_simple(postgrest_base, headers, cache_dir, "market_embeddings", "market_id")
    export_simple(
        postgrest_base, headers, cache_dir, "question_embeddings", "question_id"
    )
    export_simple(
        postgrest_base, headers, cache_dir, "daily_probabilities", "market_id,date"
    )
    export_simple(
        postgrest_base,
        headers,
        cache_dir,
        "criterion_probabilities",
        "market_id,criterion_type",
    )
    export_simple(postgrest_base, headers, cache_dir, "newsletter_signups", "date")
    export_simple(postgrest_base, headers, cache_dir, "general_feedback", "date")


def main():
    # get env vars and command line arguments
    load_dotenv()
    args = parse_arguments()

    # Set up PostgREST connection
    if args.url:
        postgrest_base = args.url
    else:
        postgrest_base = os.environ.get("PGRST_URL")
    if not postgrest_base:
        raise ValueError("Missing required environment variable PGRST_URL")
    if args.apikey:
        postgrest_apikey = args.apikey
    else:
        postgrest_apikey = os.environ.get("PGRST_APIKEY")
    if not postgrest_apikey:
        raise ValueError("Missing required environment variable PGRST_APIKEY")

    # Set up cache directory
    cache_dir = Path(args.cache_dir)
    
    # Handle table argument
    table_msg = f" (table: {args.table})" if args.table else ""

    if args.mode == "import":
        print(f"Importing data from {cache_dir} to PostgREST{table_msg}...")
        import_to_db(cache_dir, postgrest_base, postgrest_apikey, args.table)
    else:  # Export mode
        print(f"Exporting data from PostgREST to {cache_dir}{table_msg}...")
        export_to_disk(cache_dir, postgrest_base, postgrest_apikey, args.table)

    print("Operation completed.")


if __name__ == "__main__":
    main()
