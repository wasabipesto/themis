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
from tabulate import tabulate
from slugify import slugify

from common import *


def load_model(model_path):
    """Load a trained model with all its metadata."""
    with open(model_path, 'rb') as f:
        model_data = pickle.load(f)

    print(f"  Loaded model: {model_data['model_name']}")
    print(f"  Target: {model_data['target_column']}")

    return model_data


def prepare_features_for_prediction(markets_df, market_embeddings_mapped, model_data):
    """
    Prepare features for prediction using the same feature preparation as training.

    Returns features and the list of valid markets (those with embeddings).
    """
    # Filter markets that have embeddings
    valid_mask = markets_df['id'].isin(market_embeddings_mapped.keys())
    valid_markets_df = markets_df[valid_mask].copy()

    if len(valid_markets_df) == 0:
        raise ValueError("No markets found with embeddings")

    # Prepare embedding features
    embedding_features = np.array([
        market_embeddings_mapped[market_id]
        for market_id in valid_markets_df['id']
    ])

    # Add platform indicators (same as training)
    market_features = []
    for _, row in valid_markets_df.iterrows():
        features = [
            1 if row.get('platform_slug') == 'manifold' else 0,
            1 if row.get('platform_slug') == 'metaculus' else 0,
            1 if row.get('platform_slug') == 'polymarket' else 0,
            1 if row.get('platform_slug') == 'kalshi' else 0,
        ]
        market_features.append(features)
    market_features = np.array(market_features)

    # Combine features
    all_features = np.hstack([embedding_features, market_features])

    # Apply PCA if it was used during training
    if model_data.get('pca') is not None:
        all_features = model_data['pca'].transform(all_features)

    return all_features, valid_markets_df


def predict_with_model(model_data, X):
    """Make predictions using a loaded model."""
    model = model_data['model']
    predictions = model.predict(X)
    return predictions


def predict_single_market(market_id, models, markets_df, market_embeddings_mapped):
    """Predict all targets for a single market."""
    # Filter to single market
    market_df = markets_df[markets_df['id'] == market_id]

    if len(market_df) == 0:
        print(f"Error: Market {market_id} not found")
        return None

    if market_id not in market_embeddings_mapped:
        print(f"Error: No embedding found for market {market_id}")
        return None

    market = market_df.iloc[0]
    predictions = {
        'market_id': market_id,
        'title': market.get('title', 'N/A'),
        'platform': market.get('platform_slug', 'N/A'),
        'url': market.get('url', 'N/A')
    }

    # Make predictions with each model
    for model_name, model_data in models.items():
        try:
            X, _ = prepare_features_for_prediction(market_df, market_embeddings_mapped, model_data)
            pred = predict_with_model(model_data, X)[0]

            target = model_data['target_column']
            predictions[f'predicted_{target}'] = pred

            # Add actual value if available
            if target in market and pd.notna(market[target]):
                predictions[f'actual_{target}'] = market[target]
                predictions[f'error_{target}'] = market[target] - pred

        except Exception as e:
            print(f"  Warning: Could not predict {model_data['target_column']}: {e}")

    return predictions


def predict_batch(models, markets_df, market_embeddings_mapped, output_dir):
    """Make predictions for all markets with all models."""
    results_by_market = {}

    for model_name, model_data in models.items():
        target = model_data['target_column']
        print(f"\nPredicting {target}...")

        try:
            X, valid_markets_df = prepare_features_for_prediction(
                markets_df, market_embeddings_mapped, model_data
            )
            predictions = predict_with_model(model_data, X)

            print(f"  Made {len(predictions)} predictions")

            # Store predictions
            for idx, (_, market) in enumerate(valid_markets_df.iterrows()):
                market_id = market['id']
                if market_id not in results_by_market:
                    results_by_market[market_id] = {
                        'market_id': market_id,
                        'title': market.get('title', 'N/A'),
                        'platform': market.get('platform_slug', 'N/A'),
                        'url': market.get('url', 'N/A')
                    }

                results_by_market[market_id][f'predicted_{target}'] = predictions[idx]

                # Add actual value if available
                if target in market and pd.notna(market[target]):
                    results_by_market[market_id][f'actual_{target}'] = market[target]
                    results_by_market[market_id][f'error_{target}'] = market[target] - predictions[idx]

        except Exception as e:
            print(f"  Error predicting {target}: {e}")

    # Convert to DataFrame and save
    if results_by_market:
        results_df = pd.DataFrame(list(results_by_market.values()))

        # Save to CSV
        output_file = f"{output_dir}/all_predictions.csv"
        results_df.to_csv(output_file, index=False)
        print(f"\n✓ All predictions saved to: {output_file}")

        return results_df

    return None


def main():
    parser = argparse.ArgumentParser(description="Make predictions using trained models")
    parser.add_argument("--cache-dir", "-cd", default="./cache",
                       help="Cache directory (default: ./cache)")
    parser.add_argument("--output-dir", "-od", default="./output",
                       help="Output directory for results (default: ./output)")
    parser.add_argument("--model-dir", "-md", default="./output/models",
                       help="Directory containing trained models (default: ./output/models)")
    parser.add_argument("--ignore-cache", action="store_true",
                       help="Ignore cache and re-download all data")
    parser.add_argument("--market-id", "-id", type=str,
                       help="Predict for a single market ID")
    parser.add_argument("--sample-size", "-ss", type=int,
                       help="Random sample size of markets to predict on")
    parser.add_argument("--sample-platform", "-sp", type=str,
                       help="Only predict on markets from specific platform")
    parser.add_argument("--model", "-m", type=str, action='append',
                       help="Specific model file(s) to use. Can specify multiple times. If not specified, uses all models in model-dir")
    parser.add_argument("--show-top", "-n", type=int, default=20,
                       help="Number of top predictions to display (default: 20)")

    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Create output directory
    os.makedirs(args.output_dir, exist_ok=True)

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

    print(f"\n✓ Loaded {len(models)} model(s)")

    # Load markets
    print("\nLoading markets...")
    markets_cache = f"{args.cache_dir}/markets.jsonl"
    markets_df = load_dataframe_from_cache(markets_cache)
    if markets_df is None or args.ignore_cache:
        markets_df = get_data_as_dataframe(f"{postgrest_base}/markets", params={"order": "id"})
        save_dataframe_to_cache(markets_cache, markets_df)
    print(f"  Loaded {len(markets_df)} markets")

    # Load market embeddings
    print("\nLoading market embeddings...")
    embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"
    market_embeddings_df = load_dataframe_from_cache(embeddings_cache)
    if market_embeddings_df is None or args.ignore_cache:
        market_embeddings_df = get_data_as_dataframe(
            f"{postgrest_base}/market_embeddings",
            params={"order": "market_id"}
        )
        market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)
        save_dataframe_to_cache(embeddings_cache, market_embeddings_df)
    else:
        if isinstance(market_embeddings_df['embedding'].iloc[0], str):
            market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)
    print(f"  Loaded {len(market_embeddings_df)} embeddings")

    # Create mapping
    market_embeddings_mapped = dict(zip(
        market_embeddings_df['market_id'],
        market_embeddings_df['embedding']
    ))

    # Calculate derived features (needed for some models)
    markets_df['score'] = calculate_market_scores(markets_df)

    # Filter markets if requested
    if args.sample_platform:
        print(f"\nFiltering to platform: {args.sample_platform}")
        markets_df = markets_df[markets_df['platform_slug'] == args.sample_platform]
        print(f"  Filtered to {len(markets_df)} markets")

    if args.sample_size and len(markets_df) > args.sample_size:
        print(f"\nSampling {args.sample_size} random markets")
        markets_df = markets_df.sample(n=args.sample_size, random_state=42)

    # Make predictions
    print("\n" + "="*80)
    print("Making Predictions")
    print("="*80)

    if args.market_id:
        # Single market prediction
        print(f"\nPredicting for market ID: {args.market_id}")
        result = predict_single_market(args.market_id, models, markets_df, market_embeddings_mapped)

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
                        print(f"    Error: {result[f'error_{target}']:.4f}")
    else:
        # Batch prediction
        results_df = predict_batch(models, markets_df, market_embeddings_mapped, args.output_dir)

        if results_df is not None:
            # Display summary statistics
            print("\n" + "="*80)
            print("Prediction Summary")
            print("="*80 + "\n")

            # Show statistics for each predicted column
            pred_cols = [col for col in results_df.columns if col.startswith('predicted_')]

            for pred_col in pred_cols:
                target = pred_col.replace('predicted_', '')
                actual_col = f'actual_{target}'

                print(f"\n{target.upper()}:")
                print(f"  Predicted range: {results_df[pred_col].min():.3f} to {results_df[pred_col].max():.3f}")
                print(f"  Predicted mean: {results_df[pred_col].mean():.3f} ± {results_df[pred_col].std():.3f}")

                if actual_col in results_df.columns:
                    error_col = f'error_{target}'
                    valid_errors = results_df[error_col].dropna()
                    if len(valid_errors) > 0:
                        mae = valid_errors.abs().mean()
                        print(f"  Mean Absolute Error: {mae:.3f}")
                        print(f"  Error range: {valid_errors.min():.3f} to {valid_errors.max():.3f}")

            # Show top predictions for each target
            print(f"\n\nTop {args.show_top} Predictions by Target:")
            print("-"*80)

            for pred_col in pred_cols:
                target = pred_col.replace('predicted_', '')
                print(f"\n{target.upper()} (Top {args.show_top} by predicted value):")

                top_markets = results_df.nlargest(args.show_top, pred_col)

                display_cols = ['title', pred_col]
                actual_col = f'actual_{target}'
                if actual_col in results_df.columns:
                    display_cols.append(actual_col)

                # Truncate titles for display
                display_df = top_markets[display_cols].copy()
                display_df['title'] = display_df['title'].str[:50]

                print(tabulate(display_df, headers='keys', tablefmt='simple',
                             showindex=False, floatfmt='.3f'))


if __name__ == "__main__":
    main()
