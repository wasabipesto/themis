import json
import csv
import sys
import argparse
from pathlib import Path
from collections import OrderedDict

def json_to_csv(json_file, csv_file):
    """Convert JSON file to CSV format while preserving key order.

    Args:
        json_file (str): Path to input JSON file
        csv_file (str): Path to output CSV file
    """
    try:
        # Read and parse JSON file
        with open(json_file, 'r', encoding='utf-8') as jf:
            data = json.load(jf, object_pairs_hook=OrderedDict)
    except FileNotFoundError:
        print(f"❌ Error: JSON file '{json_file}' not found.")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"❌ Error: Invalid JSON in file '{json_file}': {e}")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error reading JSON file '{json_file}': {e}")
        sys.exit(1)

    # Ensure data is a list of dictionaries
    if isinstance(data, (dict, OrderedDict)):
        data = [data]
    elif not isinstance(data, list):
        print(f"❌ Error: JSON data must be an object or array of objects, got {type(data).__name__}")
        sys.exit(1)

    # Validate that all items are dictionaries
    if not all(isinstance(item, (dict, OrderedDict)) for item in data):
        print("❌ Error: All items in JSON array must be objects")
        sys.exit(1)

    if not data:
        print("❌ Error: JSON data is empty")
        sys.exit(1)

    # Extract headers while preserving order from the first occurrence
    keys = []
    keys_seen = set()

    # First pass: collect keys in order from all objects
    for item in data:
        for key in item.keys():
            if key not in keys_seen:
                keys.append(key)
                keys_seen.add(key)

    if not keys:
        print("❌ Error: No keys found in JSON objects")
        sys.exit(1)

    try:
        # Create output directory if it doesn't exist
        Path(csv_file).parent.mkdir(parents=True, exist_ok=True)

        # Write CSV file
        with open(csv_file, 'w', newline='', encoding='utf-8') as cf:
            writer = csv.DictWriter(cf, fieldnames=keys)
            writer.writeheader()
            writer.writerows(data)

        print(f"✅ Successfully converted {len(data)} records with {len(keys)} columns")

    except Exception as e:
        print(f"❌ Error writing CSV file '{csv_file}': {e}")
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser(
        description='Convert JSON file to CSV format while preserving key order',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s data.json output.csv
  %(prog)s input/users.json output/users.csv
  %(prog)s --help

The script preserves the order of keys as they appear in the JSON objects.
For arrays of objects, keys from all objects are collected in order of first appearance.
        """
    )
    parser.add_argument(
        'json_file',
        help='Path to input JSON file'
    )
    parser.add_argument(
        'csv_file',
        help='Path to output CSV file'
    )
    parser.add_argument(
        '--preserve-order',
        action='store_true',
        default=True,
        help='Preserve the order of keys from JSON (default: True)'
    )
    parser.add_argument(
        '--version',
        action='version',
        version='%(prog)s 2.0'
    )

    args = parser.parse_args()

    # Validate input file exists
    if not Path(args.json_file).exists():
        print(f"❌ Error: Input file '{args.json_file}' does not exist.")
        sys.exit(1)

    # Convert JSON to CSV
    json_to_csv(args.json_file, args.csv_file)
    print(f"✅ Converted {args.json_file} → {args.csv_file}")

if __name__ == "__main__":
    main()
