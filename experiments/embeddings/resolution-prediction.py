# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "requests",
#     "argparse",
#     "tabulate",
#     "tqdm",
#     "numpy",
#     "matplotlib",
#     "scikit-learn",
#     "pandas",
#     "seaborn",
# ]
# ///

import os
import json
import requests
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
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
import warnings
#warnings.filterwarnings('ignore')

def get_data(endpoint: str, headers={}, params={}, batch_size=20_000):
    """Get data from a PostgREST endpoint and handle the response."""
    count_response = requests.get(endpoint, headers=headers, params="select=count")
    total_count = count_response.json()[0]["count"]
    if total_count == 0:
        raise ValueError(f"No data available at {endpoint}")

    result = []
    num_batches = math.ceil(total_count / batch_size)
    for i in trange(num_batches, desc=f"Downloading {endpoint.split('/')[-1]}"):
        params["limit"] = batch_size
        params["offset"] = len(result)
        response = requests.get(endpoint, headers=headers, params=params)
        if response.ok:
            data = response.json()
            result += data
        else:
            print(f"Download returned code {response.status_code} for {endpoint}")
            try:
                error_data = response.json()
                print(json.dumps(error_data, indent=2), "\n")
            except Exception as e:
                print("Could not parse JSON response:", e)
                print("Raw response:", response.text, "\n")
            raise ValueError()

    if total_count != len(result):
        raise ValueError(
            f"Data missing at {endpoint}: {total_count} expected, {len(result)} received"
        )

    return result

def load_from_cache(cache_path):
    """Load data from cache file."""
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            return json.load(f)
    return None

def save_to_cache(cache_path, data):
    """Save data to cache file."""
    os.makedirs(os.path.dirname(cache_path), exist_ok=True)
    with open(cache_path, "w") as f:
        json.dump(data, f)

def prepare_features(markets, market_embeddings_mapped, include_market_features=True):
    """Prepare feature matrix from market embeddings and optional market metadata."""
    # Filter markets that have both embeddings and valid resolution values
    valid_markets = []
    for market in markets:
        if (market["id"] in market_embeddings_mapped and
            market.get("resolution") is not None and
            not np.isnan(market.get("resolution", np.nan))):
            valid_markets.append(market)

    print(f"Found {len(valid_markets)} markets with valid embeddings and resolution values")

    if len(valid_markets) == 0:
        raise ValueError("No markets found with both embeddings and resolution values")

    # Prepare embedding features
    embedding_features = np.array([market_embeddings_mapped[m["id"]] for m in valid_markets])

    # Prepare targets
    targets = np.array([m["resolution"] for m in valid_markets])

    # Prepare additional market features if requested
    if include_market_features:
        market_features = []
        for m in valid_markets:
            features = [
                m.get("volume_usd", 0) or 0,
                m.get("traders_count", 0) or 0,
                m.get("duration_days", 0) or 0,
                len(m.get("title", "")),
                1 if m.get("platform_slug") == "manifold" else 0,
                1 if m.get("platform_slug") == "metaculus" else 0,
                1 if m.get("platform_slug") == "polymarket" else 0,
            ]
            market_features.append(features)

        market_features = np.array(market_features)
        # Combine embedding and market features
        all_features = np.hstack([embedding_features, market_features])

        feature_names = [f"emb_{i}" for i in range(embedding_features.shape[1])] + [
            "volume_usd", "traders_count", "duration_days", "title_length",
            "is_manifold", "is_metaculus", "is_polymarket"
        ]
    else:
        all_features = embedding_features
        feature_names = [f"emb_{i}" for i in range(embedding_features.shape[1])]

    return all_features, targets, valid_markets, feature_names

def train_models(X_train, y_train, X_test, y_test):
    """Train multiple models and compare their performance."""
    models = {
        'Linear Regression': LinearRegression(),
        #'Ridge': Ridge(alpha=1.0),
        #'Lasso': Lasso(alpha=0.1, max_iter=2000),
        #'ElasticNet': ElasticNet(alpha=0.1, l1_ratio=0.5, max_iter=2000),
        #'Random Forest': RandomForestRegressor(n_estimators=100, random_state=42, n_jobs=-1),
        #'Gradient Boosting': GradientBoostingRegressor(n_estimators=100, random_state=42),
        #'SVR': SVR(kernel='rbf', C=1.0),
        #'MLP': MLPRegressor(hidden_layer_sizes=(100, 50), max_iter=1000, random_state=42)
        #.
    }

    results = {}
    trained_models = {}

    for name, model in tqdm(models.items(), desc="Training models"):
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
    plt.savefig(f"{output_dir}/resolution_prediction_results.png", dpi=300, bbox_inches='tight')
    plt.close()

    # Residual plot for best model
    residuals = y_test - predictions
    plt.figure(figsize=(10, 6))
    plt.scatter(predictions, residuals, alpha=0.6)
    plt.axhline(y=0, color='r', linestyle='--')
    plt.xlabel('Predicted Resolution')
    plt.ylabel('Residuals')
    plt.title(f'Residual Plot - {best_model}')
    plt.savefig(f"{output_dir}/residual_plot.png", dpi=300, bbox_inches='tight')
    plt.close()

def analyze_feature_importance(model, feature_names, output_dir, top_n=20):
    """Analyze and plot feature importance for tree-based models."""
    if hasattr(model, 'feature_importances_'):
        importances = model.feature_importances_
        indices = np.argsort(importances)[::-1]

        plt.figure(figsize=(12, 8))
        plt.title("Feature Importance")
        plt.bar(range(min(top_n, len(importances))),
                importances[indices[:top_n]])
        plt.xticks(range(min(top_n, len(importances))),
                  [feature_names[i] for i in indices[:top_n]], rotation=45, ha='right')
        plt.ylabel('Importance')
        plt.tight_layout()
        plt.savefig(f"{output_dir}/feature_importance.png", dpi=300, bbox_inches='tight')
        plt.close()

        # Print top features
        print(f"\nTop {top_n} Most Important Features:")
        for i in range(min(top_n, len(importances))):
            idx = indices[i]
            print(f"{i+1:2d}. {feature_names[idx]:25s}: {importances[idx]:.4f}")

def hyperparameter_tuning(X_train, y_train, model_name='Random Forest'):
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

    # Use smaller parameter grid if dataset is large
    if len(X_train) > 10000:
        print("Large dataset detected, using reduced parameter grid...")
        if model_name == 'Random Forest':
            param_grid = {
                'n_estimators': [100, 200],
                'max_depth': [10, None],
                'min_samples_split': [2, 5]
            }

    grid_search = GridSearchCV(model, param_grid, cv=5, scoring='r2', n_jobs=-1)
    grid_search.fit(X_train, y_train)

    print(f"Best parameters: {grid_search.best_params_}")
    print(f"Best cross-validation score: {grid_search.best_score_:.4f}")

    return grid_search.best_estimator_

def save_model(model, scaler, feature_names, output_dir, model_name):
    """Save trained model and preprocessing components."""
    model_data = {
        'model': model,
        'scaler': scaler,
        'feature_names': feature_names,
        'model_name': model_name
    }

    with open(f"{output_dir}/resolution_prediction_model.pkl", 'wb') as f:
        pickle.dump(model_data, f)

    print(f"Model saved to {output_dir}/resolution_prediction_model.pkl")

def load_model(model_path):
    """Load saved model and preprocessing components."""
    with open(model_path, 'rb') as f:
        model_data = pickle.load(f)
    return model_data

def predict_resolution(model_data, embeddings, market_features=None):
    """Make resolution predictions for new markets."""
    if market_features is not None:
        features = np.hstack([embeddings, market_features])
    else:
        features = embeddings

    if model_data['scaler'] is not None:
        features = model_data['scaler'].transform(features)

    predictions = model_data['model'].predict(features)
    return predictions

def main():
    parser = argparse.ArgumentParser(description="Predict market resolution values using embeddings")
    parser.add_argument("--cache-dir", "-cd", default="cache/embedding-analysis",
                       help="Cache directory (default: cache/embedding-analysis)")
    parser.add_argument("--output-dir", "-od", default=".",
                       help="Output directory for results (default: current directory)")
    parser.add_argument("--reset-cache", action="store_true",
                       help="Reset cache and re-download all data")
    parser.add_argument("--pca-dim", "-d", type=int, default=100,
                       help="PCA dimensionality reduction (default: 100, 0 to skip)")
    parser.add_argument("--include-market-features", action="store_true",
                       help="Include market metadata features alongside embeddings")
    parser.add_argument("--test-size", type=float, default=0.2,
                       help="Test set size (default: 0.2)")
    parser.add_argument("--tune-hyperparameters", action="store_true",
                       help="Perform hyperparameter tuning on best model")
    parser.add_argument("--save-model", action="store_true",
                       help="Save the best trained model")
    parser.add_argument("--scale-features", action="store_true",
                       help="Apply feature scaling")

    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Create directories
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)

    # Reset cache if requested
    if args.reset_cache:
        import shutil
        if os.path.exists(args.cache_dir):
            shutil.rmtree(args.cache_dir)
        print(f"Cache directory {args.cache_dir} cleared.")

    # Cache file names
    markets_cache = f"{args.cache_dir}/markets.json"
    embeddings_cache = f"{args.cache_dir}/market_embeddings.json"

    # Load markets
    print("Loading markets...")
    markets = load_from_cache(markets_cache)
    if markets is None:
        markets = get_data(f"{postgrest_base}/markets", params={"order": "id"})
        save_to_cache(markets_cache, markets)

    # Load market embeddings
    print("Loading market embeddings...")
    market_embeddings = load_from_cache(embeddings_cache)
    if market_embeddings is None:
        market_embeddings = get_data(f"{postgrest_base}/market_embeddings", params={"order": "market_id"})
        market_embeddings = [{"market_id": i["market_id"], "embedding": json.loads(i["embedding"])} for i in market_embeddings]
        save_to_cache(embeddings_cache, market_embeddings)

    market_embeddings_mapped = {m["market_id"]: m["embedding"] for m in market_embeddings}

    # Prepare features and targets
    print("Preparing features and targets...")
    X, y, valid_markets, feature_names = prepare_features(
        markets, market_embeddings_mapped, args.include_market_features
    )

    print(f"Feature matrix shape: {X.shape}")
    print(f"Target vector shape: {y.shape}")
    print(f"Resolution value range: {y.min():.3f} to {y.max():.3f}")
    print(f"Mean resolution: {y.mean():.3f} ± {y.std():.3f}")

    # Apply PCA if requested
    if args.pca_dim > 0 and args.pca_dim < X.shape[1]:
        print(f"Applying PCA reduction from {X.shape[1]} to {args.pca_dim} dimensions...")
        pca = PCA(n_components=args.pca_dim)
        X = pca.fit_transform(X)
        print(f"Explained variance ratio: {pca.explained_variance_ratio_.sum():.3f}")

    # Split data
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=args.test_size, random_state=42
    )

    # Scale features if requested
    scaler = None
    if args.scale_features:
        print("Scaling features...")
        scaler = RobustScaler()
        X_train = scaler.fit_transform(X_train)
        X_test = scaler.transform(X_test)

    # Train models
    print("Training models...")
    results, trained_models = train_models(X_train, y_train, X_test, y_test)

    # Display results
    print("\n" + "="*80)
    print("MODEL COMPARISON RESULTS")
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

    print(tabulate(
        sorted(results_table, key=lambda x: float(x[3]), reverse=True),
        headers=['Model', 'Test MSE', 'Test MAE', 'Test R²', 'Train R²'],
        tablefmt="github"
    ))

    # Find best model
    best_model_name = max(results.keys(), key=lambda x: results[x]['test_r2'])
    best_model = trained_models[best_model_name]
    print(f"\nBest model: {best_model_name} (R² = {results[best_model_name]['test_r2']:.4f})")

    # Hyperparameter tuning if requested
    if args.tune_hyperparameters and best_model_name in ['Random Forest', 'Gradient Boosting']:
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

    # Save model if requested
    if args.save_model:
        save_model(best_model, scaler, feature_names, args.output_dir, best_model_name)

    # Resolution distribution analysis
    print("\n" + "="*50)
    print("RESOLUTION ANALYSIS")
    print("="*50)

    resolution_df = pd.DataFrame({
        'market_id': [m['id'] for m in valid_markets],
        'resolution': y,
        'platform': [m.get('platform_slug', 'unknown') for m in valid_markets],
        'volume_usd': [m.get('volume_usd', 0) or 0 for m in valid_markets]
    })

    print("\nResolution by Platform:")
    platform_stats = resolution_df.groupby('platform')['resolution'].agg(['count', 'mean', 'std']).round(3)
    print(platform_stats)

    # Save detailed predictions
    predictions = best_model.predict(X_test if scaler is None else scaler.transform(X_test) if scaler else X_test)
    prediction_df = pd.DataFrame({
        'actual': y_test,
        'predicted': predictions,
        'error': y_test - predictions,
        'abs_error': np.abs(y_test - predictions)
    })
    prediction_df.to_csv(f"{args.output_dir}/predictions.csv", index=False)

    print(f"\nDetailed predictions saved to {args.output_dir}/predictions.csv")
    print(f"Plots saved to {args.output_dir}/")

if __name__ == "__main__":
    main()
