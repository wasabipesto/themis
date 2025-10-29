import os
import re
import math
import json
import requests
from tqdm import trange, tqdm
import pandas as pd
import numpy as np
from sklearn.decomposition import PCA
import faiss

# Constants
DEFAULT_BATCH_SIZE = 20_000
DEFAULT_FAISS_NLIST = 1024
DEFAULT_FAISS_BATCH_SIZE = 5000
JITTER_SCALE = 1e-6
TITLE_MAX_LENGTH = 100
DISPLAY_SAMPLE_SIZE = 50_000
NUM_KEYWORDS = 10

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

def get_single_item(endpoint, headers={}, params={}):
    response = requests.get(endpoint, headers=headers, params=params)
    response.raise_for_status()
    return response.json()

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
    Calculate composite market scores using logarithmic scaling and weighted averaging.

    Combines volume (USD) and trader count into a single score using logarithmic
    transformations. Uses weighted averaging where NaN values are ignored rather
    than replaced. If both values are missing, uses a hardcoded value.

    Args:
        df (pd.DataFrame): Market data with required columns:
            - volume_usd (float): Trading volume in USD
            - traders_count (int): Number of unique traders

    Returns:
        np.ndarray: Array of calculated market scores (float)

    Formula:
        For each metric:
        1. Apply log1p transformation to handle skewness
        2. Compute weighted average ignoring NaN values
        3. Use hardcoded value if both metrics are missing

    Side Effects:
        - None (pure computation, no external modifications)
    """
    # Coefficients for balanced contribution
    VOLUME_COEF = 1.0
    TRADERS_COEF = 5.0

    # Extract arrays
    volume_arr = df['volume_usd'].values.copy()
    traders_arr = df['traders_count'].values.copy()

    # Identify valid (non-NaN, positive) values
    volume_valid = ~(np.isnan(volume_arr) | (volume_arr <= 0))
    traders_valid = ~np.isnan(traders_arr)

    # Apply logarithmic transformation to valid values only
    log_volume = np.full_like(volume_arr, np.nan)
    log_traders = np.full_like(traders_arr, np.nan)

    log_volume[volume_valid] = np.log1p(volume_arr[volume_valid])
    log_traders[traders_valid] = np.log1p(traders_arr[traders_valid])

    # Calculate weighted average, ignoring NaN values
    scores = np.full(len(df), 0.0)  # Default value when both are missing

    for i in range(len(df)):
        valid_components = []
        weights = []

        if volume_valid[i]:
            valid_components.append(log_volume[i])
            weights.append(VOLUME_COEF)

        if traders_valid[i]:
            valid_components.append(log_traders[i])
            weights.append(TRADERS_COEF)

        if valid_components:
            # Weighted average of valid components
            scores[i] = np.average(valid_components, weights=weights)

    # Shift to ensure positive scores for easier interpretation
    min_score = np.min(scores)
    if min_score < 0:
        scores = scores - min_score

    return scores

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
