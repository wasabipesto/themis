import argparse
import json
import os
import pickle
import re
import time
from collections import Counter, defaultdict
from datetime import datetime, timedelta

import hdbscan
import matplotlib.pyplot as plt
import networkx as nx
import numpy as np
import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import umap
from dotenv import load_dotenv
from scipy.spatial import ConvexHull
from scipy.spatial.distance import pdist, squareform
from scipy.stats import entropy
from sklearn.decomposition import PCA
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.manifold import TSNE
from sklearn.neighbors import LocalOutlierFactor, NearestNeighbors
from tabulate import tabulate

from common import (
    DISPLAY_SAMPLE_SIZE,
    JITTER_SCALE,
    NUM_KEYWORDS,
    apply_pca_reduction,
    calculate_market_scores,
    compute_novelty_faiss,
    get_data_as_dataframe,
    load_dataframe_from_cache,
    remove_emoji,
    save_dataframe_to_cache,
)


def create_clusters_hdbscan(embeddings_df, min_cluster_size, cluster_selection_epsilon):
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
        output_dir (str): Directory path where tree plots will be saved
        cluster_selection_epsilon (float): Epsilon value for cluster selection

    Returns:
        tuple: (pd.DataFrame, hdbscan.HDBSCAN) containing:
            - pd.DataFrame: Cluster assignments with columns:
                - market_id: Market identifier from input
                - cluster (int): Cluster ID (-1 for outliers, 0+ for valid clusters)
            - hdbscan.HDBSCAN: Fitted clusterer object with hierarchy information

    Side Effects:
        - Normalizes embedding vectors using L2 normalization
        - Prints clustering progress to stdout
        - Uses fixed min_samples=10 parameter for HDBSCAN
        - Plots HDBSCAN condensed tree and single linkage tree
        - Saves condensed tree as NetworkX graph in GEXF format
    """
    market_ids = embeddings_df["market_id"].values
    embedding_vectors = np.stack(embeddings_df["embedding"].values).astype("float32")
    embedding_vectors = embedding_vectors / np.linalg.norm(
        embedding_vectors, axis=1, keepdims=True
    )

    print("Clustering with HDBSCAN...")
    clusterer = hdbscan.HDBSCAN(
        min_cluster_size=min_cluster_size,
        min_samples=10,
        cluster_selection_epsilon=cluster_selection_epsilon,
    )
    cluster_labels = clusterer.fit_predict(embedding_vectors)

    return pd.DataFrame({"market_id": market_ids, "cluster": cluster_labels}), clusterer


def generate_simplified_tree_structure(
    clusterer, cluster_info_dict, top_k_clusters=200, sampling_per_cluster=25
):
    """
    Generate a simplified tree structure from an HDBSCAN clusterer.

    This function extracts the most important clusters and builds a hierarchical
    tree structure containing all relevant information needed for visualization.

    Args:
        clusterer: fitted HDBSCAN object (must have condensed_tree_ and labels_)
        cluster_info_dict: dict mapping flat cluster_id -> {'keywords': [...], ...}
        top_k_clusters: maximum number of flat clusters to include
        sampling_per_cluster: number of members per cluster to sample when finding
                             representative condensed-tree nodes

    Returns:
        dict: Contains all tree structure information with keys:
            - 'label_to_rep_node': mapping flat cluster id -> representative node id
            - 'nodes_set': set of all nodes in the simplified tree
            - 'edges': list of (parent, child) edges in the tree
            - 'node_info': dict mapping node_id -> {label, hover_text, value, flat_labels}
            - 'label_to_members': mapping flat cluster id -> list of member indices
            - 'selection_reason': whether clusters were selected by 'persistence' or 'size'
            - 'top_labels': list of selected cluster labels
            - 'condensed_df': the condensed tree dataframe
    """
    # Validate input clusterer
    if not hasattr(clusterer, "condensed_tree_"):
        raise ValueError(
            "clusterer must have attribute 'condensed_tree_' (a fitted HDBSCAN)."
        )
    if not hasattr(clusterer, "labels_"):
        raise ValueError("clusterer must have attribute 'labels_'.")

    labels = np.asarray(clusterer.labels_)
    n_samples = len(labels)
    print(f"[INFO] Processing {n_samples} samples from fitted clusterer")

    # Convert condensed tree to DataFrame for easier processing
    condensed_df = clusterer.condensed_tree_.to_pandas().copy()
    condensed_df = condensed_df.astype(
        {"parent": int, "child": int, "child_size": int, "lambda_val": float}
    )

    # Extract flat clusters (excluding noise with label -1)
    flat_labels = labels[labels >= 0]
    if flat_labels.size == 0:
        raise ValueError(
            "No non-noise clusters found in clusterer.labels_. Nothing to process."
        )

    unique_labels = np.unique(flat_labels)
    label_to_members: dict[int, list[int]] = {
        int(lab): np.where(labels == lab)[0].tolist() for lab in unique_labels
    }  # type: ignore
    print(f"[INFO] Found {len(unique_labels)} non-noise flat clusters")

    # Select top clusters by persistence (preferred) or size (fallback)
    persistence = getattr(clusterer, "cluster_persistence_", None)
    if persistence is not None and len(persistence) >= len(unique_labels):
        # Use cluster persistence for selection (higher persistence = more stable cluster)
        label_persistence = {
            int(label): float(persistence[int(label)])
            for label in unique_labels
            if int(label) < len(persistence)
        }
        selected_sorted = sorted(
            label_persistence.items(), key=lambda kv: kv[1], reverse=True
        )
        selection_reason = "persistence"
        print("[INFO] Selecting clusters by persistence (stability measure)")
    else:
        # Fallback to cluster size
        label_size = {
            int(label): len(label_to_members[int(label)]) for label in unique_labels
        }
        selected_sorted = sorted(label_size.items(), key=lambda kv: kv[1], reverse=True)
        selection_reason = "size"
        print("[INFO] Selecting clusters by size (member count)")

    top_labels = [int(k) for k, _ in selected_sorted[:top_k_clusters]]
    print(f"[INFO] Selected top {len(top_labels)} clusters by {selection_reason}")

    # Build parent-child mapping from condensed tree
    condensed_parent = condensed_df.set_index("child")["parent"].to_dict()

    # Helper function to find the first cluster ancestor of a data point
    def first_cluster_ancestor_of_point(pt_idx, max_steps=2000):
        """
        Climb from a sample index to the first condensed-tree node >= n_samples.
        This finds the representative cluster node for a data point.
        """
        cur = int(pt_idx)
        steps = 0
        visited = set()
        while True:
            if cur in visited:
                return None  # Cycle detected
            visited.add(cur)
            if cur >= n_samples:
                return cur  # Found cluster node
            if cur not in condensed_parent:
                return None  # No parent found
            cur = int(condensed_parent[cur])
            steps += 1
            if steps > max_steps:
                return None  # Safety guard against infinite loops

    # Map each selected flat cluster to a representative condensed-tree node
    label_to_rep_node = {}
    for lab in top_labels:
        members = label_to_members.get(lab, [])
        if not members:
            label_to_rep_node[lab] = None
            continue

        # Sample members to find the most common ancestor node
        sample = (
            members[:sampling_per_cluster]
            if len(members) > sampling_per_cluster
            else members
        )
        anc_counts = Counter()
        for m in sample:
            anc = first_cluster_ancestor_of_point(int(m))
            if anc is not None:
                anc_counts[anc] += 1

        if anc_counts:
            # Choose the ancestor node seen most often among sampled members
            rep = anc_counts.most_common(1)[0][0]
            label_to_rep_node[lab] = int(rep)
        else:
            label_to_rep_node[lab] = None

    # Build the complete node set: representative nodes + all their ancestors
    nodes_set = set()
    for rep in label_to_rep_node.values():
        if rep is None:
            continue
        # Trace up to root, adding all ancestor nodes
        cur = rep
        while True:
            if cur in nodes_set:
                break  # Already processed this branch
            nodes_set.add(cur)
            if cur in condensed_parent:
                cur = int(condensed_parent[cur])
            else:
                break  # Reached root

    if not nodes_set:
        raise RuntimeError("Could not map any flat cluster to condensed-tree nodes")

    # Extract edges within our simplified tree
    edges = []
    for child, parent in condensed_parent.items():
        child_i, parent_i = int(child), int(parent)
        if child_i in nodes_set and parent_i in nodes_set:
            edges.append((parent_i, child_i))

    # Create reverse mapping: condensed node -> flat labels that use it
    node_to_flat_labels = defaultdict(list)
    for lab, rep in label_to_rep_node.items():
        if rep is not None:
            node_to_flat_labels[rep].append(lab)

    # Generate comprehensive node information
    node_info = {}
    for node in nodes_set:
        if node < n_samples:
            # Individual data point node
            info = {
                "label": f"pt-{node}",
                "hover_text": f"point {node}",
                "value": 1,
                "flat_labels": [],
            }
        else:
            linked_labels = node_to_flat_labels.get(node, [])
            if linked_labels:
                # Node representing one or more flat clusters
                sizes = [
                    len(label_to_members.get(label, [])) for label in linked_labels
                ]
                keywords = []
                for label in linked_labels:
                    cluster_info = cluster_info_dict.get(int(label), {})
                    cluster_keywords = cluster_info.get("keywords", "")
                    if isinstance(cluster_keywords, list):
                        keywords.extend(
                            cluster_keywords[:3]
                        )  # Top 3 keywords per cluster
                    elif isinstance(cluster_keywords, str):
                        keywords.append(cluster_keywords[:40])  # Truncate long strings

                info = {
                    "label": f"Cluster {'/'.join(str(x) for x in linked_labels)}",
                    "hover_text": f"flat_labels: {linked_labels}<br>size: {sum(sizes)}<br>keywords: {keywords}",
                    "value": sum(sizes) if sum(sizes) > 0 else 1,
                    "flat_labels": linked_labels,
                }
            else:
                # Internal tree node without direct flat cluster mapping
                child_rows = condensed_df[condensed_df["parent"] == node]
                est_size = (
                    int(child_rows["child_size"].sum()) if not child_rows.empty else 1
                )

                info = {
                    "label": f"Node-{node}",
                    "hover_text": f"internal node {node}<br>est_size: {est_size}",
                    "value": est_size,
                    "flat_labels": [],
                }

        node_info[node] = info

    return {
        "label_to_rep_node": label_to_rep_node,
        "nodes_set": nodes_set,
        "edges": edges,
        "node_info": node_info,
        "label_to_members": label_to_members,
        "selection_reason": selection_reason,
        "top_labels": top_labels,
        "condensed_df": condensed_df,
    }


def create_interactive_hierarchy_plot(
    tree_structure,
    output_dir,
    html_filename="hdbscan_cluster_hierarchy.html",
    icicle_height=900,
    icicle_width=1400,
):
    """
    Create an interactive HTML plot from the simplified tree structure.

    Uses Plotly Icicle chart to create a left-to-right hierarchical visualization
    with hover information containing cluster details and keywords.

    Args:
        tree_structure: dict returned from generate_simplified_tree_structure()
        output_dir: directory where the HTML file will be saved
        html_filename: name of the output HTML file
        icicle_height/icicle_width: dimensions of the interactive figure

    Returns:
        str: path to the saved HTML file
    """
    nodes_set = tree_structure["nodes_set"]
    node_info = tree_structure["node_info"]
    selection_reason = tree_structure["selection_reason"]
    top_labels = tree_structure["top_labels"]
    condensed_df = tree_structure["condensed_df"]

    # Build parent-child relationships for Plotly
    condensed_parent = condensed_df.set_index("child")["parent"].to_dict()
    parent_map = {
        int(child): int(parent)
        for child, parent in condensed_parent.items()
        if int(child) in nodes_set and int(parent) in nodes_set
    }

    # Prepare data arrays for Plotly Icicle chart
    ids = []
    labels_plot = []
    parents = []
    values = []
    hover_texts = []

    for node in nodes_set:
        info = node_info[node]

        ids.append(str(node))
        labels_plot.append(info["label"])
        values.append(info["value"])
        hover_texts.append(info["hover_text"])

        # Set parent relationship (empty string for root nodes)
        parent = parent_map.get(node, "")
        parents.append(str(parent) if parent != "" else "")

    # Create the interactive Icicle visualization
    icicle = go.Icicle(
        ids=ids,
        labels=labels_plot,
        parents=parents,
        values=values,
        hovertext=hover_texts,
        hoverinfo="text",
        tiling=dict(orientation="h"),  # Horizontal (left-to-right) layout
        branchvalues="total",
        textinfo="label+value",
    )

    fig = go.Figure(icicle)
    fig.update_layout(
        title=f"HDBSCAN Cluster Hierarchy - Top {len(top_labels)} clusters by {selection_reason}",
        width=icicle_width,
        height=icicle_height,
        margin=dict(t=60, l=20, r=20, b=20),
    )

    # Save the interactive HTML file
    html_path = os.path.join(output_dir, html_filename)
    fig.write_html(html_path, include_plotlyjs="cdn")
    print(f"[INFO] Interactive cluster hierarchy saved to: {html_path}")

    return html_path


def create_static_tree_plot(
    tree_structure, cluster_info_dict, output_dir, gv_program="dot"
):
    """
    Create a static tree plot using matplotlib and NetworkX.

    Generates a hierarchical tree visualization showing cluster relationships
    with nodes colored and labeled according to their cluster information.

    Args:
        tree_structure: dict returned from generate_simplified_tree_structure()
        cluster_info_dict: dict mapping flat cluster_id -> {'keywords': [...], ...}
        output_dir: directory where the PNG file will be saved
        gv_program: NetworkX layout program to use for the tree plot
            Options: 'dot', 'neato', 'fdp', 'sfdp', 'osage', 'twopi', 'circo'

    Returns:
        str: path to the saved PNG file, or None if creation failed
    """
    edges = tree_structure["edges"]
    nodes_set = tree_structure["nodes_set"]
    node_info = tree_structure["node_info"]
    top_labels = tree_structure["top_labels"]
    dendrogram_filename = f"hdbscan_dendrogram_{gv_program}.png"

    if not edges:
        print("[WARNING] No edges found for tree visualization")
        return None

    try:
        # Create directed graph from the tree edges
        G = nx.DiGraph()
        G.add_edges_from(edges)

        # Find nodes that represent actual clusters (have flat_labels)
        cluster_nodes = [node for node in nodes_set if node_info[node]["flat_labels"]]

        if not cluster_nodes:
            print("[WARNING] No cluster nodes found for tree visualization")
            return None

        # Set up the plot
        plt.figure(figsize=(16, 10))

        # Create hierarchical layout (try graphviz first, fallback to spring layout)
        try:
            if hasattr(nx, "nx_agraph"):
                pos = nx.nx_agraph.graphviz_layout(G, prog=gv_program)
            else:
                raise ImportError("nx_agraph not available")
        except (ImportError, Exception):
            print("[INFO] Using spring layout for tree visualization")
            pos = nx.spring_layout(G, k=2, iterations=50)

        # Draw edges first (so they appear behind nodes)
        nx.draw_networkx_edges(
            G,
            pos,
            edge_color="gray",
            arrows=True,
            alpha=0.6,
            width=1,
            node_size=1000,
            node_shape="H",
        )

        # Prepare node styling
        node_colors = []
        node_labels = {}

        for node in G.nodes():
            info = node_info[node]
            flat_labels = info["flat_labels"]

            if flat_labels:
                # Cluster node - color distinctly and add informative label
                node_colors.append("lightblue")

                # Create compact label with cluster ID and keywords
                label_parts = []
                for lab in flat_labels[:1]:  # Limit to first cluster for readability
                    cluster_info = cluster_info_dict.get(int(lab), {})
                    keywords = cluster_info.get("keywords", "")

                    if isinstance(keywords, list) and keywords:
                        kw_str = ", ".join(keywords[:2])  # Top 2 keywords
                    elif isinstance(keywords, str) and keywords:
                        kw_str = keywords[:12]  # Truncate long strings
                    else:
                        kw_str = "no keywords"

                    label_parts.append(f"C{lab}\n{kw_str}")

                node_labels[node] = "\n".join(label_parts)
            else:
                # Internal node - color as gray and use simple label
                node_colors.append("lightgray")
                node_labels[node] = f"{node}"

        # Draw nodes
        nx.draw_networkx_nodes(
            G, pos, node_color=node_colors, node_size=300, node_shape="H", alpha=0.8
        )

        # Draw labels
        nx.draw_networkx_labels(G, pos, labels=node_labels, font_size=2)

        plt.title(
            f"HDBSCAN Cluster Hierarchy Tree (Top {len(top_labels)} clusters)",
            fontsize=14,
        )
        plt.axis("off")
        plt.tight_layout()

        # Save the tree visualization
        tree_path = os.path.join(output_dir, dendrogram_filename)
        plt.savefig(tree_path, dpi=300, bbox_inches="tight", facecolor="white")
        plt.close()

        print(f"[INFO] Cluster hierarchy tree saved to: {tree_path}")
        return tree_path

    except Exception as e:
        print(f"[WARNING] Failed to create tree visualization: {str(e)}")

        # Fallback: create simple text-based summary
        try:
            plt.figure(figsize=(12, 8))
            plt.text(
                0.1,
                0.9,
                "Cluster Hierarchy Summary",
                fontsize=16,
                fontweight="bold",
                transform=plt.gca().transAxes,
            )

            y_pos = 0.8
            cluster_count = 0
            for node in nodes_set:
                info = node_info[node]
                if info["flat_labels"] and cluster_count < 20:  # Limit display
                    info_lines = []
                    for lab in info["flat_labels"][:2]:  # Max 2 labels per node
                        cluster_info = cluster_info_dict.get(int(lab), {})
                        keywords = cluster_info.get("keywords", "")

                        if isinstance(keywords, list):
                            kw_str = ", ".join(keywords[:3])
                        else:
                            kw_str = str(keywords)[:40]

                        info_lines.append(f"Cluster {lab}: {kw_str}")

                    plt.text(
                        0.1,
                        y_pos,
                        "\n".join(info_lines),
                        fontsize=10,
                        transform=plt.gca().transAxes,
                    )
                    y_pos -= 0.08
                    cluster_count += 1

            plt.axis("off")
            plt.tight_layout()

            fallback_path = os.path.join(output_dir, dendrogram_filename)
            plt.savefig(fallback_path, dpi=300, bbox_inches="tight", facecolor="white")
            plt.close()

            print(f"[INFO] Fallback cluster summary saved to: {fallback_path}")
            return fallback_path

        except Exception as fallback_e:
            print(
                f"[WARNING] Could not create any tree visualization: {str(fallback_e)}"
            )
            return None


def create_cluster_hierarchy_dendrogram(clusterer, cluster_info_dict, output_dir):
    """
    Create hierarchical visualizations from an HDBSCAN fitted clusterer.

    This wrapper function combines three focused operations:
    1. Generate a simplified tree structure from the clusterer
    2. Create an interactive HTML plot using Plotly
    3. Create a static tree plot using matplotlib

    Args:
        clusterer: fitted HDBSCAN object (must have condensed_tree_ and labels_)
        cluster_info_dict: dict mapping flat cluster_id -> {'keywords': [...], ...}
        output_dir: directory where output files will be saved
        top_k_clusters: maximum number of flat clusters to include
        sampling_per_cluster: number of members per cluster to sample for node mapping
        icicle_height/icicle_width: dimensions of the interactive figure

    Returns:
        dict: Contains the tree structure and file paths:
            - 'label_to_rep_node': mapping flat cluster id -> representative node id
            - 'html_path': path to interactive HTML file (or None if failed)
            - 'tree_path': path to static tree PNG file (or None if failed)
            - 'tree_structure': complete tree structure dict for further analysis
    """
    print("[INFO] Starting cluster hierarchy visualization generation...")

    # Save built-in HDBSCAN trees and exports
    print("Plotting HDBSCAN condensed tree...")
    try:
        plt.figure(figsize=(12, 8))
        clusterer.condensed_tree_.plot()
        plt.title("HDBSCAN Condensed Tree")
        plt.savefig(
            f"{output_dir}/hdbscan_condensed_tree.png",
            format="png",
            bbox_inches="tight",
            dpi=300,
        )
        plt.close()
    except Exception as e:
        print(f"Error plotting condensed tree: {e}")

    print("Plotting HDBSCAN single linkage tree...")
    try:
        plt.figure(figsize=(12, 8))
        clusterer.single_linkage_tree_.plot()
        plt.title("HDBSCAN Single Linkage Tree")
        plt.savefig(
            f"{output_dir}/hdbscan_single_linkage_tree.png",
            format="png",
            bbox_inches="tight",
            dpi=300,
        )
        plt.close()
    except Exception as e:
        print(f"Error plotting single linkage tree: {e}")

    print("Saving HDBSCAN condensed tree as NetworkX graph...")
    try:
        condensed_tree_graph = clusterer.condensed_tree_.to_networkx()
        nx.write_gexf(condensed_tree_graph, f"{output_dir}/hdbscan_condensed_tree.gexf")
    except Exception as e:
        print(f"Error saving condensed tree as NetworkX graph: {e}")

    print("Saving HDBSCAN single linkage tree as NetworkX graph...")
    try:
        single_linkage_tree_graph = clusterer.single_linkage_tree_.to_networkx()
        nx.write_gexf(
            single_linkage_tree_graph, f"{output_dir}/hdbscan_single_linkage_tree.gexf"
        )
    except Exception as e:
        print(f"Error saving single linkage tree as NetworkX graph: {e}")

    # Step 1: Generate simplified tree structure
    print("[INFO] Step 1: Generating simplified tree structure...")
    tree_structure = generate_simplified_tree_structure(
        clusterer=clusterer,
        cluster_info_dict=cluster_info_dict,
    )

    # Step 2: Create interactive HTML plot
    print("[INFO] Step 2: Creating interactive HTML plot...")
    html_path = None
    try:
        html_path = create_interactive_hierarchy_plot(
            tree_structure=tree_structure,
            output_dir=output_dir,
        )
    except Exception as e:
        print(f"[WARNING] Failed to create interactive plot: {str(e)}")

    # Step 3: Create static tree plot
    print("[INFO] Step 3: Creating static tree plot...")
    tree_path = None
    try:
        for gv_program in ["dot", "neato", "fdp", "sfdp", "osage", "twopi", "circo"]:
            tree_path = create_static_tree_plot(
                tree_structure=tree_structure,
                cluster_info_dict=cluster_info_dict,
                output_dir=output_dir,
                gv_program=gv_program,
            )
    except Exception as e:
        print(f"[WARNING] Failed to create static tree plot: {str(e)}")

    print("[INFO] Cluster hierarchy visualization generation complete!")

    return {
        "label_to_rep_node": tree_structure["label_to_rep_node"],
        "html_path": html_path,
        "tree_path": tree_path,
        "tree_structure": tree_structure,
    }


def collate_cluster_information(markets_df, novelty_df=None):
    """
    Aggregate and compute comprehensive statistics for each market cluster.

    Combines market data with novelty scores and computes detailed statistics
    for each cluster including top markets, platform distributions, and
    median values across various metrics. Uses efficient pandas groupby
    operations for optimal performance.

    Args:
        markets_df (pd.DataFrame): Market data with required columns:
            - market_id (int/str) or id (int/str): Unique market identifier
            - cluster (int): Cluster assignment (-1 for outliers)
            - title (str): Market title/description
            - score (float): Market score from calculate_market_scores()
            - platform_slug (str): Platform identifier
            - open_datetime (datetime): Market opening time
            - volume_usd (float): Trading volume in USD
            - traders_count (int): Number of unique traders
            - duration_days (float): Market duration in days
            - resolution (float): Market resolution (0.0-1.0)
            - novelty (float): Novelty score (0.0-1.0) - optional if already in markets_df
        novelty_df (pd.DataFrame, optional): Novelty data with required columns:
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
        - Merges DataFrames using inner join on market_id if novelty_df provided
        - Excludes outlier cluster (-1) from main analysis
        - Applies emoji removal to market titles
        - Returns empty dict if input markets_df is empty
    """
    if markets_df.empty:
        return {}

    # Check if novelty data is already in markets_df or needs to be merged
    if "novelty" in markets_df.columns:
        merged_df = markets_df
    elif novelty_df is not None:
        # Determine the market ID column name
        market_id_col = "market_id" if "market_id" in markets_df.columns else "id"
        merged_df = markets_df.merge(
            novelty_df, left_on=market_id_col, right_on="market_id", how="left"
        )
    else:
        raise ValueError(
            "Novelty data must be provided either in markets_df or as separate novelty_df parameter"
        )

    cluster_info = {}

    for cluster_id, group in merged_df.groupby("cluster"):
        if cluster_id == -1:  # Skip outliers for main analysis
            continue

        # Basic info
        info = {
            "market_count": len(group),
            "markets": group.to_dict("records"),  # Keep for backward compatibility
        }

        # Top market by score
        top_market = group.loc[group["score"].idxmax()]
        info["top_market"] = top_market.to_dict()
        info["top_market_title"] = remove_emoji(top_market["title"])

        # First market by open_datetime
        if "open_datetime" in group.columns:
            first_market = group.loc[group["open_datetime"].idxmin()]
            info["first_market"] = first_market.to_dict()
            info["first_market_platform"] = first_market.get("platform_slug", "unknown")

        # Platform proportions using value_counts
        platform_counts = group["platform_slug"].value_counts()
        total_markets = len(group)
        info["platform_proportions"] = (platform_counts / total_markets).to_dict()
        info["top_platform"] = (
            platform_counts.index[0] if len(platform_counts) > 0 else "unknown"
        )
        info["top_platform_pct"] = (
            platform_counts.iloc[0] / total_markets if len(platform_counts) > 0 else 0
        )

        # Statistical aggregations using pandas methods
        info["median_novelty"] = group["novelty"].median()
        info["median_score"] = group["score"].median()
        info["median_volume_usd"] = group["volume_usd"].median()
        info["median_traders_count"] = group["traders_count"].median()
        info["median_duration_days"] = group["duration_days"].median()
        info["mean_resolution"] = group["resolution"].mean()

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

    _fig = plt.figure(figsize=(20, 15))

    # Convert cluster info to DataFrame for easier manipulation
    cluster_data = []
    for cluster_id, info in cluster_info_dict.items():
        cluster_data.append(
            {
                "cluster_id": cluster_id,
                "market_count": info["market_count"],
                "median_novelty": info["median_novelty"],
                "median_volume_usd": info["median_volume_usd"],
                "median_traders_count": info["median_traders_count"],
                "median_duration_days": info["median_duration_days"],
                "mean_resolution": info["mean_resolution"],
                "top_platform": info["top_platform"],
                "top_platform_pct": info["top_platform_pct"],
            }
        )

    cluster_df = pd.DataFrame(cluster_data)

    # Plot 1: Bar plot of number of markets
    plt.subplot(3, 3, 1)
    plt.bar(cluster_df["cluster_id"], cluster_df["market_count"])
    plt.xlabel("Cluster ID")
    plt.ylabel("Number of Markets")
    plt.title("Markets per Cluster")
    plt.grid(True)

    # Plot 2: Histogram of market counts
    plt.subplot(3, 3, 2)
    plt.hist(cluster_df["market_count"], bins=20, alpha=0.7, edgecolor="black")
    plt.xlabel("Number of Markets")
    plt.ylabel("Frequency")
    plt.title("Distribution of Market Counts")

    # Plot 3: Platform proportions pie chart
    plt.subplot(3, 3, 3)
    all_platforms = {}
    for cluster_info in cluster_info_dict.values():
        for platform, prop in cluster_info["platform_proportions"].items():
            all_platforms[platform] = (
                all_platforms.get(platform, 0) + prop * cluster_info["market_count"]
            )

    if all_platforms:
        total_markets = sum(all_platforms.values())
        platform_props = {k: v / total_markets for k, v in all_platforms.items()}
        plt.pie(
            platform_props.values(), labels=platform_props.keys(), autopct="%1.1f%%"
        )
        plt.title("Platform Distribution")

    # Plot 4: Median novelty histogram
    plt.subplot(3, 3, 4)
    plt.hist(
        cluster_df["median_novelty"].dropna(), bins=20, alpha=0.7, edgecolor="black"
    )
    plt.xlabel("Median Novelty")
    plt.ylabel("Number of Clusters")
    plt.title("Distribution of Median Novelty")

    # Plot 5: Median volume histogram (log scale)
    plt.subplot(3, 3, 5)
    non_zero_volumes = cluster_df[cluster_df["median_volume_usd"] > 0][
        "median_volume_usd"
    ]
    if len(non_zero_volumes) > 0:
        plt.hist(non_zero_volumes, bins=20, alpha=0.7, edgecolor="black")
        plt.xscale("log")
    plt.xlabel("Median Volume USD (log scale)")
    plt.ylabel("Number of Clusters")
    plt.title("Distribution of Median Volume")

    # Plot 6: Median traders histogram
    plt.subplot(3, 3, 6)
    non_zero_traders = cluster_df[cluster_df["median_traders_count"] > 0][
        "median_traders_count"
    ]
    if len(non_zero_traders) > 0:
        plt.hist(non_zero_traders, bins=20, alpha=0.7, edgecolor="black")
    plt.xlabel("Median Traders Count")
    plt.ylabel("Number of Clusters")
    plt.title("Distribution of Median Traders")

    # Plot 7: Median duration histogram
    plt.subplot(3, 3, 7)
    non_zero_durations = cluster_df[cluster_df["median_duration_days"] > 0][
        "median_duration_days"
    ]
    if len(non_zero_durations) > 0:
        plt.hist(non_zero_durations, bins=20, alpha=0.7, edgecolor="black")
    plt.xlabel("Median Duration Days")
    plt.ylabel("Number of Clusters")
    plt.title("Distribution of Median Duration")

    # Plot 8: Mean resolution histogram
    plt.subplot(3, 3, 8)
    plt.hist(
        cluster_df["mean_resolution"].dropna(), bins=20, alpha=0.7, edgecolor="black"
    )
    plt.xlabel("Mean Resolution")
    plt.ylabel("Number of Clusters")
    plt.title("Distribution of Mean Resolution")

    # Plot 9: Scatter plot of volume vs traders
    plt.subplot(3, 3, 9)
    valid_data = cluster_df[
        (cluster_df["median_volume_usd"] > 0) & (cluster_df["median_traders_count"] > 0)
    ]
    if len(valid_data) > 0:
        plt.scatter(
            valid_data["median_volume_usd"],
            valid_data["median_traders_count"],
            alpha=0.6,
        )
        plt.xscale("log")
    plt.xlabel("Median Volume USD (log scale)")
    plt.ylabel("Median Traders Count")
    plt.title("Volume vs Traders by Cluster")

    plt.tight_layout()
    plt.savefig(
        f"{output_dir}/cluster_dashboard.png",
        format="png",
        bbox_inches="tight",
        dpi=150,
    )
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
    embeddings_matrix = np.vstack(embeddings_df["embedding"].values)

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
        market_id = result_df.iloc[idx]["market_id"]
        embedding = result_df.iloc[idx]["embedding"].copy()

        # Deterministic jitter based on market_id
        np.random.seed(hash(market_id) % (2**32))
        jitter = np.random.uniform(-JITTER_SCALE, JITTER_SCALE, len(embedding))
        result_df.iloc[idx, result_df.columns.get_loc("embedding")] = (
            np.array(embedding) + jitter
        ).tolist()

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

    embedding_vectors = np.stack(embeddings_df["embedding"].values).astype("float32")
    embedding_vectors = embedding_vectors / np.linalg.norm(
        embedding_vectors, axis=1, keepdims=True
    )

    print("Reducing embeddings to 2D with UMAP...", end="")
    reducer = umap.UMAP(n_jobs=n_jobs, verbose=True)
    embedding_2d = reducer.fit_transform(embedding_vectors)
    print("Complete.")

    return pd.DataFrame(
        {
            "market_id": embeddings_df["market_id"],
            "embedding": [row.tolist() for row in embedding_2d],  # type: ignore
        }
    )


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
    embedding_vectors = np.stack(embeddings_df["embedding"].values).astype("float32")
    embedding_vectors = embedding_vectors / np.linalg.norm(
        embedding_vectors, axis=1, keepdims=True
    )

    print("Reducing embeddings to 2D with t-SNE...", end="")
    reducer = TSNE(n_components=2, perplexity=min(30, len(embedding_vectors) - 1))
    embedding_2d = reducer.fit_transform(embedding_vectors)
    print("Complete.")

    return pd.DataFrame(
        {
            "market_id": embeddings_df["market_id"],
            "embedding": [row.tolist() for row in embedding_2d],
        }
    )


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
    embedding_vectors = np.stack(embeddings_df["embedding"].values).astype("float32")
    embedding_vectors = embedding_vectors / np.linalg.norm(
        embedding_vectors, axis=1, keepdims=True
    )

    print("Reducing embeddings to 2D with PCA...", end="")
    pca = PCA(n_components=2)
    embedding_2d = pca.fit_transform(embedding_vectors)
    print("Complete.")

    explained_var = pca.explained_variance_ratio_
    print(
        f"PCA explained variance: {explained_var[0]:.3f}, {explained_var[1]:.3f} (total: {sum(explained_var):.3f})"
    )

    return pd.DataFrame(
        {
            "market_id": embeddings_df["market_id"],
            "embedding": [row.tolist() for row in embedding_2d],
        }
    )


def plot_clusters(
    method, embeddings_2d_df, clusters_df, output_file, label_top_n_clusters=20
):
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
    plot_data = embeddings_2d_df.merge(clusters_df, on="market_id", how="inner")

    if plot_data.empty:
        print("No data available for plotting")
        return

    # Extract coordinates and labels
    embedding_2d = np.stack(plot_data["embedding"].values)
    cluster_labels = plot_data["cluster"].values

    # Count cluster sizes to identify largest clusters
    cluster_counts = Counter(cluster_labels)
    largest_clusters = [
        cluster_id for cluster_id, _ in cluster_counts.most_common() if cluster_id != -1
    ][:label_top_n_clusters]

    # Initialize figure
    plt.figure(figsize=(10, 8))

    # Plot outliers (cluster -1) with lower alpha for transparency
    outlier_mask = cluster_labels == -1
    if np.any(outlier_mask):
        plt.scatter(
            embedding_2d[outlier_mask, 0],
            embedding_2d[outlier_mask, 1],
            c="lightgray",
            s=1,
            alpha=0.3,
            label="Outliers",
        )

    # Plot regular clusters with normal alpha
    non_outlier_mask = cluster_labels != -1
    if np.any(non_outlier_mask):
        scatter = plt.scatter(
            embedding_2d[non_outlier_mask, 0],
            embedding_2d[non_outlier_mask, 1],
            c=cluster_labels[non_outlier_mask],
            cmap="tab20",
            s=3,
            alpha=0.8,
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
                f"C{cluster_id}",
                (centroid_x, centroid_y),
                fontsize=6,
                fontweight="bold",
                bbox=dict(boxstyle="round,pad=0.1", facecolor="white", alpha=0.8),
            )

    plt.title(f"Market Embeddings Clusters ({method})")
    plt.tight_layout()
    plt.savefig(output_file, format="png", bbox_inches="tight", dpi=300)
    plt.close()


def create_interactive_visualization(
    method,
    embeddings_2d_df,
    clusters_df,
    markets_df,
    cluster_info_dict,
    output_file,
    display_prob,
):
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
        viz_data = viz_data.rename(columns={"embedding": "embedding_2d"})
        viz_data = viz_data.merge(clusters_df, on="market_id", how="inner")

        # For the final merge, be explicit about suffixes and drop duplicates
        viz_data = viz_data.merge(
            markets_df,
            left_on="market_id",
            right_on="id",
            how="inner",
            suffixes=("", "_markets"),
        )

        # Handle any remaining duplicate columns by keeping the left version
        if "cluster_markets" in viz_data.columns:
            viz_data = viz_data.drop("cluster_markets", axis=1)
        if "market_id_markets" in viz_data.columns:
            viz_data = viz_data.drop("market_id_markets", axis=1)

        # Sample data for performance if needed
        if display_prob < 1.0:
            viz_data = viz_data.sample(frac=display_prob, random_state=42)

        if viz_data.empty:
            print("Warning: No valid market data found for visualization")
            return

        # Check for required columns
        required_columns = [
            "embedding_2d",
            "cluster",
            "market_id",
            "title",
            "volume_usd",
            "platform_slug",
        ]
        missing_columns = [
            col for col in required_columns if col not in viz_data.columns
        ]
        if missing_columns:
            print(f"DEBUG: Missing required columns: {missing_columns}")
            print(f"DEBUG: Available columns: {list(viz_data.columns)}")

        # Extract coordinates and prepare data
        _coordinates = np.stack(viz_data["embedding_2d"].values)

        # Create the main scatter plot
        fig = go.Figure()

        # Get unique clusters
        unique_clusters = viz_data["cluster"].unique()
        colors = (
            px.colors.qualitative.Set3
            + px.colors.qualitative.Pastel
            + px.colors.qualitative.Dark24
        )

        # Plot outliers first (cluster -1)
        if -1 in unique_clusters:
            outlier_data = viz_data[viz_data["cluster"] == -1]
            outlier_coords = np.stack(outlier_data["embedding_2d"].values)

            fig.add_trace(
                go.Scatter(
                    x=outlier_coords[:, 0],
                    y=outlier_coords[:, 1],
                    mode="markers",
                    marker=dict(size=3, color="lightgray", opacity=0.3),
                    name="Outliers",
                    text=[
                        f"Market ID: {row['market_id']}<br>Title: {str(row['title'])[:100]}<br>"
                        f"Volume: ${row['volume_usd']:,.2f}<br>Platform: {row['platform_slug']}"
                        for _, row in outlier_data.iterrows()
                    ],
                    hovertemplate="<b>%{text}</b><extra></extra>",
                    visible=True,
                )
            )

        # Plot regular clusters
        regular_clusters = sorted([c for c in unique_clusters if c != -1])
        for i, cluster_id in enumerate(regular_clusters):
            cluster_data = viz_data[viz_data["cluster"] == cluster_id]
            cluster_coords = np.stack(cluster_data["embedding_2d"].values)
            cluster_color = colors[i % len(colors)]

            # Get cluster info if available
            cluster_keywords = ""
            if cluster_id in cluster_info_dict:
                keywords = cluster_info_dict[cluster_id].get("keywords", "")
                cluster_keywords = f"<br>Keywords: {keywords}" if keywords else ""

            fig.add_trace(
                go.Scatter(
                    x=cluster_coords[:, 0],
                    y=cluster_coords[:, 1],
                    mode="markers",
                    marker=dict(size=4, color=cluster_color, opacity=0.7),
                    name=f"Cluster {cluster_id}",
                    text=[
                        f"Market ID: {row['market_id']}<br>Title: {str(row['title'])[:100]}<br>"
                        f"Volume: ${row['volume_usd']:,.2f}<br>Platform: {row['platform_slug']}<br>"
                        f"Cluster: {cluster_id}{cluster_keywords}"
                        for _, row in cluster_data.iterrows()
                    ],
                    hovertemplate="<b>%{text}</b><extra></extra>",
                    visible=True,
                )
            )

        # Update layout
        fig.update_layout(
            title=f"Interactive Market Embeddings Clusters ({method})",
            xaxis_title=f"{method} Component 1",
            yaxis_title=f"{method} Component 2",
            width=1200,
            height=800,
            hovermode="closest",
            showlegend=True,
            legend=dict(yanchor="top", y=0.99, xanchor="left", x=1.01),
            margin=dict(l=50, r=150, t=80, b=50),
            plot_bgcolor="white",
            paper_bgcolor="white",
        )

        # Add buttons to toggle outliers
        fig.update_layout(
            updatemenus=[
                dict(
                    type="buttons",
                    direction="left",
                    buttons=[
                        dict(
                            args=[{"visible": [True] * len(fig.data)}],
                            label="Show All",
                            method="update",
                        ),
                        dict(
                            args=[
                                {
                                    "visible": [
                                        trace.name != "Outliers" for trace in fig.data
                                    ]
                                }
                            ],
                            label="Hide Outliers",
                            method="update",
                        ),
                    ],
                    pad={"r": 10, "t": 10},
                    showactive=True,
                    x=0.01,
                    xanchor="left",
                    y=1.02,
                    yanchor="top",
                ),
            ]
        )

        # Add grid
        fig.update_xaxes(showgrid=True, gridwidth=1, gridcolor="lightgray")
        fig.update_yaxes(showgrid=True, gridwidth=1, gridcolor="lightgray")

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
        if not cluster_info or "markets" not in cluster_info:
            cluster_info["keywords"] = "No markets"
            continue

        markets_df = pd.DataFrame(cluster_info["markets"])
        if markets_df.empty or "title" not in markets_df.columns:
            cluster_info["keywords"] = "No titles"
            continue

        # Clean titles efficiently
        titles = markets_df["title"].dropna()
        cleaned_titles = [remove_emoji(title) for title in titles]
        cluster_doc = " ".join(cleaned_titles)
        cluster_docs[cluster_id] = cluster_doc
        all_titles.extend(cleaned_titles)

    if not cluster_docs:
        return cluster_info_dict

    if use_tfidf and len(cluster_docs) > 1:
        # Use TF-IDF for better keyword extraction
        vectorizer = TfidfVectorizer(
            max_features=1000,
            stop_words="english",
            token_pattern=r"\b[a-zA-Z]{2,}\b",
            lowercase=True,
            max_df=0.8,  # Ignore terms that appear in >80% of clusters
            min_df=1,  # Must appear at least once
        )

        try:
            docs = [cluster_docs[cid] for cid in sorted(cluster_docs.keys())]
            tfidf_matrix = vectorizer.fit_transform(docs)
            feature_names = vectorizer.get_feature_names_out()

            for i, cluster_id in enumerate(sorted(cluster_docs.keys())):
                # Get top n terms by TF-IDF score
                scores = tfidf_matrix[i].toarray()[0]
                top_indices = scores.argsort()[-n:][::-1]
                top_words = [
                    feature_names[idx] for idx in top_indices if scores[idx] > 0
                ]
                cluster_info_dict[cluster_id]["keywords"] = ", ".join(top_words[:n])  # type: ignore

        except ValueError:
            # Fall back to frequency analysis if TF-IDF fails
            use_tfidf = False

    if not use_tfidf:
        # Use traditional frequency analysis as fallback
        for cluster_id, cluster_doc in cluster_docs.items():
            words = re.findall(r"\b[a-zA-Z]{3,}\b", cluster_doc.lower())
            word_counts = Counter(words)
            # Filter common words
            common_words = {
                "the",
                "and",
                "will",
                "are",
                "for",
                "that",
                "this",
                "with",
                "from",
                "they",
            }
            filtered_words = {
                w: c for w, c in word_counts.items() if w not in common_words
            }
            top_words = [word for word, _ in Counter(filtered_words).most_common(n)]
            cluster_info_dict[cluster_id]["keywords"] = ", ".join(top_words)

    return cluster_info_dict

def timer_print(timers, key):
    """Utility function to track and print elapsed time for tasks."""
    if not key in timers:
        timers[key] = time.time()
        print(f"Started:  {key}")
    else:
        elapsed_time = time.time() - timers[key]
        print(f"Complete: {key} in {elapsed_time:.2f} seconds")
    return timers

def calculate_platform_metrics(master_df, clusterer=None, cluster_info_dict=None):
    """
    Calculate comprehensive platform-level metrics for diversity, novelty, innovation, and competition analysis.

    Args:
        master_df: DataFrame with columns: id, platform_slug, embedding, cluster, created_time, novelty
        clusterer: Fitted HDBSCAN clusterer object (optional, for persistence scores)
        cluster_info_dict: Dictionary of cluster information (optional)

    Returns:
        dict: Platform metrics organized by category
    """
    print("Calculating comprehensive platform metrics...")

    # Create a copy to avoid SettingWithCopyWarning and reset index
    master_df = master_df.copy().reset_index(drop=True)

    # Initialize results dictionary
    metrics = {
        'diversity': {},
        'novelty': {},
        'innovation': {},
        'competition': {},
        'platform_stats': {}
    }

    # Initialize timers
    timers = {}
    timers = timer_print(timers, "Metric Initialization")

    # Get unique platforms
    platforms = master_df['platform_slug'].unique()

    # Prepare embedding vectors
    embedding_vectors = np.stack(master_df['embedding'].values).astype('float32')
    embedding_vectors_norm = embedding_vectors / np.linalg.norm(embedding_vectors, axis=1, keepdims=True)

    # Convert created_time to datetime if it's not already
    if 'open_datetime' in master_df.columns and not 'created_datetime' in master_df.columns:
        print("Converting open_datetime to created_datetime")
        master_df['created_datetime'] = pd.to_datetime(master_df['open_datetime'], format='ISO8601')
    elif not 'open_datetime' in master_df.columns:
        print("Market open_datetime missing, innovation metrics will be missing.")

    timers = timer_print(timers, "Metric Initialization")

    # ================== DIVERSITY METRICS ==================
    print("Computing diversity metrics...")

    for platform in platforms:
        platform_mask = master_df['platform_slug'] == platform
        platform_df = master_df[platform_mask]
        platform_embeddings = embedding_vectors_norm[platform_mask]

        if len(platform_df) < 3:  # Need at least 3 points for convex hull
            continue

        platform_metrics = {}

        # Skip full-dim hull for now
        if False:
        # 1. Convex Hull Volume (full dimensionality)
            timers = timer_print(timers, f"Convex Hull Volume (768d) ({platform})")
            try:
                if len(platform_embeddings) > platform_embeddings.shape[1]:
                    hull = ConvexHull(platform_embeddings)
                    platform_metrics['convex_hull_volume_full'] = hull.volume
                else:
                    platform_metrics['convex_hull_volume_full'] = 0.0
            except:
                platform_metrics['convex_hull_volume_full'] = 0.0
            timers = timer_print(timers, f"Convex Hull Volume (768d) ({platform})")

        # 2. Convex Hull Volume (reduced dimensionality - 300D via PCA)
        timers = timer_print(timers, f"Convex Hull Volume (300d) ({platform})")
        try:
            if platform_embeddings.shape[1] > 300:
                pca = PCA(n_components=min(300, len(platform_embeddings)-1))
                reduced_embeddings = pca.fit_transform(platform_embeddings)
                if len(reduced_embeddings) > reduced_embeddings.shape[1]:
                    hull_reduced = ConvexHull(reduced_embeddings)
                    platform_metrics['convex_hull_volume_300d'] = hull_reduced.volume
                else:
                    platform_metrics['convex_hull_volume_300d'] = 0.0
            else:
                platform_metrics['convex_hull_volume_300d'] = platform_metrics.get('convex_hull_volume_full', 0.0)
        except:
            platform_metrics['convex_hull_volume_300d'] = 0.0
        timers = timer_print(timers, f"Convex Hull Volume (300d) ({platform})")

        # 3. Trimmed Mean Pairwise Distance
        timers = timer_print(timers, f"Mean Pairwise Distance ({platform})")
        if len(platform_embeddings) > 1:
            pairwise_dists = pdist(platform_embeddings, metric='euclidean')
            if len(pairwise_dists) > 0:
                # Middle 80%
                sorted_dists = np.sort(pairwise_dists)
                trim_start = int(len(sorted_dists) * 0.1)
                trim_end = int(len(sorted_dists) * 0.9)
                platform_metrics['trimmed_mean_distance_80'] = np.mean(sorted_dists[trim_start:trim_end]) if trim_end > trim_start else np.mean(sorted_dists)

                # Bottom 90%
                trim_end_90 = int(len(sorted_dists) * 0.9)
                platform_metrics['trimmed_mean_distance_90'] = np.mean(sorted_dists[:trim_end_90]) if trim_end_90 > 0 else np.mean(sorted_dists)

                # All markets
                platform_metrics['mean_pairwise_distance'] = np.mean(pairwise_dists)
        timers = timer_print(timers, f"Mean Pairwise Distance ({platform})")

        # 4. Cluster Diversity Score (Entropy)
        timers = timer_print(timers, f"Cluster Diversity Score ({platform})")
        platform_clusters = platform_df[platform_df['cluster'] != -1]['cluster'].values
        if len(platform_clusters) > 0:
            cluster_counts = np.bincount(platform_clusters[platform_clusters >= 0])
            cluster_probs = cluster_counts[cluster_counts > 0] / len(platform_clusters)
            platform_metrics['cluster_entropy'] = entropy(cluster_probs)

            # Participation-weighted version (if clusterer available)
            if clusterer is not None and hasattr(clusterer, 'probabilities_'):
                platform_indices = np.where(platform_mask)[0]
                valid_indices = platform_indices[platform_df['cluster'].values != -1]
                if len(valid_indices) > 0:
                    weighted_counts = {}
                    for idx, cluster_id in zip(valid_indices, platform_df[platform_df['cluster'] != -1]['cluster'].values):
                        if cluster_id not in weighted_counts:
                            weighted_counts[cluster_id] = 0
                        weighted_counts[cluster_id] += clusterer.probabilities_[idx] if idx < len(clusterer.probabilities_) else 1.0

                    total_weight = sum(weighted_counts.values())
                    if total_weight > 0:
                        weighted_probs = np.array(list(weighted_counts.values())) / total_weight
                        platform_metrics['cluster_entropy_weighted'] = entropy(weighted_probs)
        timers = timer_print(timers, f"Cluster Diversity Score ({platform})")

        # 5. Effective Topic Reach
        timers = timer_print(timers, f"Effective Topic Reach ({platform})")
        cluster_representation = {}
        for cluster_id in platform_df[platform_df['cluster'] != -1]['cluster'].unique():
            cluster_total = len(master_df[master_df['cluster'] == cluster_id])
            platform_count = len(platform_df[platform_df['cluster'] == cluster_id])
            cluster_representation[cluster_id] = platform_count / cluster_total if cluster_total > 0 else 0

        # Different thresholds
        platform_metrics['effective_reach_5pct'] = sum(1 for r in cluster_representation.values() if r >= 0.05)
        platform_metrics['effective_reach_10pct'] = sum(1 for r in cluster_representation.values() if r >= 0.10)
        platform_metrics['effective_reach_20pct'] = sum(1 for r in cluster_representation.values() if r >= 0.20)

        # Count thresholds
        cluster_counts_dict = platform_df[platform_df['cluster'] != -1]['cluster'].value_counts().to_dict()
        platform_metrics['effective_reach_1market'] = sum(1 for count in cluster_counts_dict.values() if count >= 1)
        platform_metrics['effective_reach_5markets'] = sum(1 for count in cluster_counts_dict.values() if count >= 5)
        platform_metrics['effective_reach_10markets'] = sum(1 for count in cluster_counts_dict.values() if count >= 10)
        timers = timer_print(timers, f"Effective Topic Reach ({platform})")

        # 6. Topic Concentration Coefficient (Gini)
        timers = timer_print(timers, f"Topic Concentration Coefficient ({platform})")
        if len(cluster_counts_dict) > 0:
            counts = np.array(list(cluster_counts_dict.values()))
            sorted_counts = np.sort(counts)
            n = len(sorted_counts)
            index = np.arange(1, n + 1)
            gini = (2 * index - n - 1).dot(sorted_counts) / (n * sorted_counts.sum())
            platform_metrics['topic_gini_coefficient'] = gini
        timers = timer_print(timers, f"Topic Concentration Coefficient ({platform})")

        # 7. Outlier Count
        timers = timer_print(timers, f"Outlier Count ({platform})")
        platform_metrics['outlier_count'] = len(platform_df[platform_df['cluster'] == -1])
        platform_metrics['outlier_proportion'] = platform_metrics['outlier_count'] / len(platform_df) if len(platform_df) > 0 else 0
        timers = timer_print(timers, f"Outlier Count ({platform})")

        # 8. Cross-Platform Isolation Score
        timers = timer_print(timers, f"Cross-Platform Isolation Score ({platform})")
        other_mask = ~platform_mask
        if np.any(other_mask):
            other_embeddings = embedding_vectors_norm[other_mask]
            isolation_scores = []
            for emb in platform_embeddings[:min(100, len(platform_embeddings))]:  # Sample for efficiency
                dists = np.linalg.norm(other_embeddings - emb, axis=1)
                nearest_10 = np.sort(dists)[:10]
                isolation_scores.append(np.mean(nearest_10))
            platform_metrics['cross_platform_isolation'] = np.mean(isolation_scores) if isolation_scores else 0
        timers = timer_print(timers, f"Cross-Platform Isolation Score ({platform})")

        metrics['diversity'][platform] = platform_metrics

    # ================== CLUSTER DOMINANCE METRICS ==================
    print("Computing cluster dominance metrics...")
    timers = timer_print(timers, "Dominance Initialization")

    # Calculate cluster platform distributions
    cluster_platform_dist = {}
    for cluster_id in master_df[master_df['cluster'] != -1]['cluster'].unique():
        cluster_df = master_df[master_df['cluster'] == cluster_id]
        platform_counts = cluster_df['platform_slug'].value_counts()
        total = len(cluster_df)
        cluster_platform_dist[cluster_id] = {
            'counts': platform_counts.to_dict(),
            'proportions': (platform_counts / total).to_dict(),
            'total': total,
            'dominant_platform': platform_counts.index[0],
            'dominant_proportion': platform_counts.values[0] / total
        }
    timers = timer_print(timers, "Dominance Initialization")

    # Calculate majority cluster counts and unique topic proportions
    timers = timer_print(timers, "Majority Cluster Counts")
    for threshold in [0.5, 0.75, 0.8, 0.9, 0.95]:
        threshold_key = f"majority_clusters_{int(threshold*100)}pct"
        unique_key = f"unique_topic_proportion_{int(threshold*100)}pct"

        for platform in platforms:
            majority_clusters = []
            for cluster_id, dist in cluster_platform_dist.items():
                if dist['proportions'].get(platform, 0) > threshold:
                    majority_clusters.append(cluster_id)

            platform_df = master_df[master_df['platform_slug'] == platform]
            platform_in_majority = platform_df[platform_df['cluster'].isin(majority_clusters)]

            if platform not in metrics['diversity']:
                metrics['diversity'][platform] = {}

            metrics['diversity'][platform][threshold_key] = len(majority_clusters)
            metrics['diversity'][platform][unique_key] = len(platform_in_majority) / len(platform_df) if len(platform_df) > 0 else 0
    timers = timer_print(timers, "Majority Cluster Counts")

    # Cluster Exclusivity Index
    timers = timer_print(timers, "Cluster Exclusivity Index")
    for platform in platforms:
        exclusivity_scores = []
        for cluster_id, dist in cluster_platform_dist.items():
            prop = dist['proportions'].get(platform, 0)
            exclusivity_scores.append(max(0, prop - 0.5))
        metrics['diversity'][platform]['cluster_exclusivity_index_50'] = sum(exclusivity_scores)

        # Variations with different thresholds
        exclusivity_70 = sum(max(0, dist['proportions'].get(platform, 0) - 0.7) for _, dist in cluster_platform_dist.items())
        exclusivity_80 = sum(max(0, dist['proportions'].get(platform, 0) - 0.8) for _, dist in cluster_platform_dist.items())
        metrics['diversity'][platform]['cluster_exclusivity_index_70'] = exclusivity_70
        metrics['diversity'][platform]['cluster_exclusivity_index_80'] = exclusivity_80
    timers = timer_print(timers, "Cluster Exclusivity Index")

    # ================== NOVELTY METRICS ==================
    print("Computing novelty metrics...")
    timers = timer_print(timers, "Novelty Initialization")

    # Build KNN model for novelty calculations
    nbrs = NearestNeighbors(n_neighbors=21, metric='euclidean')  # 21 to exclude self
    nbrs.fit(embedding_vectors_norm)
    timers = timer_print(timers, "Novelty Initialization")

    for platform in platforms:
        platform_mask = master_df['platform_slug'] == platform
        platform_df = master_df[platform_mask]
        platform_indices = np.where(platform_mask)[0]

        if len(platform_df) == 0:
            continue

        novelty_metrics = {}

        # Average Novelty Score (k-NN distance)
        timers = timer_print(timers, f"Average Novelty Score ({platform})")
        for k in [10, 20, 25]:
            if k < len(embedding_vectors_norm):
                nbrs_k = NearestNeighbors(n_neighbors=k+1, metric='euclidean')
                nbrs_k.fit(embedding_vectors_norm)
                distances, _ = nbrs_k.kneighbors(embedding_vectors_norm[platform_mask])
                avg_distances = np.mean(distances[:, 1:], axis=1)  # Exclude self
                novelty_metrics[f'average_novelty_k{k}'] = np.mean(avg_distances)
        timers = timer_print(timers, f"Average Novelty Score ({platform})")

        # High-Novelty Market Count
        timers = timer_print(timers, f"High-Novelty Market Count ({platform})")
        distances_20, _ = nbrs.kneighbors(embedding_vectors_norm)
        dist_to_20th = distances_20[:, 20]  # 20th neighbor (excluding self)

        for percentile in [80, 90, 95, 98]:
            threshold = np.percentile(dist_to_20th, percentile)
            platform_high_novelty = dist_to_20th[platform_mask] > threshold
            novelty_metrics[f'high_novelty_count_p{percentile}'] = np.sum(platform_high_novelty)
        timers = timer_print(timers, f"High-Novelty Market Count ({platform})")

        # Local Outlier Factor
        timers = timer_print(timers, f"Local Outlier Factor ({platform})")
        lof = LocalOutlierFactor(n_neighbors=20, contamination=0.1)
        lof.fit_predict(embedding_vectors_norm)
        lof_scores = -lof.negative_outlier_factor_  # Convert to positive scores
        platform_lof = lof_scores[platform_mask]
        novelty_metrics['lof_outliers_1.5'] = np.sum(platform_lof > 1.5)
        novelty_metrics['lof_outliers_2.0'] = np.sum(platform_lof > 2.0)
        novelty_metrics['mean_lof_score'] = np.mean(platform_lof)
        timers = timer_print(timers, f"Local Outlier Factor ({platform})")

        # Novelty-Weighted Unique Coverage
        timers = timer_print(timers, f"Novelty-Weighted Unique Coverage ({platform})")
        local_density = 1.0 / (dist_to_20th + 1e-10)  # Inverse distance as density
        density_threshold = np.percentile(local_density, 20)

        weights = np.ones(len(master_df))
        weights[local_density < density_threshold] = 2.0  # Double weight for sparse areas

        weighted_coverage = 0
        for cluster_id in cluster_platform_dist:
            if cluster_platform_dist[cluster_id]['proportions'].get(platform, 0) > 0.5:
                cluster_mask = master_df['cluster'] == cluster_id
                platform_cluster_mask = cluster_mask & platform_mask
                weighted_coverage += np.sum(weights[platform_cluster_mask])

        novelty_metrics['novelty_weighted_coverage'] = weighted_coverage
        timers = timer_print(timers, f"Novelty-Weighted Unique Coverage ({platform})")

        metrics['novelty'][platform] = novelty_metrics

    # ================== INNOVATION METRICS ==================
    timers = timer_print(timers, "Innovation Initialization")
    if 'created_datetime' in master_df.columns:
        print("Computing innovation metrics...")

        # Get cluster temporal information
        timers = timer_print(timers, "Temporal Setup")
        cluster_temporal = {}
        for cluster_id in master_df[master_df['cluster'] != -1]['cluster'].unique():
            cluster_df = master_df[master_df['cluster'] == cluster_id]
            cluster_df_sorted = cluster_df.sort_values('created_datetime')

            first_market = cluster_df_sorted.iloc[0]
            cluster_temporal[cluster_id] = {
                'first_market_id': first_market['id'],
                'first_platform': first_market['platform_slug'],
                'first_timestamp': first_market['created_datetime'],
                'size': len(cluster_df),
                'platforms_temporal': cluster_df_sorted.groupby('platform_slug')['created_datetime'].agg(['min', 'median', 'count']).to_dict('index')
            }

            # Calculate centroid
            cluster_embeddings = embedding_vectors_norm[master_df['cluster'] == cluster_id]
            cluster_temporal[cluster_id]['centroid'] = np.mean(cluster_embeddings, axis=0)

            # Distance from first market to centroid
            first_market_idx = master_df[master_df['id'] == first_market['id']].index[0]
            first_market_emb = embedding_vectors_norm[first_market_idx]
            cluster_temporal[cluster_id]['first_to_centroid_dist'] = np.linalg.norm(
                first_market_emb - cluster_temporal[cluster_id]['centroid']
            )

            # Cluster persistence (if clusterer available)
            if clusterer is not None and hasattr(clusterer, 'cluster_persistence_'):
                cluster_temporal[cluster_id]['persistence'] = clusterer.cluster_persistence_[cluster_id] if cluster_id < len(clusterer.cluster_persistence_) else 1.0
            else:
                cluster_temporal[cluster_id]['persistence'] = 1.0
        timers = timer_print(timers, "Temporal Setup")

        for platform in platforms:
            innovation_metrics = {}

            # Cluster Founder Count (Simple)
            timers = timer_print(timers, f"Cluster Founder Count ({platform})")
            founded_clusters = [cid for cid, info in cluster_temporal.items() if info['first_platform'] == platform]
            innovation_metrics['clusters_founded'] = len(founded_clusters)

            # Cluster Founder Count (Centrality-Weighted)
            centrality_score = 0
            for cluster_id in founded_clusters:
                dist = cluster_temporal[cluster_id]['first_to_centroid_dist']
                size = cluster_temporal[cluster_id]['size']
                centrality_score += (1 / (1 + dist)) * np.log1p(size)
            innovation_metrics['clusters_founded_centrality_weighted'] = centrality_score
            timers = timer_print(timers, f"Cluster Founder Count ({platform})")

            # Growth Catalyst Score
            timers = timer_print(timers, f"Growth Catalyst Score ({platform})")
            catalyst_scores = []
            for window_days in [3, 7, 14, 30]:
                window_score = 0
                for cluster_id, info in cluster_temporal.items():
                    if info['size'] > 50:  # Only large clusters
                        window_end = info['first_timestamp'] + timedelta(days=window_days)
                        early_markets = master_df[
                            (master_df['cluster'] == cluster_id) &
                            (master_df['platform_slug'] == platform) &
                            (master_df['created_datetime'] <= window_end)
                        ]

                        if len(early_markets) > 0:
                            # Score by proximity to centroid
                            early_indices = early_markets.index
                            early_embeddings = embedding_vectors_norm[early_indices]
                            distances = np.linalg.norm(early_embeddings - info['centroid'], axis=1)
                            proximity_scores = 1 / (1 + distances)

                            # Weight by cluster persistence
                            persistence = info.get('persistence', 1.0)
                            window_score += np.sum(proximity_scores) * persistence

                catalyst_scores.append(window_score)
                innovation_metrics[f'growth_catalyst_{window_days}d'] = window_score
            timers = timer_print(timers, f"Growth Catalyst Score ({platform})")

            # Innovation Index
            timers = timer_print(timers, f"Innovation Index ({platform})")
            platform_df = master_df[master_df['platform_slug'] == platform]
            if len(platform_df) > 0 and len(founded_clusters) > 0:
                avg_persistence = np.mean([cluster_temporal[cid].get('persistence', 1.0) for cid in founded_clusters])
                innovation_metrics['innovation_index'] = (len(founded_clusters) / len(platform_df)) * avg_persistence
            else:
                innovation_metrics['innovation_index'] = 0
            timers = timer_print(timers, f"Innovation Index ({platform})")

            # Temporal Cluster Precedence
            timers = timer_print(timers, f"Temporal Cluster Precedence ({platform})")
            precedence_counts = {'first': 0, 'median': 0, 'fifth': 0}
            participated_clusters = 0

            for cluster_id, info in cluster_temporal.items():
                if platform in info['platforms_temporal']:
                    participated_clusters += 1
                    platform_times = info['platforms_temporal']

                    # Check if platform was first (by median)
                    all_medians = {p: times['median'] for p, times in platform_times.items()}
                    if platform == min(all_medians.keys(), key=lambda p: all_medians[p]):
                        precedence_counts['median'] += 1

                    # Check if platform was first (by first market)
                    all_firsts = {p: times['min'] for p, times in platform_times.items()}
                    if platform == min(all_firsts.keys(), key=lambda p: all_firsts[p]):
                        precedence_counts['first'] += 1

                    # Check if platform was in first 5
                    cluster_df_time = master_df[master_df['cluster'] == cluster_id].sort_values('created_datetime')
                    if len(cluster_df_time) >= 5:
                        first_5_platforms = cluster_df_time.iloc[:5]['platform_slug'].values
                        if platform in first_5_platforms:
                            precedence_counts['fifth'] += 1

            if participated_clusters > 0:
                innovation_metrics['temporal_precedence_first'] = precedence_counts['first'] / participated_clusters
                innovation_metrics['temporal_precedence_median'] = precedence_counts['median'] / participated_clusters
                innovation_metrics['temporal_precedence_fifth'] = precedence_counts['fifth'] / participated_clusters
            timers = timer_print(timers, f"Temporal Cluster Precedence ({platform})")

            metrics['innovation'][platform] = innovation_metrics

    # ================== COMPETITION METRICS ==================
    print("Computing competition metrics...")

    # Cross-Platform Topic Flow
    timers = timer_print(timers, "Cross-Platform Topic Flow")
    if 'created_datetime' in master_df.columns:
        topic_flow = defaultdict(lambda: defaultdict(int))

        for cluster_id, info in cluster_temporal.items():
            platforms_in_cluster = list(info['platforms_temporal'].keys())
            if len(platforms_in_cluster) > 1:
                # Sort platforms by entry time
                sorted_platforms = sorted(platforms_in_cluster,
                                        key=lambda p: info['platforms_temporal'][p]['min'])

                # Record flows
                for i in range(len(sorted_platforms) - 1):
                    topic_flow[sorted_platforms[i]][sorted_platforms[i+1]] += 1

        # Calculate in-degree and out-degree
        for platform in platforms:
            in_degree = sum(topic_flow[other][platform] for other in platforms if other != platform)
            out_degree = sum(topic_flow[platform][other] for other in platforms if other != platform)

            if platform not in metrics['competition']:
                metrics['competition'][platform] = {}

            metrics['competition'][platform]['topic_flow_in_degree'] = in_degree
            metrics['competition'][platform]['topic_flow_out_degree'] = out_degree
            metrics['competition'][platform]['topic_flow_ratio'] = out_degree / (in_degree + 1)  # Avoid division by zero
    timers = timer_print(timers, "Cross-Platform Topic Flow")

    # Platform Overlap Matrix
    timers = timer_print(timers, "Platform Overlap Matrix")
    platform_overlap_matrix = {}
    for p1 in platforms:
        platform_overlap_matrix[p1] = {}
        p1_clusters = set(master_df[(master_df['platform_slug'] == p1) & (master_df['cluster'] != -1)]['cluster'].unique())

        for p2 in platforms:
            if p1 == p2:
                platform_overlap_matrix[p1][p2] = 1.0
            else:
                p2_clusters = set(master_df[(master_df['platform_slug'] == p2) & (master_df['cluster'] != -1)]['cluster'].unique())

                # Jaccard similarity
                intersection = p1_clusters & p2_clusters
                union = p1_clusters | p2_clusters

                if len(union) > 0:
                    # Weighted version
                    weighted_intersection = 0
                    weighted_union = 0

                    for cluster_id in union:
                        p1_count = len(master_df[(master_df['platform_slug'] == p1) & (master_df['cluster'] == cluster_id)])
                        p2_count = len(master_df[(master_df['platform_slug'] == p2) & (master_df['cluster'] == cluster_id)])

                        if cluster_id in intersection:
                            weighted_intersection += min(p1_count, p2_count)
                        weighted_union += max(p1_count, p2_count)

                    platform_overlap_matrix[p1][p2] = weighted_intersection / weighted_union if weighted_union > 0 else 0

                    # Unweighted version
                    metrics['competition'].setdefault(p1, {})
                    metrics['competition'][p1][f'overlap_with_{p2}_unweighted'] = len(intersection) / len(union)
                else:
                    platform_overlap_matrix[p1][p2] = 0

    metrics['competition']['overlap_matrix_weighted'] = platform_overlap_matrix
    timers = timer_print(timers, "Platform Overlap Matrix")

    # Topic Competition Intensity (HHI per cluster)
    timers = timer_print(timers, "Topic Competition Intensity")
    hhi_scores = []
    for cluster_id, dist in cluster_platform_dist.items():
        proportions = list(dist['proportions'].values())
        hhi = sum(p**2 for p in proportions)
        hhi_scores.append({'cluster_id': cluster_id, 'hhi': hhi, 'size': dist['total']})

    metrics['competition']['cluster_hhi_scores'] = sorted(hhi_scores, key=lambda x: x['hhi'])
    metrics['competition']['mean_hhi'] = np.mean([h['hhi'] for h in hhi_scores])
    metrics['competition']['weighted_mean_hhi'] = np.average(
        [h['hhi'] for h in hhi_scores],
        weights=[h['size'] for h in hhi_scores]
    )
    timers = timer_print(timers, "Topic Competition Intensity")

    # ================== PLATFORM STATISTICS ==================
    for platform in platforms:
        platform_df = master_df[master_df['platform_slug'] == platform]
        metrics['platform_stats'][platform] = {
            'total_markets': len(platform_df),
            'clustered_markets': len(platform_df[platform_df['cluster'] != -1]),
            'unique_clusters': len(platform_df[platform_df['cluster'] != -1]['cluster'].unique()),
            'mean_novelty': platform_df['novelty'].mean() if 'novelty' in platform_df.columns else 0,
            'median_novelty': platform_df['novelty'].median() if 'novelty' in platform_df.columns else 0,
        }

    return metrics


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
    .
    """
    parser = argparse.ArgumentParser(
        description="Market embedding analysis with clustering"
    )
    parser.add_argument(
        "--cache-dir",
        "-cd",
        default="./cache",
        help="Cache directory (default: ./cache)",
    )
    parser.add_argument(
        "--ignore-cache",
        action="store_true",
        help="Ignore existing cache files and regenerate all data",
    )
    parser.add_argument(
        "--output-dir",
        "-od",
        default="./output",
        help="Output directory for PNG files (default: ./output)",
    )
    parser.add_argument(
        "--pca-dim",
        "-d",
        type=int,
        default=300,
        help="PCA dimensionality reduction target (default: 300, 0 to skip)",
    )
    parser.add_argument(
        "--sample-size",
        "-ss",
        type=int,
        default=0,
        help="Sample size for clustering (default: all)",
    )
    parser.add_argument(
        "--sample-platform",
        "-sp",
        type=str,
        default=None,
        help="Filter sample to specific platform_slug (default: all)",
    )
    parser.add_argument(
        "--min-cluster-size",
        "-cs",
        type=int,
        default=20,
        help="Minimum cluster size for HDBSCAN (default: 20)",
    )
    parser.add_argument(
        "--cluster-selection-epsilon",
        "-ce",
        type=float,
        default=0,
        help="cluster_selection_epsilon size for HDBSCAN (default: 0)",
    )
    parser.add_argument(
        "--plot-method",
        "-p",
        default="tsne",
        choices=["umap", "tsne", "pca"],
        help="Plotting method for clusters (default: tsne)",
    )
    parser.add_argument(
        "--no-tables", action="store_true", help="Don't print summary tables"
    )
    args = parser.parse_args()

    load_dotenv()
    postgrest_base = os.environ.get("PGRST_URL")

    # Create cache file names with platform filtering
    platform_suffix = f"_{args.sample_platform}" if args.sample_platform else ""
    markets_cache = f"{args.cache_dir}/markets.jsonl"
    market_embeddings_cache = f"{args.cache_dir}/market_embeddings.jsonl"
    market_embeddings_pca_cache = (
        f"{args.cache_dir}/market_embeddings_pca_{args.pca_dim}.jsonl"
    )
    novelty_cache = f"{args.cache_dir}/market_novelty.jsonl"
    cluster_cache = f"{args.cache_dir}/market_clusters_{args.sample_size}_{args.min_cluster_size}_{args.cluster_selection_epsilon}{platform_suffix}.jsonl"
    clusterer_cache = f"{args.cache_dir}/clusterer_{args.sample_size}_{args.min_cluster_size}_{args.cluster_selection_epsilon}{platform_suffix}.pkl"
    cluster_info_cache = f"{args.cache_dir}/cluster_info_{args.sample_size}_{args.min_cluster_size}_{args.cluster_selection_epsilon}{platform_suffix}.jsonl"
    embeddings_2d_cache = f"{args.cache_dir}/embeddings_2d_{args.sample_size}_{args.plot_method}{platform_suffix}.jsonl"

    # Create cache & output directory if it doesn't exist
    os.makedirs(args.cache_dir, exist_ok=True)
    os.makedirs(args.output_dir, exist_ok=True)

    # Step 1: Load and prepare base data
    print("Loading base market data...")
    markets_df = None if args.ignore_cache else load_dataframe_from_cache(markets_cache)
    if markets_df is None:
        markets_df = get_data_as_dataframe(
            f"{postgrest_base}/markets", params={"order": "id"}
        )
        save_dataframe_to_cache(markets_cache, markets_df)

    # Apply platform filtering
    if args.sample_platform:
        original_count = len(markets_df)
        markets_df = markets_df[markets_df["platform_slug"] == args.sample_platform]
        print(
            f"Platform filtering: {len(markets_df)}/{original_count} markets from '{args.sample_platform}'"
        )

    # Calculate market scores
    markets_df["score"] = calculate_market_scores(markets_df)

    # Step 2: Load embeddings
    print("Loading market embeddings...")
    if args.pca_dim > 0:
        embeddings_df = (
            None
            if args.ignore_cache
            else load_dataframe_from_cache(market_embeddings_pca_cache)
        )
        if embeddings_df is not None:
            print(f"Loaded PCA-reduced embeddings from cache ({args.pca_dim}D)")

    if args.pca_dim == 0 or embeddings_df is None:
        embeddings_df = (
            None
            if args.ignore_cache
            else load_dataframe_from_cache(market_embeddings_cache)
        )
        if embeddings_df is None:
            print("Loading embeddings from API...")
            raw_embeddings = get_data_as_dataframe(
                f"{postgrest_base}/market_embeddings", params={"order": "market_id"}
            )
            # Parse JSON embeddings more efficiently
            print("Parsing embedding data...")
            embeddings_df = pd.DataFrame(
                {
                    "market_id": raw_embeddings["market_id"],
                    "embedding": raw_embeddings["embedding"].apply(json.loads),
                }
            )

            # Convert to more efficient format and validate
            print(
                f"Loaded {len(embeddings_df)} embeddings with dimension {len(embeddings_df['embedding'].iloc[0])}"
            )
            save_dataframe_to_cache(market_embeddings_cache, embeddings_df)

        # Apply PCA dimensionality reduction if requested
        if args.pca_dim > 0:
            embeddings_df = apply_pca_reduction(embeddings_df, args.pca_dim)
            save_dataframe_to_cache(market_embeddings_pca_cache, embeddings_df)

    # Ensure that all markets have embeddings
    markets_with_embeddings = set(embeddings_df["market_id"])
    missing_markets = markets_df[~markets_df["id"].isin(markets_with_embeddings)]
    if not missing_markets.empty:
        print(f"Warning: {len(missing_markets)} markets are missing embeddings")
        markets_df = markets_df[markets_df["id"].isin(markets_with_embeddings)]

    # Step 3: Load novelty scores
    print("Loading novelty scores...")
    novelty_df = None if args.ignore_cache else load_dataframe_from_cache(novelty_cache)
    if novelty_df is None:
        # Only compute novelty for markets we have
        analysis_embeddings = embeddings_df[
            embeddings_df["market_id"].isin(markets_df["id"])
        ]
        novelty_df = compute_novelty_faiss(analysis_embeddings)
        save_dataframe_to_cache(novelty_cache, novelty_df)

    # Step 4: Create master DataFrame with all market data
    print("Creating consolidated market analysis DataFrame...")
    master_df = markets_df.merge(
        embeddings_df, left_on="id", right_on="market_id", how="inner"
    ).merge(
        novelty_df,
        left_on="id",
        right_on="market_id",
        how="inner",
        suffixes=("", "_novelty"),
    )

    print(f"Master DataFrame contains {len(master_df)} markets with complete data")

    # Step 5: Create clusters
    clusters_df = (
        None if args.ignore_cache else load_dataframe_from_cache(cluster_cache)
    )
    if clusters_df is None:
        # Sample for clustering if requested
        if args.sample_size == 0 or args.sample_size >= len(master_df):
            clustering_data = pd.DataFrame(
                {"market_id": master_df["id"], "embedding": master_df["embedding"]}
            )
            # Keep track of all market IDs when not sampling
            sampled_market_ids = set(master_df["id"])
            print(f"Using all {len(clustering_data)} markets for clustering")
        else:
            clustering_sample = master_df.sample(n=args.sample_size, random_state=42)
            clustering_data = pd.DataFrame(
                {
                    "market_id": clustering_sample["id"],
                    "embedding": clustering_sample["embedding"],
                }
            )
            # Keep track of sampled market IDs for filtering later
            sampled_market_ids = set(clustering_sample["id"])
            print(f"Using sample of {len(clustering_data)} markets for clustering")

        clusters_df, clusterer = create_clusters_hdbscan(
            clustering_data, args.min_cluster_size, args.cluster_selection_epsilon
        )
        save_dataframe_to_cache(cluster_cache, clusters_df)

        # Cache the clusterer object using pickle
        try:
            with open(clusterer_cache, "wb") as f:
                pickle.dump(clusterer, f)
            print(f"Clusterer saved to cache: {clusterer_cache}")
        except Exception as e:
            print(f"Warning: Could not cache clusterer: {e}")
    else:
        # When loading from cache, reconstruct sampled_market_ids from cached cluster data
        # The cached clusters_df contains only the markets that were in the original sample
        sampled_market_ids = set(clusters_df["market_id"])
        print(f"Reconstructed sample of {len(sampled_market_ids)} markets from cache")

        # Try to load clusterer from cache
        clusterer = None
        if not args.ignore_cache:
            try:
                with open(clusterer_cache, "rb") as f:
                    clusterer = pickle.load(f)
                print(f"Clusterer loaded from cache: {clusterer_cache}")
            except Exception as e:
                print(f"Warning: Could not load clusterer from cache: {e}")

    # Add cluster information to master DataFrame
    master_df = master_df.merge(
        clusters_df,
        left_on="id",
        right_on="market_id",
        how="left",
        suffixes=("", "_cluster"),
    )
    master_df["cluster"] = (
        master_df["cluster"].fillna(-1).astype(int)
    )  # Non-clustered markets get -1
    clustered_df = master_df[master_df["cluster"] != -1]
    print(f"Successfully clustered {len(clustered_df)}/{len(master_df)} markets")

    # Step 6: Generate cluster information
    cached_cluster_info = (
        None if args.ignore_cache else load_dataframe_from_cache(cluster_info_cache)
    )
    if cached_cluster_info is None:
        cluster_info_dict = collate_cluster_information(clustered_df)
        # Cache cluster statistics (without full market data)
        cluster_stats = []
        for cluster_id, info in cluster_info_dict.items():
            stats = {
                k: v
                for k, v in info.items()
                if k not in ["markets", "top_market", "first_market"]
            }
            stats["cluster_id"] = cluster_id
            cluster_stats.append(stats)
        # Generate cluster keywords
        cluster_info_dict = generate_cluster_keywords_tfidf(cluster_info_dict)
        # Save it
        save_dataframe_to_cache(cluster_info_cache, pd.DataFrame(cluster_stats))
    else:
        # Reconstruct cluster info from cache and add current market data
        cluster_info_dict = {}
        for _, row in cached_cluster_info.iterrows():
            cluster_id = row["cluster_id"]
            cluster_info_dict[cluster_id] = row.drop("cluster_id").to_dict()

        # Add live market data for current run
        for cluster_id in cluster_info_dict.keys():
            cluster_markets = clustered_df[clustered_df["cluster"] == cluster_id]
            if len(cluster_markets) > 0:
                cluster_info_dict[cluster_id]["markets"] = cluster_markets.to_dict(
                    orient="records"
                )  # type: ignore
                top_market_idx = (
                    cluster_markets["score"].idxmax()
                    if hasattr(cluster_markets["score"], "idxmax")
                    else cluster_markets["score"].argmax()
                )  # type: ignore
                cluster_info_dict[cluster_id]["top_market"] = cluster_markets.loc[
                    top_market_idx
                ].to_dict()  # type: ignore

    # Generate 2D embeddings for visualization
    embeddings_2d_df = (
        None if args.ignore_cache else load_dataframe_from_cache(embeddings_2d_cache)
    )
    if embeddings_2d_df is None:
        print(
            f"Generating {args.plot_method.upper()} 2D embeddings for visualization..."
        )
        viz_embeddings = pd.DataFrame(
            {"market_id": clustered_df["id"], "embedding": clustered_df["embedding"]}
        )

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
    create_interactive_visualization(
        args.plot_method.upper(),
        embeddings_2d_df,
        clusters_df,
        master_df,
        cluster_info_dict,
        html_output_file,
        display_prob,
    )
    print(f"Interactive plot saved to {html_output_file}")

    # Step 8: Generate summary reports using consolidated DataFrame
    if not args.no_tables:
        print("\n" + "=" * 80)
        print("MARKET ANALYSIS SUMMARY")
        print("=" * 80)

        print("\nMost Novel Markets:")
        print(master_df.nlargest(10, "novelty")[["id", "title", "novelty"]])

        print("\nLeast Novel Markets:")
        print(master_df.nsmallest(10, "novelty")[["id", "title", "novelty"]])

        print("\nClusters Summary:")
        cluster_summary = []
        for cluster_id, info in cluster_info_dict.items():
            keywords = info.get("keywords", "")
            keywords = keywords[:52] + "..." if len(keywords) > 55 else keywords

            cluster_summary.append(
                [
                    cluster_id,
                    info.get("market_count", 0),
                    keywords,
                    f"{info.get('top_platform', 'unknown')} ({100.0 * info.get('top_platform_pct', 0):.2f}%)",
                    f"{info.get('median_novelty', 0):.3f}",
                    f"{info.get('median_score', 0):.3f}",
                    f"{info.get('mean_resolution', 0):.3f}",
                ]
            )

        print(
            tabulate(
                cluster_summary,
                headers=[
                    "ID",
                    "Count",
                    "Keywords",
                    "Top Platform",
                    "Md Novelty",
                    "Md Score",
                    "Mn Res",
                ],
                tablefmt="github",
            )
        )

    # Step 9: Calculate comprehensive platform metrics
    # Filter master_df to only include markets that were part of the clustering sample
    filtered_master_df = master_df[master_df["id"].isin(sampled_market_ids)]
    print(f"\nCalculating platform metrics on {len(filtered_master_df)}/{len(master_df)} markets (filtered to clustering sample)")
    platform_metrics = calculate_platform_metrics(filtered_master_df, clusterer, cluster_info_dict)

    # Save metrics to file
    metrics_output_file = f"{args.output_dir}/platform_metrics.json"
    with open(metrics_output_file, 'w') as f:
        # Convert numpy types to Python types for JSON serialization
        def convert_to_serializable(obj):
            if isinstance(obj, np.ndarray):
                return obj.tolist()
            elif isinstance(obj, (np.integer, np.int64, np.int32)):
                return int(obj)
            elif isinstance(obj, (np.floating, np.float64, np.float32)):
                return float(obj)
            elif isinstance(obj, dict):
                return {k: convert_to_serializable(v) for k, v in obj.items()}
            elif isinstance(obj, list):
                return [convert_to_serializable(v) for v in obj]
            elif isinstance(obj, pd.Timestamp):
                return obj.isoformat()
            else:
                return obj

        serializable_metrics = convert_to_serializable(platform_metrics)
        json.dump(serializable_metrics, f, indent=2)
    print(f"\nPlatform metrics saved to {metrics_output_file}")

    # Print summary of key metrics
    if not args.no_tables:
        print("\n" + "=" * 80)
        print("PLATFORM METRICS SUMMARY")
        print("=" * 80)

        # Create summary table
        summary_data = []
        for platform in platform_metrics['platform_stats'].keys():
            row = [
                platform,
                platform_metrics['platform_stats'][platform]['total_markets'],
                platform_metrics['platform_stats'][platform]['unique_clusters'],
                f"{platform_metrics['diversity'].get(platform, {}).get('cluster_entropy', 0):.2f}",
                f"{platform_metrics['diversity'].get(platform, {}).get('effective_reach_10pct', 0)}",
                f"{platform_metrics['innovation'].get(platform, {}).get('clusters_founded', 0)}",
                f"{platform_metrics['novelty'].get(platform, {}).get('average_novelty_k20', 0):.3f}",
            ]
            summary_data.append(row)

        print(
            tabulate(
                summary_data,
                headers=["Platform", "Markets", "Clusters", "Entropy", "Reach(10%)", "Founded", "Avg Novelty"],
                tablefmt="github",
            )
        )


if __name__ == "__main__":
    main()
