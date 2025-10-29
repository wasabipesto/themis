#!/usr/bin/env python3
"""
Prediction Script for Trained Models

Load trained models and make predictions on new market data using embeddings.
Supports predictions using one or more models on markets.
"""

import os
import json
import argparse
import pickle
import pandas as pd
import numpy as np
from dotenv import load_dotenv

from common import *

def load_model(model_path):
    """Load a trained model with all its metadata."""
    with open(model_path, 'rb') as f:
        model_data = pickle.load(f)

    print(f"  Loaded model: {model_data['model_name']}, Predicting: {model_data['target_column']}")
    return model_data


def prepare_features_for_prediction(market, model_data):
    """
    Prepare features for prediction using the same feature preparation as training.
    """
    # Add platform indicators (same order as training)
    platform_features = [
        1 if market.get('platform_slug') == 'manifold' else 0,
        1 if market.get('platform_slug') == 'metaculus' else 0,
        1 if market.get('platform_slug') == 'polymarket' else 0,
        1 if market.get('platform_slug') == 'kalshi' else 0,
    ]
    all_features = np.append(platform_features, market["embeddings"])
    all_features = all_features.reshape(1, -1)

    # Apply PCA if it was used during training
    if model_data.get('pca') is not None:
        all_features = model_data['pca'].transform(all_features)

    return all_features

def predict_single_market(market, models):
    """Predict all targets for a single market."""

    predictions = {
        'market_id': market['id'],
        'title': market.get('title', 'N/A'),
        'platform': market.get('platform_slug', 'N/A'),
        'url': market.get('url', 'N/A')
    }

    # Make predictions with each model
    for model_name, model_data in models.items():
        features = prepare_features_for_prediction(market, model_data)
        try:
            prediction = model_data['model'].predict(features)[0]

            target_column = model_data['target_column']
            predictions[f'predicted_{target_column}'] = prediction

            # Add actual value if available
            if target_column in market and pd.notna(market[target_column]):
                predictions[f'actual_{target_column}'] = market[target_column]
                #predictions[f'error_{target_column}'] = market[target_column] - prediction

        except Exception as e:
            print(f"  Warning: Could not predict {model_data['target_column']}: {e}")

    return predictions


def main():
    parser = argparse.ArgumentParser(description="Make predictions using trained models")
    parser.add_argument("--model-dir", "-md", default="./output/models",
                       help="Directory containing trained models (default: ./output/models)")
    parser.add_argument("--market-id", "-id", type=str,
                       help="Predict for a single market ID")
    parser.add_argument("--model", "-m", type=str, action='append',
                       help="Specific model file(s) to use. Can specify multiple times. If not specified, uses all models in model-dir")

    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Load models
    print("\nLoading models...")
    models = {}

    if args.model:
        # Load specific models
        for model_file in args.model:
            if not os.path.isabs(model_file):
                model_path = os.path.join(args.model_dir, model_file)
            else:
                model_path = model_file

            if not os.path.exists(model_path):
                print(f"  Warning: Model file not found: {model_path}")
                continue

            model_data = load_model(model_path)
            key = f"{model_data['model_name']}-{model_data['target_column']}"
            models[key] = model_data
    else:
        # Load all models from model directory
        if not os.path.exists(args.model_dir):
            print(f"Error: Model directory not found: {args.model_dir}")
            return

        model_files = [f for f in os.listdir(args.model_dir) if f.endswith('.pkl')]
        if not model_files:
            print(f"Error: No model files found in {args.model_dir}")
            return

        for model_file in model_files:
            model_path = os.path.join(args.model_dir, model_file)
            try:
                model_data = load_model(model_path)
                key = f"{model_data['model_name']}-{model_data['target_column']}"
                models[key] = model_data
            except Exception as e:
                print(f"  Warning: Could not load {model_file}: {e}")

    if not models:
        print("Error: No models loaded")
        return

    # Load market
    print("\nDownloading market data...", end="")
    market = get_single_item(f"{postgrest_base}/markets", {}, {"id": f"eq.{args.market_id}"})[0]
    print(" done.")

    # Load market embeddings
    print("Downloading market embeddings...", end="")
    market_with_embeddings = market
    market_embeddings = get_single_item(f"{postgrest_base}/market_embeddings", {}, {"market_id": f"eq.{args.market_id}"})[0]
    market_with_embeddings["embeddings"] = np.array([json.loads(market_embeddings["embedding"])])
    print(" done.")
    print(f"Embeddings shape: {market_with_embeddings["embeddings"].shape}")

    # Make predictions
    print("\n" + "="*80)
    print("Making Predictions")
    print("="*80)

    print(f"\nPredicting for market {market['id']}:")
    result = predict_single_market(market_with_embeddings, models)
    if result:
        print("\nPredictions:")
        print(f"  Market: {result['title']}")
        print(f"  Platform: {result['platform']}")
        print(f"  URL: {result['url']}")
        print()

        for key, value in result.items():
            if key.startswith('predicted_'):
                target = key.replace('predicted_', '')
                print(f"  {target}:")
                print(f"    Predicted: {value:.4f}")
                if f'actual_{target}' in result:
                    print(f"    Actual: {result[f'actual_{target}']:.4f}")


if __name__ == "__main__":
    main()
