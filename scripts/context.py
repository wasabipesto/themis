#!/usr/bin/env python3
import json
import sys


def show_context(filename, line_num, col_num, context_size):
    try:
        # Convert to 0-based indexing
        line_num = int(line_num) - 1
        col_num = int(col_num) - 1

        # Open the file and get the specified line
        with open(filename, 'r', encoding='utf-8') as file:
            for i, line in enumerate(file):
                if i == line_num:
                    break
            else:
                print(f"Error: Line {line_num + 1} not found in file")
                return

        # Calculate the start and end positions for context
        start = max(0, col_num - context_size)
        end = min(len(line), col_num + context_size + 1)

        # Extract the context
        context_before = line[start:col_num]
        char_at_col = line[col_num] if col_num < len(line) else ''
        context_after = line[col_num + 1:end]

        # Display the context
        print(f"Line {line_num + 1}, Column {col_num + 1}:")
        print(f"{context_before}{char_at_col}{context_after}")
        print(f"{' ' * len(context_before)}^")

        # Try to parse as JSON and show the structure around this position (optional)
        try:
            _data = json.loads(line)
            print("This line contains valid JSON.")
        except json.JSONDecodeError:
            print("This line is not valid JSON or the error is within the JSON structure.")

    except FileNotFoundError:
        print(f"Error: File '{filename}' not found")
    except ValueError:
        print("Error: Line and column numbers must be integers")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("Usage: python script.py <filename> <line_number> <column_number> [context_size]")
        sys.exit(1)

    filename = sys.argv[1]
    line_num = sys.argv[2]
    col_num = sys.argv[3]

    context_size = 40
    if len(sys.argv) >= 5:
        context_size = int(sys.argv[4])

    show_context(filename, line_num, col_num, context_size)
