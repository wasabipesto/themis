import os
import json
import time
from slugify import slugify
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from dotenv import load_dotenv
from tqdm import trange, tqdm
from tabulate import tabulate
import argparse
import math
from sklearn.model_selection import train_test_split, cross_val_score, GridSearchCV
from sklearn.ensemble import RandomForestRegressor, GradientBoostingRegressor
from sklearn.linear_model import LinearRegression, Ridge, Lasso, ElasticNet
from sklearn.svm import SVR
from sklearn.neural_network import MLPRegressor
from sklearn.metrics import mean_squared_error, mean_absolute_error, r2_score
from sklearn.preprocessing import StandardScaler, RobustScaler
from sklearn.decomposition import PCA
import pickle

from common import *

def prepare_features(markets_df, market_embeddings_mapped, target_column='resolution'):
    """Prepare feature matrix from market embeddings and optional market metadata."""
    # Filter markets that have both embeddings and valid target values
    valid_mask = (
        markets_df['id'].isin(market_embeddings_mapped.keys()) &
        markets_df[target_column].notna()
    )

    # Handle numeric columns
    if markets_df[target_column].dtype in ['float64', 'int64']:
        valid_mask = valid_mask & ~np.isnan(markets_df[target_column])

    valid_markets_df = markets_df[valid_mask].copy()

    print(f"Found {len(valid_markets_df)} markets with valid embeddings and {target_column} values")

    if len(valid_markets_df) == 0:
        raise ValueError(f"No markets found with both embeddings and {target_column} values")

    # Prepare embedding features
    embedding_features = np.array([market_embeddings_mapped[market_id] for market_id in valid_markets_df['id']])

    # Prepare targets
    targets = valid_markets_df[target_column].values

    # Add indicator for platform
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

    # Combine embedding and market features
    all_features = np.hstack([embedding_features, market_features])

    feature_names = [f"emb_{i}" for i in range(embedding_features.shape[1])] + [
        "is_manifold", "is_metaculus", "is_polymarket", "is_kalshi"
    ]

    return all_features, targets, valid_markets_df.to_dict('records'), feature_names

def train_models(X_train, y_train, X_test, y_test, output_dir, target_column='resolution'):
    """Train multiple models and compare their performance."""
    models = {
        'Linear Regression': LinearRegression(),
        'Ridge': Ridge(alpha=1.0),
        'Lasso': Lasso(alpha=0.1, max_iter=2000),
        'ElasticNet': ElasticNet(alpha=0.1, l1_ratio=0.5, max_iter=2000),
        'Random Forest': RandomForestRegressor(n_estimators=200, min_samples_split=10, min_samples_leaf=4, random_state=42, n_jobs=-1),
        #'Gradient Boosting': GradientBoostingRegressor(n_estimators=100, random_state=42),
        #'SVR': SVR(kernel='rbf', C=1.0),
        #'MLP': MLPRegressor(hidden_layer_sizes=(100, 50), max_iter=1000, random_state=42)
    }

    results = {}
    trained_models = {}

    for name, model in models.items():
        print(f"Training {name}...", end="")
        start_time = time.time()
        try:
            # Train model
            model.fit(X_train, y_train)
            trained_models[name] = model

            # Make predictions
            y_pred_train = model.predict(X_train)
            y_pred_test = model.predict(X_test)

            # Calculate metrics
            results[name] = {
                'train_mse': mean_squared_error(y_train, y_pred_train),
                'test_mse': mean_squared_error(y_test, y_pred_test),
                'train_mae': mean_absolute_error(y_train, y_pred_train),
                'test_mae': mean_absolute_error(y_test, y_pred_test),
                'train_r2': r2_score(y_train, y_pred_train),
                'test_r2': r2_score(y_test, y_pred_test),
                'predictions': y_pred_test
            }

            # Print timing information
            end_time = time.time()
            duration = end_time - start_time
            save_model(model, None, None, output_dir, name, target_column)
            print(f" Completed in {duration:.2f} seconds (R² = {results[name]['test_r2']:.4f})")

        except Exception as e:
            print(f"Error training {name}: {e}")
            continue

    return results, trained_models

def plot_results(results, y_test, output_dir):
    """Plot model comparison and prediction accuracy."""
    # Model comparison plot
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 10))

    model_names = list(results.keys())

    # Test MSE comparison
    test_mses = [results[name]['test_mse'] for name in model_names]
    ax1.bar(model_names, test_mses)
    ax1.set_title('Test MSE by Model')
    ax1.set_ylabel('MSE')
    ax1.tick_params(axis='x', rotation=45)

    # Test R² comparison
    test_r2s = [results[name]['test_r2'] for name in model_names]
    ax2.bar(model_names, test_r2s)
    ax2.set_title('Test R² by Model')
    ax2.set_ylabel('R²')
    ax2.tick_params(axis='x', rotation=45)

    # Test MAE comparison
    test_maes = [results[name]['test_mae'] for name in model_names]
    ax3.bar(model_names, test_maes)
    ax3.set_title('Test MAE by Model')
    ax3.set_ylabel('MAE')
    ax3.tick_params(axis='x', rotation=45)

    # Best model predictions vs actual
    best_model = max(model_names, key=lambda x: results[x]['test_r2'])
    predictions = results[best_model]['predictions']

    ax4.scatter(y_test, predictions, alpha=0.6)
    min_val = min(y_test.min(), predictions.min())
    max_val = max(y_test.max(), predictions.max())
    ax4.plot([min_val, max_val], [min_val, max_val], 'r--', lw=2)
    ax4.set_xlabel('Actual Resolution')
    ax4.set_ylabel('Predicted Resolution')
    ax4.set_title(f'Best Model: {best_model}\nR² = {results[best_model]["test_r2"]:.3f}')

    plt.tight_layout()
    plt.savefig(f"{output_dir}/prediction_resolution_results.png", dpi=300, bbox_inches='tight')
    plt.close()

    # Residual plot for best model
    residuals = y_test - predictions
    plt.figure(figsize=(10, 6))
    plt.scatter(predictions, residuals, alpha=0.6)
    plt.axhline(y=0, color='r', linestyle='--')
    plt.xlabel('Predicted Resolution')
    plt.ylabel('Residuals')
    plt.title(f'Residual Plot - {best_model}')
    plt.savefig(f"{output_dir}/prediction_resolution_residual_plot.png", dpi=300, bbox_inches='tight')
    plt.close()

def analyze_feature_importance(model, feature_names, output_dir):
    """Analyze and plot feature importance for tree-based models."""
    if hasattr(model, 'feature_importances_'):
        importances = model.feature_importances_
        indices = np.argsort(importances)[::-1]
        top_n = min(50, len(importances))

        plt.figure(figsize=(12, 8))
        plt.title("Feature Importance")
        plt.bar(range(top_n),
                importances[indices[:top_n]])
        plt.xticks(range(top_n),
                  [feature_names[i] for i in indices[:top_n]], rotation=45, ha='right')
        plt.ylabel('Importance')
        plt.tight_layout()
        plt.savefig(f"{output_dir}/prediction_resolution_feature_importance.png", dpi=300, bbox_inches='tight')
        plt.close()

        # Print top features
        top_n = 5
        print(f"\nTop {top_n} Most Important Features:")
        for i in range(min(top_n, len(importances))):
            idx = indices[i]
            print(f"{i+1:2d}. {feature_names[idx]:25s}: {importances[idx]:.4f}")

def hyperparameter_tuning(X_train, y_train, model_name):
    """Perform hyperparameter tuning for the specified model."""
    print(f"Performing hyperparameter tuning for {model_name}...")

    if model_name == 'Random Forest':
        model = RandomForestRegressor(random_state=42, n_jobs=-1)
        param_grid = {
            'n_estimators': [50, 100, 200],
            'max_depth': [10, 20, None],
            'min_samples_split': [2, 5, 10],
            'min_samples_leaf': [1, 2, 4]
        }
    elif model_name == 'Gradient Boosting':
        model = GradientBoostingRegressor(random_state=42)
        param_grid = {
            'n_estimators': [50, 100, 200],
            'learning_rate': [0.01, 0.1, 0.2],
            'max_depth': [3, 5, 7],
            'subsample': [0.8, 0.9, 1.0]
        }
    else:
        raise ValueError(f"Hyperparameter tuning not implemented for {model_name}")

    grid_search = GridSearchCV(model, param_grid, cv=5, scoring='r2', n_jobs=-1)
    grid_search.fit(X_train, y_train)

    print(f"Best parameters: {grid_search.best_params_}")
    print(f"Best cross-validation score: {grid_search.best_score_:.4f}")

    return grid_search.best_estimator_

def save_model(model, scaler, feature_names, output_dir, model_name, target_column='resolution'):
    """Save trained model and preprocessing components."""
    model_data = {
        'model': model,
        'scaler': scaler,
        'feature_names': feature_names,
        'model_name': model_name,
        'target_column': target_column
    }

    filename = f"{slugify(model_name)}-{slugify(target_column)}-{int(time.time())}"

    with open(f"{output_dir}/models/{filename}.pkl", 'wb') as f:
        pickle.dump(model_data, f)

def main():
    parser = argparse.ArgumentParser(description="Predict market values using embeddings")
    parser.add_argument("--cache-dir", "-cd", default="./cache",
                       help="Cache directory (default: ./cache)")
    parser.add_argument("--output-dir", "-od", default="./output",
                       help="Output directory for results (default: ./output)")
    parser.add_argument("--ignore-cache", action="store_true",
                       help="Ignore cache and re-download all data")
    parser.add_argument("--pca-dim", "-d", type=int, default=50,
                       help="PCA dimensionality reduction (default: 50, 0 to skip)")
    parser.add_argument("--include-market-features", action="store_true",
                       help="Include market metadata features alongside embeddings")
    parser.add_argument("--test-size", type=float, default=0.2,
                       help="Test set size (default: 0.2)")
    parser.add_argument("--tune-hyperparameters", action="store_true",
                       help="Perform hyperparameter tuning on best model")
    parser.add_argument("--scale-features", action="store_true",
                       help="Apply feature scaling")
    parser.add_argument("--sample-platform", "-sp", type=str,
                       help="Sample markets from specific platform slug")
    parser.add_argument("--sample-size", "-ss", type=int,
                       help="Random sample size of markets to use")
    parser.add_argument("--target", "-t", type=str, default='resolution',
                       help="Target column to predict (default: resolution). Examples: resolution, volume_usd, traders_count")

    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Create directories
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)
    os.makedirs(f"{args.output_dir}/models", exist_ok=True)

    # Cache file names
    markets_cache = f"{args.cache_dir}/markets.jsonl"
    embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"

    # Load markets
    print("Loading markets...")
    markets_df = load_dataframe_from_cache(markets_cache)
    if markets_df is None or args.ignore_cache:
        markets_df = get_data_as_dataframe(f"{postgrest_base}/markets", params={"order": "id"})
        save_dataframe_to_cache(markets_cache, markets_df)

    # Load market embeddings
    print("Loading market embeddings...")
    market_embeddings_df = load_dataframe_from_cache(embeddings_cache)
    if market_embeddings_df is None or args.ignore_cache:
        market_embeddings_df = get_data_as_dataframe(f"{postgrest_base}/market_embeddings", params={"order": "market_id"})
        # Parse embeddings from JSON strings
        market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)
        save_dataframe_to_cache(embeddings_cache, market_embeddings_df)
    else:
        # If loaded from cache, embeddings might already be lists or need parsing
        if isinstance(market_embeddings_df['embedding'].iloc[0], str):
            market_embeddings_df['embedding'] = market_embeddings_df['embedding'].apply(json.loads)

    # Create mapping for efficient lookup
    market_embeddings_mapped = dict(zip(market_embeddings_df['market_id'], market_embeddings_df['embedding']))

    # Sample down markets if requested
    if args.sample_platform:
        print(f"Filtering markets by platform: {args.sample_platform}")
        markets_df = markets_df[markets_df['platform_slug'] == args.sample_platform]
        print(f"Found {len(markets_df)} markets for platform {args.sample_platform}")

    if args.sample_size and len(markets_df) > args.sample_size:
        print(f"Randomly sampling {args.sample_size} markets from {len(markets_df)} total")
        markets_df = markets_df.sample(n=args.sample_size, random_state=42)
        print(f"Using {len(markets_df)} sampled markets")

    # Calculate market scores and some indicators
    markets_df['score'] = calculate_market_scores(markets_df)
    markets_df['high_score'] = markets_df['score'] > markets_df['score'].quantile(0.75)
    markets_df['high_volume'] = markets_df['volume_usd'] > markets_df['volume_usd'].quantile(0.75)
    markets_df['high_traders'] = markets_df['traders_count'] > markets_df['traders_count'].quantile(0.75)
    markets_df['high_duration'] = markets_df['duration_days'] > markets_df['duration_days'].quantile(0.75)

    # Prepare features and targets
    print("Preparing features and targets...")
    print(f"Target column: {args.target}")
    X, y, valid_markets, feature_names = prepare_features(markets_df, market_embeddings_mapped, args.target)

    print(f"Feature matrix shape: {X.shape}")
    print(f"Target vector shape: {y.shape}")
    print(f"{args.target} value range: {y.min():.3f} to {y.max():.3f}")
    print(f"Mean {args.target}: {y.mean():.3f} ± {y.std():.3f}")

    # Apply PCA if requested
    if args.pca_dim > 0 and args.pca_dim < X.shape[1]:
        print(f"Applying PCA reduction from {X.shape[1]} to {args.pca_dim} dimensions...")
        pca = PCA(n_components=args.pca_dim)
        X = pca.fit_transform(X)
        print(f"Explained variance ratio: {pca.explained_variance_ratio_.sum():.3f}")

    # Split data between test and train
    # Set a static random state for consistency
    random_state = 42
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=args.test_size, random_state=random_state
    )
    # Keep track of test set markets by splitting the markets list directly
    train_markets, test_markets = train_test_split(
        valid_markets, test_size=args.test_size, random_state=random_state
    )
    # Double-check that the split is consistent
    for i, market in enumerate(test_markets):
        if args.target in market and not market[args.target] == y_test[i]:
            print(f"Warning: Market {market['id']} ({market[args.target]}) has a different {args.target} than the corresponding #{i} y_test value ({y_test[i]})")

    # Scale features if requested
    scaler = None
    if args.scale_features:
        print("Scaling features...")
        scaler = RobustScaler()
        X_train = scaler.fit_transform(X_train)
        X_test = scaler.transform(X_test)

    # Train models
    print("Training models...")
    print(f"Training feature matrix shape: {X_train.shape}")
    results, trained_models = train_models(X_train, y_train, X_test, y_test, args.output_dir, args.target)

    # Display results
    print("\n" + "="*80)
    print(f"MODEL COMPARISON RESULTS - Target: {args.target}")
    print("="*80)

    results_table = []
    for name, result in results.items():
        results_table.append([
            name,
            f"{result['test_mse']:.4f}",
            f"{result['test_mae']:.4f}",
            f"{result['test_r2']:.4f}",
            f"{result['train_r2']:.4f}"
        ])

    if results_table:
        headers = ['Model', 'Test MSE', 'Test MAE', 'Test R²', 'Train R²']
        print(tabulate(
            sorted(results_table, key=lambda row: row[3], reverse=True),  # type: ignore
            headers=headers,
            tablefmt="github"
        ))
    else:
        print("No models were successfully trained - results table is empty!")

    # Find best model
    if not results:
        print("Error: No models were successfully trained!")
        return

    best_model_name = max(results.keys(), key=lambda x: results[x]['test_r2'])
    best_model = trained_models[best_model_name]
    print(f"\nBest model: {best_model_name} (R² = {results[best_model_name]['test_r2']:.4f})")

    # Hyperparameter tuning if requested
    if args.tune_hyperparameters:
        tuned_model = hyperparameter_tuning(X_train, y_train, best_model_name)

        # Evaluate tuned model
        y_pred_tuned = tuned_model.predict(X_test)
        tuned_r2 = r2_score(y_test, y_pred_tuned)
        print(f"Tuned model R²: {tuned_r2:.4f}")

        if tuned_r2 > results[best_model_name]['test_r2']:
            print("Tuned model performs better, using tuned version")
            best_model = tuned_model
        else:
            print("Original model performs better, keeping original")

    # Generate plots
    print("Generating plots...")
    plot_results(results, y_test, args.output_dir)

    # Analyze feature importance
    if hasattr(best_model, 'feature_importances_'):
        analyze_feature_importance(best_model, feature_names, args.output_dir)

    # Target distribution analysis
    analysis_df = pd.DataFrame({
        'market_id': [m['id'] for m in valid_markets],
        args.target: y,
        'platform': [m.get('platform_slug', 'unknown') for m in valid_markets]
    })

    print(f"\n{args.target.upper()} by Platform:")
    platform_stats = analysis_df.groupby('platform')[args.target].agg(['count', 'mean', 'std']).round(3)
    print(platform_stats)

    # Save detailed predictions
    predictions = best_model.predict(X_test)

    prediction_df = pd.DataFrame({
        'market_id': [market['id'] for market in test_markets],
        'platform': [market['platform_slug'] for market in test_markets],
        'title': [market.get('title', 'N/A') for market in test_markets],
        'url': [market.get('url', 'N/A') for market in test_markets],
        f'actual_{args.target}': y_test,
        f'predicted_{args.target}': predictions,
        f'error_{args.target}': y_test - predictions,
        f'abs_error_{args.target}': np.abs(y_test - predictions)
    })

    prediction_df.to_csv(f"{args.output_dir}/{slugify(best_model_name)}-{slugify(args.target)}-{int(time.time())}-predictions.csv", index=False)
    prediction_df.to_csv(f"{args.output_dir}/latest-predictions-{slugify(args.target)}.csv", index=False)
    print(f"\nDetailed predictions saved to {args.output_dir}/latest-predictions-{slugify(args.target)}.csv")

    print("\nSample Market Predictions:")
    display_cols = ['title', f'actual_{args.target}', f'predicted_{args.target}']
    print(prediction_df.head(20)[display_cols])

if __name__ == "__main__":
    main()
