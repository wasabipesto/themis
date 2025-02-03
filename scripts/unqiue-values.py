import json
import argparse
from collections import Counter
from tabulate import tabulate

def get_nested_value(d, keys):
    """
    Recursively fetch the nested value from a dictionary using a list of keys.
    """
    for key in keys:
        if isinstance(d, dict) and key in d:
            d = d[key]
        else:
            return None
    return d

def get_value_counts_from_jsonl(file_path, nested_key):
    value_counts = Counter()
    keys = nested_key.split('.')

    with open(file_path, 'r') as file:
        for line in file:
            try:
                data = json.loads(line)
                value = get_nested_value(data, keys)
                if value is not None:
                    value_counts[value] += 1
            except json.JSONDecodeError:
                continue

    return value_counts

def main():
    parser = argparse.ArgumentParser(description="Get the count of each unique value of a key from a JSONL file.")
    parser.add_argument("file_path", help="Path to the JSONL file")
    parser.add_argument("key", help="JSON key, nested with '.' if required")
    args = parser.parse_args()

    value_counts = get_value_counts_from_jsonl(args.file_path, args.key)

    # Prepare data for the table
    table_data = [(value, count) for value, count in value_counts.items()]
    headers = ['Value', 'Count']

    # Print the result as a markdown table
    print(tabulate(table_data, headers, tablefmt="github"))

if __name__ == "__main__":
    main()
