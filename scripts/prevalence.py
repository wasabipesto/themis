import json
import argparse
from collections import defaultdict
from tabulate import tabulate

blacklist = [None, ""]
display_cutoff = 5

def load_jsonl(file_path):
    with open(file_path, "r", encoding="utf-8") as f:
        return [json.loads(line) for line in f]

def flatten_value(value, parent_key=""):
    """Flatten values whether they are dicts, lists, or simple values."""
    if isinstance(value, dict):
        return flatten_dict(value, parent_key)
    elif isinstance(value, list):
        return flatten_list(value, parent_key)
    elif value not in blacklist:
        return {parent_key: value}
    return {}

def flatten_dict(d, parent_key=""):
    """Recursively flatten a nested dictionary."""
    items = {}
    for k, v in d.items():
        new_key = f"{parent_key}.{k}" if parent_key else k
        items.update(flatten_value(v, new_key))
    return items

def flatten_list(lst, parent_key=""):
    """Recursively flatten elements in a list."""
    items = {}
    for item in lst:
        new_key = f"{parent_key}[]"
        items.update(flatten_value(item, new_key))
    return items

def calculate_key_prevalence(jsonl_data):
    key_counts = defaultdict(int)
    total_records = len(jsonl_data)

    for obj in jsonl_data:
        flattened_obj = flatten_dict(obj)
        for key in flattened_obj.keys():
            key_counts[key] += 1

    prevalence = {key: (count / total_records) * 100 for key, count in key_counts.items()}
    return prevalence

def main():
    parser = argparse.ArgumentParser(description="Calculate key prevalence in a JSONL file.")
    parser.add_argument("file_path", help="Path to the JSONL file")
    args = parser.parse_args()

    jsonl_data = load_jsonl(args.file_path)
    prevalence = calculate_key_prevalence(jsonl_data)

    table = [
        [key, f"{percent:.2f}%"]
        for key, percent in sorted(prevalence.items(), key=lambda x: -x[1])
        if percent > display_cutoff
    ]
    print(tabulate(table, headers=["Key", "Prevalence"], tablefmt="github"))

if __name__ == "__main__":
    main()
