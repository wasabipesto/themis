# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "dotenv",
#     "matplotlib",
#     "requests",
#     "numpy",
# ]
# ///

import os
import requests
import numpy as np
from dotenv import load_dotenv
import matplotlib.pyplot as plt
from collections import defaultdict

load_dotenv()
postgrest_url = os.environ.get('PGRST_URL')

all_market_scores = requests.get(f"{postgrest_url}/market_scores_details").json()
#linked_market_scores = requests.get(f"{postgrest_url}/market_scores_details?question_id=is.not.null").json()
#platform_category_scores = requests.get(f"{postgrest_url}/platform_category_scores_details").json()
#other_scores = requests.get(f"{postgrest_url}/other_scores").json()
scores = all_market_scores

# Group scores by score_type
scores_by_type = defaultdict(list)
for score_data in scores:
    if 'score_type' in score_data and 'score' in score_data:
        scores_by_type[score_data['score_type']].append(score_data['score'])

# Define percentile cutoffs for grades (for lower is better)
# We need to reverse the percentiles since lower scores are better
grade_percentiles = {
    'A': 10,  # 0-10 percentile (lowest 10% of scores)
    'B': 20,  # 10-20 percentile
    'C': 30,  # 20-30 percentile
    'D': 40,  # 30-40 percentile
    'F': 100  # above 40 percentile
}

# Create a histogram for each score_type with grade cutoffs
for score_type, scores_list in scores_by_type.items():
    plt.figure(figsize=(12, 7))

    # Calculate percentiles for grade cutoffs
    grade_cutoffs = {}
    for grade, percentile in grade_percentiles.items():
        grade_cutoffs[grade] = np.percentile(scores_list, percentile)

    # Create histogram
    n, bins, patches = plt.hist(scores_list, bins=50, alpha=0.7, color='skyblue', edgecolor='black')

    # Add grade cutoff lines
    colors = {'A': 'green', 'B': 'blue', 'C': 'purple', 'D': 'orange', 'F': 'red'}
    for grade, cutoff in grade_cutoffs.items():
        if grade != 'F':  # Skip F as it's the highest grade in our "lower is better" scale
            plt.axvline(cutoff, color=colors[grade], linestyle='dashed', linewidth=2,
                       label=f'Grade {grade} cutoff: {cutoff:.5f} ({grade_percentiles[grade]}th percentile)')

    # Add mean line
    mean_score = sum(scores_list)/len(scores_list)
    plt.axvline(mean_score, color='black', linestyle='solid', linewidth=2,
               label=f'Mean: {mean_score:.5f}')

    # Annotate grade regions (for lower is better)
    grade_regions = [
        (min(scores_list), grade_cutoffs['A'], 'A'),
        (grade_cutoffs['A'], grade_cutoffs['B'], 'B'),
        (grade_cutoffs['B'], grade_cutoffs['C'], 'C'),
        (grade_cutoffs['C'], grade_cutoffs['D'], 'D'),
        (grade_cutoffs['D'], max(scores_list), 'F')
    ]

    for min_val, max_val, grade in grade_regions:
        plt.annotate(f'Grade {grade}',
                    xy=((min_val + max_val)/2, max(n)/2),
                    color=colors.get(grade, 'black'),
                    fontsize=12,
                    weight='bold',
                    alpha=0.7)

    # Set title and labels
    plt.title(f'Distribution of Scores for {score_type} with Grade Cutoffs (Lower is Better)', fontsize=14)
    plt.xlabel('Score', fontsize=12)
    plt.ylabel('Frequency', fontsize=12)
    plt.grid(axis='y', alpha=0.75)

    plt.legend(loc='upper right')
    plt.tight_layout(rect=[0, 0.08, 1, 1])  # Adjust layout to make room for the table

    # Save the histogram
    plt.savefig(f'cache/{score_type}-histogram-with-grades.png')

# Print a summary of all grade cutoffs
print("Grade Cutoffs Summary (Lower is Better):")
for score_type in scores_by_type:
    scores_list = scores_by_type[score_type]
    print(f"\n{score_type}:")
    for grade in ['A', 'B', 'C', 'D']:
        cutoff = np.percentile(scores_list, grade_percentiles[grade])
        if grade == 'A':
            print(f"  Grade {grade}: ≤ {cutoff:.5f} (Bottom {grade_percentiles[grade]}%)")
        else:
            prev_grade = chr(ord(grade) - 1)  # Get previous grade letter
            prev_percentile = grade_percentiles[prev_grade]
            print(f"  Grade {grade}: ≤ {cutoff:.5f} ({prev_percentile}-{grade_percentiles[grade]}%)")
    print(f"  Grade F: > {np.percentile(scores_list, grade_percentiles['D']):.5f} (Above {grade_percentiles['D']}%)")
