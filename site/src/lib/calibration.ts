import type { CalibrationPoint, MarketDetails } from "@types";
import { getCriterionProb } from "@lib/api";

export interface PlatformData {
  sum: number;
  weight: number;
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

// Creates calibration points from market data for visualization
export async function calculateCalibrationPoints(
  markets: MarketDetails[],
  criterion_type: string,
  weight_type: string | null,
): Promise<CalibrationPoint[]> {
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
      platformsObj[platform] = { sum: 0, weight: 0, count: 0 };
    });

    buckets.push({
      x_start: i,
      x_center: i + bucketWidth / 2,
      x_end: i + bucketWidth,
      platforms: platformsObj,
    });
  }

  function getWeight(
    market: MarketDetails,
    weight_type: string | null,
  ): number | null {
    if (weight_type == null) {
      return 1;
    } else if (weight_type == "volume_usd") {
      return market.volume_usd;
    } else if (weight_type == "traders_count") {
      return market.traders_count;
    } else if (weight_type == "duration_days") {
      return market.duration_days;
    } else if (weight_type == "recency") {
      return new Date().getTime() - new Date(market.close_datetime).getTime();
    } else {
      throw new Error(`Invalid weight: ${weight_type}`);
    }
  }

  // Categorize markets into buckets by chosen probability and market.platform_slug
  for (const market of markets) {
    const criterionProb = await getCriterionProb(market.id, criterion_type);
    const weight_value = getWeight(market, weight_type);
    if (criterionProb && weight_value) {
      const prediction = criterionProb.prob;
      const bucketIndex = Math.min(
        Math.floor(prediction / bucketWidth),
        buckets.length - 1,
      );
      if (buckets[bucketIndex].platforms[market.platform_name]) {
        buckets[bucketIndex].platforms[market.platform_name].sum +=
          market.resolution * weight_value;
        buckets[bucketIndex].platforms[market.platform_name].weight +=
          weight_value;
        buckets[bucketIndex].platforms[market.platform_name].count += 1;
      }
    } else {
      console.error(
        `No criterion probability found for market ${market.id}/${criterion_type}`,
      );
    }
  }

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
          y_center: data.sum / data.weight,
          count: data.count,
        });
      }
    });
  });

  return points;
}
