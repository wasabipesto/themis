#!/usr/bin/env python3
"""
Generate comprehensive metrics report from platform analysis results.
Creates detailed HTML report with visualizations and tables.
"""

import argparse
import json
import os
from datetime import datetime
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import seaborn as sns
from jinja2 import Template

# Set plotting style
plt.style.use('seaborn-v0_8-darkgrid')
sns.set_palette("husl")


def load_metrics(metrics_file):
    """Load platform metrics from JSON file."""
    with open(metrics_file, 'r') as f:
        return json.load(f)


def create_diversity_plots(metrics, output_dir):
    """Create diversity metric visualizations."""
    platforms = list(metrics['platform_stats'].keys())

    # Create figure with subplots
    fig, axes = plt.subplots(2, 3, figsize=(15, 10))
    fig.suptitle('Platform Diversity Metrics', fontsize=16, fontweight='bold')

    # 1. Cluster Entropy
    entropy_data = [metrics['diversity'].get(p, {}).get('cluster_entropy', 0) for p in platforms]
    axes[0, 0].bar(platforms, entropy_data, color='steelblue')
    axes[0, 0].set_title('Cluster Diversity (Entropy)')
    axes[0, 0].set_ylabel('Entropy')
    axes[0, 0].tick_params(axis='x', rotation=45)

    # 2. Effective Topic Reach
    reach_10 = [metrics['diversity'].get(p, {}).get('effective_reach_10pct', 0) for p in platforms]
    reach_20 = [metrics['diversity'].get(p, {}).get('effective_reach_20pct', 0) for p in platforms]
    x = np.arange(len(platforms))
    width = 0.35
    axes[0, 1].bar(x - width/2, reach_10, width, label='10% threshold', color='lightcoral')
    axes[0, 1].bar(x + width/2, reach_20, width, label='20% threshold', color='darkred')
    axes[0, 1].set_title('Effective Topic Reach')
    axes[0, 1].set_ylabel('Number of Topics')
    axes[0, 1].set_xticks(x)
    axes[0, 1].set_xticklabels(platforms, rotation=45)
    axes[0, 1].legend()

    # 3. Trimmed Mean Distance
    distance_data = [metrics['diversity'].get(p, {}).get('trimmed_mean_distance_80', 0) for p in platforms]
    axes[0, 2].bar(platforms, distance_data, color='forestgreen')
    axes[0, 2].set_title('Internal Diversity (Trimmed Mean Distance)')
    axes[0, 2].set_ylabel('Distance')
    axes[0, 2].tick_params(axis='x', rotation=45)

    # 4. Topic Concentration (Gini)
    gini_data = [metrics['diversity'].get(p, {}).get('topic_gini_coefficient', 0) for p in platforms]
    axes[1, 0].bar(platforms, gini_data, color='purple')
    axes[1, 0].set_title('Topic Concentration (Gini Coefficient)')
    axes[1, 0].set_ylabel('Gini Coefficient')
    axes[1, 0].set_ylim(0, 1)
    axes[1, 0].tick_params(axis='x', rotation=45)

    # 5. Outlier Proportion
    outlier_prop = [metrics['diversity'].get(p, {}).get('outlier_proportion', 0) for p in platforms]
    axes[1, 1].bar(platforms, outlier_prop, color='orange')
    axes[1, 1].set_title('Outlier Proportion')
    axes[1, 1].set_ylabel('Proportion')
    axes[1, 1].tick_params(axis='x', rotation=45)

    # 6. Majority Clusters
    majority_75 = [metrics['diversity'].get(p, {}).get('majority_clusters_75pct', 0) for p in platforms]
    axes[1, 2].bar(platforms, majority_75, color='teal')
    axes[1, 2].set_title('Dominated Topics (>75% majority)')
    axes[1, 2].set_ylabel('Number of Clusters')
    axes[1, 2].tick_params(axis='x', rotation=45)

    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'diversity_metrics.png'), dpi=150, bbox_inches='tight')
    plt.close()

    return 'diversity_metrics.png'


def create_innovation_plots(metrics, output_dir):
    """Create innovation metric visualizations."""
    if 'innovation' not in metrics or not metrics['innovation']:
        return None

    platforms = list(metrics['platform_stats'].keys())

    fig, axes = plt.subplots(2, 2, figsize=(12, 10))
    fig.suptitle('Platform Innovation Metrics', fontsize=16, fontweight='bold')

    # 1. Clusters Founded
    founded = [metrics['innovation'].get(p, {}).get('clusters_founded', 0) for p in platforms]
    axes[0, 0].bar(platforms, founded, color='darkblue')
    axes[0, 0].set_title('Topics Created (Cluster Founder)')
    axes[0, 0].set_ylabel('Number of Topics')
    axes[0, 0].tick_params(axis='x', rotation=45)

    # 2. Innovation Index
    innovation_idx = [metrics['innovation'].get(p, {}).get('innovation_index', 0) for p in platforms]
    axes[0, 1].bar(platforms, innovation_idx, color='crimson')
    axes[0, 1].set_title('Innovation Index')
    axes[0, 1].set_ylabel('Index Value')
    axes[0, 1].tick_params(axis='x', rotation=45)

    # 3. Growth Catalyst Score
    catalyst_7d = [metrics['innovation'].get(p, {}).get('growth_catalyst_7d', 0) for p in platforms]
    axes[1, 0].bar(platforms, catalyst_7d, color='darkgreen')
    axes[1, 0].set_title('Growth Catalyst Score (7-day)')
    axes[1, 0].set_ylabel('Score')
    axes[1, 0].tick_params(axis='x', rotation=45)

    # 4. Temporal Precedence
    precedence = [metrics['innovation'].get(p, {}).get('temporal_precedence_median', 0) for p in platforms]
    axes[1, 1].bar(platforms, precedence, color='darkorange')
    axes[1, 1].set_title('Temporal Precedence (Median)')
    axes[1, 1].set_ylabel('Proportion')
    axes[1, 1].set_ylim(0, 1)
    axes[1, 1].tick_params(axis='x', rotation=45)

    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'innovation_metrics.png'), dpi=150, bbox_inches='tight')
    plt.close()

    return 'innovation_metrics.png'


def create_novelty_plots(metrics, output_dir):
    """Create novelty metric visualizations."""
    platforms = list(metrics['platform_stats'].keys())

    fig, axes = plt.subplots(1, 3, figsize=(15, 5))
    fig.suptitle('Platform Novelty Metrics', fontsize=16, fontweight='bold')

    # 1. Average Novelty
    avg_novelty = [metrics['novelty'].get(p, {}).get('average_novelty_k20', 0) for p in platforms]
    axes[0].bar(platforms, avg_novelty, color='indigo')
    axes[0].set_title('Average Novelty (k=20)')
    axes[0].set_ylabel('Novelty Score')
    axes[0].tick_params(axis='x', rotation=45)

    # 2. High Novelty Markets
    high_novelty = [metrics['novelty'].get(p, {}).get('high_novelty_count_p95', 0) for p in platforms]
    axes[1].bar(platforms, high_novelty, color='darkmagenta')
    axes[1].set_title('Highly Novel Markets (>95th percentile)')
    axes[1].set_ylabel('Count')
    axes[1].tick_params(axis='x', rotation=45)

    # 3. Local Outlier Factor
    lof_outliers = [metrics['novelty'].get(p, {}).get('lof_outliers_1.5', 0) for p in platforms]
    axes[2].bar(platforms, lof_outliers, color='brown')
    axes[2].set_title('Local Outliers (LOF > 1.5)')
    axes[2].set_ylabel('Count')
    axes[2].tick_params(axis='x', rotation=45)

    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'novelty_metrics.png'), dpi=150, bbox_inches='tight')
    plt.close()

    return 'novelty_metrics.png'


def create_competition_heatmap(metrics, output_dir):
    """Create platform overlap heatmap."""
    if 'overlap_matrix_weighted' not in metrics.get('competition', {}):
        return None

    overlap_matrix = metrics['competition']['overlap_matrix_weighted']
    platforms = list(overlap_matrix.keys())

    # Convert to numpy array
    matrix = np.zeros((len(platforms), len(platforms)))
    for i, p1 in enumerate(platforms):
        for j, p2 in enumerate(platforms):
            matrix[i, j] = overlap_matrix[p1].get(p2, 0)

    # Create heatmap
    fig, ax = plt.subplots(figsize=(10, 8))
    sns.heatmap(matrix, annot=True, fmt='.2f', cmap='coolwarm',
                xticklabels=platforms, yticklabels=platforms,
                vmin=0, vmax=1, cbar_kws={'label': 'Jaccard Similarity'})
    plt.title('Platform Topic Overlap (Weighted)', fontsize=14, fontweight='bold')
    plt.tight_layout()
    plt.savefig(os.path.join(output_dir, 'platform_overlap.png'), dpi=150, bbox_inches='tight')
    plt.close()

    return 'platform_overlap.png'


def generate_summary_table(metrics):
    """Generate comprehensive summary table."""
    platforms = list(metrics['platform_stats'].keys())

    summary_data = []
    for platform in platforms:
        row = {
            'Platform': platform,
            'Total Markets': metrics['platform_stats'][platform]['total_markets'],
            'Clustered Markets': metrics['platform_stats'][platform]['clustered_markets'],
            'Unique Clusters': metrics['platform_stats'][platform]['unique_clusters'],
            'Mean Novelty': f"{metrics['platform_stats'][platform].get('mean_novelty', 0):.3f}",
            'Cluster Entropy': f"{metrics['diversity'].get(platform, {}).get('cluster_entropy', 0):.2f}",
            'Topic Reach (10%)': metrics['diversity'].get(platform, {}).get('effective_reach_10pct', 0),
            'Majority Clusters (75%)': metrics['diversity'].get(platform, {}).get('majority_clusters_75pct', 0),
            'Gini Coefficient': f"{metrics['diversity'].get(platform, {}).get('topic_gini_coefficient', 0):.3f}",
            'Topics Founded': metrics['innovation'].get(platform, {}).get('clusters_founded', 0),
            'Innovation Index': f"{metrics['innovation'].get(platform, {}).get('innovation_index', 0):.3f}",
            'Avg Novelty (k=20)': f"{metrics['novelty'].get(platform, {}).get('average_novelty_k20', 0):.3f}",
            'High Novelty Markets': metrics['novelty'].get(platform, {}).get('high_novelty_count_p95', 0),
        }
        summary_data.append(row)

    return pd.DataFrame(summary_data)


def generate_html_report(metrics, plots, output_dir):
    """Generate HTML report from metrics and plots."""

    html_template = Template("""
<!DOCTYPE html>
<html>
<head>
    <title>Platform Metrics Analysis Report</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            margin: 40px;
            background-color: #f5f5f5;
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 10px;
        }
        h2 {
            color: #555;
            margin-top: 30px;
            border-bottom: 2px solid #ddd;
            padding-bottom: 5px;
        }
        .metric-section {
            background: white;
            padding: 20px;
            margin: 20px 0;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        table {
            border-collapse: collapse;
            width: 100%;
            margin: 20px 0;
        }
        th {
            background-color: #4CAF50;
            color: white;
            padding: 12px;
            text-align: left;
        }
        td {
            border: 1px solid #ddd;
            padding: 8px;
        }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .plot-container {
            text-align: center;
            margin: 30px 0;
        }
        .plot-container img {
            max-width: 100%;
            border: 1px solid #ddd;
            border-radius: 4px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        .timestamp {
            color: #888;
            font-style: italic;
            margin: 20px 0;
        }
        .key-insights {
            background-color: #e8f5e9;
            border-left: 5px solid #4CAF50;
            padding: 15px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <h1>Platform Metrics Analysis Report</h1>
    <p class="timestamp">Generated: {{ timestamp }}</p>

    <div class="metric-section">
        <h2>Executive Summary</h2>
        <div class="key-insights">
            <h3>Key Insights</h3>
            <ul>
                <li><strong>Total Platforms Analyzed:</strong> {{ n_platforms }}</li>
                <li><strong>Total Markets:</strong> {{ total_markets }}</li>
                <li><strong>Most Diverse Platform:</strong> {{ most_diverse }} (Entropy: {{ max_entropy }})</li>
                <li><strong>Most Innovative Platform:</strong> {{ most_innovative }} (Topics Founded: {{ max_founded }})</li>
                <li><strong>Highest Novelty Platform:</strong> {{ highest_novelty }} (Avg Novelty: {{ max_novelty }})</li>
            </ul>
        </div>
    </div>

    <div class="metric-section">
        <h2>Platform Summary Table</h2>
        {{ summary_table }}
    </div>

    <div class="metric-section">
        <h2>Diversity Metrics</h2>
        <p>These metrics measure how broadly each platform covers the topic space and how diverse their market portfolios are.</p>
        {% if diversity_plot %}
        <div class="plot-container">
            <img src="{{ diversity_plot }}" alt="Diversity Metrics">
        </div>
        {% endif %}
    </div>

    <div class="metric-section">
        <h2>Innovation Metrics</h2>
        <p>These metrics identify which platforms are creating new topics and leading market innovation.</p>
        {% if innovation_plot %}
        <div class="plot-container">
            <img src="{{ innovation_plot }}" alt="Innovation Metrics">
        </div>
        {% endif %}
    </div>

    <div class="metric-section">
        <h2>Novelty Metrics</h2>
        <p>These metrics measure how unique and unusual each platform's markets are compared to the overall market landscape.</p>
        {% if novelty_plot %}
        <div class="plot-container">
            <img src="{{ novelty_plot }}" alt="Novelty Metrics">
        </div>
        {% endif %}
    </div>

    <div class="metric-section">
        <h2>Competition Analysis</h2>
        <p>This heatmap shows the topic overlap between platforms, indicating which platforms compete in similar spaces.</p>
        {% if competition_plot %}
        <div class="plot-container">
            <img src="{{ competition_plot }}" alt="Platform Competition">
        </div>
        {% endif %}
    </div>

    <div class="metric-section">
        <h2>Detailed Metrics</h2>
        <details>
            <summary>Click to expand detailed metrics JSON</summary>
            <pre>{{ detailed_metrics }}</pre>
        </details>
    </div>
</body>
</html>
    """)

    # Calculate summary statistics
    platforms = list(metrics['platform_stats'].keys())

    # Find platforms with max values
    entropy_values = {p: metrics['diversity'].get(p, {}).get('cluster_entropy', 0) for p in platforms}
    most_diverse = max(entropy_values, key=entropy_values.get)

    founded_values = {p: metrics['innovation'].get(p, {}).get('clusters_founded', 0) for p in platforms}
    most_innovative = max(founded_values, key=founded_values.get) if founded_values else 'N/A'

    novelty_values = {p: metrics['novelty'].get(p, {}).get('average_novelty_k20', 0) for p in platforms}
    highest_novelty = max(novelty_values, key=novelty_values.get)

    total_markets = sum(metrics['platform_stats'][p]['total_markets'] for p in platforms)

    # Generate summary table
    summary_df = generate_summary_table(metrics)
    summary_html = summary_df.to_html(index=False, classes='summary-table')

    # Pretty print detailed metrics
    detailed_metrics = json.dumps(metrics, indent=2)

    # Render HTML
    html_content = html_template.render(
        timestamp=datetime.now().strftime('%Y-%m-%d %H:%M:%S'),
        n_platforms=len(platforms),
        total_markets=total_markets,
        most_diverse=most_diverse,
        max_entropy=f"{entropy_values[most_diverse]:.2f}",
        most_innovative=most_innovative,
        max_founded=founded_values.get(most_innovative, 0) if most_innovative != 'N/A' else 0,
        highest_novelty=highest_novelty,
        max_novelty=f"{novelty_values[highest_novelty]:.3f}",
        summary_table=summary_html,
        diversity_plot=plots.get('diversity'),
        innovation_plot=plots.get('innovation'),
        novelty_plot=plots.get('novelty'),
        competition_plot=plots.get('competition'),
        detailed_metrics=detailed_metrics
    )

    # Save report
    report_path = os.path.join(output_dir, 'metrics_report.html')
    with open(report_path, 'w') as f:
        f.write(html_content)

    return report_path


def main():
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
    print("Loading metrics...")
    metrics = load_metrics(args.metrics_file)

    # Generate plots
    print("Creating visualizations...")
    plots = {}
    plots['diversity'] = create_diversity_plots(metrics, args.output_dir)
    plots['innovation'] = create_innovation_plots(metrics, args.output_dir)
    plots['novelty'] = create_novelty_plots(metrics, args.output_dir)
    plots['competition'] = create_competition_heatmap(metrics, args.output_dir)

    # Generate HTML report
    print("Generating HTML report...")
    report_path = generate_html_report(metrics, plots, args.output_dir)

    print(f"\n✅ Report generated successfully: {report_path}")
    print(f"📊 Plots saved to: {args.output_dir}/")

    # Also save summary CSV
    summary_df = generate_summary_table(metrics)
    csv_path = os.path.join(args.output_dir, 'platform_summary.csv')
    summary_df.to_csv(csv_path, index=False)
    print(f"📄 Summary CSV saved: {csv_path}")


if __name__ == '__main__':
    main()
