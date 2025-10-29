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
import ollama
import pandas as pd
import numpy as np
from dotenv import load_dotenv

from common import *

def load_model(model_path):
    """Load a trained model with all its metadata."""
    with open(model_path, 'rb') as f:
        model_data = pickle.load(f)

    print(f"  Loaded model freom {model_path}, predicts {model_data['target_column']} via {model_data['model_name']}")
    return model_data


def load_all_models(selected_models, model_dir):
    """Load all models or specified models from the model directory."""
    models = {}
    if selected_models:
        # Load specific models
        for model_file in selected_models:
            if not os.path.isabs(model_file):
                model_path = os.path.join(model_dir, model_file)
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
        if not os.path.exists(model_dir):
            print(f"Error: Model directory not found: {model_dir}")
            return

        model_files = [f for f in os.listdir(model_dir) if f.endswith('.pkl')]
        if not model_files:
            print(f"Error: No model files found in {model_dir}")
            return

        for model_file in model_files:
            model_path = os.path.join(model_dir, model_file)
            try:
                model_data = load_model(model_path)
                key = f"{model_data['model_name']}-{model_data['target_column']}"
                models[key] = model_data
            except Exception as e:
                print(f"  Warning: Could not load {model_file}: {e}")

    return models


def get_market_from_db(market_id):
    """Load market data from the database."""
    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Load market
    print("\nDownloading market data...", end="")
    market = get_single_item(f"{postgrest_base}/markets", {}, {"id": f"eq.{market_id}"})[0]
    print(" done.")

    # Load market embeddings
    print("Downloading market embeddings...", end="")
    market_embeddings = get_single_item(f"{postgrest_base}/market_embeddings", {}, {"market_id": f"eq.{market_id}"})[0]
    market["embeddings"] = json.loads(market_embeddings["embedding"])
    print(" done.")

    return market


def generate_embeddings(title, description):
    """Generate embeddings for a given title and description."""
    prompt = f"{title}\n {description}"
    response = ollama.embeddings(
        model="nomic-embed-text",
        prompt=prompt,
    )
    embedding = response.get("embedding")
    if not embedding or len(embedding) != 768:
        raise Exception(f"Invalid embedding response: {response}")

    return embedding


def get_market_from_manifold(url):
    """Loads a live market from Manifold."""
    print(f"\nLoading market from Manifold...", end="")
    slug = url.split("/")[-1]
    manifold_api_url = f"https://api.manifold.markets/v0/slug/{slug}"
    raw_market = get_single_item(manifold_api_url)
    embeddings = generate_embeddings(raw_market["question"], raw_market["textDescription"])
    market = {
        "id": f"manifold:{raw_market["id"]}",
        "platform_slug": "manifold",
        "title": raw_market["question"],
        "description": raw_market["textDescription"],
        "url": raw_market["url"],
        "embeddings": embeddings
    }
    print(" done.")

    return market


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
    all_features = np.append(market["embeddings"], platform_features)
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
        try:
            features = prepare_features_for_prediction(market, model_data)
        except Exception as e:
            print(f"  Warning: Could not prepare features for {model_name}: {e}")
            continue

        try:
            prediction = model_data['model'].predict(features)[0]

            target_column = model_data['target_column']
            predictions[f'predicted_{target_column}'] = prediction

            # Add actual value if available
            if target_column in market and pd.notna(market[target_column]):
                predictions[f'actual_{target_column}'] = market[target_column]

        except Exception as e:
            print(f"  Warning: Could not predict {model_data['target_column']}: {e}")

    return predictions


def main():
    parser = argparse.ArgumentParser(description="Make predictions using trained models")
    parser.add_argument("--model-dir", "-md", default="./output/models",
                       help="Directory containing trained models (default: ./output/models)")
    parser.add_argument("--market-id", "-id", type=str,
                       help="Predict for a single market ID")
    parser.add_argument("--live-url", "-url", type=str,
                       help="Predict for a live market")
    parser.add_argument("--models", "-m", type=str, action='append',
                       help="Specific model file(s) to use. Can specify multiple times. If not specified, uses all models in model-dir")

    args = parser.parse_args()

    # Load models
    print("\nLoading models...")
    models = load_all_models(args.models, args.model_dir)
    if not models:
        print("Error: No models loaded")
        return

    # Get market information
    if args.market_id:
        market = get_market_from_db(args.market_id)
    elif args.live_url:
        if "manifold.markets" in args.live_url:
            market = get_market_from_manifold(args.live_url)
    else:
        print("Error: No market specified")
        return

    print(f"Predicting for market {market['id']}...", end="")
    result = predict_single_market(market, models)
    print(" done.")

    if result:
        print(f"\nMarket: {market['id']}")
        print(f"  Title: {market['title']}")
        print(f"  URL: {market['url']}")

        print("\nPredictions:")
        for key, value in result.items():
            if key.startswith('predicted_'):
                target = key.replace('predicted_', '')
                if target == "high_volume" and value > 0.6:
                    print("  This market will have a high trade volume.")
                if target == "high_volume" and value < 0.4:
                    print("  This market will have a low trade volume.")
                if target == "high_traders" and value > 0.6:
                    print("  This market will attract a lot of traders.")
                if target == "high_traders" and value < 0.4:
                    print("  This market will not attract many traders.")
                if target == "resolution" and value > 0.6:
                    print("  This market will resolve YES.")
                if target == "resolution" and value < 0.4:
                    print("  This market will resolve NO.")

        print("\nDetails:")
        for key, value in result.items():
            if key.startswith('predicted_'):
                target = key.replace('predicted_', '')
                print(f"  {target}:")
                print(f"    Predicted: {value:.4f}")
                if f'actual_{target}' in result:
                    print(f"    Actual: {result[f'actual_{target}']:.4f}")


if __name__ == "__main__":
    main()
