import collections
import json


def main():
    # Initialize a counter for tags
    tag_counter = collections.Counter()
    total_items = 0

    # Read the jsonl file
    with open('cache/metaculus-data.jsonl', 'r') as f:
        for line in f:
            total_items += 1
            data = json.loads(line)

            # Check if the required path exists
            if (
                'extended_data' in data and
                'question' in data['extended_data'] and
                'projects' in data['extended_data']['question'] and
                'tag' in data['extended_data']['question']['projects']
            ):
                # Extract tag slugs
                tags = data['extended_data']['question']['projects']['tag']
                for tag in tags:
                    if 'slug' in tag:
                        tag_counter[tag['slug']] += 1

    # Calculate and display top 20 tags by frequency with percentages
    print(f"Total items: {total_items}")
    print("\nTop 20 tags by frequency:")
    print("Rank | Tag Slug | Count | Percentage")
    print("-" * 50)

    for rank, (tag_slug, count) in enumerate(tag_counter.most_common(20), 1):
        percentage = (count / total_items) * 100
        print(f"{rank:<4} | {tag_slug:<30} | {count:<5} | {percentage:.2f}%")

if __name__ == "__main__":
    main()
