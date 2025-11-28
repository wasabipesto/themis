#!/usr/bin/env python3
"""
Generate comprehensive metrics report from platform analysis results.
Creates detailed HTML report with visualizations, tables, and interpretations for each metric.
"""

import argparse
import json
import os
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any, Tuple

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import seaborn as sns
from jinja2 import Template

# Set plotting style
plt.style.use('seaborn-v0_8-whitegrid')
sns.set_palette("husl")
plt.rcParams['figure.dpi'] = 100
plt.rcParams['savefig.dpi'] = 300
plt.rcParams['font.size'] = 10


class MetricsReportGenerator:
    """Generate comprehensive metrics report with detailed analysis."""

    def __init__(self, metrics_data: Dict, output_dir: str):
        self.metrics_data = metrics_data
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        self.platforms = list(metrics_data['platform_stats'].keys())
        self.metric_descriptions = self._load_metric_descriptions()

    def _load_metric_descriptions(self) -> Dict:
        """Load metric descriptions and interpretations."""
        return {
            'diversity': {
                'convex_hull_volume_300d': {
                    'name': 'Convex Hull Volume (300D)',
                    'description': 'Measures the volume of the smallest convex polytope containing all platform embeddings in 300-dimensional space. The algorithm first reduces embedding dimensionality from 768D to 300D using Principal Component Analysis (PCA) to make computation tractable, then calculates the convex hull using the Quickhull algorithm. The convex hull represents the "envelope" containing all data points, and its volume quantifies the total semantic space occupied.',
                    'interpretation': 'Higher volume indicates broader topic coverage and more diverse market portfolio. This metric captures the overall "spread" of a platform\'s markets in semantic space. Unlike simple distance metrics, it accounts for the actual shape and extent of coverage. However, it can be inflated by a few outlier markets and doesn\'t distinguish between uniform coverage and clustering at extremes. It also loses information during PCA dimensionality reduction and may not capture local density variations within the hull.',
                    'variations': ['convex_hull_volume_300d']
                },
                'trimmed_mean_distance': {
                    'name': 'Trimmed Mean Pairwise Distance',
                    'description': 'Calculates all pairwise Euclidean distances between a platform\'s market embeddings using pdist, sorts these distances, then removes extreme values (top and bottom percentiles) before averaging. The trimmed_mean_distance_80 variant keeps the middle 80% of distances (removing top/bottom 10%), while trimmed_mean_distance_90 keeps the bottom 90% (removing only the top 10%). The mean_pairwise_distance includes all distances without trimming.',
                    'interpretation': 'Higher values indicate greater internal diversity - markets are more spread out in semantic space rather than clustered together. This metric is robust to outliers that might skew a simple mean. Unlike convex hull volume, it directly measures internal structure rather than external boundaries. The trimmed versions prevent a few very similar or very different market pairs from dominating the metric. However, it treats all pairwise relationships equally and doesn\'t capture clustering patterns or the actual distribution shape.',
                    'variations': ['trimmed_mean_distance_80', 'trimmed_mean_distance_90', 'mean_pairwise_distance']
                },
                'cluster_entropy': {
                    'name': 'Cluster Diversity Score (Entropy)',
                    'description': 'Applies Shannon entropy formula H = -Σ(p_i * log(p_i)) where p_i is the proportion of platform markets in cluster i. Shannon entropy originates from information theory and measures the "surprise" or unpredictability in the distribution. The weighted version (cluster_entropy_weighted) uses HDBSCAN membership probabilities to weight market contributions, accounting for uncertain cluster assignments near boundaries.',
                    'interpretation': 'Higher entropy indicates more even distribution across topic clusters - the platform doesn\'t concentrate in just a few areas. Maximum entropy occurs when markets are equally distributed across all clusters (log(n_clusters)). Zero entropy means all markets are in a single cluster. Unlike simple cluster counts, entropy accounts for the distribution balance. It differs from Gini coefficient by measuring evenness rather than inequality. However, it doesn\'t consider cluster sizes or semantic distances between clusters, treating all clusters as equally distinct.',
                    'variations': ['cluster_entropy', 'cluster_entropy_weighted']
                },
                'effective_reach': {
                    'name': 'Effective Topic Reach',
                    'description': 'Counts the number of distinct clusters where a platform has "meaningful" representation, using various thresholds. Percentage-based variants (5pct, 10pct, 20pct) count clusters where the platform has at least X% of that cluster\'s total markets. Count-based variants (1market, 5markets, 10markets) use absolute market counts as thresholds. This filters out clusters where the platform has only token presence.',
                    'interpretation': 'Measures breadth of meaningful participation across topic areas. Unlike raw cluster count, this metric filters out negligible presence that might be noise or accidental overlap. Higher thresholds provide increasingly conservative estimates of true topic coverage. The percentage variants normalize for cluster size, while count variants ensure minimum absolute presence. This metric complements entropy by focusing on breadth rather than balance, but doesn\'t capture the depth or dominance within reached clusters.',
                    'variations': ['effective_reach_5pct', 'effective_reach_10pct', 'effective_reach_20pct',
                                 'effective_reach_1market', 'effective_reach_5markets', 'effective_reach_10markets']
                },
                'majority_clusters': {
                    'name': 'Majority Cluster Count',
                    'description': 'Counts clusters where the platform holds a majority position, defined as having more than X% of all markets in that cluster. Calculated by iterating through all clusters, computing each platform\'s market share within each cluster, and counting clusters exceeding the threshold. Different thresholds (50%, 75%, 80%, 90%, 95%) provide increasingly strict definitions of "dominance".',
                    'interpretation': 'Identifies the number of topic areas a platform dominates or leads. This measures market leadership rather than just participation. Unlike effective reach which measures breadth, this focuses on depth and competitive advantage. Higher thresholds indicate stronger monopolistic positions. The metric reveals platform specialization patterns and competitive dynamics but doesn\'t account for cluster importance or size - dominating a 2-market cluster counts the same as dominating a 200-market cluster.',
                    'variations': ['majority_clusters_50pct', 'majority_clusters_75pct', 'majority_clusters_80pct',
                                 'majority_clusters_90pct', 'majority_clusters_95pct']
                },
                'unique_topic_proportion': {
                    'name': 'Unique Topic Proportion',
                    'description': 'Calculates the fraction of a platform\'s total markets that fall within clusters where that platform holds majority position (>X% market share). Computed as (count of platform markets in majority clusters) / (total platform markets). Links to majority_clusters metric but normalized by platform size.',
                    'interpretation': 'Measures the degree of platform specialization - what percentage of their portfolio is in areas they dominate. High values (approaching 1.0) indicate focused platforms that concentrate in their areas of strength. Low values indicate platforms that spread across many topics without dominating any. This differs from majority_clusters by accounting for platform size and revealing strategic focus. However, it may penalize large diverse platforms and doesn\'t distinguish between intentional specialization and limited reach.',
                    'variations': ['unique_topic_proportion_50pct', 'unique_topic_proportion_75pct', 'unique_topic_proportion_80pct',
                                 'unique_topic_proportion_90pct', 'unique_topic_proportion_95pct']
                },
                'cluster_exclusivity_index': {
                    'name': 'Cluster Exclusivity Index',
                    'description': 'Sums the platform\'s "excess share" across all clusters: Σ(max(0, p_i - threshold)) where p_i is platform proportion in cluster i. Only counts share exceeding the threshold (50%, 70%, or 80%). This aggregates dominance across all clusters rather than just counting dominated clusters.',
                    'interpretation': 'Cumulative measure of market power across all topics. Unlike majority_clusters which uses binary classification, this captures the degree of dominance. A platform with 60% share in 5 clusters scores higher than one with 51% share in 5 clusters at the 50% threshold. This rewards both the number of dominated clusters and the strength of dominance. However, it can be skewed by a few highly dominated small clusters and doesn\'t normalize for total market opportunity.',
                    'variations': ['cluster_exclusivity_index_50', 'cluster_exclusivity_index_70', 'cluster_exclusivity_index_80']
                },
                'topic_gini_coefficient': {
                    'name': 'Topic Concentration Coefficient (Gini)',
                    'description': 'Calculates the Gini coefficient for the distribution of platform markets across clusters. Uses the standard formula: G = (2 * Σ(i * x_i)) / (n * Σ(x_i)) - (n+1)/n where x_i are sorted cluster counts. The Gini coefficient, originally used for income inequality, measures how unequally markets are distributed across topics.',
                    'interpretation': 'Measures concentration vs. diversification strategy. Values near 0 indicate uniform distribution across topics (generalist strategy), while values near 1 indicate concentration in few topics (specialist strategy). Unlike entropy which measures disorder, Gini specifically quantifies inequality. A platform with markets evenly split between 2 clusters has different Gini than one with 90% in one cluster and 10% in another, though both might have similar entropy. This metric reveals strategic positioning but doesn\'t indicate whether concentration is in important or niche topics.',
                    'variations': ['topic_gini_coefficient']
                },
                'outlier_metrics': {
                    'name': 'Outlier Metrics',
                    'description': 'Counts and calculates the proportion of markets assigned to cluster -1 by HDBSCAN, which indicates points that don\'t belong to any cluster. These are markets that fall below the minimum cluster size or density thresholds. HDBSCAN assigns -1 to points in low-density regions that don\'t form stable clusters.',
                    'interpretation': 'Measures the uniqueness or unconventionality of a platform\'s portfolio. High outlier counts suggest the platform explores unusual or pioneering market areas that don\'t fit established categories. This could indicate innovation or niche targeting. However, outliers might also represent failed experiments or poorly-defined markets. Unlike novelty metrics, this specifically identifies markets that are so unique they don\'t cluster with others. The proportion variant normalizes for platform size.',
                    'variations': ['outlier_count', 'outlier_proportion']
                },
                'cross_platform_isolation': {
                    'name': 'Cross-Platform Isolation Score',
                    'description': 'For each platform market (sampling up to 100 for efficiency), finds the 10 nearest markets from OTHER platforms only, calculates the average distance to these neighbors, then averages across all sampled markets. Uses normalized embeddings and Euclidean distance. Explicitly excludes same-platform markets to measure inter-platform positioning.',
                    'interpretation': 'Measures how distinct a platform\'s markets are from competitors\' offerings. Higher scores indicate the platform occupies unique semantic space not explored by others - true differentiation. This differs from novelty metrics by explicitly comparing against competition rather than the global distribution. It reveals competitive positioning and market gaps. However, high isolation could indicate either innovative leadership or irrelevant market selection. The metric doesn\'t distinguish between being ahead of or behind the market.',
                    'variations': ['cross_platform_isolation']
                }
            },
            'novelty': {
                'average_novelty': {
                    'name': 'Average Novelty Score',
                    'description': 'Computes k-nearest neighbor distances for each market using sklearn\'s NearestNeighbors with Euclidean distance on normalized embeddings. For each market, finds its k nearest neighbors (k=10, 20, or 25), averages the distances to these neighbors (excluding self), then averages across all platform markets. The k+1 parameter accounts for self-exclusion.',
                    'interpretation': 'Measures the average "loneliness" or uniqueness of a platform\'s markets in semantic space. Higher values indicate markets that are generally far from their nearest neighbors - more unique or novel offerings. Unlike outlier detection, this provides a continuous novelty score for all markets. Different k values provide different locality perspectives: smaller k focuses on immediate neighborhood, larger k captures broader regional density. However, this treats all markets equally and doesn\'t identify which specific markets are most novel.',
                    'variations': ['average_novelty_k10', 'average_novelty_k20', 'average_novelty_k25']
                },
                'high_novelty_count': {
                    'name': 'High-Novelty Market Count',
                    'description': 'Identifies markets with k-NN distances exceeding global percentile thresholds. First calculates the distance to the 20th nearest neighbor for all markets globally, then determines percentile thresholds (80th, 90th, 95th, 98th percentiles), and counts platform markets exceeding these thresholds. Uses the same k-NN infrastructure as average novelty but focuses on the tail of the distribution.',
                    'interpretation': 'Counts truly unique or pioneering markets rather than averaging across all. These are markets in sparse regions of semantic space - potential innovations or underserved niches. Higher percentile thresholds identify increasingly exceptional markets. Unlike average novelty, this highlights the exceptional rather than the typical. The global percentile comparison ensures platform-independent standards. However, high novelty might indicate either innovation or irrelevance, and the metric doesn\'t distinguish between beneficial and detrimental uniqueness.',
                    'variations': ['high_novelty_count_p80', 'high_novelty_count_p90', 'high_novelty_count_p95', 'high_novelty_count_p98']
                },
                'lof_analysis': {
                    'name': 'Local Outlier Factor Analysis',
                    'description': 'Applies the Local Outlier Factor (LOF) algorithm with 20 neighbors to detect anomalies. LOF compares the local density around a point to the local densities of its neighbors. Points with substantially lower density than their neighbors receive high LOF scores (>1). The algorithm calculates reachability distances and local reachability densities, then computes the ratio of average neighbor density to point density. Thresholds of 1.5 and 2.0 identify mild and strong outliers.',
                    'interpretation': 'LOF identifies markets that are outliers relative to their local context, not just globally sparse regions. A market might be in a dense cluster but still be an outlier within that cluster. LOF scores >1 indicate lower density than neighbors, suggesting unusual positioning. This is more sophisticated than simple distance metrics as it accounts for varying density across the space. It can identify markets that break from local patterns even in dense regions. However, LOF is sensitive to the n_neighbors parameter and can miss outliers in uniformly sparse regions.',
                    'variations': ['lof_outliers_1.5', 'lof_outliers_2.0', 'mean_lof_score']
                },
                'novelty_weighted_coverage': {
                    'name': 'Novelty-Weighted Unique Coverage',
                    'description': 'Combines novelty and dominance metrics. First calculates local density as inverse of distance to 20th neighbor, identifies markets in the bottom 20% density (most novel), assigns 2x weight to these sparse-region markets, then sums weights for markets in majority clusters (>50% platform share). This rewards platforms that dominate in novel/sparse topic areas rather than crowded mainstream topics.',
                    'interpretation': 'Measures pioneering leadership - dominating topics that are themselves novel or underexplored. Higher scores indicate platforms that not only lead clusters but lead unusual or innovative clusters. This differs from simple majority cluster counts by rewarding difficulty and uniqueness. It identifies platforms creating new categories rather than competing in established ones. The metric synthesizes novelty and competition perspectives but may overweight small, obscure niches and doesn\'t account for market value or growth potential of novel areas.',
                    'variations': ['novelty_weighted_coverage']
                }
            },
            'innovation': {
                'clusters_founded': {
                    'name': 'Cluster Founder Count (Simple)',
                    'description': 'Identifies the first market in each cluster by timestamp (created_datetime), assigns cluster founding credit to that market\'s platform, then counts total clusters founded per platform. Uses simple temporal ordering without considering cluster characteristics. Only counts clusters with valid timestamps.',
                    'interpretation': 'Direct measure of topic creation and innovation leadership. Platforms with high founder counts are market creators rather than followers. This metric identifies who introduces new categories to the ecosystem. Unlike participation metrics, this specifically rewards being first. However, it treats all clusters equally regardless of size or importance, doesn\'t account for near-simultaneous creation by multiple platforms, and may credit random early outliers rather than intentional innovation.',
                    'variations': ['clusters_founded']
                },
                'clusters_founded_centrality': {
                    'name': 'Cluster Founder Count (Centrality-Weighted)',
                    'description': 'Enhances basic founder count by weighting each founded cluster by (1/(1 + distance_to_centroid)) * log(cluster_size). Distance to centroid measures how representative the first market is of the eventual cluster. The log(cluster_size) term gives more weight to founding large clusters. Combines temporal precedence with cluster significance and founder representativeness.',
                    'interpretation': 'Rewards founding important, coherent topics where the initial market accurately represents the category\'s eventual center. High scores indicate platforms that don\'t just create random first markets but establish meaningful new categories that others follow. This differs from simple founder count by filtering out accidental or peripheral founding. It identifies true innovation leaders whose early markets define new spaces. However, it may undervalue radical innovations that only later find their center.',
                    'variations': ['clusters_founded_centrality_weighted']
                },
                'growth_catalyst': {
                    'name': 'Growth Catalyst Score',
                    'description': 'For large clusters (>50 markets), identifies platform markets created within time windows (3, 7, 14, 30 days) of cluster founding, scores each by proximity to cluster centroid (1/(1+distance)), weights by cluster persistence (HDBSCAN stability), and sums across all large clusters. Measures early participation in eventually successful topics.',
                    'interpretation': 'Identifies platforms that recognize and rapidly develop promising new topics. High scores indicate platforms with good market instincts - they quickly identify and invest in areas that become important. Unlike founder metrics, this rewards fast following and early development. Different time windows capture different reaction speeds. The centroid proximity ensures early markets are relevant, not peripheral. However, the 50-market threshold may exclude important niche topics, and the metric doesn\'t distinguish between correlation and causation in growth.',
                    'variations': ['growth_catalyst_3d', 'growth_catalyst_7d', 'growth_catalyst_14d', 'growth_catalyst_30d']
                },
                'innovation_index': {
                    'name': 'Innovation Index',
                    'description': 'Composite metric: (clusters_founded / total_markets) * average_persistence_of_founded_clusters. The first term is innovation rate (founding per market created), the second is average HDBSCAN persistence (cluster stability/significance) of founded clusters. Multiplying them rewards quality over quantity in innovation.',
                    'interpretation': 'Measures innovation efficiency - creating significant, lasting topics relative to total market volume. High values indicate platforms that consistently create important new categories rather than many markets in existing categories. This normalizes for platform size and rewards founding stable, persistent clusters over ephemeral ones. It differs from raw founder count by penalizing platforms that create many markets but few new topics. However, persistence may not capture all forms of value, and the metric may undervalue platforms in rapidly evolving spaces.',
                    'variations': ['innovation_index']
                },
                'temporal_precedence': {
                    'name': 'Temporal Cluster Precedence',
                    'description': 'Analyzes temporal entry patterns across clusters. For each cluster, calculates entry statistics (first market, median market time, first 5 markets) per platform. Then counts clusters where the platform systematically enters early: temporal_precedence_first (first by minimum time), temporal_precedence_median (first by median time), temporal_precedence_fifth (appears in first 5 markets). Normalized by participated clusters.',
                    'interpretation': 'Measures systematic early adoption rather than one-off founding. High precedence indicates platforms that consistently recognize and enter new topics early. The median variant is robust to outliers, the first variant rewards any early entry, and the fifth variant captures rapid early participation. This differs from founder metrics by including fast-following and measures consistent behavior across many topics. However, early entry might indicate either innovation or low barriers to entry, and the metric doesn\'t account for entry success or longevity.',
                    'variations': ['temporal_precedence_first', 'temporal_precedence_median', 'temporal_precedence_fifth']
                }
            },
            'competition': {
                'topic_flow': {
                    'name': 'Cross-Platform Topic Flow',
                    'description': 'Constructs a directed graph of topic adoption by tracking platform entry order in each cluster. For clusters with multiple platforms, sorts platforms by entry time and creates directed edges from earlier to later entrants. Aggregates across all clusters to build a platform-to-platform flow network. Calculates in-degree (topics adopted from others), out-degree (topics given to others), and their ratio.',
                    'interpretation': 'Reveals innovation and imitation patterns in the ecosystem. High out-degree indicates innovation leaders whose topics are copied. High in-degree indicates fast followers who adopt others\' innovations. The ratio (out/in) distinguishes net innovators from net imitators. This network perspective shows competitive dynamics and knowledge flows. Unlike static overlap metrics, this captures temporal causality. However, temporal correlation doesn\'t prove influence, and the metric doesn\'t account for independent discovery or external factors driving similar timing.',
                    'variations': ['topic_flow_in_degree', 'topic_flow_out_degree', 'topic_flow_ratio']
                },
                'overlap_matrix': {
                    'name': 'Platform Overlap Matrix',
                    'description': 'Calculates pairwise Jaccard similarity between platforms based on cluster participation. For each platform pair, computes intersection and union of participated clusters. The unweighted version uses binary participation, while the weighted version uses min(count_A, count_B) for intersection and max(count_A, count_B) for union, accounting for participation depth. Creates a symmetric similarity matrix.',
                    'interpretation': 'Quantifies competitive overlap and strategic similarity between platforms. High similarity indicates direct competition in similar topic spaces. Low similarity suggests platforms occupy distinct niches. The weighted version accounts for participation intensity, not just presence. This reveals the competitive landscape and potential partnership or acquisition targets. Unlike topic flow, this is symmetric and doesn\'t indicate direction of influence. However, cluster participation overlap doesn\'t necessarily mean direct market competition if platforms serve different customer segments.',
                    'variations': ['overlap_with_*_unweighted', 'overlap_matrix_weighted']
                },
                'competition_intensity': {
                    'name': 'Topic Competition Intensity',
                    'description': 'Calculates the Herfindahl-Hirschman Index (HHI) for each cluster to measure market concentration. HHI = Σ(s_i²) where s_i is each platform\'s market share in the cluster. Ranges from near 0 (perfect competition) to 10,000 (monopoly, if using percentages). The implementation squares each platform\'s proportion and sums them. Also computes mean HHI across all clusters and weighted mean using cluster sizes as weights.',
                    'interpretation': 'Measures competitive dynamics within each topic. Lower HHI (<0.15 or <1500 if using percentages) indicates highly competitive topics with many platforms. Higher HHI (>0.25 or >2500) indicates concentrated topics dominated by few platforms. This reveals which topics are open battlegrounds vs. established territories. Unlike overlap metrics that show who competes, HHI quantifies competition intensity. The weighted mean better represents overall ecosystem competition by accounting for topic importance. However, HHI doesn\'t capture potential competition or barriers to entry, and treats all platforms as equally capable competitors.',
                    'variations': ['cluster_hhi_scores', 'mean_hhi', 'weighted_mean_hhi']
                }
            }
        }

    def create_metric_visualization(self, category: str, metric_group: str, metric_info: Dict) -> str:
        """Create visualization for a specific metric group."""
        fig_width = 15 if len(metric_info['variations']) > 3 else 12
        fig_height = 8 if len(metric_info['variations']) > 4 else 6

        n_variations = len(metric_info['variations'])
        n_cols = min(3, n_variations)
        n_rows = (n_variations + n_cols - 1) // n_cols

        fig, axes = plt.subplots(n_rows, n_cols, figsize=(fig_width, fig_height))
        # Convert axes to numpy array and reshape for consistent indexing
        axes = np.array(axes).reshape(n_rows, n_cols) if n_variations > 1 else np.array([axes])

        fig.suptitle(f'{metric_info["name"]} - {category.title()} Metrics', fontsize=16, fontweight='bold')

        for i, variation in enumerate(metric_info['variations']):
            if n_variations == 1:
                ax = axes.flat[0]
            else:
                row, col = divmod(i, n_cols)
                ax = axes.flat[row * n_cols + col]

            # Get data for this variation
            if category == 'competition' and 'overlap_with_' in variation:
                # Handle overlap metrics specially
                data = []
                for platform in self.platforms:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)
                    data.append(value)
            elif category == 'competition' and variation == 'overlap_matrix_weighted':
                # Create heatmap for overlap matrix
                overlap_data = self.metrics_data[category][variation]
                overlap_df = pd.DataFrame(overlap_data)
                sns.heatmap(overlap_df, annot=True, fmt='.3f', ax=ax, cmap='Blues')
                ax.set_title(f'{variation.replace("_", " ").title()}')
                continue
            elif category == 'competition' and 'hhi' in variation:
                if variation == 'cluster_hhi_scores':
                    # Show distribution of HHI scores
                    hhi_scores = [item['hhi'] for item in self.metrics_data[category][variation]]
                    ax.hist(hhi_scores, bins=30, alpha=0.7, color='skyblue', edgecolor='black')
                    ax.set_title('Distribution of Cluster HHI Scores')
                    ax.set_xlabel('HHI Score')
                    ax.set_ylabel('Frequency')
                    continue
                else:
                    # mean_hhi or weighted_mean_hhi
                    data = [self.metrics_data[category][variation]] * len(self.platforms)
                    ax.bar(['Overall'], [self.metrics_data[category][variation]], color='coral')
                    ax.set_title(f'{variation.replace("_", " ").title()}')
                    continue
            else:
                data = []
                for platform in self.platforms:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)
                    data.append(value)

            # Create bar plot
            bars = ax.bar(self.platforms, data, color=plt.cm.Set3(np.linspace(0, 1, len(self.platforms))))
            ax.set_title(f'{variation.replace("_", " ").title()}')
            ax.tick_params(axis='x', rotation=45)

            # Add value labels on bars
            for bar, value in zip(bars, data):
                height = bar.get_height()
                ax.annotate(f'{value:.3f}' if isinstance(value, float) else str(value),
                          xy=(bar.get_x() + bar.get_width() / 2, height),
                          xytext=(0, 3),  # 3 points vertical offset
                          textcoords="offset points",
                          ha='center', va='bottom', fontsize=8)

        # Hide empty subplots
        for i in range(n_variations, n_rows * n_cols):
            if i < len(axes.flat):
                axes.flat[i].set_visible(False)

        plt.tight_layout()

        # Save plot
        filename = f"{category}_{metric_group}_metrics.png"
        filepath = self.output_dir / filename
        plt.savefig(filepath, dpi=300, bbox_inches='tight')
        plt.close()

        return filename

    def create_metric_table(self, category: str, metric_group: str, metric_info: Dict) -> str:
        """Create data table for a specific metric group."""
        table_data = []

        for platform in self.platforms:
            row = {'Platform': platform}

            for variation in metric_info['variations']:
                if category == 'competition' and 'overlap_with_' in variation:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)
                elif category == 'competition' and variation in ['mean_hhi', 'weighted_mean_hhi']:
                    value = self.metrics_data[category][variation]
                elif category == 'competition' and variation == 'cluster_hhi_scores':
                    # Skip this one for platform-specific table
                    continue
                else:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)

                row[variation.replace('_', ' ').title()] = f"{value:.4f}" if isinstance(value, float) else str(value)

            table_data.append(row)

        if not table_data or len(table_data[0]) <= 1:
            return "<p>No tabular data available for this metric.</p>"

        df = pd.DataFrame(table_data)
        return df.to_html(classes='metrics-table', table_id=f'{category}-{metric_group}-table', escape=False, index=False)

    def generate_metric_section(self, category: str, metric_group: str, metric_info: Dict) -> Dict:
        """Generate complete section for a metric group."""
        # Create visualization
        plot_filename = self.create_metric_visualization(category, metric_group, metric_info)

        # Create data table
        table_html = self.create_metric_table(category, metric_group, metric_info)

        # Generate interpretation
        interpretation = self._generate_interpretation(category, metric_group, metric_info)

        return {
            'name': metric_info['name'],
            'description': metric_info['description'],
            'interpretation': metric_info['interpretation'],
            'plot_filename': plot_filename,
            'table_html': table_html,
            'detailed_interpretation': interpretation,
            'variations': metric_info['variations']
        }

    def _generate_interpretation(self, category: str, metric_group: str, metric_info: Dict) -> str:
        """Generate detailed interpretation based on actual data."""
        interpretations = []

        # Get top performers for each variation
        for variation in metric_info['variations']:
            if category == 'competition' and variation == 'cluster_hhi_scores':
                continue
            elif category == 'competition' and variation in ['mean_hhi', 'weighted_mean_hhi']:
                value = self.metrics_data[category][variation]
                interpretations.append(f"**{variation.replace('_', ' ').title()}**: {value:.4f}")
                continue

            platform_values = []
            for platform in self.platforms:
                if category == 'competition' and 'overlap_with_' in variation:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)
                else:
                    value = self.metrics_data[category].get(platform, {}).get(variation, 0)
                platform_values.append((platform, value))

            # Sort by value (descending for most metrics, ascending for some)
            reverse_sort = True
            if 'gini' in variation.lower() or 'hhi' in variation.lower():
                reverse_sort = False  # Lower is more diverse

            platform_values.sort(key=lambda x: x[1], reverse=reverse_sort)

            if platform_values:
                top_platform, top_value = platform_values[0]
                interpretations.append(
                    f"**{variation.replace('_', ' ').title()}**: {top_platform} leads with {top_value:.4f}"
                )

        return " | ".join(interpretations)

    def generate_category_summary(self, category: str) -> Dict:
        """Generate summary for an entire category."""
        category_data = self.metrics_data[category]
        summary = {
            'category': category.title(),
            'description': self._get_category_description(category),
            'platform_rankings': self._calculate_category_rankings(category)
        }
        return summary

    def _get_category_description(self, category: str) -> str:
        """Get description for a metric category."""
        descriptions = {
            'diversity': 'These metrics measure how broadly each platform covers the topic space and the diversity of their market portfolios.',
            'novelty': 'These metrics measure how unique and unusual each platform\'s markets are compared to the overall market landscape.',
            'innovation': 'These metrics identify which platforms are creating new topics and leading market innovation. Requires timestamp data.',
            'competition': 'These metrics analyze the competitive landscape and relationships between platforms.'
        }
        return descriptions.get(category, '')

    def _calculate_category_rankings(self, category: str) -> List[Tuple[str, float]]:
        """Calculate overall ranking for platforms in a category."""
        if category not in self.metrics_data:
            return []

        # Simple approach: average normalized scores across key metrics
        platform_scores = {platform: [] for platform in self.platforms}

        # Get key metrics for each category
        key_metrics = {
            'diversity': ['cluster_entropy', 'effective_reach_20pct', 'topic_gini_coefficient'],
            'novelty': ['average_novelty_k20', 'high_novelty_count_p95'],
            'innovation': ['clusters_founded', 'innovation_index'],
            'competition': ['topic_flow_ratio']
        }

        metrics_to_use = key_metrics.get(category, [])

        for metric in metrics_to_use:
            values = []
            for platform in self.platforms:
                value = self.metrics_data[category].get(platform, {}).get(metric, 0)
                values.append(value)

            # Normalize values (0-1 scale)
            if values and max(values) > min(values):
                min_val, max_val = min(values), max(values)
                for i, platform in enumerate(self.platforms):
                    normalized = (values[i] - min_val) / (max_val - min_val)

                    # For Gini coefficient, lower is better (more diverse)
                    if 'gini' in metric.lower():
                        normalized = 1 - normalized

                    platform_scores[platform].append(normalized)

        # Calculate average scores
        rankings = []
        for platform in self.platforms:
            if platform_scores[platform]:
                avg_score = np.mean(platform_scores[platform])
                rankings.append((platform, avg_score))

        rankings.sort(key=lambda x: x[1], reverse=True)
        return rankings

    def generate_report(self) -> str:
        """Generate complete HTML report."""
        report_sections = {}

        # Generate sections for each category
        for category, metrics in self.metric_descriptions.items():
            category_sections = {}

            for metric_group, metric_info in metrics.items():
                section = self.generate_metric_section(category, metric_group, metric_info)
                category_sections[metric_group] = section

            category_summary = self.generate_category_summary(category)
            report_sections[category] = {
                'summary': category_summary,
                'metrics': category_sections
            }

        # Generate platform stats summary
        platform_stats = self._generate_platform_stats_summary()

        # Create HTML report
        html_content = self._generate_html_report(report_sections, platform_stats)

        return html_content

    def _generate_platform_stats_summary(self) -> Dict:
        """Generate summary of platform statistics."""
        stats = {}
        for platform in self.platforms:
            platform_data = self.metrics_data['platform_stats'][platform]
            stats[platform] = {
                'total_markets': platform_data.get('total_markets', 0),
                'clustered_markets': platform_data.get('clustered_markets', 0),
                'unique_clusters': platform_data.get('unique_clusters', 0),
                'mean_novelty': platform_data.get('mean_novelty', 0),
                'median_novelty': platform_data.get('median_novelty', 0)
            }
        return stats

    def _generate_html_report(self, report_sections: Dict, platform_stats: Dict) -> str:
        """Generate complete HTML report."""
        html_template = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Platform Metrics Comprehensive Report</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background-color: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 0 20px rgba(0,0,0,0.1);
        }
        h1 {
            color: #2c3e50;
            text-align: center;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }
        h2 {
            color: #34495e;
            border-left: 4px solid #3498db;
            padding-left: 15px;
            margin-top: 30px;
        }
        h3 {
            color: #2980b9;
            margin-top: 25px;
        }
        .metric-section {
            margin: 30px 0;
            padding: 20px;
            background-color: #f8f9fa;
            border-radius: 8px;
            border-left: 4px solid #3498db;
        }
        .metric-description {
            background-color: #e3f2fd;
            padding: 15px;
            border-radius: 5px;
            margin: 10px 0;
        }
        .metric-interpretation {
            background-color: #f1f8e9;
            padding: 15px;
            border-radius: 5px;
            margin: 10px 0;
        }
        .plot-container {
            text-align: center;
            margin: 20px 0;
        }
        .plot-container img {
            max-width: 100%;
            height: auto;
            border: 1px solid #ddd;
            border-radius: 5px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .metrics-table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            font-size: 14px;
        }
        .metrics-table th, .metrics-table td {
            border: 1px solid #ddd;
            padding: 12px;
            text-align: center;
        }
        .metrics-table th {
            background-color: #3498db;
            color: white;
            font-weight: bold;
        }
        .metrics-table tr:nth-child(even) {
            background-color: #f2f2f2;
        }
        .metrics-table tr:hover {
            background-color: #e8f4fd;
        }
        .category-summary {
            background-color: #fff3e0;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
            border: 2px solid #ff9800;
        }
        .platform-stats {
            background-color: #e8f5e8;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
        }
        .rankings {
            display: flex;
            flex-wrap: wrap;
            gap: 10px;
            margin: 10px 0;
        }
        .ranking-item {
            background-color: #3498db;
            color: white;
            padding: 5px 10px;
            border-radius: 20px;
            font-size: 14px;
        }
        .toc {
            background-color: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
            margin: 20px 0;
        }
        .toc ul {
            list-style-type: none;
            padding-left: 0;
        }
        .toc li {
            margin: 5px 0;
        }
        .toc a {
            text-decoration: none;
            color: #3498db;
            font-weight: 500;
        }
        .toc a:hover {
            text-decoration: underline;
        }
        .detailed-interpretation {
            font-style: italic;
            color: #555;
            margin-top: 10px;
        }
        .timestamp {
            text-align: center;
            color: #7f8c8d;
            font-style: italic;
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #ecf0f1;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Platform Metrics Comprehensive Analysis Report</h1>

        <div class="toc">
            <h3>Table of Contents</h3>
            <ul>
                <li><a href="#platform-overview">Platform Overview</a></li>
                {% for category_name, category_data in report_sections.items() %}
                <li><a href="#{{ category_name }}">{{ category_name.title() }} Metrics</a>
                    <ul>
                        {% for metric_name in category_data.metrics.keys() %}
                        <li><a href="#{{ category_name }}-{{ metric_name }}">{{ category_data.metrics[metric_name].name }}</a></li>
                        {% endfor %}
                    </ul>
                </li>
                {% endfor %}
            </ul>
        </div>

        <div id="platform-overview" class="platform-stats">
            <h2>Platform Overview</h2>
            <p>This report analyzes {{ platforms|length }} platforms: <strong>{{ platforms|join(', ') }}</strong></p>

            <h3>Basic Statistics</h3>
            <table class="metrics-table">
                <thead>
                    <tr>
                        <th>Platform</th>
                        <th>Total Markets</th>
                        <th>Clustered Markets</th>
                        <th>Unique Clusters</th>
                        <th>Mean Novelty</th>
                        <th>Median Novelty</th>
                    </tr>
                </thead>
                <tbody>
                    {% for platform, stats in platform_stats.items() %}
                    <tr>
                        <td><strong>{{ platform }}</strong></td>
                        <td>{{ stats.total_markets }}</td>
                        <td>{{ stats.clustered_markets }}</td>
                        <td>{{ stats.unique_clusters }}</td>
                        <td>{{ "%.4f"|format(stats.mean_novelty) }}</td>
                        <td>{{ "%.4f"|format(stats.median_novelty) }}</td>
                    </tr>
                    {% endfor %}
                </tbody>
            </table>
        </div>

        {% for category_name, category_data in report_sections.items() %}
        <div id="{{ category_name }}">
            <h2>{{ category_name.title() }} Metrics</h2>

            <div class="category-summary">
                <h3>Category Overview</h3>
                <p>{{ category_data.summary.description }}</p>

                {% if category_data.summary.platform_rankings %}
                <h4>Platform Rankings for {{ category_name.title() }}</h4>
                <div class="rankings">
                    {% for platform, score in category_data.summary.platform_rankings %}
                    <div class="ranking-item">{{ loop.index }}. {{ platform }} ({{ "%.3f"|format(score) }})</div>
                    {% endfor %}
                </div>
                {% endif %}
            </div>

            {% for metric_name, metric_data in category_data.metrics.items() %}
            <div id="{{ category_name }}-{{ metric_name }}" class="metric-section">
                <h3>{{ metric_data.name }}</h3>

                <div class="metric-description">
                    <h4>Description</h4>
                    <p>{{ metric_data.description }}</p>
                </div>

                <div class="metric-interpretation">
                    <h4>How to Interpret</h4>
                    <p>{{ metric_data.interpretation }}</p>
                </div>

                <div class="plot-container">
                    <img src="{{ metric_data.plot_filename }}" alt="{{ metric_data.name }} Visualization">
                </div>

                <h4>Raw Data</h4>
                {{ metric_data.table_html|safe }}

                {% if metric_data.detailed_interpretation %}
                <div class="detailed-interpretation">
                    <h4>Results Summary</h4>
                    <p>{{ metric_data.detailed_interpretation }}</p>
                </div>
                {% endif %}

                <h4>Variations Analyzed</h4>
                <ul>
                    {% for variation in metric_data.variations %}
                    <li><code>{{ variation }}</code></li>
                    {% endfor %}
                </ul>
            </div>
            {% endfor %}
        </div>
        {% endfor %}

        <div class="timestamp">
            Generated on {{ timestamp }} | Total Platforms: {{ platforms|length }} |
            Total Markets: {{ total_markets }}
        </div>
    </div>
</body>
</html>
        """

        from jinja2 import Template
        template = Template(html_template)

        # Calculate total markets
        total_markets = sum(stats['total_markets'] for stats in platform_stats.values())

        html_content = template.render(
            report_sections=report_sections,
            platform_stats=platform_stats,
            platforms=self.platforms,
            total_markets=total_markets,
            timestamp=datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        )

        # Save report
        report_path = self.output_dir / 'comprehensive_metrics_report.html'
        with open(report_path, 'w', encoding='utf-8') as f:
            f.write(html_content)

        return str(report_path)


def load_metrics(metrics_file: str) -> Dict:
    """Load platform metrics from JSON file."""
    with open(metrics_file, 'r') as f:
        return json.load(f)


def main():
    """Main function to generate comprehensive metrics report."""
    parser = argparse.ArgumentParser(description='Generate comprehensive metrics report')
    parser.add_argument('--metrics-file', '-m',
                       default='./output/platform_metrics.json',
                       help='Path to platform metrics JSON file')
    parser.add_argument('--output-dir', '-o',
                       default='./output',
                       help='Output directory for report and plots')
    args = parser.parse_args()

    # Create output directory if needed
    os.makedirs(args.output_dir, exist_ok=True)

    # Load metrics
    print("Loading metrics data...")
    try:
        metrics_data = load_metrics(args.metrics_file)
    except Exception as e:
        print(f"Error loading metrics file: {e}")
        return

    # Generate comprehensive report
    print("Generating comprehensive metrics report...")
    generator = MetricsReportGenerator(metrics_data, args.output_dir)

    try:
        report_path = generator.generate_report()
        print(f"\n✅ Comprehensive report generated successfully!")
        print(f"📄 HTML Report: {report_path}")
        print(f"📊 Visualizations saved to: {args.output_dir}/")
        print(f"\n📋 Report includes:")
        print("   • Detailed analysis of each metric category")
        print("   • Visualizations for all metric variations")
        print("   • Data tables with raw numbers")
        print("   • Interpretations and explanations")
        print("   • Platform rankings per category")

    except Exception as e:
        print(f"❌ Error generating report: {e}")
        import traceback
        traceback.print_exc()


if __name__ == '__main__':
    main()
