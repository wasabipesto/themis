// Takes an array of numbers and a percentile value
// Returns the value at that percentile of the array

import type { MarketScoreDetails, PlatformDetails } from "@types";

// Assumes the scores are pre-sorted
export function percentile(arr: number[], percentile: number): number {
  const index = percentile * (arr.length - 1);

  if (Number.isInteger(index)) {
    return arr[index];
  } else {
    // interpolate to get percentile between points
    const lowerIndex = Math.floor(index);
    const upperIndex = Math.ceil(index);
    return (
      arr[lowerIndex] +
      (index - lowerIndex) * (arr[upperIndex] - arr[lowerIndex])
    );
  }
}

// Takes an array of numbers and returns some stats
// Assumes the scores are pre-sorted
export function quartiles(arr: number[]): {
  min: number;
  c1: number;
  q1: number;
  q2: number;
  q3: number;
  c3: number;
  c9: number;
  max: number;
  iqr: number;
} {
  const min = arr[0];
  const q1 = percentile(arr, 0.25);
  const q2 = percentile(arr, 0.5);
  const q3 = percentile(arr, 0.75);
  const max = arr[arr.length - 1];
  const iqr = q3 - q1;
  const c1 = Math.max(q1 - 1.5 * iqr, min);
  const c3 = Math.min(q3 + 1.5 * iqr, max);
  const c9 = percentile(arr, 0.975);
  return {
    min,
    c1,
    q1,
    q2,
    q3,
    c3,
    c9,
    max,
    iqr,
  };
}

// Takes an array of scores and generates key points per platform
export function quartilesByPlatform(
  platforms: PlatformDetails[],
  scores: MarketScoreDetails[],
): {
  platform_name: string;
  min: number;
  c1: number;
  q1: number;
  q2: number;
  q3: number;
  c3: number;
  max: number;
}[] {
  return platforms.map((p) => {
    const platformScores = scores
      .filter((s) => s.platform_slug == p.slug)
      .map((s) => s.score);
    const stats = quartiles(platformScores);
    return {
      platform_name: p.name,
      min: stats.min,
      c1: stats.c1,
      q1: stats.q1,
      q2: stats.q2,
      q3: stats.q3,
      c3: stats.c3,
      max: stats.max,
    };
  });
}

// Takes an array of numbers and sorts them
export function sort(arr: number[]): number[] {
  return arr.sort((a, b) => a - b);
}

// Rounds a number to n significant figures
// Only necessary to work with numbers > 1
export function roundSF(num: number, sigfigs: number): number {
  const magnitude = Math.floor(Math.log10(Math.abs(num)));
  const scale = Math.pow(10, sigfigs - magnitude - 1);
  const roundedNum = Math.round(num * scale) / scale;

  return roundedNum;
}

// Get some stats around market scores for the index page
export function getScoreStats(
  scores: MarketScoreDetails[],
  scoreType: string,
  scoreCutoffMin: number | null,
  scoreCutoffMax: number | null,
): {
  numMatchingType: number;
  numMatchingCutoff: number;
  medianScore: number;
} {
  const scoresMatchingType = scores.filter((s) => s.score_type == scoreType);
  const numMatchingType = scoresMatchingType.length;
  if (numMatchingType === 0) {
    throw new Error(`No scores matched filter for ${scoreType}`);
  }

  const scoresMatchingCutoff = scoresMatchingType.filter(
    (s) =>
      (scoreCutoffMin === null || s.score >= scoreCutoffMin) &&
      (scoreCutoffMax === null || s.score <= scoreCutoffMax),
  );
  const numMatchingCutoff = scoresMatchingCutoff.length;
  if (numMatchingCutoff === 0) {
    throw new Error(`No scores matched filter for ${scoreType}`);
  }

  const scoreValuesSorted = sort(scoresMatchingType.map((p) => p.score));
  const medianScore = percentile(scoreValuesSorted, 0.5);
  if (isNaN(medianScore)) {
    throw new Error(`No scores matched filter for ${scoreType}`);
  }

  return {
    numMatchingType,
    numMatchingCutoff,
    medianScore,
  };
}
