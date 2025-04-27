# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
#     "ollama",
#     "tqdm",
# ]
# ///

import os
import requests
from dotenv import load_dotenv
import ollama
from tqdm import tqdm


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
            f"{postgrest_url}/markets",
            params={"order": "id", "limit": limit, "offset": offset},
        )
        if not response.ok:
            raise Exception(f"Failed to fetch markets: {response.text}")
        markets_batch = response.json()
        if not markets_batch:
            break
        all_markets.extend(markets_batch)
        if len(markets_batch) < limit:
            break
        offset += limit

    return all_markets


def upload_embedding(data):
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
        f"{postgrest_url}/market_embeddings", headers=headers, json=data
    )
    if not response.ok:
        raise Exception(f"Failed to upload embedding: {response.text}")


def main():
    markets = download_markets()

    for market in tqdm(markets):
        prompt = f"{market['title']}\n {market['description']}"
        response = ollama.embeddings(
            model="nomic-embed-text",
            prompt=prompt,
        )
        embedding = response.get("embedding")
        if not embedding or len(embedding) != 768:
            raise Exception(f"Invalid embedding response: {response}")
        data = {"market_id": market["id"], "embedding": embedding}
        upload_embedding(data)


if __name__ == "__main__":
    main()
