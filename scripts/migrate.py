# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
#     "argparse",
# ]
# ///

import argparse
import json
import itertools
import os
from pathlib import Path
import requests
from dotenv import load_dotenv


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
    return parser.parse_args()


def setup_postgrest_connection():
    """Set up and validate the PostgREST connection details."""
    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")
    postgrest_apikey = os.environ.get("PGRST_APIKEY")

    if not postgrest_base:
        raise ValueError("Missing required environment variable PGRST_URL")

    return postgrest_base, postgrest_apikey


def get_data(endpoint, headers=None, params=None, batch_size=100_000):
    """Get data from a PostgREST endpoint and handle the response."""

    limit = batch_size
    result = []
    while True:
        params["limit"] = limit
        params["offset"] = len(result)
        response = requests.get(endpoint, headers=headers, params=params)
        if response.ok:
            data = response.json()
            if len(data) > 0:
                result += data
            if len(data) < limit:
                break
        else:
            print(f"Download returned code {response.status_code} for {endpoint}")
            try:
                error_data = response.json()
                print(json.dumps(error_data, indent=2), "\n")
            except Exception as e:
                print("Could not parse JSON response:", e)
                print("Raw response:", response.text, "\n")
            return False

    if len(result) == 0:
        print(f"No data found at {endpoint}")
        return False
    else:
        return result


def post_data(endpoint, data, headers=None, params=None, batch_size=10_000):
    """Post data to a PostgREST endpoint and handle the response."""

    for batch in itertools.batched(data, batch_size):
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

    print(f"{len(data)} items uploaded successfully to {endpoint.split('/')[-1]}.")


def load_json_file(filename):
    """Load and return data from a JSON file."""
    with open(filename, "r") as f:
        return json.load(f)


def import_to_db(cache_dir, postgrest_base, postgrest_apikey):
    """Import data from JSON files to PostgREST."""
    # Ensure cache directory exists for import
    if not cache_dir.exists() or not cache_dir.is_dir():
        raise ValueError(
            f"Cache directory {cache_dir} does not exist, nothing to import!"
        )

    # Common headers for all requests
    default_headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Prefer": "resolution=merge-duplicates",
        "Content-Type": "application/json",
    }

    # Upload Questions
    questions_file = cache_dir / "questions.json"
    if questions_file.exists():
        questions_data = load_json_file(questions_file)
        post_data(
            f"{postgrest_base}/questions",
            questions_data,
            headers=default_headers,
            # Merge if the slug already exists
            params={"on_conflict": "slug"},
        )
    else:
        print(f"Warning: {questions_file} not found")

    # Upload Markets
    markets_file = cache_dir / "markets.json"
    if markets_file.exists():
        markets_data = load_json_file(markets_file)

        # Upload dismissals
        market_dismissals = [
            {
                "market_id": market["market_id"],
                "dismissed_status": market["question_dismissed"],
            }
            for market in markets_data
            if market["question_dismissed"] > 0
        ]
        post_data(
            f"{postgrest_base}/market_dismissals",
            market_dismissals,
            headers=default_headers,
        )

        # Replace question slugs with IDs
        questions = requests.get(
            f"{postgrest_base}/questions", params={"select": "id,slug"}
        ).json()
        question_map = {question["slug"]: question["id"] for question in questions}
        market_links = [
            {
                "market_id": market["market_id"],
                "question_invert": market["question_invert"],
                "question_id": question_map[market["question_slug"]],
            }
            for market in markets_data
            if market["question_slug"]
        ]

        # Upload market links
        post_data(
            f"{postgrest_base}/market_questions", market_links, headers=default_headers
        )
    else:
        print(f"Warning: {markets_file} not found")

    # Upload Market Embeddings
    market_embeddings_file = cache_dir / "market_embeddings.json"
    if market_embeddings_file.exists():
        market_embeddings = load_json_file(market_embeddings_file)
        post_data(
            f"{postgrest_base}/market_embeddings",
            market_embeddings,
            headers=default_headers,
        )
    else:
        print(f"Warning: {market_embeddings_file} not found")


def export_to_disk(cache_dir, postgrest_base, postgrest_apikey):
    """Export data from PostgREST to JSON files."""
    # Ensure cache directory exists
    cache_dir.mkdir(exist_ok=True)

    # Export Questions
    questions = get_data(
        f"{postgrest_base}/questions",
        params={
            "order": "slug.asc",
            "select": "title,slug,description,category_slug,start_date_override,end_date_override",
        },
    )
    output_file = cache_dir / "questions.json"
    with open(output_file, "w") as f:
        json.dump(questions, f, indent=2)
    print(f"Exported {len(questions)} questions to {output_file}")

    # Export Markets
    markets = get_data(
        f"{postgrest_base}/market_details",
        params={
            "or": "(question_slug.not.is.null,question_dismissed.gt.0)",
            "order": "id.asc",
            "select": "market_id:id,question_slug,question_invert,question_dismissed",
        },
    )
    output_file = cache_dir / "markets.json"
    with open(output_file, "w") as f:
        json.dump(markets, f, indent=2)
    print(f"Exported {len(markets)} markets to {output_file}")

    # Export Market Embeddings
    market_embeddings = get_data(
        f"{postgrest_base}/market_embeddings",
        params={
            "order": "market_id.asc",
        },
    )
    output_file = cache_dir / "market_embeddings.json"
    with open(output_file, "w") as f:
        json.dump(market_embeddings, f, indent=2)
    print(f"Exported {len(market_embeddings)} market embeddings to {output_file}")


def main():
    # Parse command line arguments
    args = parse_arguments()

    # Set up cache directory
    cache_dir = Path(args.cache_dir)

    # Set up PostgREST connection
    postgrest_base, postgrest_apikey = setup_postgrest_connection()

    if args.mode == "import":
        print(f"Importing data from {cache_dir} to PostgREST...")
        import_to_db(cache_dir, postgrest_base, postgrest_apikey)
    else:  # Export mode
        print(f"Exporting data from PostgREST to {cache_dir}...")
        export_to_disk(cache_dir, postgrest_base, postgrest_apikey)

    print("Operation completed.")


if __name__ == "__main__":
    main()
