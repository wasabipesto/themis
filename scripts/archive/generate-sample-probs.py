import json
from datetime import datetime, timedelta
import random
import math

def create_trend_function(platform):
    # Different parameters for each platform
    if platform == "Manifold":
        # More volatile, higher frequency changes
        return lambda day, total: (
            0.20 * math.sin(2.5 * math.pi * day/total) +
            0.08 * math.sin(8 * math.pi * day/total)
        )
    elif platform == "Polymarket":
        # Slower, wave-like movements
        return lambda day, total: (
            0.15 * math.sin(1.5 * math.pi * day/total) +
            0.05 * math.sin(4 * math.pi * day/total)
        )
    elif platform == "Kalshi":
        # Sharp trends with quick reversals
        return lambda day, total: (
            0.12 * math.sin(3 * math.pi * day/total) +
            0.07 * math.sin(9 * math.pi * day/total) +
            0.03 * math.sin(15 * math.pi * day/total)
        )
    else:  # Metaculus
        # More gradual, smoother changes
        return lambda day, total: (
            0.10 * math.sin(math.pi * day/total) +
            0.04 * math.sin(3 * math.pi * day/total) +
            0.02 * math.sin(7 * math.pi * day/total)
        )

def generate_data(platform):
    start_date = datetime(2024, 1, 1)
    end_date = datetime(2024, 11, 5)
    data = []

    # Calculate total number of days
    total_days = (end_date - start_date).days

    # Get platform-specific trend function
    trend = create_trend_function(platform)

    for day in range(total_days + 1):
        current_date = start_date + timedelta(days=day)

        # Calculate base probability (linear increase from 0.10 to 1.00)
        base_prob = 0.10 + (0.90 * day / total_days)

        # Add platform-specific trending component
        trend_factor = trend(day, total_days)

        # Add smaller random noise
        random_factor = random.uniform(-0.02, 0.02)

        # Combine all factors
        prob = min(1.0, max(0.0, base_prob + trend_factor + random_factor))

        data_point = {
            "platform": platform,
            "date": current_date.strftime("%Y-%m-%d"),
            "prob": round(prob, 2)
        }

        data.append(data_point)

    return data

# Generate and save the data
data = generate_data("Manifold") + \
    generate_data("Polymarket") + \
    generate_data("Kalshi") + \
    generate_data("Metaculus")
with open('probability_data.json', 'w') as f:
    json.dump(data, f, indent=2)

print(json.dumps(data, indent=2))
