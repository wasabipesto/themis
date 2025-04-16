import type {
  CalibrationPoint,
  CriterionProbability,
  MarketDetails,
} from "@types";

export interface PlatformData {
  sum: number;
  count: number;
}

export interface Bucket {
  x_start: number;
  x_center: number;
  x_end: number;
  platforms: {
    [key: string]: PlatformData;
  };
}

/**
 * Creates calibration points from market data for visualization
 *
 * @param markets - Array of market details from the API
 * @param bucketWidth - Width of each probability bucket (default: 0.05)
 * @param metricType - Type of metric to use ("midpoint" or "average") (default: "midpoint")
 * @returns Array of calibration points ready for plotting
 */
export function calculateCalibrationPoints(
  markets: MarketDetails[],
  criterionProbs: CriterionProbability[],
  criterion: string,
): CalibrationPoint[] {
  // Set bucket width
  const bucketWidth = 0.05;

  // Extract unique platform names from markets
  const platformSet = new Set<string>();
  markets.forEach((market) => {
    if (market.platform_slug) {
      const platformName =
        market.platform_slug.charAt(0).toUpperCase() +
        market.platform_slug.slice(1);
      platformSet.add(platformName);
    }
  });

  // Convert Set to Array
  const platforms = Array.from(platformSet);

  // Create buckets with the specified width from 0 to 1
  const buckets: Bucket[] = [];

  for (let i = 0; i < 1; i += bucketWidth) {
    const platformsObj: { [key: string]: PlatformData } = {};

    // Initialize platform data for each platform
    platforms.forEach((platform) => {
      platformsObj[platform] = { sum: 0, count: 0 };
    });

    buckets.push({
      x_start: i,
      x_center: i + bucketWidth / 2,
      x_end: i + bucketWidth,
      platforms: platformsObj,
    });
  }

  // Choose the appropriate probability metric
  const getPrediction = (market: MarketDetails) => {
    const prediction = criterionProbs.find(
      (p) => p.market_id == market.id && p.criterion_type == criterion,
    );
    if (prediction) {
      return prediction.prob;
    } else {
      throw new Error(
        `Could not find criterion probability for ${market.id}/${criterion}`,
      );
    }
  };

  // Categorize markets into buckets by chosen probability and market.platform_slug
  markets.forEach((market) => {
    const prediction = getPrediction(market);
    if (prediction !== null && market.resolution !== null) {
      const bucketIndex = Math.min(
        Math.floor(prediction / bucketWidth),
        buckets.length - 1,
      );
      if (buckets[bucketIndex].platforms[market.platform_name]) {
        buckets[bucketIndex].platforms[market.platform_name].sum +=
          market.resolution;
        buckets[bucketIndex].platforms[market.platform_name].count += 1;
      }
    }
  });

  // Create points where y_center is average of market.resolution for all markets in that bucket
  let points: CalibrationPoint[] = [];
  buckets.forEach((bucket) => {
    Object.entries(bucket.platforms).forEach(([platform, data]) => {
      if (data.count > 0) {
        points.push({
          platform_slug: platform,
          x_start: bucket.x_start,
          x_center: bucket.x_center,
          x_end: bucket.x_end,
          y_center: data.sum / data.count,
          count: data.count,
        });
      }
    });
  });

  return points;
}
