# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "numpy",
# ]
# ///

import json
import numpy as np

with open("cache/migration/markets.json", "r") as f:
    markets = json.load(f)

attributes = ["traders_count", "volume_usd", "duration_days"]
percentiles = [25, 50, 75]

for att in attributes:
    for pct in percentiles:
        result = np.percentile([m[att] for m in markets if m[att]], pct)
        print(f"{pct}th percentile of {att} is {result}")
