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
from sklearn.metrics import mean_squared_error, mean_absolute_error, r2_score, accuracy_score, precision_score, recall_score, f1_score
from sklearn.preprocessing import StandardScaler, RobustScaler
from sklearn.decomposition import PCA
import pickle

from common import *

def prepare_features(markets_df, market_embeddings_mapped, target_column):
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

def get_model(model_name):
    """Get the specified model instance."""
    models = {
        'random_forest': RandomForestRegressor(n_estimators=200, min_samples_split=10, min_samples_leaf=4, random_state=42, n_jobs=-1),
        'linear_regression': LinearRegression(),
        'ridge': Ridge(alpha=1.0),
        'lasso': Lasso(alpha=0.1, max_iter=2000),
        'elasticnet': ElasticNet(alpha=0.1, l1_ratio=0.5, max_iter=2000),
        'gradient_boosting': GradientBoostingRegressor(n_estimators=100, random_state=42),
        'svr': SVR(kernel='rbf', C=1.0),
        'mlp': MLPRegressor(hidden_layer_sizes=(100, 50), max_iter=1000, random_state=42)
    }

    if model_name not in models:
        available_models = ', '.join(models.keys())
        raise ValueError(f"Model '{model_name}' not available. Choose from: {available_models}")

    return models[model_name]

def train_model(X_train, y_train, X_test, y_test, model_name, output_dir, target_column):
    """Train a single model and evaluate its performance."""
    model = get_model(model_name)

    print(f"Training {model_name}...", end="")
    start_time = time.time()

    try:
        # Train model
        model.fit(X_train, y_train)

        # Make predictions
        y_pred_train = model.predict(X_train)
        y_pred_test = model.predict(X_test)

        # Calculate metrics
        result = {
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
        print(f" Completed in {duration:.2f} seconds (R² = {result['test_r2']:.4f})")

        return result, model

    except Exception as e:
        print(f"Error training {model_name}: {e}")
        raise

def plot_predicted_vs_actual(actual_values, predicted_values, model_name, target_column, output_dir, r2_score_val=None, n_bins=10):
    """
    Create a detailed scatterplot of predicted vs actual values from the test set,
    with a box plot showing binned trends underneath.

    Handles both continuous and boolean values. Boolean values are mapped to 0/1.

    Args:
        y_test: Array of actual test values
        predictions: Array of predicted values
        model_name: Name of the model used for predictions
        target_column: Name of the target column being predicted
        output_dir: Directory to save the plot
        r2_score_val: Optional R² score to display on the plot
        n_bins: Number of bins for the box plot (default: 10)
    """
    # Check if values are boolean and convert actual values to numeric
    is_boolean = False
    original_predicted_values = predicted_values.copy()  # Keep original for x-axis
    if actual_values.dtype == bool or np.all(np.isin(actual_values, [0, 1, True, False])):
        is_boolean = True
        actual_values = actual_values.astype(int)
        # Only convert predicted values if they are also boolean, otherwise keep as probabilities
        if hasattr(predicted_values, 'dtype') and predicted_values.dtype == bool:
            # For metrics calculation, convert boolean predictions to int
            predicted_values_for_metrics = predicted_values.astype(int)
        elif is_boolean:
            # For metrics calculation, round continuous predictions to nearest integer
            predicted_values_for_metrics = np.round(np.clip(predicted_values, 0, 1)).astype(int)
    else:
        predicted_values_for_metrics = predicted_values
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 12), height_ratios=[2, 1])

    # Upper subplot: Scatterplot
    if is_boolean:
        # For boolean values, add jitter to avoid overlapping points
        jitter_strength = 0.05
        actual_jitter = actual_values + np.random.normal(0, jitter_strength, len(actual_values))
        predicted_jitter = original_predicted_values + np.random.normal(0, jitter_strength, len(original_predicted_values))
        ax1.scatter(predicted_jitter, actual_jitter, alpha=0.6, s=50, edgecolors='black', linewidth=0.5)

        # Set y-axis to show boolean labels, but keep x-axis showing predicted values
        ax1.set_yticks([0, 1])
        ax1.set_yticklabels(['False', 'True'])
        # Let x-axis show the actual predicted values for calibration
    else:
        ax1.scatter(predicted_values, actual_values, alpha=0.6, s=50, edgecolors='black', linewidth=0.5)

    # Add perfect prediction line (diagonal)
    if is_boolean:
        min_val = min(actual_values.min(), original_predicted_values.min())
        max_val = max(actual_values.max(), original_predicted_values.max())
    else:
        min_val = min(actual_values.min(), predicted_values.min())
        max_val = max(actual_values.max(), predicted_values.max())
    ax1.plot([min_val, max_val], [min_val, max_val], 'r--', lw=2, label='Perfect Prediction')

    # Calculate and display metrics
    if is_boolean:
        # Use classification metrics for boolean values
        accuracy = accuracy_score(actual_values, predicted_values_for_metrics)
        precision = precision_score(actual_values, predicted_values_for_metrics, zero_division=0)
        recall = recall_score(actual_values, predicted_values_for_metrics, zero_division=0)
        f1 = f1_score(actual_values, predicted_values_for_metrics, zero_division=0)
        stats_text = f'Accuracy = {accuracy:.4f}\nPrecision = {precision:.4f}\nRecall = {recall:.4f}\nF1 = {f1:.4f}\nN = {len(actual_values)}'
    else:
        # Use regression metrics for continuous values
        mse = mean_squared_error(actual_values, predicted_values)
        mae = mean_absolute_error(actual_values, predicted_values)
        if r2_score_val is None:
            r2_score_val = r2_score(actual_values, predicted_values)
        stats_text = f'R² = {r2_score_val:.4f}\nMSE = {mse:.4f}\nMAE = {mae:.4f}\nN = {len(actual_values)}'

    # Add labels and title
    ax1.set_ylabel(f'Actual {target_column.replace("_", " ").title()}', fontsize=12)
    ax1.set_title(f'Predicted vs Actual Values\nModel: {model_name}', fontsize=14, fontweight='bold')

    # Add statistics text box
    ax1.text(0.02, 0.95, stats_text, transform=ax1.transAxes,
             verticalalignment='top', bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.8),
             fontsize=10)

    # Make plot square and add grid
    if not is_boolean:
        ax1.axis('equal')
    ax1.grid(True, alpha=0.3)

    # Lower subplot: For boolean data show calibration bins, for continuous show box plot
    # Create bins based on predicted values (use original predicted values for boolean)
    pred_vals_for_binning = original_predicted_values if is_boolean else predicted_values
    bin_edges = np.linspace(pred_vals_for_binning.min(), pred_vals_for_binning.max(), n_bins + 1)
    bin_centers = (bin_edges[:-1] + bin_edges[1:]) / 2
    bin_labels = [f'{bin_edges[i]:.2f}-{bin_edges[i+1]:.2f}' for i in range(len(bin_edges)-1)]

    # Assign each point to a bin
    bin_indices = np.digitize(pred_vals_for_binning, bin_edges) - 1
    bin_indices = np.clip(bin_indices, 0, n_bins - 1)  # Ensure all indices are valid

    # Group actual values by bin
    binned_actuals = []
    predicted_bin_centers = []
    for i in range(n_bins):
        mask = bin_indices == i
        if np.sum(mask) > 0:  # Only include bins with data
            binned_actuals.append(actual_values[mask])
            predicted_bin_centers.append(bin_centers[i])

    # Create appropriate visualization based on data type
    if binned_actuals:  # Only create plot if we have data
        if is_boolean:
            # For boolean data, show calibration: mean actual value per predicted bin
            bin_means = [np.mean(bin_vals) for bin_vals in binned_actuals]
            bin_counts = [len(bin_vals) for bin_vals in binned_actuals]

            # Create bar plot showing calibration
            bars = ax2.bar(predicted_bin_centers, bin_means,
                          width=(bin_edges[1] - bin_edges[0]) * 0.8,
                          alpha=0.7, edgecolor='black', color='skyblue')

            # Add count labels on bars
            for bar, count, mean_val in zip(bars, bin_counts, bin_means):
                height = bar.get_height()
                ax2.text(bar.get_x() + bar.get_width()/2., height + 0.02,
                        f'n={count}', ha='center', va='bottom', fontsize=9)

            ax2.set_ylabel('Actual Rate (0=False, 1=True)', fontsize=12)
            ax2.set_xlabel(f'Predicted {target_column.replace("_", " ").title()}', fontsize=12)
            ax2.set_title('Calibration Plot', fontsize=12)
            ax2.set_ylim(-0.1, 1.1)
            ax2.set_yticks([0, 0.5, 1])
            ax2.set_yticklabels(['0%', '50%', '100%'])

        else:
            # For continuous data, use box plot
            bp = ax2.boxplot(binned_actuals, positions=predicted_bin_centers,
                           tick_labels=bin_labels, widths=(bin_edges[1] - bin_edges[0]) * 0.8)
            ax2.set_xlabel(f'Predicted {target_column.replace("_", " ").title()}', fontsize=12)
            ax2.set_ylabel(f'Actual {target_column.replace("_", " ").title()}', fontsize=12)

        # Add perfect prediction line
        ax2.plot([min_val, max_val], [min_val, max_val], 'r--', lw=2, alpha=0.7)
        ax2.grid(True, alpha=0.3)
        ax2.set_xlim(ax1.get_xlim())

    # Adjust layout and save
    plt.tight_layout()
    filename = f"{output_dir}/predicted_vs_actual_{slugify(model_name)}_{slugify(target_column)}.png"
    plt.savefig(filename, dpi=300, bbox_inches='tight')
    plt.close()

    print(f"Scatterplot with box plot saved to: {filename}")

def analyze_feature_importance(model, feature_names):
    """Analyze and plot feature importance."""
    if not hasattr(model, 'feature_importances_'):
        print("Model does not support feature importance analysis.")
        return

    importances = model.feature_importances_
    indices = np.argsort(importances)[::-1]

    # Display top 5 most important features
    print("\nTop 5 Most Important Features:")
    for i in range(min(5, len(feature_names))):
        idx = indices[i]
        print(f"{i+1:2d}. {feature_names[idx]:20s} ({importances[idx]:.6f})")

def save_model(model, output_dir, model_name, target_column, pca, feature_names):
    """Save trained model, scalers, and metadata for prediction."""
    model_data = {
        'model': model,
        'model_name': model_name,
        'target_column': target_column,
        'pca': pca,
        'feature_names': feature_names,
        'timestamp': time.time()
    }

    filename = f"{output_dir}/models/{slugify(model_name)}-{slugify(target_column)}.pkl"
    with open(filename, 'wb') as f:
        pickle.dump(model_data, f)

    print(f"Model saved to: {filename}")
    return filename

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
    parser.add_argument("--test-size", "-ts", type=float, default=0.2,
                       help="Test set size (default: 0.2)")
    parser.add_argument("--sample-platform", "-sp", type=str,
                       help="Sample markets from specific platform slug")
    parser.add_argument("--sample-size", "-ss", type=int,
                       help="Random sample size of markets to use")
    parser.add_argument("--target", "-t", type=str, default='resolution',
                       help="Target column to predict (default: resolution). Examples: resolution, volume_usd, traders_count")
    parser.add_argument("--model", "-m", type=str, default='random_forest',
                       help="Model to use (default: random_forest). Options: random_forest, linear_regression, ridge, lasso, elasticnet, gradient_boosting, svr, mlp")

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
    markets_df['high_score'] = markets_df['score'] > markets_df['score'].quantile(0.5)
    markets_df['high_volume'] = markets_df['volume_usd'] > markets_df['volume_usd'].quantile(0.5)
    markets_df['high_traders'] = markets_df['traders_count'] > markets_df['traders_count'].quantile(0.5)
    markets_df['high_duration'] = markets_df['duration_days'] > markets_df['duration_days'].quantile(0.5)
    markets_df['resolution_bool'] = markets_df['resolution'] == 1.0

    # Prepare features and targets
    print("Preparing features and targets...")
    print(f"Target column: {args.target}")
    print(f"Model: {args.model}")
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
    else:
        pca = None

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

    # Train model
    print("Training model...")
    print(f"Training feature matrix shape: {X_train.shape}")
    result, trained_model = train_model(X_train, y_train, X_test, y_test, args.model, args.output_dir, args.target)

    # Display results
    print(f"\nMODEL RESULTS - Target: {args.target}, Model: {args.model}")

    print(f"Test MSE: {result['test_mse']:.4f}")
    print(f"Test MAE: {result['test_mae']:.4f}")
    print(f"Test R²:  {result['test_r2']:.4f}")
    print(f"Train R²: {result['train_r2']:.4f}")

    # Save model with metadata
    save_model(trained_model, args.output_dir, args.model, args.target, pca=pca, feature_names=feature_names)

    # Generate plots
    print("Generating plots...")
    plot_predicted_vs_actual(
        y_test,
        result['predictions'],
        args.model,
        args.target,
        args.output_dir,
        result['test_r2']
    )
    analyze_feature_importance(trained_model, feature_names)

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
    predictions = trained_model.predict(X_test)

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

    prediction_df.to_csv(f"{args.output_dir}/{slugify(args.model)}-{slugify(args.target)}-{int(time.time())}-predictions.csv", index=False)
    prediction_df.to_csv(f"{args.output_dir}/latest-predictions-{slugify(args.target)}.csv", index=False)
    print(f"\nDetailed predictions saved to {args.output_dir}/latest-predictions-{slugify(args.target)}.csv")

    print("\nSample Market Predictions:")
    display_cols = ['title', f'actual_{args.target}', f'predicted_{args.target}']
    print(prediction_df.head(20)[display_cols])

if __name__ == "__main__":
    main()
