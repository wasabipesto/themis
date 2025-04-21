# %%
import requests
import json
import pandas as pd
import matplotlib.pyplot as plt
from datetime import datetime
from dotenv import load_dotenv
import random
import os
import subprocess

load_dotenv()
postgrest_base = os.environ.get('PGRST_URL')
if not postgrest_base:
    raise ValueError("Missing required environment variable PGRST_URL")

# %%
# Get a random market with some criteria
response = requests.get(
    f"{postgrest_base}/markets",
    params={
        #"id": f"eq.kalshi:PRES-2024-DJT",
        "platform_slug": f"eq.kalshi",
        "duration_days": f"gt.90",
        "volume_usd": f"gt.10000",
    }
)
markets = response.json()
if not markets:
    raise ValueError("No markets found matching criteria")
market = random.choice(markets)

# Get the daily probabilities for this market
response = requests.get(
    f"{postgrest_base}/daily_probabilities",
    params={
        "platform_slug": f"eq.kalshi",
        "market_id": f"eq.{market['id']}",
        "order": "date.asc"
    }
)
daily_probabilities = response.json()
pg_df = pd.DataFrame(daily_probabilities)
pg_df['date'] = pd.to_datetime(pg_df['date'])

# Get the events returned from the source
platform = market.get("platform_slug")
if platform != "kalshi":
    raise ValueError(f"Platform {platform} not supported.")
file_path = f"cache/{platform}-data.jsonl"
market_id = market.get("id").split(':', 1)[1]
source_market = None
with open(file_path, 'r') as file:
    for line_number, line in enumerate(file, 1):
        if market_id in line:
            source_market = json.loads(line)
            break
if not source_market:
    raise ValueError(f"Could not find market {market_id} in file {file_path}")
source_df = pd.DataFrame(source_market.get("history"))
source_df['date'] = pd.to_datetime(source_df['created_time'])
source_df['prob'] = source_df['yes_price'] / 100
source_df = source_df.sort_values(by='date')

# Get the progress prob segments
current_dir = os.path.dirname(os.path.abspath(__file__))
segment_script = os.path.join(current_dir, "build-prob-segments.rs")
result = subprocess.run(
    [
        "rust-script",
        "--force",
        segment_script,
        "--platform",
        platform,
        "--search",
        market_id,
    ],
    capture_output=True,
    text=True
)
if result.returncode != 0:
    raise ValueError(f"Segment script failed: {result.stderr}")
segment_data = json.loads(result.stdout)
segment_df = pd.DataFrame(segment_data)
segment_df['start'] = pd.to_datetime(segment_df['start'])
#segment_df['end'] = pd.to_datetime(segment_df['end'])

# %%
# Create the plot
plt.figure(figsize=(12, 6))

plt.step(source_df['date'], source_df['prob'], '-', color='red', where='post', alpha=0.5, label='Source Events')
plt.scatter(source_df['date'], source_df['prob'], color='red', s=20, alpha=0.5)

plt.step(segment_df['start'], segment_df['prob'], '-', color='orange', where='post', alpha=0.5, label='Segments')
plt.scatter(segment_df['start'], segment_df['prob'], color='orange', s=20, alpha=0.5)

plt.step(pg_df['date'], pg_df['prob'], '-', color='blue', where='mid', alpha=0.5, label='Database')
plt.scatter(pg_df['date'], pg_df['prob'], color='blue', s=20, alpha=0.5)

plt.grid(True, alpha=0.3)
plt.title(f"{market['id']}: {market['title']}\nProbability Over Time")
plt.xlabel('Date')
plt.ylabel('Probability')
plt.ylim(0, 1)

# Rotate x-axis labels for better readability
plt.xticks(rotation=45)

# Add market resolution as horizontal line
plt.axhline(y=market['resolution'], color='r', linestyle='--', alpha=0.5,
            label=f'Resolution: {market["resolution"]}')
plt.legend()

# Adjust layout to prevent label cutoff
plt.tight_layout()

plt.show()
