# Embedding Analysis Tools

This folder has some exploratory data analysis scripts mainly focused on natural language embeddings from Manifold and other prediction market sites. As an experiment this was mostly vibe-coded and is not explicitly supported.

## Overview

The analysis pipeline performs:
- **Clustering Analysis**: HDBSCAN-based topic discovery from market embeddings
- **Diversity Metrics**: Measure how broadly platforms cover the topic space
- **Novelty Analysis**: Identify unique and unusual markets
- **Innovation Tracking**: Detect platforms that create new topics
- **Competition Analysis**: Map platform overlap and competitive dynamics
- **Interactive Visualizations**: Explore clusters and relationships

## Main Components

### 1. `embedding-analysis.py`
Main analysis script that orchestrates the complete pipeline.

**Features:**
- Loads market data and embeddings via PostgREST API
- Performs HDBSCAN clustering with configurable parameters
- Calculates comprehensive platform metrics across 4 categories
- Generates static and interactive visualizations
- Produces detailed cluster analysis with keywords

**Usage:**
```bash
uv run embedding-analysis.py \
  --cache-dir ./cache \
  --output-dir ./output \
  --pca-dim 300 \
  --min-cluster-size 20 \
  --plot-method tsne
```

**Key Parameters:**
- `--pca-dim`: Dimensionality reduction target (0 to skip)
- `--min-cluster-size`: Minimum cluster size for HDBSCAN
- `--cluster-selection-epsilon`: Epsilon for cluster selection
- `--plot-method`: Visualization method (umap, tsne, pca)
- `--sample-size`: Sample size for clustering (0 for all)
- `--sample-platform`: Filter to specific platform

## Metrics Categories

### Diversity Metrics
- **Convex Hull Volume**: Breadth of topic coverage
- **Trimmed Mean Pairwise Distance**: Internal diversity
- **Cluster Entropy**: Distribution across topics
- **Effective Topic Reach**: Meaningful presence in topics
- **Topic Concentration (Gini)**: Specialization vs generalization
- **Cross-Platform Isolation**: Unique positioning

### Novelty Metrics
- **Average Novelty Score**: Portfolio uniqueness
- **High-Novelty Market Count**: Frontier exploration
- **Local Outlier Factor**: Statistical outliers
- **Novelty-Weighted Coverage**: Dominance in sparse areas

### Innovation Metrics
- **Cluster Founder Count**: Topics created
- **Growth Catalyst Score**: Sparking topic growth
- **Innovation Index**: Creation efficiency
- **Temporal Precedence**: Systematic early entry

### Competition Metrics
- **Topic Flow Analysis**: Originator vs adopter dynamics
- **Platform Overlap Matrix**: Competitive landscape
- **Topic Competition Intensity**: Market concentration (HHI)

## Output Files

### Data Files
- `platform_metrics.json`: All calculated metrics in structured format
- `platform_summary.csv`: Summary table for analysis
- `market_clusters_*.jsonl`: Cluster assignments
- `cluster_info_*.jsonl`: Cluster statistics

### Visualizations
- `clusters_*.png`: Static cluster visualization
- `clusters_*_interactive.html`: Interactive Plotly visualization
- `diversity_metrics.png`: Diversity metric plots
- `innovation_metrics.png`: Innovation metric plots
- `novelty_metrics.png`: Novelty metric plots
- `platform_overlap.png`: Competition heatmap

### Reports
- `metrics_report.html`: Comprehensive HTML report with all visualizations

## Installation

```bash
# Install required packages
uv sync
```

## Data Requirements

The analysis expects market data with:
- `id`: Unique market identifier
- `platform_slug`: Platform identifier
- `title`: Market title
- `created_time`: Market creation timestamp
- `volume_usd`: Trading volume
- `traders_count`: Number of traders
- `embedding`: Dense vector representation (768D or similar)

## Caching

The pipeline implements intelligent caching to avoid redundant computations:
- Market data cached as JSONL
- Embeddings cached with optional PCA reduction
- Clustering results cached per configuration
- 2D projections cached per visualization method

Use `--ignore-cache` to force recomputation.
