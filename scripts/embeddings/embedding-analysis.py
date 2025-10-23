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
#     "faiss-cpu",
#     "hdbscan",
#     "umap-learn",
#     "scikit-learn",
#     "plotly",
#     "pandas",
#     "pyarrow",
# ]
# ///

import os
import re
import json
import requests
from dotenv import load_dotenv
from tqdm import trange, tqdm
from tabulate import tabulate
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import math
import random
import faiss
import hdbscan
import umap
import argparse
from sklearn.decomposition import PCA
from sklearn.manifold import TSNE
from sklearn.feature_extraction.text import TfidfVectorizer
from collections import Counter
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import warnings

# Constants for better maintainability
DEFAULT_BATCH_SIZE = 20_000
DEFAULT_FAISS_NLIST = 1024
DEFAULT_FAISS_BATCH_SIZE = 5000
JITTER_SCALE = 1e-6
TITLE_MAX_LENGTH = 100
DISPLAY_SAMPLE_SIZE = 50_000
NUM_KEYWORDS = 10

# Suppress pandas warnings for cleaner output
# warnings.filterwarnings('ignore', category=pd.errors.PerformanceWarning)

def get_data_as_dataframe(endpoint: str, headers={}, params={}, batch_size=DEFAULT_BATCH_SIZE):
    """
    Download data from a PostgREST API endpoint in batches and return as pandas DataFrame.

    This function first queries the total count, then downloads data in configurable batches
    to handle large datasets efficiently. Includes progress tracking and error handling.

    Args:
        endpoint (str): PostgREST API endpoint URL
        headers (dict): HTTP headers to include with requests (default: {})
        params (dict): Query parameters to include with requests (default: {})
        batch_size (int): Number of records to download per batch (default: 20,000)

    Returns:
        pd.DataFrame: Complete dataset from the endpoint

    Raises:
        ValueError: If no data is available or if downloaded count doesn't match expected count

    Side Effects:
        - Makes multiple HTTP requests to the endpoint
        - Prints progress information and error details to stdout
        - May print JSON error responses for debugging
    """
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

    return pd.DataFrame(result)

def load_dataframe_from_cache(cache_file):
    """
    Load a pandas DataFrame from a JSONL (JSON Lines) cache file.

    Efficiently reads cached data with error handling for corrupted or missing files.
    Uses pandas' optimized JSONL reader for better performance on large files.

    Args:
        cache_file (str): Path to the JSONL cache file

    Returns:
        pd.DataFrame or None: Loaded DataFrame if successful, None if file doesn't exist or fails to load

    Side Effects:
        - Prints loading status and row count to stdout
        - Prints warning messages for load failures
        - No modification to the cache file
    """
    if not os.path.exists(cache_file):
        return None

    try:
        # Use more efficient data types and chunked reading for large files
        df = pd.read_json(cache_file, lines=True)
        print(f"Loaded {len(df)} rows from {os.path.basename(cache_file)}")
        return df
    except (ValueError, OSError) as e:
        print(f"Warning: Failed to load cache file ({e}). Re-downloading.")
        return None

def save_dataframe_to_cache(cache_file, df):
    """
    Save a pandas DataFrame to a JSONL (JSON Lines) cache file.

    Creates the directory structure if needed and saves the DataFrame in an efficient
    JSONL format for fast loading. Handles file system errors gracefully.

    Args:
        cache_file (str): Path where the JSONL file should be saved
        df (pd.DataFrame): DataFrame to save to cache

    Returns:
        None

    Side Effects:
        - Creates directory structure if it doesn't exist
        - Writes/overwrites the cache file
        - Prints save status and row count to stdout
        - Prints warning messages for save failures
    """
    os.makedirs(os.path.dirname(cache_file), exist_ok=True)
    try:
        df.to_json(cache_file, orient='records', lines=True)
        print(f"Saved {len(df)} rows to {os.path.basename(cache_file)}")
    except OSError as e:
        print(f"Warning: Failed to save cache file ({e}).")

def calculate_market_scores(df):
    """
    Calculate composite market scores using weighted metrics and vectorized operations.

    Combines volume (USD), trader count, and duration into a single score using predefined
    coefficients. Uses numpy operations for optimal performance on large datasets.

    Args:
        df (pd.DataFrame): Market data with required columns:
            - volume_usd (float): Trading volume in USD
            - traders_count (int): Number of unique traders
            - duration_days (float): Market duration in days

    Returns:
        np.ndarray: Array of calculated market scores (float)

    Formula:
        score = 0.001 * volume_usd + 10.0 * traders_count + 1.0 * duration_days

    Side Effects:
        - None (pure computation, no external modifications)
        - Handles NaN values by converting them to 0.0
    """
    # Constants moved to module level for consistency
    VOLUME_COEF = 0.001
    TRADERS_COEF = 10.0
    DURATION_COEF = 1.0

    # Use numpy operations for maximum speed
    volume_arr = np.nan_to_num(df['volume_usd'].values, nan=0.0)
    traders_arr = np.nan_to_num(df['traders_count'].values, nan=0.0)
    duration_arr = np.nan_to_num(df['duration_days'].values, nan=0.0)

    return VOLUME_COEF * volume_arr + TRADERS_COEF * traders_arr + DURATION_COEF * duration_arr

def compute_novelty_faiss(embeddings_df, n=10, nlist=DEFAULT_FAISS_NLIST, batch_size=DEFAULT_FAISS_BATCH_SIZE):
    """
    Compute novelty scores for market embeddings using FAISS for efficient similarity search.

    Novelty is calculated as the average distance to the n nearest neighbors in embedding space.
    Uses FAISS library for optimized similarity search with automatic index selection based
    on dataset size. Supports both IVF and flat indices for different scales.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors (all same dimension)
        n (int): Number of nearest neighbors to consider (default: 10)
        nlist (int): Number of clusters for IVF index (default: 1024)
        batch_size (int): Batch size for processing (default: 5000)

    Returns:
        pd.DataFrame: Novelty scores with columns:
            - market_id: Market identifier from input
            - novelty (float): Novelty score (0.0 = least novel, 1.0 = most novel)

    Side Effects:
        - Normalizes input vectors for cosine similarity computation
        - Uses all available CPU cores via FAISS OpenMP settings
        - Prints progress information and index statistics to stdout
        - Temporarily uses significant memory for FAISS index construction
    """
    print(f"Computing novelty for {len(embeddings_df)} embeddings...")

    # Extract data efficiently
    market_ids = embeddings_df['market_id'].values

    # Convert embeddings more efficiently - handle both list and array formats
    if isinstance(embeddings_df['embedding'].iloc[0], list):
        vectors = np.vstack(embeddings_df['embedding'].values).astype(np.float32)
    else:
        vectors = np.stack(embeddings_df['embedding'].values).astype(np.float32)

    # Normalize for cosine similarity
    faiss.normalize_L2(vectors)
    dim = vectors.shape[1]

    # Optimize FAISS settings
    faiss.omp_set_num_threads(0)  # Use all available cores

    # Choose index type based on dataset size
    if len(vectors) > 10000:
        # IVF index for larger datasets
        quantizer = faiss.IndexFlatIP(dim)
        index = faiss.IndexIVFFlat(quantizer, dim, min(nlist, len(vectors) // 4), faiss.METRIC_INNER_PRODUCT)
        print("Training FAISS IVF index...")
        index.train(vectors)
    else:
        # Flat index for smaller datasets (more accurate)
        index = faiss.IndexFlatIP(dim)
        print("Using FAISS flat index for high accuracy...")

    index.add(vectors)
    print(f"Index ready with {len(vectors)} vectors")

    # Vectorized batch processing
    novelty_scores = np.zeros(len(vectors), dtype=np.float32)

    for start in tqdm(range(0, len(vectors), batch_size), desc="Computing novelty", unit="batch"):
        end = min(start + batch_size, len(vectors))
        batch_vectors = vectors[start:end]
        distances, _ = index.search(batch_vectors, min(n + 1, len(vectors)))

        # Vectorized novelty calculation (skip self-similarity at index 0)
        novelty_scores[start:end] = np.mean(1 - distances[:, 1:min(n+1, distances.shape[1])], axis=1)

    return pd.DataFrame({
        'market_id': market_ids,
        'novelty': novelty_scores
    })

def create_clusters_hdbscan(embeddings_df, min_cluster_size):
    """
    Perform density-based clustering on market embeddings using HDBSCAN algorithm.

    Applies HDBSCAN clustering to normalized embedding vectors to identify market groups
    with similar characteristics. HDBSCAN automatically determines the number of clusters
    and marks outliers as cluster -1.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors (all same dimension)
        min_cluster_size (int): Minimum number of points required to form a cluster

    Returns:
        pd.DataFrame: Cluster assignments with columns:
            - market_id: Market identifier from input
            - cluster (int): Cluster ID (-1 for outliers, 0+ for valid clusters)

    Side Effects:
        - Normalizes embedding vectors using L2 normalization
        - Prints clustering progress to stdout
        - Uses fixed min_samples=10 parameter for HDBSCAN
    """
    market_ids = embeddings_df['market_id'].values
    embedding_vectors = np.stack(embeddings_df['embedding'].values).astype('float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Clustering with HDBSCAN...")
    clusterer = hdbscan.HDBSCAN(
        min_cluster_size=min_cluster_size,
        min_samples=10,
    )
    cluster_labels = clusterer.fit_predict(embedding_vectors)

    return pd.DataFrame({
        'market_id': market_ids,
        'cluster': cluster_labels
    })

def apply_pca_reduction(embeddings_df, target_dim):
    """
    Apply Principal Component Analysis (PCA) to reduce embedding dimensionality.

    Reduces high-dimensional embeddings to a lower-dimensional space while preserving
    the most important variance. Skips reduction if target dimension is 0 or greater
    than current dimension.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - embedding (list/np.array): Dense embedding vectors (all same dimension)
            - (preserves all other columns unchanged)
        target_dim (int): Target number of dimensions (0 to skip reduction)

    Returns:
        pd.DataFrame: Updated DataFrame with reduced embeddings:
            - embedding: Reduced embedding vectors (list format)
            - (all other columns preserved unchanged)

    Side Effects:
        - Prints reduction status and explained variance ratio to stdout
        - Returns original DataFrame unchanged if target_dim is 0 or >= current dimension
        - Converts embeddings to float32 for memory efficiency
    """
    current_dim = len(embeddings_df['embedding'].iloc[0])
    if target_dim == 0 or target_dim >= current_dim:
        print(f"Skipping PCA: target_dim={target_dim}, embedding_dim={current_dim}")
        return embeddings_df

    print(f"Applying PCA reduction from {current_dim} to {target_dim} dimensions...")

    # Extract embeddings matrix
    embedding_matrix = np.stack(embeddings_df['embedding'].values).astype('float32')

    # Apply PCA
    pca = PCA(n_components=target_dim)
    reduced_embeddings = pca.fit_transform(embedding_matrix)

    # Update DataFrame with reduced dimensions
    result_df = embeddings_df.copy()
    result_df['embedding'] = [row.tolist() for row in reduced_embeddings]

    print(f"PCA explained variance ratio: {sum(pca.explained_variance_ratio_):.3f}")
    return result_df

def remove_emoji(string):
    """
    Remove emoji characters from a text string using regex pattern matching.

    Removes various categories of Unicode emoji including emoticons, symbols,
    pictographs, transport symbols, and flags. Used for cleaning market titles
    in cluster analysis and visualization.

    Args:
        string (str): Input text that may contain emoji characters

    Returns:
        str: Text with all emoji characters removed

    Side Effects:
        - None (pure string transformation function)
    """
    emoji_pattern = re.compile("["
        u"\U0001F600-\U0001F64F"  # emoticons
        u"\U0001F300-\U0001F5FF"  # symbols & pictographs
        u"\U0001F680-\U0001F6FF"  # transport & map symbols
        u"\U0001F1E0-\U0001F1FF"  # flags (iOS)
        u"\U00002702-\U000027B0"
        u"\U000024C2-\U0001F251"
    "]+", flags=re.UNICODE)
    return emoji_pattern.sub(r'', string)

def collate_cluster_information(markets_df, novelty_df):
    """
    Aggregate and compute comprehensive statistics for each market cluster.

    Combines market data with novelty scores and computes detailed statistics
    for each cluster including top markets, platform distributions, and
    median values across various metrics. Uses efficient pandas groupby
    operations for optimal performance.

    Args:
        markets_df (pd.DataFrame): Market data with required columns:
            - market_id (int/str): Unique market identifier
            - cluster (int): Cluster assignment (-1 for outliers)
            - title (str): Market title/description
            - score (float): Market score from calculate_market_scores()
            - platform_slug (str): Platform identifier
            - open_datetime (datetime): Market opening time
            - volume_usd (float): Trading volume in USD
            - traders_count (int): Number of unique traders
            - duration_days (float): Market duration in days
            - resolution (float): Market resolution (0.0-1.0)
        novelty_df (pd.DataFrame): Novelty data with required columns:
            - market_id (int/str): Unique market identifier
            - novelty (float): Novelty score (0.0-1.0)

    Returns:
        dict: Nested dictionary with cluster_id as keys and statistics as values:
            - market_count (int): Number of markets in cluster
            - markets (list): All market records as dictionaries
            - top_market (dict): Highest scoring market in cluster
            - top_market_title (str): Emoji-cleaned title of top market
            - first_market (dict): Earliest market by open_datetime
            - first_market_platform (str): Platform of first market
            - platform_proportions (dict): Platform distribution ratios
            - top_platform (str): Most common platform in cluster
            - top_platform_pct (float): Percentage of top platform
            - median_novelty (float): Median novelty score
            - median_volume_usd (float): Median trading volume
            - median_traders_count (float): Median trader count
            - median_duration_days (float): Median duration
            - mean_resolution (float): Average resolution rate

    Side Effects:
        - Merges DataFrames using inner join on market_id
        - Excludes outlier cluster (-1) from main analysis
        - Applies emoji removal to market titles
        - Returns empty dict if input markets_df is empty
    """
    if markets_df.empty:
        return {}

    # Merge with novelty data
    merged_df = markets_df.merge(novelty_df, on='market_id', how='left')

    cluster_info = {}

    for cluster_id, group in merged_df.groupby('cluster'):
        if cluster_id == -1:  # Skip outliers for main analysis
            continue

        # Basic info
        info = {
            "market_count": len(group),
            "markets": group.to_dict('records')  # Keep for backward compatibility
        }

        # Top market by score
        top_market = group.loc[group['score'].idxmax()]
        info["top_market"] = top_market.to_dict()
        info["top_market_title"] = remove_emoji(top_market["title"])

        # First market by open_datetime
        if 'open_datetime' in group.columns:
            first_market = group.loc[group['open_datetime'].idxmin()]
            info["first_market"] = first_market.to_dict()
            info["first_market_platform"] = first_market.get("platform_slug", "unknown")

        # Platform proportions using value_counts
        platform_counts = group['platform_slug'].value_counts()
        total_markets = len(group)
        info["platform_proportions"] = (platform_counts / total_markets).to_dict()
        info["top_platform"] = platform_counts.index[0] if len(platform_counts) > 0 else "unknown"
        info["top_platform_pct"] = platform_counts.iloc[0] / total_markets if len(platform_counts) > 0 else 0

        # Statistical aggregations using pandas methods
        info["median_novelty"] = group['novelty'].median()
        info["median_volume_usd"] = group['volume_usd'].median()
        info["median_traders_count"] = group['traders_count'].median()
        info["median_duration_days"] = group['duration_days'].median()
        info["mean_resolution"] = group['resolution'].mean()

        cluster_info[cluster_id] = info

    return cluster_info

def create_cluster_dashboard(cluster_info_dict, output_dir):
    """
    Generate a comprehensive multi-panel dashboard visualizing cluster statistics.

    Creates a 3x3 grid of matplotlib plots showing various aspects of cluster analysis
    including market counts, distributions, platform breakdowns, and metric correlations.
    Saves the complete dashboard as a high-resolution PNG file.

    Args:
        cluster_info_dict (dict): Cluster information from collate_cluster_information()
            Must contain cluster statistics with keys like market_count, median_novelty,
            median_volume_usd, platform_proportions, etc.
        output_dir (str): Directory path where dashboard PNG will be saved

    Returns:
        None

    Generated Plots:
        1. Bar chart of markets per cluster
        2. Histogram of market count distribution
        3. Pie chart of overall platform distribution
        4. Histogram of median novelty scores
        5. Log-scale histogram of median volumes
        6. Histogram of median trader counts
        7. Histogram of median durations
        8. Histogram of mean resolutions
        9. Scatter plot of volume vs traders

    Side Effects:
        - Creates/overwrites cluster_dashboard.png in output_dir
        - Prints warning if no cluster information available
        - Handles missing/zero values gracefully in visualizations
        - Uses tight layout and closes matplotlib figure to free memory
    """
    if not cluster_info_dict:
        print("No cluster information available for dashboard")
        return

    fig = plt.figure(figsize=(20, 15))

    # Convert cluster info to DataFrame for easier manipulation
    cluster_data = []
    for cluster_id, info in cluster_info_dict.items():
        cluster_data.append({
            'cluster_id': cluster_id,
            'market_count': info['market_count'],
            'median_novelty': info['median_novelty'],
            'median_volume_usd': info['median_volume_usd'],
            'median_traders_count': info['median_traders_count'],
            'median_duration_days': info['median_duration_days'],
            'mean_resolution': info['mean_resolution'],
            'top_platform': info['top_platform'],
            'top_platform_pct': info['top_platform_pct']
        })

    cluster_df = pd.DataFrame(cluster_data)

    # Plot 1: Bar plot of number of markets
    plt.subplot(3, 3, 1)
    plt.bar(cluster_df['cluster_id'], cluster_df['market_count'])
    plt.xlabel('Cluster ID')
    plt.ylabel('Number of Markets')
    plt.title('Markets per Cluster')
    plt.grid(True)

    # Plot 2: Histogram of market counts
    plt.subplot(3, 3, 2)
    plt.hist(cluster_df['market_count'], bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Number of Markets')
    plt.ylabel('Frequency')
    plt.title('Distribution of Market Counts')

    # Plot 3: Platform proportions pie chart
    plt.subplot(3, 3, 3)
    all_platforms = {}
    for cluster_info in cluster_info_dict.values():
        for platform, prop in cluster_info["platform_proportions"].items():
            all_platforms[platform] = all_platforms.get(platform, 0) + prop * cluster_info["market_count"]

    if all_platforms:
        total_markets = sum(all_platforms.values())
        platform_props = {k: v/total_markets for k, v in all_platforms.items()}
        plt.pie(platform_props.values(), labels=platform_props.keys(), autopct='%1.1f%%')
        plt.title('Platform Distribution')

    # Plot 4: Median novelty histogram
    plt.subplot(3, 3, 4)
    plt.hist(cluster_df['median_novelty'].dropna(), bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Novelty')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Novelty')

    # Plot 5: Median volume histogram (log scale)
    plt.subplot(3, 3, 5)
    non_zero_volumes = cluster_df[cluster_df['median_volume_usd'] > 0]['median_volume_usd']
    if len(non_zero_volumes) > 0:
        plt.hist(non_zero_volumes, bins=20, alpha=0.7, edgecolor='black')
        plt.xscale('log')
    plt.xlabel('Median Volume USD (log scale)')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Volume')

    # Plot 6: Median traders histogram
    plt.subplot(3, 3, 6)
    non_zero_traders = cluster_df[cluster_df['median_traders_count'] > 0]['median_traders_count']
    if len(non_zero_traders) > 0:
        plt.hist(non_zero_traders, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Traders Count')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Traders')

    # Plot 7: Median duration histogram
    plt.subplot(3, 3, 7)
    non_zero_durations = cluster_df[cluster_df['median_duration_days'] > 0]['median_duration_days']
    if len(non_zero_durations) > 0:
        plt.hist(non_zero_durations, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Duration Days')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Duration')

    # Plot 8: Mean resolution histogram
    plt.subplot(3, 3, 8)
    plt.hist(cluster_df['mean_resolution'].dropna(), bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Mean Resolution')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Mean Resolution')

    # Plot 9: Scatter plot of volume vs traders
    plt.subplot(3, 3, 9)
    valid_data = cluster_df[(cluster_df['median_volume_usd'] > 0) & (cluster_df['median_traders_count'] > 0)]
    if len(valid_data) > 0:
        plt.scatter(valid_data['median_volume_usd'], valid_data['median_traders_count'], alpha=0.6)
        plt.xscale('log')
    plt.xlabel('Median Volume USD (log scale)')
    plt.ylabel('Median Traders Count')
    plt.title('Volume vs Traders by Cluster')

    plt.tight_layout()
    plt.savefig(f"{output_dir}/cluster_dashboard.png", format="png", bbox_inches="tight", dpi=150)
    plt.close()

def jitter_duplicate_embeddings(embeddings_df):
    """
    Add small random noise to duplicate embeddings to ensure uniqueness for visualization.

    Detects embedding vectors that are identical and applies deterministic jitter
    based on market_id hash for reproducible results. Essential for dimensionality
    reduction algorithms that may fail with identical input vectors.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors

    Returns:
        pd.DataFrame: Modified DataFrame with jittered embeddings:
            - Preserves all original columns and structure
            - Only duplicate embeddings are modified with small noise
            - Original unique embeddings remain unchanged

    Jitter Details:
        - Jitter scale: 1e-6 (very small to preserve similarity structure)
        - Deterministic: Same market_id will always get same jitter
        - Uses numpy.random with seed based on market_id hash

    Side Effects:
        - Prints duplicate detection status and count to stdout
        - Temporarily modifies numpy random state for each duplicate
        - Returns original DataFrame if no duplicates found
    """
    print("Checking for duplicate embeddings...", end=" ")

    # More efficient duplicate detection using numpy
    embeddings_matrix = np.vstack(embeddings_df['embedding'].values)

    # Find duplicates using numpy operations (much faster than JSON hashing)
    _, unique_indices = np.unique(embeddings_matrix, axis=0, return_index=True)
    all_indices = set(range(len(embeddings_df)))
    duplicate_indices = list(all_indices - set(unique_indices))

    if not duplicate_indices:
        print("No duplicates found.")
        return embeddings_df

    print(f"Found {len(duplicate_indices)} duplicates. Applying jitter...")
    result_df = embeddings_df.copy()

    # Vectorized jitter application
    for idx in duplicate_indices:
        market_id = result_df.iloc[idx]['market_id']
        embedding = result_df.iloc[idx]['embedding'].copy()

        # Deterministic jitter based on market_id
        np.random.seed(hash(market_id) % (2**32))
        jitter = np.random.uniform(-JITTER_SCALE, JITTER_SCALE, len(embedding))
        result_df.iloc[idx, result_df.columns.get_loc('embedding')] = (np.array(embedding) + jitter).tolist()

    print(f"Applied jitter to {len(duplicate_indices)} duplicate embeddings.")
    return result_df

def dimension_reduction_umap(embeddings_df, n_jobs=6):
    """
    Reduce high-dimensional embeddings to 2D using UMAP algorithm for visualization.

    UMAP (Uniform Manifold Approximation and Projection) preserves both local and
    global structure of the data, making it excellent for cluster visualization.
    Automatically applies jitter to handle duplicate embeddings.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors (any dimension)
        n_jobs (int): Number of parallel jobs for UMAP computation (default: 6)

    Returns:
        pd.DataFrame: 2D embeddings with columns:
            - market_id: Market identifier from input
            - embedding (list): 2D coordinates as [x, y] lists

    Side Effects:
        - Normalizes embedding vectors using L2 normalization
        - Applies jitter to duplicate embeddings via jitter_duplicate_embeddings()
        - Prints progress information to stdout
        - Uses multiple CPU cores for computation
    """
    # Add jitter to handle duplicates
    embeddings_df = jitter_duplicate_embeddings(embeddings_df)

    embedding_vectors = np.stack(embeddings_df['embedding'].values).astype('float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with UMAP...", end="")
    reducer = umap.UMAP(n_jobs=n_jobs, verbose=True)
    embedding_2d = reducer.fit_transform(embedding_vectors)
    print("Complete.")

    return pd.DataFrame({
        'market_id': embeddings_df['market_id'],
        'embedding': [row.tolist() for row in embedding_2d]
    })

def dimension_reduction_tsne(embeddings_df):
    """
    Reduce high-dimensional embeddings to 2D using t-SNE algorithm for visualization.

    t-SNE (t-distributed Stochastic Neighbor Embedding) excels at preserving local
    structure and revealing cluster patterns. Automatically adjusts perplexity
    based on dataset size to avoid errors with small datasets.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors (any dimension)

    Returns:
        pd.DataFrame: 2D embeddings with columns:
            - market_id: Market identifier from input
            - embedding (list): 2D coordinates as [x, y] lists

    Side Effects:
        - Normalizes embedding vectors using L2 normalization
        - Adjusts perplexity to min(30, dataset_size-1) to prevent errors
        - Prints progress information to stdout
        - May take significant time for large datasets (t-SNE is O(n²))
    """
    embedding_vectors = np.stack(embeddings_df['embedding'].values).astype('float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with t-SNE...", end="")
    reducer = TSNE(n_components=2, perplexity=min(30, len(embedding_vectors)-1))
    embedding_2d = reducer.fit_transform(embedding_vectors)
    print("Complete.")

    return pd.DataFrame({
        'market_id': embeddings_df['market_id'],
        'embedding': [row.tolist() for row in embedding_2d]
    })

def dimension_reduction_pca(embeddings_df):
    """
    Reduce high-dimensional embeddings to 2D using PCA for linear dimensionality reduction.

    PCA (Principal Component Analysis) finds the two directions of maximum variance
    in the data. Fastest of the dimensionality reduction methods but may not capture
    non-linear cluster structure as well as UMAP or t-SNE.

    Args:
        embeddings_df (pd.DataFrame): Embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list/np.array): Dense embedding vectors (any dimension)

    Returns:
        pd.DataFrame: 2D embeddings with columns:
            - market_id: Market identifier from input
            - embedding (list): 2D coordinates as [x, y] lists

    Side Effects:
        - Normalizes embedding vectors using L2 normalization
        - Prints explained variance ratio for each component and total
        - Prints progress information to stdout
        - Fastest dimensionality reduction method available
    """
    embedding_vectors = np.stack(embeddings_df['embedding'].values).astype('float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with PCA...", end="")
    pca = PCA(n_components=2)
    embedding_2d = pca.fit_transform(embedding_vectors)
    print("Complete.")

    explained_var = pca.explained_variance_ratio_
    print(f"PCA explained variance: {explained_var[0]:.3f}, {explained_var[1]:.3f} (total: {sum(explained_var):.3f})")

    return pd.DataFrame({
        'market_id': embeddings_df['market_id'],
        'embedding': [row.tolist() for row in embedding_2d]
    })

def plot_clusters(method, embeddings_2d_df, clusters_df, output_file, label_top_n_clusters=20):
    """
    Create a static scatter plot visualization of clustered embeddings in 2D space.

    Generates a matplotlib scatter plot showing market clusters with different colors,
    outliers in gray, and labels for the largest clusters. Saves as high-resolution PNG.

    Args:
        method (str): Dimensionality reduction method name for plot title (e.g., "UMAP", "t-SNE")
        embeddings_2d_df (pd.DataFrame): 2D embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list): 2D coordinates as [x, y] lists
        clusters_df (pd.DataFrame): Cluster assignments with required columns:
            - market_id (int/str): Unique market identifier
            - cluster (int): Cluster ID (-1 for outliers, 0+ for valid clusters)
        output_file (str): Path where the PNG plot will be saved
        label_top_n_clusters (int): Number of largest clusters to label on plot (default: 20)

    Returns:
        None

    Plot Features:
        - Outliers (cluster -1) shown in light gray with low opacity
        - Regular clusters colored using matplotlib's tab20 colormap
        - Cluster centroids labeled with "C{cluster_id}" annotations
        - Colorbar showing cluster ID mapping
        - High-resolution output (300 DPI)

    Side Effects:
        - Creates/overwrites the output PNG file
        - Uses matplotlib figure (10x8 inches) and closes it after saving
        - Prints warning if no data available for plotting
        - Merges input DataFrames using inner join on market_id
    """
    # Merge embeddings with cluster data
    plot_data = embeddings_2d_df.merge(clusters_df, on='market_id', how='inner')

    if plot_data.empty:
        print("No data available for plotting")
        return

    # Extract coordinates and labels
    embedding_2d = np.stack(plot_data['embedding'].values)
    cluster_labels = plot_data['cluster'].values

    # Count cluster sizes to identify largest clusters
    cluster_counts = Counter(cluster_labels)
    largest_clusters = [cluster_id for cluster_id, _ in cluster_counts.most_common()
                       if cluster_id != -1][:label_top_n_clusters]

    # Initialize figure
    plt.figure(figsize=(10, 8))

    # Plot outliers (cluster -1) with lower alpha for transparency
    outlier_mask = cluster_labels == -1
    if np.any(outlier_mask):
        plt.scatter(
            embedding_2d[outlier_mask, 0], embedding_2d[outlier_mask, 1],
            c='lightgray', s=1, alpha=0.3, label='Outliers'
        )

    # Plot regular clusters with normal alpha
    non_outlier_mask = cluster_labels != -1
    if np.any(non_outlier_mask):
        scatter = plt.scatter(
            embedding_2d[non_outlier_mask, 0], embedding_2d[non_outlier_mask, 1],
            c=cluster_labels[non_outlier_mask], cmap='tab20', s=3, alpha=0.8
        )
        plt.colorbar(scatter, label="Cluster")

    # Add labels to the largest clusters
    for cluster_id in largest_clusters:
        cluster_mask = cluster_labels == cluster_id
        if np.any(cluster_mask):
            # Calculate cluster centroid
            cluster_points = embedding_2d[cluster_mask]
            centroid_x = np.mean(cluster_points[:, 0])
            centroid_y = np.mean(cluster_points[:, 1])
            plt.annotate(
                f'C{cluster_id}', (centroid_x, centroid_y),
                fontsize=6,
                fontweight='bold',
                bbox=dict(boxstyle='round,pad=0.1', facecolor='white', alpha=0.8)
            )

    plt.title(f"Market Embeddings Clusters ({method})")
    plt.tight_layout()
    plt.savefig(output_file, format="png", bbox_inches="tight", dpi=300)
    plt.close()

def create_interactive_visualization(method, embeddings_2d_df, clusters_df, markets_df,
                                   cluster_info_dict, output_file, display_prob):
    """
    Generate an interactive HTML visualization using Plotly with rich hover tooltips and controls.

    Creates a comprehensive interactive scatter plot with hover information showing market
    details, cluster keywords, platform information, and interactive controls to toggle
    outliers on/off. Includes performance optimizations for large datasets.

    Args:
        method (str): Dimensionality reduction method name for labels and title
        embeddings_2d_df (pd.DataFrame): 2D embedding data with required columns:
            - market_id (int/str): Unique market identifier
            - embedding (list): 2D coordinates as [x, y] lists
        clusters_df (pd.DataFrame): Cluster assignments with required columns:
            - market_id (int/str): Unique market identifier
            - cluster (int): Cluster ID (-1 for outliers, 0+ for valid clusters)
        markets_df (pd.DataFrame): Market data with required columns:
            - id (int/str): Market identifier (matches market_id)
            - title (str): Market title/description
            - volume_usd (float): Trading volume in USD
            - platform_slug (str): Platform identifier
        cluster_info_dict (dict): Cluster information with keywords from generate_cluster_keywords_tfidf()
        output_file (str): Path where the interactive HTML file will be saved
        display_prob (float): Probability for sampling data (0.0-1.0) to improve performance

    Returns:
        None

    Interactive Features:
        - Hover tooltips with market details, volume, platform, and cluster keywords
        - Toggle buttons to show/hide outliers
        - Color-coded clusters with legend
        - Responsive layout (1200x800 pixels)
        - Grid lines and clean white background

    Side Effects:
        - Creates/overwrites the output HTML file with embedded Plotly.js
        - Samples data based on display_prob for performance (uses random seed 42)
        - Merges multiple DataFrames with suffix handling for column conflicts
        - Prints warnings for missing data or merge failures
        - Falls back gracefully if visualization creation fails
        - Uses significant memory for large datasets during processing
    """
    try:
        # Merge all data together - handle potential column conflicts
        viz_data = embeddings_2d_df.copy()

        # Rename embedding column first to avoid conflicts
        viz_data = viz_data.rename(columns={'embedding': 'embedding_2d'})
        viz_data = viz_data.merge(clusters_df, on='market_id', how='inner')

        # For the final merge, be explicit about suffixes and drop duplicates
        viz_data = viz_data.merge(
            markets_df,
            left_on='market_id',
            right_on='id',
            how='inner',
            suffixes=('', '_markets')
        )

        # Handle any remaining duplicate columns by keeping the left version
        if 'cluster_markets' in viz_data.columns:
            viz_data = viz_data.drop('cluster_markets', axis=1)
        if 'market_id_markets' in viz_data.columns:
            viz_data = viz_data.drop('market_id_markets', axis=1)

        # Sample data for performance if needed
        if display_prob < 1.0:
            viz_data = viz_data.sample(frac=display_prob, random_state=42)

        if viz_data.empty:
            print("Warning: No valid market data found for visualization")
            return

        # Check for required columns
        required_columns = ['embedding_2d', 'cluster', 'market_id', 'title', 'volume_usd', 'platform_slug']
        missing_columns = [col for col in required_columns if col not in viz_data.columns]
        if missing_columns:
            print(f"DEBUG: Missing required columns: {missing_columns}")
            print(f"DEBUG: Available columns: {list(viz_data.columns)}")

        # Extract coordinates and prepare data
        coordinates = np.stack(viz_data['embedding_2d'].values)

        # Create the main scatter plot
        fig = go.Figure()

        # Get unique clusters
        unique_clusters = viz_data['cluster'].unique()
        colors = px.colors.qualitative.Set3 + px.colors.qualitative.Pastel + px.colors.qualitative.Dark24

        # Plot outliers first (cluster -1)
        if -1 in unique_clusters:
            outlier_data = viz_data[viz_data['cluster'] == -1]
            outlier_coords = np.stack(outlier_data['embedding_2d'].values)

            fig.add_trace(go.Scatter(
                x=outlier_coords[:, 0],
                y=outlier_coords[:, 1],
                mode='markers',
                marker=dict(size=3, color='lightgray', opacity=0.3),
                name='Outliers',
                text=[f"Market ID: {row['market_id']}<br>Title: {str(row['title'])[:100]}<br>"
                      f"Volume: ${row['volume_usd']:,.2f}<br>Platform: {row['platform_slug']}"
                      for _, row in outlier_data.iterrows()],
                hovertemplate='<b>%{text}</b><extra></extra>',
                visible=True
            ))

        # Plot regular clusters
        regular_clusters = sorted([c for c in unique_clusters if c != -1])
        for i, cluster_id in enumerate(regular_clusters):
            cluster_data = viz_data[viz_data['cluster'] == cluster_id]
            cluster_coords = np.stack(cluster_data['embedding_2d'].values)
            cluster_color = colors[i % len(colors)]

            # Get cluster info if available
            cluster_keywords = ""
            if cluster_id in cluster_info_dict:
                keywords = cluster_info_dict[cluster_id].get('keywords', '')
                cluster_keywords = f"<br>Keywords: {keywords}" if keywords else ""

            fig.add_trace(go.Scatter(
                x=cluster_coords[:, 0],
                y=cluster_coords[:, 1],
                mode='markers',
                marker=dict(size=4, color=cluster_color, opacity=0.7),
                name=f'Cluster {cluster_id}',
                text=[f"Market ID: {row['market_id']}<br>Title: {str(row['title'])[:100]}<br>"
                      f"Volume: ${row['volume_usd']:,.2f}<br>Platform: {row['platform_slug']}<br>"
                      f"Cluster: {cluster_id}{cluster_keywords}"
                      for _, row in cluster_data.iterrows()],
                hovertemplate='<b>%{text}</b><extra></extra>',
                visible=True
            ))

        # Update layout
        fig.update_layout(
            title=f"Interactive Market Embeddings Clusters ({method})",
            xaxis_title=f"{method} Component 1",
            yaxis_title=f"{method} Component 2",
            width=1200, height=800,
            hovermode='closest',
            showlegend=True,
            legend=dict(yanchor="top", y=0.99, xanchor="left", x=1.01),
            margin=dict(l=50, r=150, t=80, b=50),
            plot_bgcolor='white',
            paper_bgcolor='white'
        )

        # Add buttons to toggle outliers
        fig.update_layout(
            updatemenus=[
                dict(
                    type="buttons",
                    direction="left",
                    buttons=[
                        dict(args=[{"visible": [True] * len(fig.data)}],
                             label="Show All", method="update"),
                        dict(args=[{"visible": [trace.name != 'Outliers' for trace in fig.data]}],
                             label="Hide Outliers", method="update")
                    ],
                    pad={"r": 10, "t": 10},
                    showactive=True,
                    x=0.01, xanchor="left",
                    y=1.02, yanchor="top"
                ),
            ]
        )

        # Add grid
        fig.update_xaxes(showgrid=True, gridwidth=1, gridcolor='lightgray')
        fig.update_yaxes(showgrid=True, gridwidth=1, gridcolor='lightgray')

        # Save as HTML
        fig.write_html(output_file, include_plotlyjs=True)
        print(f"Static plot saved to {output_file}")

    except Exception as e:
        print(f"Error creating interactive visualization: {e}")
        print("Falling back to static visualization only")

def generate_cluster_keywords_tfidf(cluster_info_dict, n=NUM_KEYWORDS, use_tfidf=True):
    """
    Extract representative keywords for each cluster using TF-IDF or frequency analysis.

    Analyzes market titles within each cluster to identify the most characteristic terms.
    TF-IDF approach considers term importance across all clusters for better keyword quality,
    while frequency analysis serves as a fallback method.

    Args:
        cluster_info_dict (dict): Cluster information dictionary with structure:
            - cluster_id (key): Integer cluster identifier
            - markets (list): List of market dictionaries containing 'title' field
        n (int): Number of keywords to extract per cluster (default: NUM_KEYWORDS=10)
        use_tfidf (bool): Whether to use TF-IDF analysis (default: True)

    Returns:
        dict: Updated cluster_info_dict with added 'keywords' field for each cluster:
            - All original fields preserved
            - keywords (str): Comma-separated list of representative terms

    Algorithm Details:
        TF-IDF Mode:
        - Combines all titles in each cluster into a single document
        - Uses scikit-learn TfidfVectorizer with English stop words
        - Parameters: max_features=1000, max_df=0.8, min_df=1
        - Extracts top n terms by TF-IDF score per cluster

        Frequency Mode (fallback):
        - Counts word occurrences within each cluster
        - Filters out common English words manually
        - Selects most frequent terms (minimum 3 characters)

    Side Effects:
        - Modifies cluster_info_dict in-place by adding 'keywords' field
        - Applies emoji removal to all market titles via remove_emoji()
        - Prints keyword generation progress to stdout
        - Falls back to frequency analysis if TF-IDF fails
        - Handles missing or empty market data gracefully
        - Sets keywords to 'No markets' or 'No titles' for invalid clusters
    """
    print("Generating cluster keywords...")

    if not cluster_info_dict:
        return cluster_info_dict

    # Collect all cluster documents
    cluster_docs = {}
    all_titles = []

    for cluster_id, cluster_info in cluster_info_dict.items():
        if not cluster_info or 'markets' not in cluster_info:
            cluster_info['keywords'] = 'No markets'
            continue

        markets_df = pd.DataFrame(cluster_info['markets'])
        if markets_df.empty or 'title' not in markets_df.columns:
            cluster_info['keywords'] = 'No titles'
            continue

        # Clean titles efficiently
        titles = markets_df['title'].dropna()
        cleaned_titles = [remove_emoji(title) for title in titles]
        cluster_doc = ' '.join(cleaned_titles)
        cluster_docs[cluster_id] = cluster_doc
        all_titles.extend(cleaned_titles)

    if not cluster_docs:
        return cluster_info_dict

    if use_tfidf and len(cluster_docs) > 1:
        # Use TF-IDF for better keyword extraction
        vectorizer = TfidfVectorizer(
            max_features=1000,
            stop_words='english',
            token_pattern=r'\b[a-zA-Z]{2,}\b',
            lowercase=True,
            max_df=0.8,  # Ignore terms that appear in >80% of clusters
            min_df=1     # Must appear at least once
        )

        try:
            docs = [cluster_docs[cid] for cid in sorted(cluster_docs.keys())]
            tfidf_matrix = vectorizer.fit_transform(docs)
            feature_names = vectorizer.get_feature_names_out()

            for i, cluster_id in enumerate(sorted(cluster_docs.keys())):
                # Get top n terms by TF-IDF score
                scores = tfidf_matrix[i].toarray()[0]
                top_indices = scores.argsort()[-n:][::-1]
                top_words = [feature_names[idx] for idx in top_indices if scores[idx] > 0]
                cluster_info_dict[cluster_id]['keywords'] = ', '.join(top_words[:n])

        except ValueError:
            # Fall back to frequency analysis if TF-IDF fails
            use_tfidf = False

    if not use_tfidf:
        # Use traditional frequency analysis as fallback
        for cluster_id, cluster_doc in cluster_docs.items():
            words = re.findall(r'\b[a-zA-Z]{3,}\b', cluster_doc.lower())
            word_counts = Counter(words)
            # Filter common words
            common_words = {'the', 'and', 'will', 'are', 'for', 'that', 'this', 'with', 'from', 'they'}
            filtered_words = {w: c for w, c in word_counts.items() if w not in common_words}
            top_words = [word for word, _ in Counter(filtered_words).most_common(n)]
            cluster_info_dict[cluster_id]['keywords'] = ', '.join(top_words)

    return cluster_info_dict

def main():
    """
    Main entry point for market embedding analysis and clustering pipeline.

    Orchestrates the complete analysis workflow including data loading, caching,
    embedding processing, clustering, visualization, and report generation.
    Supports extensive command-line configuration and intelligent caching.

    Workflow Steps:
        1. Load market data from PostgREST API with caching
        2. Calculate composite market scores using volume/traders/duration
        3. Load and optionally apply PCA reduction to embeddings
        4. Compute novelty scores using FAISS similarity search
        5. Create consolidated master DataFrame with all market data
        6. Perform HDBSCAN clustering on embeddings
        7. Generate comprehensive cluster statistics and keywords
        8. Create dashboard visualizations and interactive plots
        9. Generate summary reports with top novel markets and cluster analysis

    Output Files:
        - cluster_dashboard.png: Multi-panel statistical dashboard
        - clusters_{method}.png: Static cluster visualization
        - clusters_{method}_interactive.html: Interactive Plotly visualization

    Environment Variables:
        - PGRST_URL: PostgREST base URL for API endpoints

    Side Effects:
        - Creates cache and output directories as needed
        - Downloads data from external APIs (markets, embeddings)
        - Creates/updates multiple cache files for performance
        - Generates visualization files in output directory
        - Prints extensive progress information and summary statistics
        - Uses significant memory and CPU for large datasets
        - May take considerable time for large datasets (especially UMAP)
    """
    parser = argparse.ArgumentParser(description="Market embedding analysis with clustering")
    parser.add_argument("--cache-dir", "-cd", default="./cache",
                       help="Cache directory (default: ./cache)")
    parser.add_argument("--reset-cache", action="store_true",
                       help="Reset cache and re-download all data")
    parser.add_argument("--output-dir", "-od", default=".",
                       help="Output directory for PNG files (default: current directory)")
    parser.add_argument("--pca-dim", "-d", type=int, default=300,
                       help="PCA dimensionality reduction target (default: 300, 0 to skip)")
    parser.add_argument("--sample-size", "-ss", type=int, default=0,
                       help="Sample size for clustering (default: all)")
    parser.add_argument("--sample-platform", "-sp", type=str, default=None,
                       help="Filter sample to specific platform_slug (default: all)")
    parser.add_argument("--min-cluster-size", "-c", type=int, default=250,
                       help="Minimum cluster size for HDBSCAN (default: 250)")
    parser.add_argument("--plot-method", "-p", default="tsne",
                       choices=["umap", "tsne", "pca"],
                       help="Plotting method for clusters (default: tsne)")
    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Create cache file names with platform filtering
    platform_suffix = f"_{args.sample_platform}" if args.sample_platform else ""
    markets_cache = f"{args.cache_dir}/markets.jsonl"
    market_embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"
    market_embeddings_pca_cache = f"{args.cache_dir}/market_embeddings_pca_{args.pca_dim}.jsonl"
    novelty_cache = f"{args.cache_dir}/market_novelty.jsonl"
    cluster_cache = f"{args.cache_dir}/market_clusters_{args.sample_size}_{args.min_cluster_size}{platform_suffix}.jsonl"
    cluster_info_cache = f"{args.cache_dir}/cluster_info_{args.sample_size}_{args.min_cluster_size}{platform_suffix}.jsonl"
    embeddings_2d_cache = f"{args.cache_dir}/embeddings_2d_{args.sample_size}_{args.plot_method}{platform_suffix}.jsonl"

    # Reset cache if requested
    if args.reset_cache:
        import shutil
        if os.path.exists(args.cache_dir):
            shutil.rmtree(args.cache_dir)
        print(f"Cache directory {args.cache_dir} cleared.")

    # Create cache & output directory if it doesn't exist
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)

    # Step 1: Load and prepare base data
    print("Loading base market data...")
    markets_df = load_dataframe_from_cache(markets_cache)
    if markets_df is None:
        markets_df = get_data_as_dataframe(f"{postgrest_base}/markets", params={"order": "id"})
        save_dataframe_to_cache(markets_cache, markets_df)

    # Apply platform filtering
    if args.sample_platform:
        original_count = len(markets_df)
        markets_df = markets_df[markets_df["platform_slug"] == args.sample_platform]
        print(f"Platform filtering: {len(markets_df)}/{original_count} markets from '{args.sample_platform}'")

    # Calculate market scores using vectorized operations
    markets_df['score'] = calculate_market_scores(markets_df)

    # Step 2: Load embeddings
    print("Loading market embeddings...")
    if args.pca_dim > 0:
        embeddings_df = load_dataframe_from_cache(market_embeddings_pca_cache)
        if embeddings_df is not None:
            print(f"Loaded PCA-reduced embeddings from cache ({args.pca_dim}D)")

    if args.pca_dim == 0 or embeddings_df is None:
        embeddings_df = load_dataframe_from_cache(market_embeddings_cache)
        if embeddings_df is None:
            print("Loading embeddings from API...")
            raw_embeddings = get_data_as_dataframe(f"{postgrest_base}/market_embeddings", params={"order": "market_id"})
            # Parse JSON embeddings more efficiently
            print("Parsing embedding data...")
            embeddings_df = pd.DataFrame({
                'market_id': raw_embeddings['market_id'],
                'embedding': raw_embeddings['embedding'].apply(json.loads)
            })

            # Convert to more efficient format and validate
            print(f"Loaded {len(embeddings_df)} embeddings with dimension {len(embeddings_df['embedding'].iloc[0])}")
            save_dataframe_to_cache(market_embeddings_cache, embeddings_df)

        # Apply PCA dimensionality reduction if requested
        if args.pca_dim > 0:
            embeddings_df = apply_pca_reduction(embeddings_df, args.pca_dim)
            save_dataframe_to_cache(market_embeddings_pca_cache, embeddings_df)

    # Step 3: Load novelty scores
    print("Loading novelty scores...")
    novelty_df = load_dataframe_from_cache(novelty_cache)
    if novelty_df is None:
        # Only compute novelty for markets we have
        analysis_embeddings = embeddings_df[embeddings_df['market_id'].isin(markets_df['id'])]
        novelty_df = compute_novelty_faiss(analysis_embeddings)
        save_dataframe_to_cache(novelty_cache, novelty_df)

    # Step 4: Create master DataFrame with all market data
    print("Creating consolidated market analysis DataFrame...")
    master_df = (markets_df
                 .merge(embeddings_df, left_on='id', right_on='market_id', how='inner')
                 .merge(novelty_df, left_on='id', right_on='market_id', how='inner', suffixes=('', '_novelty')))

    print(f"Master DataFrame contains {len(master_df)} markets with complete data")

    # Step 5: Create clusters
    clusters_df = load_dataframe_from_cache(cluster_cache)
    if clusters_df is None:
        # Sample for clustering if requested
        if args.sample_size == 0 or args.sample_size >= len(master_df):
            clustering_data = pd.DataFrame({
                'market_id': master_df['id'],
                'embedding': master_df['embedding']
            })
            print(f"Using all {len(clustering_data)} markets for clustering")
        else:
            clustering_sample = master_df.sample(n=args.sample_size, random_state=42)
            clustering_data = pd.DataFrame({
                'market_id': clustering_sample['id'],
                'embedding': clustering_sample['embedding']
            })
            print(f"Using sample of {len(clustering_data)} markets for clustering")

        clusters_df = create_clusters_hdbscan(clustering_data, args.min_cluster_size)
        save_dataframe_to_cache(cluster_cache, clusters_df)

    # Add cluster information to master DataFrame
    master_df = master_df.merge(clusters_df, left_on='id', right_on='market_id', how='left', suffixes=('', '_cluster'))
    master_df['cluster'] = master_df['cluster'].fillna(-1).astype(int)  # Non-clustered markets get -1
    clustered_df = master_df[master_df['cluster'] != -1]
    print(f"Successfully clustered {len(clustered_df)}/{len(master_df)} markets")

    # Step 6: Generate cluster information
    cached_cluster_info = load_dataframe_from_cache(cluster_info_cache)
    if cached_cluster_info is None:
        cluster_info_dict = collate_cluster_information(clustered_df, novelty_df)
        # Cache cluster statistics (without full market data)
        cluster_stats = []
        for cluster_id, info in cluster_info_dict.items():
            stats = {k: v for k, v in info.items() if k not in ['markets', 'top_market', 'first_market']}
            stats['cluster_id'] = cluster_id
            cluster_stats.append(stats)
        save_dataframe_to_cache(cluster_info_cache, pd.DataFrame(cluster_stats))
    else:
        # Reconstruct cluster info from cache and add current market data
        cluster_info_dict = {}
        for _, row in cached_cluster_info.iterrows():
            cluster_id = row['cluster_id']
            cluster_info_dict[cluster_id] = row.drop('cluster_id').to_dict()

        # Add live market data for current run
        for cluster_id in cluster_info_dict.keys():
            cluster_markets = clustered_df[clustered_df['cluster'] == cluster_id]
            if len(cluster_markets) > 0:
                cluster_info_dict[cluster_id]['markets'] = cluster_markets.to_dict(orient='records')  # type: ignore
                top_market_idx = cluster_markets['score'].idxmax() if hasattr(cluster_markets['score'], 'idxmax') else cluster_markets['score'].argmax()  # type: ignore
                cluster_info_dict[cluster_id]['top_market'] = cluster_markets.loc[top_market_idx].to_dict()  # type: ignore

    # Generate cluster keywords
    cluster_info_dict = generate_cluster_keywords_tfidf(cluster_info_dict)

    # Step 7: Create visualizations
    create_cluster_dashboard(cluster_info_dict, args.output_dir)

    # Generate 2D embeddings for visualization
    embeddings_2d_df = load_dataframe_from_cache(embeddings_2d_cache)
    if embeddings_2d_df is None:
        print(f"Generating {args.plot_method.upper()} 2D embeddings for visualization...")
        viz_embeddings = pd.DataFrame({
            'market_id': clustered_df['id'],
            'embedding': clustered_df['embedding']
        })

        if args.plot_method == "umap":
            embeddings_2d_df = dimension_reduction_umap(viz_embeddings)
        elif args.plot_method == "tsne":
            embeddings_2d_df = dimension_reduction_tsne(viz_embeddings)
        elif args.plot_method == "pca":
            embeddings_2d_df = dimension_reduction_pca(viz_embeddings)
        else:
            raise ValueError(f"Invalid plot method: {args.plot_method}")

        save_dataframe_to_cache(embeddings_2d_cache, embeddings_2d_df)

    # Create visualizations
    print("Creating visualizations...")
    output_file = f"{args.output_dir}/clusters_{args.plot_method}.png"
    plot_clusters(args.plot_method.upper(), embeddings_2d_df, clusters_df, output_file)

    html_output_file = f"{args.output_dir}/clusters_{args.plot_method}_interactive.html"
    display_prob = min(1.0, DISPLAY_SAMPLE_SIZE / len(embeddings_2d_df))
    create_interactive_visualization(args.plot_method.upper(), embeddings_2d_df, clusters_df,
                                   master_df, cluster_info_dict, html_output_file, display_prob)
    print(f"Interactive plot saved to {html_output_file}")

    # Step 8: Generate summary reports using consolidated DataFrame
    print("\n" + "="*80)
    print("MARKET ANALYSIS SUMMARY")
    print("="*80)

    print("\nMost Novel Markets:")
    print(master_df.nlargest(10, 'novelty')[['id', 'title', 'novelty']])

    print("\nLeast Novel Markets:")
    print(master_df.nsmallest(10, 'novelty')[['id', 'title', 'novelty']])

    print("\nClusters Summary:")
    cluster_summary = []
    for cluster_id, info in cluster_info_dict.items():
        keywords = info.get("keywords", "")
        keywords = keywords[:52] + "..." if len(keywords) > 55 else keywords

        cluster_summary.append([
            cluster_id,
            info.get("market_count", 0),
            keywords,
            f"{info.get('top_platform', 'unknown')} ({100.0*info.get('top_platform_pct', 0):.2f}%)",
            f"{info.get('median_novelty', 0):.3f}",
            f"${info.get('median_volume_usd', 0):.0f}",
            f"{info.get('mean_resolution', 0):.3f}"
        ])

    print(tabulate(cluster_summary,
                  headers=['ID', 'Count', 'Keywords', 'Top Platform', 'Md Novelty', 'Md Volume', 'Mn Res'],
                  tablefmt="github"))

if __name__ == "__main__":
    main()
