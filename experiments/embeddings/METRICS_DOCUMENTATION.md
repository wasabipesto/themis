# Platform Metrics Documentation

## Overview

This document describes the comprehensive platform metrics implemented for analyzing prediction market embeddings. The metrics are organized into four main categories: Diversity, Novelty, Innovation, and Competition.

## Metric Categories

### 1. Diversity Metrics

These metrics measure how broadly each platform covers the topic space and the diversity of their market portfolios.

#### Convex Hull Volume
- **Implementation**: Computes the convex hull of each platform's embeddings in the full embedding space using `scipy.spatial.ConvexHull`, then extracts the volume.
- **Interpretation**: Higher volume indicates broader topic coverage. This is a raw measure of the topic space occupied by a platform.
- **Variations**: 
  - `convex_hull_volume_full`: Full dimensionality (768D)
  - `convex_hull_volume_300d`: Reduced dimensionality (300D via PCA)

#### Trimmed Mean Pairwise Distance
- **Implementation**: For each platform, computes all pairwise distances between markets, removes top/bottom percentiles, and averages the remaining distances.
- **Interpretation**: Higher values indicate more internal diversity. This metric is robust to both outliers and overly-clustered markets.
- **Variations**:
  - `trimmed_mean_distance_80`: Middle 80% of distances
  - `trimmed_mean_distance_90`: Bottom 90% of distances
  - `mean_pairwise_distance`: All pairwise distances

#### Cluster Diversity Score (Entropy)
- **Implementation**: Calculates Shannon entropy: -Σ(p_j * log(p_j)) where p_j is the proportion of markets in cluster j.
- **Interpretation**: Higher entropy indicates more even distribution across clusters. Maximum value is log(num_clusters).
- **Variations**:
  - `cluster_entropy`: Standard entropy calculation
  - `cluster_entropy_weighted`: Uses HDBSCAN membership probabilities

#### Effective Topic Reach
- **Implementation**: Counts unique clusters where platform has meaningful representation.
- **Interpretation**: Number of distinct topic areas with meaningful presence. Less sensitive to noise than raw cluster count.
- **Variations**:
  - Percentage thresholds: `effective_reach_5pct`, `effective_reach_10pct`, `effective_reach_20pct`
  - Count thresholds: `effective_reach_1market`, `effective_reach_5markets`, `effective_reach_10markets`

#### Majority Cluster Count
- **Implementation**: Counts clusters where one platform has >X% of markets.
- **Interpretation**: Number of topics dominated by each platform.
- **Variations**: `majority_clusters_50pct`, `majority_clusters_75pct`, `majority_clusters_80pct`, `majority_clusters_90pct`, `majority_clusters_95pct`

#### Unique Topic Proportion
- **Implementation**: (Platform's markets in majority clusters) / (platform's total markets)
- **Interpretation**: Percentage of platform's portfolio in topics they dominate. Higher values indicate more specialization.
- **Variations**: `unique_topic_proportion_50pct` through `unique_topic_proportion_95pct`

#### Cluster Exclusivity Index
- **Implementation**: Σ(max(0, p_i - threshold)) across all clusters, where p_i is platform's proportion in cluster i.
- **Interpretation**: Cumulative dominance score. Higher values indicate more total dominance across all topics.
- **Variations**: `cluster_exclusivity_index_50`, `cluster_exclusivity_index_70`, `cluster_exclusivity_index_80`

#### Topic Concentration Coefficient (Gini)
- **Implementation**: Gini coefficient of platform's market distribution across clusters.
- **Interpretation**: 0 = perfectly uniform across topics, 1 = concentrated in single topic. Measures specialization vs generalization.
- **Output**: `topic_gini_coefficient`

#### Outlier Metrics
- **Implementation**: Counts markets that fall into cluster -1 (unclustered).
- **Interpretation**: Higher counts indicate more unique/unusual markets that don't fit established topics.
- **Variations**:
  - `outlier_count`: Absolute count
  - `outlier_proportion`: Proportion of platform's markets

#### Cross-Platform Isolation Score
- **Implementation**: For each market, computes average distance to nearest 10 markets from OTHER platforms.
- **Interpretation**: Higher values indicate platform's markets are more isolated from competitors' markets. Shows unique positioning.
- **Output**: `cross_platform_isolation`

### 2. Novelty Metrics

These metrics measure how unique and unusual each platform's markets are compared to the overall market landscape.

#### Average Novelty Score
- **Implementation**: Mean k-nearest-neighbor distance per platform.
- **Interpretation**: Average uniqueness of platform's markets. Higher values indicate generally more unique portfolio.
- **Variations**: `average_novelty_k10`, `average_novelty_k20`, `average_novelty_k25`

#### High-Novelty Market Count
- **Implementation**: Counts markets with k-NN distance above global percentile threshold.
- **Interpretation**: Number of highly isolated/unique markets per platform.
- **Variations**: `high_novelty_count_p80`, `high_novelty_count_p90`, `high_novelty_count_p95`, `high_novelty_count_p98`

#### Local Outlier Factor Analysis
- **Implementation**: Uses `sklearn.neighbors.LocalOutlierFactor` with n_neighbors=20.
- **Interpretation**: Markets that are outliers relative to their local neighborhood density.
- **Outputs**:
  - `lof_outliers_1.5`: Count of markets with LOF > 1.5
  - `lof_outliers_2.0`: Count of markets with LOF > 2.0
  - `mean_lof_score`: Average LOF score for platform

#### Novelty-Weighted Unique Coverage
- **Implementation**: 
  1. Computes local density for each market (inverse of distance to 20th nearest neighbor)
  2. Weights markets in bottom 20% density as 2x, others as 1x
  3. Counts weighted markets in majority clusters per platform
- **Interpretation**: Credit for being unique in sparse/novel topic areas, not just dense popular areas.
- **Output**: `novelty_weighted_coverage`

### 3. Innovation Metrics

These metrics identify which platforms are creating new topics and leading market innovation. Requires timestamp data.

#### Cluster Founder Count (Simple)
- **Implementation**: For each cluster, identifies earliest market by timestamp. Counts per platform.
- **Interpretation**: Number of topics each platform created. Direct measure of innovation leadership.
- **Output**: `clusters_founded`

#### Cluster Founder Count (Centrality-Weighted)
- **Implementation**: 
  1. Calculates cluster centroid and distance from first market to centroid
  2. Score = 1/(1 + distance_to_centroid) * log(cluster_size)
- **Interpretation**: Rewards being first AND becoming central to significant topics.
- **Output**: `clusters_founded_centrality_weighted`

#### Growth Catalyst Score
- **Implementation**:
  1. For clusters with >50 markets, identifies markets created within time window of first market
  2. Scores early markets by proximity to final centroid weighted by cluster persistence
  3. Sums scores per platform
- **Interpretation**: Identifies platforms that create early markets that become central to growing topics.
- **Variations**: `growth_catalyst_3d`, `growth_catalyst_7d`, `growth_catalyst_14d`, `growth_catalyst_30d`

#### Innovation Index
- **Implementation**: (clusters founded) / (total markets) * (average HDBSCAN persistence of founded clusters)
- **Interpretation**: Innovation efficiency - creating significant lasting topics relative to market volume.
- **Output**: `innovation_index`

#### Temporal Cluster Precedence
- **Implementation**:
  1. For each cluster, calculates temporal statistics per platform
  2. Counts clusters where platform enters systematically early
  3. Normalizes by clusters participated in
- **Interpretation**: Proportion of topics where platform enters systematically early (not just one-off first markets).
- **Variations**:
  - `temporal_precedence_first`: Based on first market
  - `temporal_precedence_median`: Based on median timestamp
  - `temporal_precedence_fifth`: Based on being in first 5 markets

### 4. Competition Metrics

These metrics analyze the competitive landscape and relationships between platforms.

#### Cross-Platform Topic Flow
- **Implementation**:
  1. For each cluster, identifies temporal order of platform entry
  2. Builds directed graph of platform→platform topic flows
  3. Calculates in-degree (receives topics) vs out-degree (originates topics)
- **Interpretation**: Shows which platforms are topic originators vs adopters in the ecosystem.
- **Outputs**:
  - `topic_flow_in_degree`: Number of topics received from other platforms
  - `topic_flow_out_degree`: Number of topics originated that others adopted
  - `topic_flow_ratio`: Out-degree / (In-degree + 1)

#### Platform Overlap Matrix
- **Implementation**: For each pair of platforms, calculates Jaccard similarity of their cluster participation.
- **Interpretation**: Shows which platforms compete in similar topic spaces vs occupy distinct niches.
- **Outputs**:
  - `overlap_matrix_weighted`: Weighted by market counts in each cluster
  - `overlap_with_{platform}_unweighted`: Unweighted overlap with each other platform

#### Topic Competition Intensity
- **Implementation**: For each cluster, calculates Herfindahl-Hirschman Index (HHI) of platform concentration.
- **Interpretation**: Lower HHI = more competitive topic, higher HHI = dominated by one platform.
- **Outputs**:
  - `cluster_hhi_scores`: List of HHI scores per cluster
  - `mean_hhi`: Average HHI across all clusters
  - `weighted_mean_hhi`: Market count-weighted average HHI

## Usage

### Running the Analysis

```bash
# Run main embedding analysis with metrics calculation
python embedding-analysis.py --cache-dir ./cache --output-dir ./output

# The script will generate:
# - output/platform_metrics.json: All calculated metrics
# - output/clusters_*.png: Cluster visualizations
# - output/clusters_*_interactive.html: Interactive visualizations
```

### Generating Reports

```bash
# Generate comprehensive HTML report from metrics
python generate_metrics_report.py --metrics-file ./output/platform_metrics.json --output-dir ./output

# This creates:
# - output/metrics_report.html: Comprehensive HTML report
# - output/platform_summary.csv: Summary table in CSV format
# - output/*.png: Various metric visualizations
```

### Testing

```bash
# Run test suite with synthetic data
python test_metrics.py

# This validates all metric calculations using synthetic test data
```

## Output Format

The metrics are saved as a JSON file with the following structure:

```json
{
  "diversity": {
    "platform_name": {
      "convex_hull_volume_full": float,
      "trimmed_mean_distance_80": float,
      "cluster_entropy": float,
      ...
    }
  },
  "novelty": {
    "platform_name": {
      "average_novelty_k20": float,
      "high_novelty_count_p95": int,
      ...
    }
  },
  "innovation": {
    "platform_name": {
      "clusters_founded": int,
      "innovation_index": float,
      ...
    }
  },
  "competition": {
    "platform_name": {
      "topic_flow_in_degree": int,
      "topic_flow_out_degree": int,
      ...
    },
    "overlap_matrix_weighted": {...},
    "cluster_hhi_scores": [...]
  },
  "platform_stats": {
    "platform_name": {
      "total_markets": int,
      "clustered_markets": int,
      "unique_clusters": int,
      "mean_novelty": float,
      "median_novelty": float
    }
  }
}
```

## Interpretation Guidelines

### Diversity Metrics
- **High entropy + high reach**: Platform covers many topics evenly
- **Low entropy + high Gini**: Platform is specialized in few topics
- **High exclusivity index**: Platform dominates multiple topics
- **High cross-platform isolation**: Platform operates in unique space

### Novelty Metrics
- **High average novelty**: Platform creates unusual markets overall
- **Many high-novelty markets**: Platform explores frontier topics
- **High LOF scores**: Platform has many statistical outliers
- **High novelty-weighted coverage**: Platform dominates sparse/novel areas

### Innovation Metrics
- **Many clusters founded**: Platform creates new topics
- **High centrality-weighted score**: Platform's new topics become important
- **High growth catalyst**: Platform sparks topic growth
- **High temporal precedence**: Platform systematically enters topics early

### Competition Metrics
- **High out-degree flow**: Platform originates topics others follow
- **Low overlap with others**: Platform occupies unique niche
- **Clusters with low HHI**: Platform operates in competitive spaces
- **Clusters with high HHI**: Platform dominates less competitive topics

## Notes and Assumptions

1. **Clustering**: Metrics assume HDBSCAN clustering has been performed. Outliers (cluster -1) are handled appropriately.

2. **Embeddings**: All distance calculations use normalized embeddings for cosine similarity.

3. **Temporal Metrics**: Innovation metrics require `created_time` or `created_datetime` fields.

4. **Minimum Data**: Most metrics require at least 3 markets per platform for meaningful results.

5. **Computational Complexity**: Some metrics (especially convex hull in high dimensions) may be computationally expensive for large datasets.

## Future Enhancements

Potential metrics that could be added:

1. **Topic Evolution**: Track how platforms' topic focus changes over time
2. **Market Success Correlation**: Correlate metrics with market resolution/volume
3. **Network Effects**: Analyze how platforms influence each other's topic choices
4. **Semantic Coherence**: Measure how semantically coherent each platform's portfolio is
5. **Trend Detection**: Identify platforms that consistently identify emerging trends

## Questions and Assumptions Made

During implementation, the following assumptions were made:

1. **Convex Hull**: Limited to cases where n_points > n_dimensions to avoid degenerate hulls
2. **Local Outlier Factor**: Used contamination=0.1 as default
3. **Growth Catalyst**: Only considers clusters with >50 markets as "significant"
4. **Temporal Precedence**: Uses median as default aggregation for platform entry times
5. **Cross-Platform Isolation**: Samples up to 100 markets per platform for efficiency
6. **Topic Flow**: Only records flows when multiple platforms participate in a cluster

All these parameters can be adjusted based on specific analysis needs.