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
import os
from pathlib import Path

import requests
from dotenv import load_dotenv


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description='Import or export data to/from PostgREST API.')
    parser.add_argument('--mode', '-m', type=str, choices=['import', 'export'], required=True,
                       help='Operation mode: import data to API or export data from API')
    parser.add_argument('--cache-dir', '-c', type=str, default="cache",
                        help='Directory containing/for JSON files (default: "cache")')
    return parser.parse_args()


def setup_postgrest_connection():
    """Set up and validate the PostgREST connection details."""
    load_dotenv()
    postgrest_host = os.environ.get('PGRST_HOST')
    postgrest_port = os.environ.get('PGRST_PORT')
    postgrest_apikey = os.environ.get('PGRST_APIKEY')

    if not postgrest_host or not postgrest_port:
        raise ValueError("Missing required environment variables")

    postgrest_base = f"http://{postgrest_host}:{postgrest_port}"
    return postgrest_base, postgrest_apikey


def post_data(endpoint, data, headers=None, params=None):
    """Post data to a PostgREST endpoint and handle the response."""
    response = requests.post(
        endpoint,
        headers=headers,
        json=data,
        params=params
    )

    if response.ok:
        print(f"Data uploaded successfully to {endpoint.split('/')[-1]}.")
        return True
    else:
        print(f"Upload returned code {response.status_code} for {endpoint.split('/')[-1]}")
        try:
            error_data = response.json()
            print(json.dumps(error_data, indent=2), '\n')
        except Exception as e:
            print("Could not parse JSON response:", e)
            print("Raw response:", response.text, '\n')
        return False


def load_json_file(filename):
    """Load and return data from a JSON file."""
    with open(filename, 'r') as f:
        return json.load(f)


def import_data(cache_dir, postgrest_base, postgrest_apikey):
    """Import data from JSON files to PostgREST."""
    # Common headers for all requests
    default_headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Prefer": "resolution=merge-duplicates",
        "Content-Type": "application/json"
    }

    # Upload Questions
    questions_file = cache_dir / 'questions.json'
    if questions_file.exists():
        questions_data = load_json_file(questions_file)
        post_data(
            f"{postgrest_base}/questions",
            questions_data,
            headers=default_headers,
            # Merge if the slug already exists
            params={"on_conflict": "slug"}
        )
    else:
        print(f"Warning: {questions_file} not found")

    # Upload Markets
    markets_file = cache_dir / 'markets.json'
    if markets_file.exists():
        markets_data = load_json_file(markets_file)

        # Upload dismissals
        market_dismissals = [
            {
                "market_id": market["market_id"],
                "dismissed_status": market["question_dismissed"]
            }
            for market in markets_data
            if market["question_dismissed"] > 0
        ]
        post_data(
            f"{postgrest_base}/market_dismissals",
            market_dismissals,
            headers=default_headers
        )

        # Replace question slugs with IDs
        questions = requests.get(
            f"{postgrest_base}/questions",
            params={"select": "id,slug"}
        ).json()
        question_map = {question["slug"]: question["id"] for question in questions}
        market_links = [
            {
                "market_id": market["market_id"],
                "question_invert": market["question_invert"],
                "question_id": question_map[market["question_slug"]]
            }
            for market in markets_data
            if market["question_slug"]
        ]

        # Upload market links
        post_data(
            f"{postgrest_base}/market_questions",
            market_links,
            headers=default_headers
        )
    else:
        print(f"Warning: {markets_file} not found")


def export_data(cache_dir, postgrest_base, postgrest_apikey):
    """Export data from PostgREST to JSON files."""
    # Ensure cache directory exists
    cache_dir.mkdir(exist_ok=True)

    # Export Questions
    response = requests.get(
        f"{postgrest_base}/questions",
        params={
            "order": "slug.asc",
            "select": "title,slug,description,category_slug,start_date_override,end_date_override"
        }
    )
    if response.ok:
        data = response.json()
        output_file = cache_dir / 'questions.json'
        with open(output_file, 'w') as f:
            json.dump(data, f, indent=2)
        print(f"Exported {len(data)} questions to {output_file}")
    else:
        print(f"Failed to export questions: {response.status_code}")
        print(json.dumps(response.json(), indent=2), '\n')

    # Export Markets
    response = requests.get(
        f"{postgrest_base}/market_details",
        params={
            "or": "(question_slug.not.is.null,question_dismissed.gt.0)",
            "select": "market_id:id,question_slug,question_invert,question_dismissed"
        }
    )
    if response.ok:
        data = response.json()
        output_file = cache_dir / 'markets.json'
        with open(output_file, 'w') as f:
            json.dump(data, f, indent=2)
        print(f"Exported {len(data)} markets to {output_file}")
    else:
        print(f"Failed to export markets: {response.status_code}")
        print(json.dumps(response.json(), indent=2), '\n')


def main():
    # Parse command line arguments
    args = parse_arguments()

    # Set up cache directory
    cache_dir = Path(args.cache_dir)

    # Set up PostgREST connection
    postgrest_base, postgrest_apikey = setup_postgrest_connection()

    if args.mode == 'import':
        # Ensure cache directory exists for import
        if not cache_dir.exists() or not cache_dir.is_dir():
            raise ValueError(f"Cache directory {cache_dir} does not exist or is not a directory")
        print(f"Importing data from {cache_dir} to PostgREST...")
        import_data(cache_dir, postgrest_base, postgrest_apikey)
    else:  # Export mode
        print(f"Exporting data from PostgREST to {cache_dir}...")
        export_data(cache_dir, postgrest_base, postgrest_apikey)

    print("Operation completed.")


if __name__ == "__main__":
    main()
