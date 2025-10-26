#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "numpy",
#     "scikit-learn",
#     "pandas",
#     "argparse",
#     "tabulate",
# ]
# ///

import pickle
import argparse
import json
import numpy as np
import pandas as pd
from tabulate import tabulate

def load_model(model_path):
    """Load saved resolution prediction model."""
    with open(model_path, 'rb') as f:
        model_data = pickle.load(f)
    return model_data

def predict_from_embeddings(model_data, embeddings, market_features=None):
    """Make resolution predictions from embeddings."""
    # Prepare features
    if market_features is not None:
        features = np.hstack([embeddings, market_features])
    else:
        features = embeddings

    # Apply scaling if model was trained with scaling
    if model_data.get('scaler') is not None:
        features = model_data['scaler'].transform(features)

    # Make predictions
    predictions = model_data['model'].predict(features)

    return predictions

def predict_from_market_data(model_data, markets, embeddings_dict):
    """Make predictions for a list of markets with their embeddings."""
    valid_markets = []
    embeddings = []
    market_features = []

    for market in markets:
        if market["id"] not in embeddings_dict:
            continue

        valid_markets.append(market)
        embeddings.append(embeddings_dict[market["id"]])

        # Prepare market features (should match training features)
        features = [
            market.get("volume_usd", 0) or 0,
            market.get("traders_count", 0) or 0,
            market.get("duration_days", 0) or 0,
            len(market.get("title", "")),
            1 if market.get("platform_slug") == "manifold" else 0,
            1 if market.get("platform_slug") == "metaculus" else 0,
            1 if market.get("platform_slug") == "polymarket" else 0,
        ]
        market_features.append(features)

    if not valid_markets:
        return [], []

    embeddings = np.array(embeddings)
    market_features = np.array(market_features)

    # Check if model expects market features
    expected_features = len(model_data.get('feature_names', []))
    embedding_dim = embeddings.shape[1]

    if expected_features > embedding_dim:
        # Model expects market features too
        predictions = predict_from_embeddings(model_data, embeddings, market_features)
    else:
        # Model only uses embeddings
        predictions = predict_from_embeddings(model_data, embeddings)

    return valid_markets, predictions

def load_embeddings_from_file(embeddings_file):
    """Load embeddings from JSON file."""
    with open(embeddings_file, 'r') as f:
        embeddings_data = json.load(f)

    if isinstance(embeddings_data, list):
        # Format: [{"market_id": X, "embedding": [...]}, ...]
        return {item["market_id"]: item["embedding"] for item in embeddings_data}
    else:
        # Format: {"market_id": [...], "market_id": [...], ...}
        return embeddings_data

def main():
    parser = argparse.ArgumentParser(description="Predict market resolution using trained model")
    parser.add_argument("--model", "-m", required=True,
                       help="Path to saved model file (.pkl)")
    parser.add_argument("--markets", required=True,
                       help="Path to markets JSON file")
    parser.add_argument("--embeddings", "-e", required=True,
                       help="Path to embeddings JSON file")
    parser.add_argument("--output", "-o", default=None,
                       help="Output CSV file for predictions (default: print to console)")
    parser.add_argument("--top-n", "-n", type=int, default=20,
                       help="Show top N predictions (default: 20)")
    parser.add_argument("--filter-platform", "-fp", default=None,
                       help="Filter markets by platform slug")
    parser.add_argument("--min-volume", "-mv", type=float, default=0,
                       help="Minimum volume USD filter (default: 0)")

    args = parser.parse_args()

    # Load model
    print(f"Loading model from {args.model}...")
    try:
        model_data = load_model(args.model)
        print(f"Loaded {model_data.get('model_name', 'Unknown')} model")
    except Exception as e:
        print(f"Error loading model: {e}")
        return 1

    # Load markets
    print(f"Loading markets from {args.markets}...")
    with open(args.markets, 'r') as f:
        markets = json.load(f)
    print(f"Loaded {len(markets)} markets")

    # Load embeddings
    print(f"Loading embeddings from {args.embeddings}...")
    embeddings_dict = load_embeddings_from_file(args.embeddings)
    print(f"Loaded embeddings for {len(embeddings_dict)} markets")

    # Filter markets if requested
    original_count = len(markets)

    if args.filter_platform:
        markets = [m for m in markets if m.get("platform_slug") == args.filter_platform]
        print(f"Filtered to {len(markets)} markets from platform '{args.filter_platform}'")

    if args.min_volume > 0:
        markets = [m for m in markets if (m.get("volume_usd", 0) or 0) >= args.min_volume]
        print(f"Filtered to {len(markets)} markets with volume >= ${args.min_volume}")

    # Make predictions
    print("Making predictions...")
    valid_markets, predictions = predict_from_market_data(model_data, markets, embeddings_dict)

    if not valid_markets:
        print("No valid markets found for prediction!")
        return 1

    print(f"Made predictions for {len(valid_markets)} markets")

    # Prepare results
    results = []
    for market, pred in zip(valid_markets, predictions):
        results.append({
            'market_id': market['id'],
            'title': market.get('title', ''),
            'platform': market.get('platform_slug', ''),
            'volume_usd': market.get('volume_usd', 0) or 0,
            'traders_count': market.get('traders_count', 0) or 0,
            'predicted_resolution': pred,
            'actual_resolution': market.get('resolution', None)
        })

    # Sort by predicted resolution (descending)
    results.sort(key=lambda x: x['predicted_resolution'], reverse=True)

    # Display results
    print(f"\n{'='*80}")
    print("RESOLUTION PREDICTIONS")
    print(f"{'='*80}")

    # Summary statistics
    predictions_array = np.array(predictions)
    print(f"\nPrediction Statistics:")
    print(f"  Mean: {predictions_array.mean():.3f}")
    print(f"  Std:  {predictions_array.std():.3f}")
    print(f"  Min:  {predictions_array.min():.3f}")
    print(f"  Max:  {predictions_array.max():.3f}")

    # Top predictions
    print(f"\nTop {args.top_n} Highest Predicted Resolutions:")
    table_data = []
    for i, result in enumerate(results[:args.top_n]):
        title = result['title'][:50] + "..." if len(result['title']) > 50 else result['title']
        table_data.append([
            result['market_id'],
            title,
            result['platform'],
            f"${result['volume_usd']:.0f}",
            result['traders_count'],
            f"{result['predicted_resolution']:.3f}",
            f"{result['actual_resolution']:.3f}" if result['actual_resolution'] is not None else "N/A"
        ])

    print(tabulate(
        table_data,
        headers=['ID', 'Title', 'Platform', 'Volume', 'Traders', 'Pred Res', 'Actual Res'],
        tablefmt="github"
    ))

    # Bottom predictions
    print(f"\nTop {args.top_n} Lowest Predicted Resolutions:")
    table_data = []
    for i, result in enumerate(results[-args.top_n:]):
        title = result['title'][:50] + "..." if len(result['title']) > 50 else result['title']
        table_data.append([
            result['market_id'],
            title,
            result['platform'],
            f"${result['volume_usd']:.0f}",
            result['traders_count'],
            f"{result['predicted_resolution']:.3f}",
            f"{result['actual_resolution']:.3f}" if result['actual_resolution'] is not None else "N/A"
        ])

    print(tabulate(
        table_data,
        headers=['ID', 'Title', 'Platform', 'Volume', 'Traders', 'Pred Res', 'Actual Res'],
        tablefmt="github"
    ))

    # Accuracy analysis (if actual resolutions available)
    actual_resolutions = [r['actual_resolution'] for r in results if r['actual_resolution'] is not None]
    if actual_resolutions:
        predicted_for_actual = [results[i]['predicted_resolution'] for i, r in enumerate(results) if r['actual_resolution'] is not None]

        mae = np.mean(np.abs(np.array(actual_resolutions) - np.array(predicted_for_actual)))
        mse = np.mean((np.array(actual_resolutions) - np.array(predicted_for_actual))**2)

        print(f"\nAccuracy on markets with known resolutions ({len(actual_resolutions)} markets):")
        print(f"  MAE: {mae:.3f}")
        print(f"  MSE: {mse:.3f}")
        print(f"  RMSE: {np.sqrt(mse):.3f}")

    # Save to CSV if requested
    if args.output:
        df = pd.DataFrame(results)
        df.to_csv(args.output, index=False)
        print(f"\nPredictions saved to {args.output}")

    return 0

if __name__ == "__main__":
    exit(main())
