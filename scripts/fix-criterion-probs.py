# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
# ]
# ///

import os
import requests
from dotenv import load_dotenv
from datetime import datetime, timedelta


def get_questions():
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")

    response = requests.get(
        f"{postgrest_url}/question_details",
        params={"order": "id"},
    )
    response.raise_for_status()
    return response.json()


def get_question_markets(question_id):
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")

    response = requests.get(
        f"{postgrest_url}/market_details",
        params={"question_id": f"eq.{question_id}"},
    )
    response.raise_for_status()
    return response.json()


def get_probability(market_id, date):
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")

    response = requests.get(
        f"{postgrest_url}/daily_probabilities",
        params={"market_id": f"eq.{market_id}", "date": f"eq.{date}"},
    )
    response.raise_for_status()
    return response.json()[0]["prob"]


def push_criterion_probs(criterion_probs):
    load_dotenv()
    postgrest_url = os.environ.get("PGRST_URL")
    postgrest_apikey = os.environ.get("PGRST_APIKEY")

    headers = {
        "Authorization": f"Bearer {postgrest_apikey}",
        "Prefer": "resolution=merge-duplicates",
        "Content-Type": "application/json",
    }
    response = requests.post(
        f"{postgrest_url}/criterion_probabilities",
        headers=headers,
        json=criterion_probs,
    )
    response.raise_for_status()
    return


def main():
    print("Getting questions...")
    questions = get_questions()

    markets_to_fix = []
    print(f"Evaluating {len(questions)} questions...")
    for question in questions:
        if question["start_date_override"] is not None:
            question_start = datetime.fromisoformat(
                f"{question['start_date_override']}T00:00:00Z"
            )
        else:
            question_start = None
        if question["end_date_override"] is not None:
            question_end = datetime.fromisoformat(
                f"{question['end_date_override']}T00:00:00Z"
            )
        else:
            question_end = None
        if question_start is None and question_end is None:
            continue

        markets = get_question_markets(question["id"])
        for market in markets:
            market_open = datetime.fromisoformat(market["open_datetime"])
            market_close = datetime.fromisoformat(market["close_datetime"])

            # Determine new start date
            if question_start is not None and question_start > market_open:
                new_start = question_start
            else:
                new_start = market_open

            # Determine new end date
            if question_end is not None and question_end < market_close:
                new_end = question_end
            else:
                new_end = market_close

            # Only add to fix list if dates actually changed
            if new_start != market_open or new_end != market_close:
                markets_to_fix.append(
                    {
                        "market": market,
                        "new_start": new_start,
                        "new_end": new_end,
                    }
                )

    print(f"Getting new criteria for {len(markets_to_fix)} markets...")
    new_criterion_probs = []
    for i in markets_to_fix:
        market_id = i["market"]["id"]
        new_start = i["new_start"]
        new_end = i["new_end"]

        # Criterion: 7 Days
        date = new_end - timedelta(days=7)
        if date > new_start:
            prob = get_probability(market_id, date.strftime("%Y-%m-%dT12:00:00+00:00"))
            new_criterion_probs.append(
                {
                    "market_id": market_id,
                    "criterion_type": "before-close-days-7",
                    "prob": prob,
                }
            )

        # Criterion: 30 Days
        date = new_end - timedelta(days=30)
        if date > new_start:
            prob = get_probability(market_id, date.strftime("%Y-%m-%dT12:00:00+00:00"))
            new_criterion_probs.append(
                {
                    "market_id": market_id,
                    "criterion_type": "before-close-days-30",
                    "prob": prob,
                }
            )

        # Criterion: Midpoint
        date = new_start + (new_end - new_start) / 2
        prob = get_probability(market_id, date.strftime("%Y-%m-%dT12:00:00+00:00"))
        new_criterion_probs.append(
            {
                "market_id": market_id,
                "criterion_type": "midpoint",
                "prob": prob,
            }
        )

    print(f"Uploading {len(new_criterion_probs)} new criterion probs...")
    push_criterion_probs(new_criterion_probs)


if __name__ == "__main__":
    main()
