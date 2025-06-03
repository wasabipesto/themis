import type { MarketDetails } from "@types";
import { getCriterionProb } from "@lib/api";

export interface PlatformData {
  sum: number;
  weight: number;
  count: number;
  count_no: number;
  count_yes: number;
}

export interface Bucket {
  x_start: number;
  x_center: number;
  x_end: number;
  platforms: {
    [key: string]: PlatformData;
  };
}

export interface CalibrationPoint {
  platform_slug: string;
  pred_start: number;
  pred_center: number;
  pred_end: number;
  pred_description: string;
  res_mean: number;
  count: number;
  count_no: number; // negative count
  count_yes: number;
  uncertainty: number;
}

// Creates calibration points from market data for visualization
export async function calculateCalibrationPoints(
  markets: MarketDetails[],
  criterion_type: string,
  weight_type: string | null,
  aggregate_platforms: boolean = false,
): Promise<CalibrationPoint[]> {
  // Set bucket width
  const bucketWidth = 0.05;

  // Extract unique platform names from markets
  const platformSet = new Set<string>();
  if (aggregate_platforms) {
    platformSet.add("All Platforms");
  } else {
    markets.forEach((market) => {
      if (market.platform_slug) {
        const platformName =
          market.platform_slug.charAt(0).toUpperCase() +
          market.platform_slug.slice(1);
        platformSet.add(platformName);
      }
    });
  }

  // Convert Set to Array
  const platforms = Array.from(platformSet);

  // Create buckets with the specified width from 0 to 1
  const buckets: Bucket[] = [];

  for (let i = 0; i < 1; i += bucketWidth) {
    const platformsObj: { [key: string]: PlatformData } = {};

    // Initialize platform data for each platform
    platforms.forEach((platform) => {
      platformsObj[platform] = {
        sum: 0,
        weight: 0,
        count: 0,
        count_no: 0,
        count_yes: 0,
      };
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

      const platformKey = aggregate_platforms
        ? "All Platforms"
        : market.platform_name;

      if (buckets[bucketIndex].platforms[platformKey]) {
        buckets[bucketIndex].platforms[platformKey].sum +=
          market.resolution * weight_value;
        buckets[bucketIndex].platforms[platformKey].weight += weight_value;
        buckets[bucketIndex].platforms[platformKey].count += 1;
        if (market.resolution === 0) {
          buckets[bucketIndex].platforms[platformKey].count_no -= 1;
        }
        if (market.resolution === 1) {
          buckets[bucketIndex].platforms[platformKey].count_yes += 1;
        }
      }
    }
  }

  // Create points where y_center is average of market.resolution for all markets in that bucket
  let points: CalibrationPoint[] = [];
  buckets.forEach((bucket) => {
    Object.entries(bucket.platforms).forEach(([platform, data]) => {
      if (data.count > 0) {
        points.push({
          platform_slug: platform,
          pred_start: bucket.x_start,
          pred_center: bucket.x_center,
          pred_end: bucket.x_end,
          pred_description:
            (bucket.x_start < 0.1 ? "0" : "") +
            (bucket.x_start * 100).toFixed(0) +
            "-" +
            (bucket.x_end < 0.1 ? "0" : "") +
            (bucket.x_end * 100).toFixed(0) +
            "%",
          res_mean: data.sum / data.weight,
          count: data.count,
          count_no: data.count_no,
          count_yes: data.count_yes,
          uncertainty: Math.min(Math.max(3 / Math.sqrt(data.count), 0), 1),
        });
      }
    });
  });

  return points;
}
