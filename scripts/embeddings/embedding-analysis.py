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
# ]
# ///

"""
Market Embedding Analysis with Clustering - Pandas Refactor

Major improvements made by converting from lists/dicts to pandas DataFrames:

1. Memory Efficiency:
   - DataFrames use columnar storage, reducing memory overhead
   - Efficient data types and vectorized operations
   - Better handling of missing values with built-in NaN support

2. Performance Gains:
   - Vectorized operations for market score calculations
   - Efficient merging instead of manual dictionary lookups
   - Pandas groupby operations for cluster analysis
   - Built-in statistical operations (median, mean, etc.)

3. Code Quality:
   - More readable data manipulation with pandas methods
   - Consistent DataFrame-based caching (JSONL format)
   - Better data filtering and selection capabilities
   - Reduced manual loops and list comprehensions

4. Enhanced Analysis:
   - Streamlined cluster information collation using groupby
   - Improved data preparation for visualizations
   - More efficient novelty computation with batch processing
   - Better handling of duplicate embeddings

5. Consolidated Data Management (main() function):
   - Single master DataFrame combining markets, embeddings, novelty, and clusters
   - Eliminated redundant data structures and mapping dictionaries
   - Reduced multiple merge operations to single consolidated workflow
   - Streamlined filtering and sampling using DataFrame operations
   - More efficient memory usage with fewer data copies

The refactor maintains backward compatibility with existing cache files
and preserves all original functionality while providing significant
performance and maintainability improvements.
"""

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
from collections import Counter
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots

def get_data_as_dataframe(endpoint: str, headers={}, params={}, batch_size=20_000):
    """Get data from a PostgREST endpoint and return as pandas DataFrame."""
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
    """Load DataFrame from JSONL cache file."""
    if not os.path.exists(cache_file):
        return None

    try:
        df = pd.read_json(cache_file, lines=True)
        print(f"Loaded {len(df)} rows from {os.path.basename(cache_file)}")
        return df
    except (ValueError, OSError) as e:
        print(f"Warning: Failed to load cache file ({e}). Re-downloading.")
        return None

def save_dataframe_to_cache(cache_file, df):
    """Save DataFrame to JSONL cache file."""
    os.makedirs(os.path.dirname(cache_file), exist_ok=True)
    try:
        df.to_json(cache_file, orient='records', lines=True)
        print(f"Saved {len(df)} rows to {os.path.basename(cache_file)}")
    except OSError as e:
        print(f"Warning: Failed to save cache file ({e}).")

def calculate_market_scores(df):
    """
    Calculate market scores using vectorized operations.
    Assumes 0 if any values are None/NaN.
    """
    volume_coef = 0.001
    traders_coef = 10.0
    duration_coef = 1.0

    # Fill NaN values with 0 for calculation
    volume_usd = df['volume_usd'].fillna(0)
    traders_count = df['traders_count'].fillna(0)
    duration_days = df['duration_days'].fillna(0)

    return volume_coef * volume_usd + traders_coef * traders_count + duration_coef * duration_days

def compute_novelty_faiss(embeddings_df, n=10, nlist=1024, batch_size=5000):
    """
    Memory-efficient, CPU-optimized novelty computation using FAISS.
    Returns DataFrame with market_id and novelty columns.
    """
    # Extract vectors and IDs
    market_ids = embeddings_df['market_id'].values
    vectors = np.stack(embeddings_df['embedding'].values).astype('float32')

    # Normalize for cosine similarity
    faiss.normalize_L2(vectors)

    dim = vectors.shape[1]

    # Use all CPU cores
    faiss.omp_set_num_threads(0)

    # IVF index (approximate nearest neighbors)
    quantizer = faiss.IndexFlatIP(dim)
    index = faiss.IndexIVFFlat(quantizer, dim, nlist, faiss.METRIC_INNER_PRODUCT)

    # Train index (required for IVF)
    print("Training FAISS index...")
    index.train(vectors)
    index.add(vectors)
    print(f"Index trained and added {len(vectors)} vectors.")

    novelty_scores = []

    # Process in batches
    num_batches = (len(vectors) + batch_size - 1) // batch_size
    for start in tqdm(range(0, len(vectors), batch_size), desc="Computing novelty"):
        end = min(start + batch_size, len(vectors))
        batch_vectors = vectors[start:end]
        distances, _ = index.search(batch_vectors, n + 1)  # n+1 because first neighbor is self

        # Convert similarity → distance
        batch_novelty = np.mean(1 - distances[:, 1:], axis=1)
        novelty_scores.extend(batch_novelty)

    return pd.DataFrame({
        'market_id': market_ids,
        'novelty': novelty_scores
    })

def create_clusters_hdbscan(embeddings_df, min_cluster_size):
    """
    Cluster markets using HDBSCAN on embeddings.
    Returns DataFrame with market_id and cluster columns.
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
    Apply PCA dimensionality reduction to embeddings DataFrame.
    Returns updated DataFrame with reduced embeddings.
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
    Collate comprehensive cluster information using
 pandas groupby operations.
    Returns dictionary with cluster statistics.
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
    Create a comprehensive dashboard showing cluster analysis.
    All plots on one matplotlib canvas.
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
    Add slight jitter to duplicate embedding values for unique locations.
    Uses deterministic jitter based on market_id for reproducibility.
    """
    print(f"Adding jitter to duplicate embeddings...", end="")

    # Convert embeddings to hashable format for duplicate detection
    embedding_hashes = embeddings_df['embedding'].apply(lambda x: json.dumps(x))
    duplicate_mask = embedding_hashes.duplicated()

    if not duplicate_mask.any():
        print("No duplicates found.")
        return embeddings_df

    result_df = embeddings_df.copy()
    markets_updated = 0

    for idx in result_df[duplicate_mask].index:
        market_id = result_df.loc[idx, 'market_id']
        embedding = result_df.loc[idx, 'embedding'].copy()

        # Add deterministic jitter based on market ID
        random.seed(hash(market_id) % (2**32))
        jitter_scale = 1e-6
        new_embedding = [x + random.uniform(-jitter_scale, jitter_scale) for x in embedding]
        result_df.loc[idx, 'embedding'] = new_embedding
        markets_updated += 1

    print(f"Done. Updated {markets_updated} duplicate embeddings.")
    return result_df

def dimension_reduction_umap(embeddings_df, n_jobs=6):
    """
    Reduce embeddings to 2D using UMAP.
    Returns DataFrame with market_id and 2D embedding.
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
    Reduce embeddings to 2D using t-SNE.
    Returns DataFrame with market_id and 2D embedding.
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
    Reduce embeddings to 2D using PCA.
    Returns DataFrame with market_id and 2D embedding.
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
    Plot clusters given 2D embeddings and cluster assignments.
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
    Create an interactive HTML visualization with hover tooltips and interactive features.
    """
    try:
        # Merge all data together
        viz_data = (embeddings_2d_df
                    .merge(clusters_df, on='market_id', how='inner')
                    .merge(markets_df, left_on='market_id', right_on='id', how='inner'))

        # Sample data for performance if needed
        if display_prob < 1.0:
            viz_data = viz_data.sample(frac=display_prob, random_state=42)

        if viz_data.empty:
            print("Warning: No valid market data found for visualization")
            return

        # Extract coordinates and prepare data
        coordinates = np.stack(viz_data['embedding'].values)
        embeddings_x = coordinates[:, 0]
        embeddings_y = coordinates[:, 1]

        # Create the main scatter plot
        fig = go.Figure()

        # Get unique clusters
        unique_clusters = viz_data['cluster'].unique()
        colors = px.colors.qualitative.Set3 + px.colors.qualitative.Pastel + px.colors.qualitative.Dark24

        # Plot outliers first (cluster -1)
        if -1 in unique_clusters:
            outlier_data = viz_data[viz_data['cluster'] == -1]
            outlier_coords = np.stack(outlier_data['embedding'].values)

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
            cluster_coords = np.stack(cluster_data['embedding'].values)
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

    except Exception as e:
        print(f"Error creating interactive visualization: {e}")
        print("Falling back to static visualization only")

def generate_cluster_keywords(cluster_info_dict, n=10):
    """
    Generate keywords for each cluster using word frequency analysis.
    """
    # Common stop words to filter out
    stop_words = {
        'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by',
        'will', 'be', 'is', 'are', 'was', 'were', 'been', 'have', 'has', 'had', 'do', 'does',
        'did', 'can', 'could', 'would', 'should', 'may', 'might', 'must', 'shall', 'this',
        'that', 'these', 'those', 'i', 'you', 'he', 'she', 'it', 'we', 'they', 'me', 'him',
        'her', 'us', 'them', 'my', 'your', 'his', 'its', 'our', 'their', 'what', 'which',
        'who', 'when', 'where', 'why', 'how', 'if', 'than', 'then', 'now', 'here', 'there',
        'up', 'down', 'out', 'off', 'over', 'under', 'again', 'further', 'once', 'more',
        'most', 'other', 'some', 'any', 'only', 'own', 'same', 'so', 'than', 'too', 'very',
        'yes', 'no'
    }

    for cluster_id, cluster_info in cluster_info_dict.items():
        if not cluster_info or 'markets' not in cluster_info:
            cluster_info['keywords'] = 'No markets'
            continue

        # Extract all titles from markets in this cluster using pandas
        markets_df = pd.DataFrame(cluster_info['markets'])
        if markets_df.empty or 'title' not in markets_df.columns:
            cluster_info['keywords'] = 'No titles'
            continue

        # Clean and process titles
        titles = markets_df['title'].dropna().apply(remove_emoji)

        # Tokenize and count words
        word_counts = Counter()
        for title in titles:
            # Convert to lowercase and extract words (letters only, minimum 2 characters)
            words = re.findall(r'\b[a-zA-Z]{2,}\b', title.lower())
            for word in words:
                if word not in stop_words:
                    word_counts[word] += 1

        # Get most frequent words
        if word_counts:
            top_words = [word for word, count in word_counts.most_common(n)]
            cluster_info['keywords'] = ', '.join(top_words)
        else:
            cluster_info['keywords'] = 'No keywords'

    return cluster_info_dict

def main():
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
            embeddings_df = pd.DataFrame({
                'market_id': raw_embeddings['market_id'],
                'embedding': raw_embeddings['embedding'].apply(json.loads)
            })
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
    cluster_info_dict = generate_cluster_keywords(cluster_info_dict)

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
    print(f"Static plot saved to {output_file}")

    html_output_file = f"{args.output_dir}/clusters_{args.plot_method}_interactive.html"
    display_prob = min(1.0, 50_000 / len(embeddings_2d_df))
    create_interactive_visualization(args.plot_method.upper(), embeddings_2d_df, clusters_df,
                                   master_df, cluster_info_dict, html_output_file, display_prob)
    print(f"Interactive plot saved to {html_output_file}")

    # Step 8: Generate summary reports using consolidated DataFrame
    print("\n" + "="*80)
    print("MARKET ANALYSIS SUMMARY")
    print("="*80)

    print("\n| Most Novel Markets")
    most_novel = master_df.nlargest(20, 'novelty')[['id', 'title', 'volume_usd', 'novelty']].copy()
    most_novel['novelty_fmt'] = most_novel['novelty'].apply(lambda x: f"{x:.4f}")  # type: ignore
    print(tabulate(most_novel[['id', 'title', 'volume_usd', 'novelty_fmt']].values,  # type: ignore
                  headers=['ID', 'Title', 'Volume', 'Novelty'], tablefmt="github"))

    print("\n| Most Novel Markets, >$10 Volume")
    high_volume = master_df[master_df['volume_usd'] > 10].nlargest(20, 'novelty')[['id', 'title', 'volume_usd', 'novelty']].copy()
    high_volume['novelty_fmt'] = high_volume['novelty'].apply(lambda x: f"{x:.4f}")  # type: ignore
    print(tabulate(high_volume[['id', 'title', 'volume_usd', 'novelty_fmt']].values,  # type: ignore
                  headers=['ID', 'Title', 'Volume', 'Novelty'], tablefmt="github"))

    print("\n| Least Novel Markets")
    least_novel = master_df.nsmallest(10, 'novelty')[['id', 'title', 'volume_usd', 'novelty']].copy()
    least_novel['novelty_fmt'] = least_novel['novelty'].apply(lambda x: f"{x:.4f}")  # type: ignore
    print(tabulate(least_novel[['id', 'title', 'volume_usd', 'novelty_fmt']].values,  # type: ignore
                  headers=['ID', 'Title', 'Volume', 'Novelty'], tablefmt="github"))

    print("\n| Clusters Summary:")
    cluster_summary = []
    for cluster_id, info in cluster_info_dict.items():
        title = info.get("top_market_title", "Unknown")
        title = title[:62] + "..." if len(title) > 65 else title

        keywords = info.get("keywords", "")
        keywords = keywords[:52] + "..." if len(keywords) > 55 else keywords

        cluster_summary.append([
            cluster_id,
            info.get("market_count", 0),
            title,
            keywords,
            f"{info.get('top_platform', 'unknown')} ({100.0*info.get('top_platform_pct', 0):.2f}%)",
            f"{info.get('median_novelty', 0):.3f}",
            f"${info.get('median_volume_usd', 0):.0f}",
            f"{info.get('mean_resolution', 0):.3f}"
        ])

    print(tabulate(cluster_summary,
                  headers=['ID', 'Count', 'Top Market', 'Keywords', 'Top Platform', 'Md Novelty', 'Md Volume', 'Mn Res'],
                  tablefmt="github"))

if __name__ == "__main__":
    main()
