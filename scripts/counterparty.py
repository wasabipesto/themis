# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "tabulate",
#     "argparse",
# ]
# ///
import argparse
import os
import json
from collections import defaultdict
from tabulate import tabulate


def regenerate_cache(data_file_path, cache_dir):
    positions_file_path = cache_dir + "/positions.json"
    markets_file_path = cache_dir + "/markets.json"
    users_file_path = cache_dir + "/users.json"
    positions_map = defaultdict(int)
    markets = {}
    users = {}
    with open(data_file_path, "r", encoding="utf-8") as f:
        for i, line in enumerate(f):
            # deserialize
            item = json.loads(line)

            # binary only for now
            if not item["full_market"]["mechanism"] == "cpmm-1":
                continue

            # update market
            market_id = item["id"]
            markets[market_id] = {
                "question": item["full_market"]["question"],
                "url": item["full_market"]["url"],
                "isResolved": item["full_market"]["isResolved"],
            }

            # process bets
            for bet in item["bets"]:
                # get data
                user_id = bet["userId"]
                shares = bet["shares"]

                # invert NO shares
                if bet["outcome"] == "NO":
                    shares = shares * -1

                # update position
                positions_map[(market_id, user_id)] += shares

                # update user
                if (
                    user_id not in users
                    and bet.get("userName")
                    and bet.get("userUsername")
                ):
                    users[user_id] = {
                        "userName": bet.get("userName"),
                        "userUsername": bet.get("userUsername"),
                    }

            # notify progress
            if i % 20_000 == 0:
                print(f"Processed {i} lines...")

    # format for output
    positions = [
        {
            "market_id": market_id,
            "user_id": user_id,
            "position": position,
        }
        for (market_id, user_id), position in positions_map.items()
    ]
    for user_id in {p["user_id"] for p in positions}:
        if user_id not in users:
            users[user_id] = {
                "userId": user_id,
                "userName": "Unkown",
                "userUsername": "Unkown",
            }

    # save to disk
    with open(positions_file_path, "w") as f:
        json.dump(positions, f, indent=2)
    with open(markets_file_path, "w") as f:
        json.dump(markets, f, indent=2)
    with open(users_file_path, "w") as f:
        json.dump(users, f, indent=2)


def get_user_id(user, users):
    if user is None:
        raise Exception("Target user required (--user)")
    if user in users:
        return users[user]["userId"]
    for user_item in users.items():
        if user_item[1]["userName"] == user:
            return user_item[0]
        if user_item[1]["userUsername"] == user:
            return user_item[0]
    raise Exception(f"User {user} not found")


def main():
    parser = argparse.ArgumentParser(
        description="Script to find potential counterparties for a specified Manifold user. Uses data saved from the `download` tool, generates some intermediate cache files and the calculates market and overall agreement scores for all other users. Shows the top 10 results but saves them all."
    )
    parser.add_argument(
        "--data-file",
        "-d",
        type=str,
        help="Path to the JSONL file with Manifold market data.",
    )
    parser.add_argument(
        "--cache-dir",
        "-p",
        type=str,
        default="cache/counterparty",
        help="Path to the positions data file. Default: cache/counterparty",
    )
    parser.add_argument(
        "--regen-cache",
        action="store_true",
        help="Signal to generate or regenerate cached data. Must be run at least once.",
    )
    parser.add_argument(
        "--user",
        "-u",
        type=str,
        help="User to get counterparties for. Accepts user ID or username.",
    )
    parser.add_argument(
        "--min-position",
        "-m",
        type=int,
        default=10,
        help="Minimum position to consider the target user to be interested in.",
    )
    args = parser.parse_args()

    if args.regen_cache:
        print("Regenerating cache files...")
        regenerate_cache(args.data_file, args.cache_dir)
    else:
        for filename in ["positions.json", "markets.json", "users.json"]:
            filepath = args.cache_dir + "/" + filename
            if not os.path.exists(filepath):
                raise ValueError(
                    "Cache file",
                    filepath,
                    "does not exist!",
                    "Try again with --regen-cache and --data-file.",
                )

    with open(args.cache_dir + "/positions.json", "r", encoding="utf-8") as f:
        positions = json.load(f)
    with open(args.cache_dir + "/markets.json", "r", encoding="utf-8") as f:
        markets = json.load(f)
    with open(args.cache_dir + "/users.json", "r", encoding="utf-8") as f:
        users = json.load(f)

    # define target user
    target_user_id = get_user_id(args.user, users)
    print(
        "Getting counterparties for user",
        users[target_user_id]["userName"],
        "(User ID",
        target_user_id,
        ")",
    )

    # get all markets target user holds positions in
    target_user_positions = {
        p["market_id"]: p["position"]
        for p in positions
        if p["user_id"] == target_user_id and p["position"] > args.min_position
    }

    # sort positions into markets for faster lookup
    market_positions = defaultdict(list)
    for p in positions:
        market_id = p["market_id"]
        if market_id in target_user_positions:
            market_positions[market_id].append(p)

    # get market agreement scores
    market_agreement_scores = []
    for market_id in target_user_positions:
        target_user_position = target_user_positions[market_id]

        for position in market_positions[market_id]:
            other_user_id = position["user_id"]
            if other_user_id == target_user_id:
                continue
            other_user_position = position["position"]
            agreement_score = target_user_position * other_user_position
            market_agreement_scores.append(
                {
                    "market_id": market_id,
                    "user_id": other_user_id,
                    "agreement_score": agreement_score,
                }
            )

    # get other unique user IDs
    other_user_ids = {i["user_id"] for i in market_agreement_scores}

    # get overall agreement scores
    overall_agreement_scores = defaultdict(float)
    for user_id in other_user_ids:
        overall_agreement_scores[user_id] += sum(
            [
                i["agreement_score"]
                for i in market_agreement_scores
                if i["user_id"] == user_id
            ]
        )
    results_path = args.cache_dir + "/results.json"
    with open(results_path, "w") as f:
        json.dump(
            {
                "market_agreement_scores": market_agreement_scores,
                "overall_agreement_scores": overall_agreement_scores,
            },
            f,
            indent=2,
        )
    print(
        "Found",
        len(other_user_ids),
        "potential counterparties with",
        len(market_agreement_scores),
        "total market agreement scores.",
    )
    print("Saved results to", results_path)

    # Sort users by total agreement scores
    sorted_users = sorted(
        overall_agreement_scores.items(), key=lambda x: x[1], reverse=True
    )

    # Tabulate and display results
    print("\nUsers Most Agreed-With:")
    top_table = [
        [user_id, users[user_id]["userName"], score]
        for user_id, score in sorted_users[:10]
    ]
    print(
        tabulate(
            top_table,
            headers=["User ID", "User Name", "Total Agreement Score"],
            tablefmt="github",
        )
    )

    print("\nUsers Most Disagreed-With:")
    bottom_table = [
        [user_id, users[user_id]["userName"], score]
        for user_id, score in sorted_users[-10:]
    ]
    print(
        tabulate(
            bottom_table,
            headers=["User ID", "User Name", "Total Agreement Score"],
            tablefmt="github",
        )
    )

    # Get top and bottom ten market agreement scores
    sorted_market_agreements = sorted(
        market_agreement_scores, key=lambda x: x["agreement_score"], reverse=True
    )

    # Filter for unresolved markets first
    unresolved_market_agreements = [
        entry
        for entry in sorted_market_agreements
        if not markets[entry["market_id"]]["isResolved"]
    ]

    print("\nTop Agreements on Open Markets:")
    top_market_table = [
        [
            markets[entry["market_id"]]["url"],
            users[entry["user_id"]]["userName"],
            entry["agreement_score"],
        ]
        for entry in unresolved_market_agreements[:10]
    ]
    print(
        tabulate(
            top_market_table,
            headers=["Market URL", "User", "Agreement Score"],
            tablefmt="github",
        )
    )

    print("\nTop Disagreements on Open Markets:")
    bottom_market_table = [
        [
            markets[entry["market_id"]]["url"],
            users[entry["user_id"]]["userName"],
            entry["agreement_score"],
        ]
        for entry in unresolved_market_agreements[-20:]
    ]
    print(
        tabulate(
            bottom_market_table,
            headers=["Market URL", "User", "Agreement Score"],
            tablefmt="github",
        )
    )


if __name__ == "__main__":
    main()
