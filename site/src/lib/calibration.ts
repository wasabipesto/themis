import type { CalibrationPoint, MarketLite } from "@types";

export function marketsIntoCalibrationPoints(
  markets: MarketLite[],
): CalibrationPoint[] {
  // Define types for our data structures
  type PlatformData = {
    sum: number;
    count: number;
  };

  type Bucket = {
    x_start: number;
    x_center: number;
    x_end: number;
    platforms: {
      [key: string]: PlatformData;
    };
  };

  // Create buckets with width 0.05 from 0 to 1
  const bucketWidth = 0.05;
  const buckets: Bucket[] = [];

  for (let i = 0; i < 1; i += bucketWidth) {
    buckets.push({
      x_start: i,
      x_center: i + bucketWidth / 2,
      x_end: i + bucketWidth,
      platforms: {
        Manifold: { sum: 0, count: 0 },
        Kalshi: { sum: 0, count: 0 },
        Metaculus: { sum: 0, count: 0 },
        Polymarket: { sum: 0, count: 0 },
      },
    });
  }

  // Categorize markets into buckets by market.prob_at_midpoint and market.platform_slug
  markets.forEach((market) => {
    if (market.prob_at_midpoint !== null && market.resolution !== null) {
      const probability = market.prob_at_midpoint;
      const bucketIndex = Math.min(
        Math.floor(probability / bucketWidth),
        buckets.length - 1,
      );
      const platformName =
        market.platform_slug.charAt(0).toUpperCase() +
        market.platform_slug.slice(1);

      if (buckets[bucketIndex].platforms[platformName]) {
        buckets[bucketIndex].platforms[platformName].sum += market.resolution;
        buckets[bucketIndex].platforms[platformName].count += 1;
      }
    }
  });

  // Create points where y_center is average of market.resolution for all markets in that bucket
  const points: CalibrationPoint[] = [];
  buckets.forEach((bucket) => {
    Object.entries(bucket.platforms).forEach(([platform, data]) => {
      if (data.count > 0) {
        points.push({
          platform_slug: platform,
          x_start: bucket.x_start,
          x_center: bucket.x_center,
          x_end: bucket.x_end,
          y_start: null,
          y_center: data.sum / data.count,
          y_end: null,
          count: data.count,
        });
      }
    });
  });
  return points;
}
