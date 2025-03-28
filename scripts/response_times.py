# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "argparse",
#     "matplotlib",
#     "numpy",
#     "requests",
#     "tqdm",
# ]
# ///
import argparse
import time
from datetime import datetime

import matplotlib.pyplot as plt
import numpy as np
import requests
from tqdm import tqdm


def test_endpoint(url, num_requests=100, headers=None, verify=True):
    """
    Test an endpoint's response time.

    Args:
        url (str): The URL to test
        num_requests (int): Number of requests to make
        headers (dict): Optional HTTP headers to include
        verify (bool): Whether to verify SSL certificates

    Returns:
        list: Response times in seconds
    """
    response_times = []

    # Use tqdm for a progress bar
    for _ in tqdm(range(num_requests), desc="Testing endpoint"):
        start_time = time.time()
        try:
            response = requests.get(url, headers=headers, timeout=30, verify=verify)
            if response.status_code != 200:
                print(f"Warning: Request returned status code {response.status_code}")
        except Exception as e:
            print(f"Error during request: {e}")
            continue

        end_time = time.time()
        response_time = end_time - start_time
        response_times.append(response_time)

    return response_times

def analyze_results(response_times):
    """Analyze and print statistics about response times"""
    if not response_times:
        print("No valid responses received")
        return

    times = np.array(response_times)

    # Calculate statistics
    median = np.median(times)
    std_dev = np.std(times)
    p99 = np.percentile(times, 99)

    # Print results
    print("\nResults:")
    print(f"Median response time: {median:.4f} seconds")
    print(f"Standard deviation: {std_dev:.4f} seconds")
    print(f"99th percentile: {p99:.4f} seconds")

    return {
        'median': median,
        'std_dev': std_dev,
        'p99': p99,
        'times': times
    }

def plot_histogram(times, output_file=None):
    """Create a histogram of response times"""
    plt.figure(figsize=(10, 6))
    plt.hist(times, bins=30, alpha=0.7, color='blue')
    plt.axvline(np.mean(times), color='red', linestyle='dashed', linewidth=1, label=f'Mean: {np.mean(times):.4f}s')
    plt.axvline(np.median(times), color='green', linestyle='dashed', linewidth=1, label=f'Median: {np.median(times):.4f}s')

    plt.title('Response Time Distribution')
    plt.xlabel('Response Time (seconds)')
    plt.ylabel('Frequency')
    plt.grid(True, alpha=0.3)
    plt.legend()

    if output_file:
        plt.savefig(output_file)
        print(f"Histogram saved to {output_file}")
    else:
        plt.show()

def main():
    parser = argparse.ArgumentParser(description='Test endpoint response time')
    parser.add_argument('url', help='URL to test')
    parser.add_argument('-n', '--num-requests', type=int, default=1000, help='Number of requests to make (default: 100)')
    parser.add_argument('--header', action='append', help='Headers in format "Key: Value"')
    parser.add_argument('--no-verify', action='store_true', help='Disable SSL certificate verification')
    parser.add_argument('--plot', action='store_true', help='Plot response time histogram')
    parser.add_argument('--output', help='Save plot to this file')

    args = parser.parse_args()

    # Process headers if provided
    headers = {}
    if args.header:
        for header in args.header:
            try:
                key, value = header.split(':', 1)
                headers[key.strip()] = value.strip()
            except ValueError:
                print(f"Invalid header format: {header}, expected 'Key: Value'")

    print(f"Testing endpoint: {args.url}")
    print(f"Number of requests: {args.num_requests}")

    response_times = test_endpoint(args.url, args.num_requests, headers, not args.no_verify)

    results = analyze_results(response_times)

    if args.plot or args.output:
        output_file = args.output
        if not output_file and args.plot:
            # Generate a default filename based on current time
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            output_file = f"response_time_{timestamp}.png"

        plot_histogram(results['times'], output_file)

if __name__ == "__main__":
    main()
