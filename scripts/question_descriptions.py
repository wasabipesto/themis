# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "argparse",
#     "requests",
#     "ollama",
#     "tqdm",
# ]
# ///

import argparse
import json
import os
import ollama
from pathlib import Path
import requests
from dotenv import load_dotenv
from tqdm import tqdm


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Generate descriptions for questions missing them."
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

    if not postgrest_base:
        raise ValueError("Missing required environment variable")

    return postgrest_base


def get_market_data(postgrest_base, market_id):
    response = requests.get(
        f"{postgrest_base}/market_details", params={"id": f"eq.{market_id}"}
    )
    if response.ok:
        return response.json()[0]
    else:
        print(
            f"Download returned code {response.status_code} for market ID {market_id}"
        )
        print(json.dumps(response.json(), indent=2), "\n")
        return False


def query_ollama(prompt):
    model = "llama3.2"
    response = ollama.generate(
        model=model,
        prompt=prompt,
        stream=False,
    )
    return response.response


def get_questions(cache_dir):
    filename = cache_dir / "questions.json"
    with open(filename, "r") as f:
        return json.load(f)


def get_markets(cache_dir):
    filename = cache_dir / "markets.json"
    with open(filename, "r") as f:
        return json.load(f)


def put_questions(cache_dir, data):
    filename = cache_dir / "questions_generated.json"
    with open(filename, "w") as f:
        json.dump(data, f, indent=2)


def main():
    # Parse command line arguments
    args = parse_arguments()

    # Set up cache directory
    cache_dir = Path(args.cache_dir)

    # Set up PostgREST connection
    postgrest_base = setup_postgrest_connection()

    # Load questions & markets from cache
    questions = get_questions(cache_dir)
    markets = get_markets(cache_dir)

    # Filter to just questions with no description
    linked_markets = [m for m in markets if m["question_slug"]]
    questions_to_process = [q for q in questions if not q["description"]]

    for question in tqdm(questions_to_process):
        question_markets = [
            m for m in linked_markets if m["question_slug"] == question["slug"]
        ]
        market_details = [
            get_market_data(postgrest_base, m["market_id"]) for m in question_markets
        ]
        market_text = [
            {
                "title": m["title"],
                "description": m["description"],
            }
            for m in market_details
        ]
        prompt = f"The following items are titles and descriptions for equivalent prediction markets on different platforms. They have been created to predict the high-level question: ${question['title']} Since they all refer to the same past event, generate a summarized description that captures the spirit of the markets in 5 sentences. Include details from the market descriptions such as the resolution criteria. Don't preface your response, editorialize, or include any additional information. Don't mention prediction markets, stick to the subject matter. Text input: ${market_text}"
        response = query_ollama(prompt)
        question["description"] = response

    # Put questions back to cache
    put_questions(cache_dir, questions_to_process)


if __name__ == "__main__":
    main()
