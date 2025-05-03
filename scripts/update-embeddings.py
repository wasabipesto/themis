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

import os
import argparse
import requests
from dotenv import load_dotenv
import ollama
from tqdm import tqdm


def parse_arguments():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Create or update embeddings for markets and questions."
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Update all embeddings. Defaults to only processing missing items.",
    )
    parser.add_argument(
        "--questions-only",
        action="store_true",
        help="Only process questions.",
    )
    return parser.parse_args()


# Download markets or questions from PostgREST API
def download_collection(all, collection):
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")
    if not postgrest_url:
        raise ValueError("Missing required environment variable PGRST_URL")

    limit = 100_000
    offset = 0
    all_markets = []

    if all:
        endpoint = f"/{collection}"
    else:
        endpoint = f"/rpc/find_{collection}_missing_embeddings"
    print(f"Getting {collection} from {endpoint}...")

    while True:
        response = requests.get(
            f"{postgrest_url}{endpoint}",
            params={"order": "id", "limit": limit, "offset": offset},
        )
        if not response.ok:
            raise Exception(f"Failed to fetch {collection}: {response.text}")
        markets_batch = response.json()
        if not markets_batch:
            break
        all_markets.extend(markets_batch)
        if len(markets_batch) < limit:
            break
        offset += limit

    return all_markets


def upload_embeddings_batch(endpoint, embeddings_batch):
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")
    postgrest_apikey = os.environ.get("PGRST_APIKEY")
    if not postgrest_url or not postgrest_apikey:
        raise ValueError(
            "Missing required environment variable PGRST_URL or PGRST_APIKEY"
        )

    headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Prefer": "resolution=merge-duplicates",
        "Content-Type": "application/json",
    }

    response = requests.post(
        f"{postgrest_url}{endpoint}", headers=headers, json=embeddings_batch
    )
    if not response.ok:
        raise Exception(f"Failed to upload embeddings batch: {response.text}")


def main():
    args = parse_arguments()
    batch_size = 200
    embeddings_batch = []

    if not args.questions_only:
        markets = download_collection(args.all, "markets")
        print("Generating market embeddings...")
        for market in tqdm(markets):
            prompt = f"{market['title']}\n {market['description']}"
            response = ollama.embeddings(
                model="nomic-embed-text",
                prompt=prompt,
            )
            embedding = response.get("embedding")
            if not embedding or len(embedding) != 768:
                raise Exception(f"Invalid embedding response: {response}")

            embeddings_batch.append({"market_id": market["id"], "embedding": embedding})

            # Upload in batches of batch_size
            if len(embeddings_batch) >= batch_size:
                upload_embeddings_batch("/market_embeddings", embeddings_batch)
                embeddings_batch = []

        # Upload any remaining items
        if embeddings_batch:
            upload_embeddings_batch("/market_embeddings", embeddings_batch)
        embeddings_batch = []

    questions = download_collection(args.all, "questions")
    print("Generating question embeddings...")
    for question in tqdm(questions):
        prompt = f"{question['title']}\n {question['description']}"
        response = ollama.embeddings(
            model="nomic-embed-text",
            prompt=prompt,
        )
        embedding = response.get("embedding")
        if not embedding or len(embedding) != 768:
            raise Exception(f"Invalid embedding response: {response}")

        embeddings_batch.append({"question_id": question["id"], "embedding": embedding})

        # Upload in batches of batch_size
        if len(embeddings_batch) >= batch_size:
            upload_embeddings_batch("/question_embeddings", embeddings_batch)
            embeddings_batch = []

    # Upload any remaining items
    if embeddings_batch:
        upload_embeddings_batch("/question_embeddings", embeddings_batch)


if __name__ == "__main__":
    main()
