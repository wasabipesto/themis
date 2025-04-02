# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "tabulate",
# ]
# ///
import argparse
import json
from collections import Counter

from tabulate import tabulate


def get_nested_value(d, keys):
    """
    Recursively fetch the nested value from a dictionary using a list of keys,
    handling array notation "[]" to extract values from lists.
    """
    results = [d]

    for key in keys:
        next_results = []
        for item in results:
            if isinstance(item, dict) and key in item:
                next_item = item[key]
                if isinstance(next_item, list):
                    next_results.extend(next_item)
                else:
                    next_results.append(next_item)
        results = next_results

    return results

def get_value_counts_from_jsonl(file_path, nested_key):
    value_counts = Counter()
    if isinstance(nested_key, str):
        keys = nested_key.replace('[]', '').split('.')
    else:
        keys = nested_key

    with open(file_path, 'r') as file:
        for line in file:
            try:
                data = json.loads(line)
                values = get_nested_value(data, keys)
                for value in values:
                    if value is not None:
                        value_counts[value] += 1
            except json.JSONDecodeError:
                continue

    return value_counts

def main():
    parser = argparse.ArgumentParser(description="Get the count of each unique value of a key from a JSONL file.")
    parser.add_argument("file_path", help="Path to the JSONL file")
    parser.add_argument("key", help="JSON key, nested with '.' and use '[]' for lists (e.g., 'market.tokens[].outcome')")
    args = parser.parse_args()

    value_counts = get_value_counts_from_jsonl(args.file_path, args.key)

    table_data = [(value, count) for value, count in value_counts.items() if count > 1]
    headers = ['Value', 'Count']

    print(tabulate(table_data, headers, tablefmt="github"))

    missing_items = len(value_counts) - len(table_data)
    if missing_items > 0:
        print("And", missing_items, "other unique values.")

if __name__ == "__main__":
    main()
