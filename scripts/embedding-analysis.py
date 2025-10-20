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
# ]
# ///

import os
import json
import requests
from dotenv import load_dotenv
from tqdm import trange, tqdm
from tabulate import tabulate
import matplotlib.pyplot as plt
import numpy as np
import math
import random
import faiss
import hdbscan
import umap
import argparse
from sklearn.decomposition import PCA
from sklearn.manifold import TSNE
from collections import Counter

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

def load_from_cache(cache_file):
    if not os.path.exists(cache_file):
        return None

    try:
        with open(cache_file, "r", encoding="utf-8") as f:
            lines = f.readlines()
        result = []
        for line in tqdm(lines, desc=f"Loading {os.path.basename(cache_file)}"):
            result.append(json.loads(line))
        return result
    except (json.JSONDecodeError, OSError) as e:
        print(f"Warning: Failed to load cache file ({e}). Re-downloading.")
        return None

def save_to_cache(cache_file, data):
    os.makedirs(os.path.dirname(cache_file), exist_ok=True)
    try:
        with open(cache_file, "w", encoding="utf-8") as f:
            for item in tqdm(data, desc=f"Saving {os.path.basename(cache_file)}"):
                json.dump(item, f, ensure_ascii=False)
                f.write("\n")
    except OSError as e:
        print(f"Warning: Failed to save cache file ({e}).")

def calculate_market_score(volume_usd, traders_count, duration_days):
    """
    Calculate market score based on volume_usd, traders_count, and duration_days.
    Assumes 0 if any are None.
    """
    volume_coef = 0.001
    traders_coef = 10.0
    duration_coef = 1.0

    volume_usd = volume_usd or 0
    traders_count = traders_count or 0
    duration_days = duration_days or 0

    return volume_coef * volume_usd + traders_coef * traders_count + duration_coef * duration_days

def compute_novelty_faiss(market_embeddings, n=10, nlist=1024, batch_size=5000):
    """
    Memory-efficient, CPU-optimized novelty computation using FAISS (approximate, multi-threaded).
    Processes vectors in batches to save memory and shows progress with tqdm.
    """
    # Extract vectors and IDs
    market_ids = [i["market_id"] for i in market_embeddings]
    vectors = np.array([i["embedding"] for i in market_embeddings], dtype='float32')

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

    novelty_results = []

    # Process in batches
    num_batches = (len(vectors) + batch_size - 1) // batch_size
    for start in tqdm(range(0, len(vectors), batch_size), desc="Computing novelty"):
        end = min(start + batch_size, len(vectors))
        batch_vectors = vectors[start:end]
        distances, _ = index.search(batch_vectors, n + 1)  # n+1 because first neighbor is self

        # Convert similarity → distance
        for i, idx in enumerate(range(start, end)):
            novelty_score = float(np.mean(1 - distances[i][1:]))
            novelty_results.append({"market_id": market_ids[idx], "novelty": novelty_score})

    return novelty_results

def create_clusters_hdbscan(market_embeddings, min_cluster_size):
    """
    Cluster markets using HDBSCAN on FAISS embeddings.
    Returns a list of dicts: {"market_id": ..., "cluster": ...}
    """
    market_ids = [i["market_id"] for i in market_embeddings]
    embedding_vectors = np.array([i["embedding"] for i in market_embeddings], dtype='float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Clustering with HDBSCAN...")
    clusterer = hdbscan.HDBSCAN(
        min_cluster_size=min_cluster_size,
        min_samples=10,
        # prediction_data=True
    )
    cluster_labels = clusterer.fit_predict(embedding_vectors)

    clustered_results = [{"market_id": mid, "cluster": int(label)} for mid, label in zip(market_ids, cluster_labels)]
    return clustered_results

def apply_pca_reduction(embeddings, target_dim):
    """
    Apply PCA dimensionality reduction to embeddings.
    Skip if target_dim is zero or greater than raw dimensionality.
    """
    current_dim = len(embeddings[0]['embedding'])
    if target_dim == 0 or target_dim >= current_dim:
        print(f"Skipping PCA: target_dim={target_dim}, embedding_dim={current_dim}")
        return embeddings

    print(f"Applying PCA reduction from {current_dim} to {target_dim} dimensions...")

    # Extract embeddings matrix
    embedding_matrix = np.array([item['embedding'] for item in embeddings], dtype='float32')

    # Apply PCA
    pca = PCA(n_components=target_dim)
    reduced_embeddings = pca.fit_transform(embedding_matrix)

    # Update embeddings with reduced dimensions
    for i, item in enumerate(embeddings):
        item['embedding'] = reduced_embeddings[i].tolist()

    print(f"PCA explained variance ratio: {sum(pca.explained_variance_ratio_):.3f}")
    return embeddings

def collate_cluster_information(cluster_id, markets, market_novelty_mapped):
    """
    Collate comprehensive cluster information.
    """
    if not markets:
        return None

    # Basic info
    info = {
        "markets": markets,
        "market_count": len(markets),
    }

    # Top market by score
    top_market = max(markets, key=lambda x: x["score"])
    info["top_market"] = top_market
    info["top_market_title"] = top_market["title"]

    # First market by open_datetime
    markets_with_dates = [m for m in markets if m.get("open_datetime")]
    if markets_with_dates:
        first_market = min(markets_with_dates, key=lambda x: x["open_datetime"])
        info["first_market"] = first_market
        info["first_market_platform"] = first_market.get("platform_slug", "unknown")
    else:
        info["first_market"] = None
        info["first_market_platform"] = "unknown"

    # Platform proportions
    platforms = [m.get("platform_slug") for m in markets]
    platform_counts = Counter(platforms)
    total_markets = len(markets)
    info["platform_proportions"] = {platform: count/total_markets for platform, count in platform_counts.items()}
    info["top_platform"] = platform_counts.most_common(1)[0][0] if platform_counts else "unknown"

    # Statistical aggregations
    novelty_values = [market_novelty_mapped.get(m["id"]) for m in markets]
    volume_values = [m.get("volume_usd") for m in markets if m.get("volume_usd")]
    traders_values = [m.get("traders_count") for m in markets if m.get("traders_count")]
    duration_values = [m.get("duration_days") for m in markets]
    resolution_values = [m.get("resolution") for m in markets]

    info["median_novelty"] = np.median(novelty_values) if novelty_values else 0
    info["median_volume_usd"] = np.median(volume_values) if volume_values else 0
    info["median_traders_count"] = np.median(traders_values) if traders_values else 0
    info["median_duration_days"] = np.median(duration_values) if duration_values else 0
    info["mean_resolution"] = np.mean(resolution_values) if resolution_values else 0

    return info

def create_cluster_dashboard(cluster_info_dict, output_dir):
    """
    Create a comprehensive dashboard showing cluster analysis.
    All plots on one matplotlib canvas.
    """
    fig = plt.figure(figsize=(20, 15))

    # Prepare data
    cluster_ids = list(cluster_info_dict.keys())
    market_counts = [cluster_info_dict[cid]["market_count"] for cid in cluster_ids]
    median_novelties = [cluster_info_dict[cid]["median_novelty"] for cid in cluster_ids]
    median_volumes = [cluster_info_dict[cid]["median_volume_usd"] for cid in cluster_ids]
    median_traders = [cluster_info_dict[cid]["median_traders_count"] for cid in cluster_ids]
    median_durations = [cluster_info_dict[cid]["median_duration_days"] for cid in cluster_ids]
    mean_resolutions = [cluster_info_dict[cid]["mean_resolution"] for cid in cluster_ids]

    # Plot 1: Bar plot of number of markets
    plt.subplot(3, 3, 1)
    plt.bar(cluster_ids, market_counts)
    plt.xlabel('Cluster ID')
    plt.ylabel('Number of Markets')
    plt.title('Markets per Cluster')
    plt.grid(True)

    # Plot 2: Histogram of market counts
    plt.subplot(3, 3, 2)
    plt.hist(market_counts, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Number of Markets')
    plt.ylabel('Frequency')
    plt.title('Distribution of Market Counts')

    # Plot 3: Platform proportions pie chart
    plt.subplot(3, 3, 3)
    all_platforms = {}
    for cluster_info in cluster_info_dict.values():
        for platform, prop in cluster_info["platform_proportions"].items():
            all_platforms[platform] = all_platforms.get(platform, 0) + prop * cluster_info["market_count"]

    total_markets = sum(all_platforms.values())
    platform_props = {k: v/total_markets for k, v in all_platforms.items()}

    if platform_props:
        plt.pie(platform_props.values(), labels=platform_props.keys(), autopct='%1.1f%%')
        plt.title('Most Prominent Platform Distribution')

    # Plot 4: Median novelty histogram
    plt.subplot(3, 3, 4)
    plt.hist(median_novelties, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Novelty')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Novelty')

    # Plot 5: Median volume histogram (log scale)
    plt.subplot(3, 3, 5)
    non_zero_volumes = [v for v in median_volumes if v > 0]
    if non_zero_volumes:
        plt.hist(non_zero_volumes, bins=20, alpha=0.7, edgecolor='black')
        plt.xscale('log')
    plt.xlabel('Median Volume USD (log scale)')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Volume')

    # Plot 6: Median traders histogram
    plt.subplot(3, 3, 6)
    non_zero_traders = [t for t in median_traders if t > 0]
    if non_zero_traders:
        plt.hist(non_zero_traders, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Traders Count')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Traders')

    # Plot 7: Median duration histogram
    plt.subplot(3, 3, 7)
    non_zero_durations = [d for d in median_durations if d > 0]
    if non_zero_durations:
        plt.hist(non_zero_durations, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Median Duration Days')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Median Duration')

    # Plot 8: Mean resolution histogram
    plt.subplot(3, 3, 8)
    plt.hist(mean_resolutions, bins=20, alpha=0.7, edgecolor='black')
    plt.xlabel('Mean Resolution')
    plt.ylabel('Number of Clusters')
    plt.title('Distribution of Mean Resolution')

    # Plot 9: Scatter plot of volume vs traders
    plt.subplot(3, 3, 9)
    plt.scatter([v for v in median_volumes if v > 0],
               [t for v, t in zip(median_volumes, median_traders) if v > 0],
               alpha=0.6)
    plt.xscale('log')
    plt.xlabel('Median Volume USD (log scale)')
    plt.ylabel('Median Traders Count')
    plt.title('Volume vs Traders by Cluster')

    plt.tight_layout()
    plt.savefig(f"{output_dir}/cluster_dashboard.png", format="png", bbox_inches="tight", dpi=150)
    plt.close()

def remove_duplicates_by_embedding(market_embeddings_mapped, market_clusters):
    """
    Return a copy of market_clusters, but all markets with duplicate embeddings are removed.
    """
    print(f"Deduplicating {len(market_clusters)} markets by embedding...", end="")
    unique_embeddings = set()
    unique_market_clusters = []
    for mc in market_clusters:
        embedding = json.dumps(market_embeddings_mapped[mc["market_id"]])
        if embedding not in unique_embeddings:
            unique_embeddings.add(embedding)
            unique_market_clusters.append(mc)
    print(f"Done. Reduced to {len(unique_market_clusters)} markets.")
    return unique_market_clusters

def plot_clusters_umap(market_embeddings_mapped, market_clusters_orig, output_file):
    """
    Reduce embeddings to 2D using UMAP and plot clusters.
    """
    # Remove duplicate markets
    market_clusters = remove_duplicates_by_embedding(market_embeddings_mapped, market_clusters_orig)

    market_ids = [i["market_id"] for i in market_clusters]
    cluster_labels = np.array([i["cluster"] for i in market_clusters])
    embedding_vectors = np.array([market_embeddings_mapped[id] for id in market_ids], dtype='float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with UMAP...")
    reducer = umap.UMAP()
    embedding_2d = reducer.fit_transform(embedding_vectors)

    plt.figure(figsize=(10, 8))
    scatter = plt.scatter(embedding_2d[:, 0], embedding_2d[:, 1], c=cluster_labels, cmap='tab20', s=4, alpha=0.8)
    plt.colorbar(scatter, label="Cluster")
    plt.title("Market Embeddings Clusters (UMAP)")
    plt.tight_layout()
    plt.savefig(output_file, format="png", bbox_inches="tight")
    plt.close()

def plot_clusters_tsne(market_embeddings_mapped, market_clusters, output_file):
    """
    Reduce embeddings to 2D using t-SNE and plot clusters.
    """
    market_ids = [i["market_id"] for i in market_clusters]
    cluster_labels = np.array([i["cluster"] for i in market_clusters])
    embedding_vectors = np.array([market_embeddings_mapped[id] for id in market_ids], dtype='float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with t-SNE...")
    reducer = TSNE(n_components=2, random_state=42, perplexity=min(30, len(embedding_vectors)-1))
    embedding_2d = reducer.fit_transform(embedding_vectors)

    plt.figure(figsize=(10, 8))
    scatter = plt.scatter(embedding_2d[:, 0], embedding_2d[:, 1], c=cluster_labels, cmap='tab20', s=4, alpha=0.8)
    plt.colorbar(scatter, label="Cluster")
    plt.title("Market Embeddings Clusters (t-SNE)")
    plt.tight_layout()
    plt.savefig(output_file, format="png", bbox_inches="tight")
    plt.close()

def plot_clusters_pca(market_embeddings_mapped, market_clusters, output_file):
    """
    Reduce embeddings to 2D using PCA and plot clusters.
    """
    market_ids = [i["market_id"] for i in market_clusters]
    cluster_labels = np.array([i["cluster"] for i in market_clusters])
    embedding_vectors = np.array([market_embeddings_mapped[id] for id in market_ids], dtype='float32')
    embedding_vectors = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    print("Reducing embeddings to 2D with PCA...")
    pca = PCA(n_components=2, random_state=42)
    embedding_2d = pca.fit_transform(embedding_vectors)

    explained_var = pca.explained_variance_ratio_
    print(f"PCA explained variance: {explained_var[0]:.3f}, {explained_var[1]:.3f} (total: {sum(explained_var):.3f})")

    plt.figure(figsize=(10, 8))
    scatter = plt.scatter(embedding_2d[:, 0], embedding_2d[:, 1], c=cluster_labels, cmap='tab20', s=4, alpha=0.8)
    plt.colorbar(scatter, label="Cluster")
    plt.title(f"Market Embeddings Clusters (PCA) - Explained Var: {sum(explained_var):.1%}")
    plt.xlabel(f"PC1 ({explained_var[0]:.1%})")
    plt.ylabel(f"PC2 ({explained_var[1]:.1%})")
    plt.tight_layout()
    plt.savefig(output_file, format="png", bbox_inches="tight")
    plt.close()

def main():
    parser = argparse.ArgumentParser(description="Market embedding analysis with clustering")
    parser.add_argument("--cache-dir", "-cd", default="cache/embedding-analysis",
                       help="Cache directory (default: cache/embedding-analysis)")
    parser.add_argument("--reset-cache", action="store_true",
                       help="Reset cache and re-download all data")
    parser.add_argument("--output-dir", "-od", default=".",
                       help="Output directory for PNG files (default: current directory)")
    parser.add_argument("--pca-dim", "-d", type=int, default=300,
                       help="PCA dimensionality reduction target (default: 300, 0 to skip)")
    parser.add_argument("--sample-size", "-s", type=int, default=10000,
                       help="Sample size for clustering (default: 10,000)")
    parser.add_argument("--min-cluster-size", "-c", type=int, default=50,
                       help="Minimum cluster size for HDBSCAN (default: 50)")
    parser.add_argument("--plot-method", "-p", default="umap",
                       choices=["umap", "tsne", "pca"],
                       help="Plotting method for clusters (default: umap)")
    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    markets_cache = f"{args.cache_dir}/markets.jsonl"
    market_embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"
    market_embeddings_pca_cache = f"{args.cache_dir}/market_embeddings_pca_{args.pca_dim}.jsonl"
    novelty_cache = f"{args.cache_dir}/market_novelty.jsonl"
    cluster_cache = f"{args.cache_dir}/market_clusters_{args.sample_size}_{args.min_cluster_size}.jsonl"
    cluster_info_cache = f"{args.cache_dir}/cluster_info_{args.sample_size}_{args.min_cluster_size}.jsonl"

    # Reset cache if requested
    if args.reset_cache:
        import shutil
        if os.path.exists(args.cache_dir):
            shutil.rmtree(args.cache_dir)
        print(f"Cache directory {args.cache_dir} cleared.")

    # Create cache & output directory if it doesn't exist
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)

    # Load markets
    markets = load_from_cache(markets_cache)
    if markets is None:
        markets = get_data(f"{postgrest_base}/markets", params={"order": "id"})
        save_to_cache(markets_cache, markets)

    # Calculate market scores
    for m in markets:
        m["score"] = calculate_market_score(m.get("volume_usd"), m.get("traders_count"), m.get("duration_days"))
    markets_mapped = {m["id"]: m for m in markets}

    # Load market embeddings with PCA cache optimization
    market_embeddings_pca = load_from_cache(market_embeddings_pca_cache) if args.pca_dim > 0 else None

    if market_embeddings_pca is not None:
        # PCA cache exists and is valid, use it directly
        embeddings_for_analysis = market_embeddings_pca
        print(f"Using cached PCA embeddings from {market_embeddings_pca_cache}")
    else:
        # Need to load original embeddings
        market_embeddings = load_from_cache(market_embeddings_cache)
        if market_embeddings is None:
            market_embeddings = get_data(f"{postgrest_base}/market_embeddings", params={"order": "market_id"})
            market_embeddings = [{"market_id": i["market_id"], "embedding": json.loads(i["embedding"])} for i in market_embeddings]
            save_to_cache(market_embeddings_cache, market_embeddings)

        # Apply PCA dimensionality reduction if requested
        if args.pca_dim > 0:
            market_embeddings_pca = apply_pca_reduction(market_embeddings.copy(), args.pca_dim)
            save_to_cache(market_embeddings_pca_cache, market_embeddings_pca)
            embeddings_for_analysis = market_embeddings_pca
        else:
            embeddings_for_analysis = market_embeddings
    market_embeddings_mapped = {m["market_id"]: m["embedding"] for m in embeddings_for_analysis}

    # Compute novelty
    market_novelty = load_from_cache(novelty_cache)
    if market_novelty is None:
        market_novelty = compute_novelty_faiss(embeddings_for_analysis)
        save_to_cache(novelty_cache, market_novelty)
    market_novelty_mapped = {m["market_id"]: m["novelty"] for m in market_novelty}

    # Create clusters
    # TODO: Disable sampling (use all markets) if args.sample_size is 0 or greater than number of markets
    market_clusters = load_from_cache(cluster_cache)
    if market_clusters is None:
        market_embeddings_sample = random.sample(embeddings_for_analysis, min(args.sample_size, len(embeddings_for_analysis)))
        market_clusters = create_clusters_hdbscan(market_embeddings_sample, args.min_cluster_size)
        save_to_cache(cluster_cache, market_clusters)

    # Collate cluster information
    cluster_info_dict = {}
    cached_cluster_info = load_from_cache(cluster_info_cache)
    if cached_cluster_info is None:
        cluster_ids = set([mc["cluster"] for mc in market_clusters if mc["cluster"] >= 0])

        for cluster_id in tqdm(cluster_ids, desc="Collating cluster information"):
            market_ids = [m["market_id"] for m in market_clusters if m["cluster"] == cluster_id]
            markets_in_cluster = [markets_mapped[mid] for mid in market_ids if mid in markets_mapped]
            cluster_info_dict[cluster_id] = collate_cluster_information(cluster_id, markets_in_cluster, market_novelty_mapped)

        # Save cluster info (convert to list for JSON serialization)
        cluster_info_list = [{"cluster_id": cid, **info} for cid, info in cluster_info_dict.items()]
        save_to_cache(cluster_info_cache, cluster_info_list)
    else:
        # Reconstruct dict from cached list
        for item in cached_cluster_info:
            cluster_id = item.pop("cluster_id")
            cluster_info_dict[cluster_id] = item

    # Generate plots
    plt.title("Market Novelty Histogram")
    plt.hist([i["novelty"] for i in market_novelty], bins=30)
    plt.savefig(f"{args.output_dir}/novelty_histogram.png", format="png", bbox_inches="tight")
    plt.close()

    # Create cluster dashboard
    create_cluster_dashboard(cluster_info_dict, args.output_dir)

    print("\n| Most Novel Markets")
    print(tabulate(
        [
            [m["market_id"], markets_mapped[m["market_id"]]["title"], markets_mapped[m["market_id"]]["volume_usd"], f"{m["novelty"]:.4f}"]
            for m in sorted(market_novelty, key=lambda x: x["novelty"], reverse=True)[:10]
        ],
        headers=['ID', 'Title', 'Volume', 'Novelty'],
        tablefmt="github"
    ))

    print("\n| Most Novel Markets, >$10 Volume")
    print(tabulate(
        [
            [m["market_id"], markets_mapped[m["market_id"]]["title"], markets_mapped[m["market_id"]]["volume_usd"], f"{m["novelty"]:.4f}"]
            for m in sorted([
                m for m in market_novelty if markets_mapped[m["market_id"]]["volume_usd"] and markets_mapped[m["market_id"]]["volume_usd"] > 10
            ], key=lambda x: x["novelty"], reverse=True)[:30]
        ],
        headers=['ID', 'Title', 'Volume', 'Novelty'],
        tablefmt="github"
    ))

    print("\n| Least Novel Markets")
    print(tabulate(
        [
            [m["market_id"], markets_mapped[m["market_id"]]["title"], markets_mapped[m["market_id"]]["volume_usd"], f"{m["novelty"]:.4f}"]
            for m in sorted(market_novelty, key=lambda x: x["novelty"])[:10]
        ],
        headers=['ID', 'Title', 'Volume', 'Novelty'],
        tablefmt="github"
    ))

    if cluster_info_dict:
        plt.title("Count of Markets per Cluster")
        cluster_ids = list(cluster_info_dict.keys())
        market_counts = [cluster_info_dict[cid]["market_count"] for cid in cluster_ids]
        plt.bar(cluster_ids, market_counts)
        plt.savefig(f"{args.output_dir}/cluster_counts.png", format="png", bbox_inches="tight")
        plt.close()

        print("\n| Clusters Summary:")
        print(tabulate(
            [
                [
                    cluster_id,
                    info["market_count"],
                    info["top_market_title"][:50] + "..." if len(info["top_market_title"]) > 50 else info["top_market_title"],
                    info["top_platform"],
                    f"{info['median_novelty']:.3f}",
                    f"${info['median_volume_usd']:.0f}",
                    f"{info['median_traders_count']:.0f}",
                    f"{info['median_duration_days']:.0f}",
                    f"{info['mean_resolution']:.3f}"
                ]
                for cluster_id, info in sorted(cluster_info_dict.items(), key=lambda x: x[1]["market_count"], reverse=True)
            ],
            headers=['ID', 'Count', 'Top Market', 'Top Platform', 'Med Novelty', 'Med Volume', 'Med Traders', 'Med Duration', 'Mean Res'],
            tablefmt="github"
        ))

    # Generate cluster visualization based on selected method
    output_filename = f"{args.output_dir}/clusters_{args.plot_method}.png"
    if args.plot_method == "umap":
        plot_clusters_umap(market_embeddings_mapped, market_clusters, output_filename)
    elif args.plot_method == "tsne":
        plot_clusters_tsne(market_embeddings_mapped, market_clusters, output_filename)
    elif args.plot_method == "pca":
        plot_clusters_pca(market_embeddings_mapped, market_clusters, output_filename)

if __name__ == "__main__":
    main()
