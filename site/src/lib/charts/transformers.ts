import { quartiles, roundSF } from "@lib/stats";
import type {
  ChartContext,
  BoxPlotDataPoint,
  HistogramDataPoint,
  CalibrationDataPoint,
  TimeBarDataPoint,
  ChartDataPoint,
} from "./types";
import type { MarketDetails, MarketScoreDetails, PlatformDetails } from "@types";

/**
 * Transform market scores into box plot data points
 */
export function transformToBoxPlotData(
  scores: MarketScoreDetails[],
  platforms: PlatformDetails[],
  scoreType: string = "brier-midpoint"
): BoxPlotDataPoint[] {
  // Filter scores by the specified score type
  const filteredScores = scores.filter((ms) => ms.score_type === scoreType);
  
  // Group by platform and calculate quartiles
  const platformData: BoxPlotDataPoint[] = [];
  
  platforms.forEach(platform => {
    const platformScores = filteredScores.filter(
      (ms) => ms.platform_slug === platform.slug
    );
    
    if (platformScores.length === 0) return;
    
    const scores = platformScores.map((s) => s.score);
    const { c1, q1, q2, q3, c3 } = quartiles(scores);
    
    platformData.push({
      platformSlug: platform.slug,
      platformName: platform.name,
      colorPrimary: platform.color_primary,
      colorAccent: platform.color_accent,
      whiskerStart: c1,
      boxStart: q1,
      boxCenter: q2,
      boxEnd: q3,
      whiskerEnd: c3,
    });
  });
  
  return platformData;
}

/**
 * Transform values into histogram data points
 */
export function transformToHistogramData(
  items: Array<{ value: number; platformSlug: string }>,
  platforms: PlatformDetails[],
  options: {
    binCount?: number;
    rangeStart?: number;
    rangeEnd?: number;
  } = {}
): HistogramDataPoint[] {
  const numBins = options.binCount || 20;
  const valuesSorted = items.map((item) => item.value).sort((a, b) => a - b);
  const { c9 } = quartiles(valuesSorted);
  const c9Rounded = roundSF(c9, 2);

  const rangeStart = options.rangeStart !== undefined ? options.rangeStart : 0;
  const rangeEnd = options.rangeEnd !== undefined ? options.rangeEnd : c9Rounded;
  const binWidth = (rangeEnd - rangeStart) / numBins;

  // Filter items to those within range
  const itemsInRange = items.filter(
    (i) => i.value >= rangeStart && i.value <= rangeEnd
  );

  // Create a nested array to count bins per platform
  const bins: Record<string, number[]> = {};

  // Count items per bin per platform
  itemsInRange.forEach((i) => {
    if (!bins[i.platformSlug]) {
      bins[i.platformSlug] = Array(numBins).fill(0);
    }
    const binIndex = Math.min(
      numBins - 1,
      Math.floor((i.value - rangeStart) / binWidth)
    );
    bins[i.platformSlug][binIndex]++;
  });

  // Convert to histogram data points
  const points: HistogramDataPoint[] = [];
  
  for (const [platformSlug, platformBins] of Object.entries(bins)) {
    const platformDetails = platforms.find((p) => p.slug === platformSlug);
    if (!platformDetails) continue;
    
    const platformNumItems = itemsInRange.filter(
      (v) => v.platformSlug === platformSlug
    ).length;

    platformBins.forEach((count, index) => {
      if (count > 0) {
        const startX = rangeStart + index * binWidth;
        const endX = startX + binWidth;
        points.push({
          platformSlug: platformDetails.slug,
          platformName: platformDetails.name,
          colorPrimary: platformDetails.color_primary,
          colorAccent: platformDetails.color_accent,
          startX,
          endX,
          count,
          percent: count / platformNumItems,
        });
      }
    });
  }

  return points;
}

/**
 * Transform market data to time-based bar chart data
 */
export function transformToTimeBarData(
  markets: MarketDetails[],
  platforms: PlatformDetails[],
  options: {
    dateField: 'open_datetime' | 'close_datetime';
    groupBy?: 'day' | 'week' | 'month' | 'year';
    startDate?: Date;
    endDate?: Date;
    numBins?: number;
  } = { dateField: 'close_datetime' }
): TimeBarDataPoint[] {
  const numBins = options.numBins || 100;
  const startDate = options.startDate || new Date("2021-01-01");
  const endDate = options.endDate || new Date();
  
  const minValue = startDate.getTime();
  const maxValue = endDate.getTime();
  const binWidth = (maxValue - minValue) / numBins;

  const bins: Record<string, Record<number, number>> = {};

  // Initialize bins for each platform
  platforms.forEach(platform => {
    bins[platform.slug] = {};
  });

  // Count markets per bin per platform
  markets.forEach((market) => {
    const dateTime = new Date(market[options.dateField]).getTime();
    if (dateTime < minValue || dateTime > maxValue) return;
    
    const binIndex = Math.min(
      numBins - 1,
      Math.floor((dateTime - minValue) / binWidth)
    );
    
    if (!bins[market.platform_slug]) {
      bins[market.platform_slug] = {};
    }
    
    if (!bins[market.platform_slug][binIndex]) {
      bins[market.platform_slug][binIndex] = 0;
    }
    
    bins[market.platform_slug][binIndex]++;
  });

  // Convert to time bar data points
  const points: TimeBarDataPoint[] = [];
  
  Object.entries(bins).forEach(([platformSlug, platformBins]) => {
    const platform = platforms.find(p => p.slug === platformSlug);
    if (!platform) return;
    
    const platformMarkets = markets.filter(m => m.platform_slug === platformSlug);
    const platformTotal = platformMarkets.length;
    
    Object.entries(platformBins).forEach(([binIndexStr, count]) => {
      const binIndex = parseInt(binIndexStr);
      const startXMs = minValue + binIndex * binWidth;
      const endXMs = startXMs + binWidth;
      
      points.push({
        platformSlug: platform.slug,
        platformName: platform.name,
        colorPrimary: platform.color_primary,
        colorAccent: platform.color_accent,
        startXDate: new Date(startXMs).toISOString(),
        startXMs,
        endXDate: new Date(endXMs).toISOString(),
        endXMs,
        value: count / platformTotal, // percentage
        count
      });
    });
  });

  return points;
}

/**
 * Base class for chart data processors
 */
export abstract class ChartDataProcessor<T, R> {
  constructor(protected context: ChartContext) {}
  
  abstract process(data: T[]): R[];
}

/**
 * Box plot data processor
 */
export class BoxPlotProcessor extends ChartDataProcessor<MarketScoreDetails, BoxPlotDataPoint> {
  constructor(
    context: ChartContext,
    private scoreType: string = "brier-midpoint"
  ) {
    super(context);
  }
  
  process(scores: MarketScoreDetails[]): BoxPlotDataPoint[] {
    return transformToBoxPlotData(scores, this.context.platforms, this.scoreType);
  }
}

/**
 * Histogram data processor
 */
export class HistogramProcessor extends ChartDataProcessor<{value: number, platformSlug: string}, HistogramDataPoint> {
  constructor(
    context: ChartContext,
    private options: { binCount?: number; rangeStart?: number; rangeEnd?: number } = {}
  ) {
    super(context);
  }
  
  process(items: Array<{ value: number; platformSlug: string }>): HistogramDataPoint[] {
    return transformToHistogramData(items, this.context.platforms, this.options);
  }
}

/**
 * Time bar data processor
 */
export class TimeBarProcessor extends ChartDataProcessor<MarketDetails, TimeBarDataPoint> {
  constructor(
    context: ChartContext,
    private options: {
      dateField: 'open_datetime' | 'close_datetime';
      groupBy?: 'day' | 'week' | 'month' | 'year';
      startDate?: Date;
      endDate?: Date;
      numBins?: number;
    } = { dateField: 'close_datetime' }
  ) {
    super(context);
  }
  
  process(markets: MarketDetails[]): TimeBarDataPoint[] {
    return transformToTimeBarData(markets, this.context.platforms, this.options);
  }
}