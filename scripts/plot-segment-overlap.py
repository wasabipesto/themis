import json
import os
import subprocess
import matplotlib.pyplot as plt
from datetime import datetime
import matplotlib.dates as mdates
import argparse

# Set up argument parser
parser = argparse.ArgumentParser(description='Plot probability segments for a market')
parser.add_argument('--platform', type=str, required=True,
                    help='Platform name')
parser.add_argument('--search', type=str, required=True,
                    help='Search query/market ID')

# Parse arguments
args = parser.parse_args()

# Get the progress prob segments
current_dir = os.path.dirname(os.path.abspath(__file__))
segment_script = os.path.join(current_dir, "build-prob-segments.rs")
result = subprocess.run(
    [
        "rust-script",
        "--force",
        segment_script,
        "--platform",
        args.platform,
        "--search",
        args.search,
    ],
    capture_output=True,
    text=True
)
if result.returncode != 0:
    raise ValueError(f"Segment script failed: {result.stderr}")
data = json.loads(result.stdout)

# Create figure and axis
fig, ax = plt.subplots(figsize=(12, 6))

# Plot each segment
prev_end = None
for i, segment in enumerate(data):
    start_time = datetime.strptime(segment['start'], '%Y-%m-%dT%H:%M:%S.%fZ')
    end_time = datetime.strptime(segment['end'], '%Y-%m-%dT%H:%M:%S.%fZ')
    prob = segment['prob']

    # Plot horizontal line for each segment
    ax.hlines(y=i, xmin=start_time, xmax=end_time, linewidth=2)

    # Add points at start and end
    if prev_end and start_time < prev_end:
        ax.plot(start_time, i, 'o', color='red')
    prev_end = end_time

# Format x-axis
ax.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d %H:%M'))
plt.xticks(rotation=45)

# Set labels and title
plt.ylabel('Segment Index')
plt.title(f'Prob Segments for Market ({args.platform}: {args.search})')
plt.grid(True)

# Adjust layout to prevent label cutoff
plt.tight_layout()

# Show plot
plt.show()
