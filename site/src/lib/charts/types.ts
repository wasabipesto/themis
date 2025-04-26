import type { PlatformDetails, MarketDetails, MarketScoreDetails } from "@types";

/**
 * Base options for all chart visualizations
 */
export interface BaseChartOptions {
  id: string;
  title?: string;
  description: string;
  icon?: string | null;
  axisConfig?: {
    x?: { title?: string; range?: [number, number] };
    y?: { title?: string; range?: [number, number] };
  };
}

/**
 * Common context data provided to all chart types
 */
export interface ChartContext {
  platforms: PlatformDetails[];
}

/**
 * Base interface for all chart data points
 */
export interface ChartDataPoint {
  platformSlug: string;
  platformName: string;
  colorPrimary: string;
  colorAccent: string;
}

/**
 * Box plot data point structure
 */
export interface BoxPlotDataPoint extends ChartDataPoint {
  whiskerStart: number;
  boxStart: number;
  boxCenter: number;
  boxEnd: number;
  whiskerEnd: number;
}

/**
 * Histogram data point structure
 */
export interface HistogramDataPoint extends ChartDataPoint {
  startX: number;
  endX: number;
  count: number;
  percent: number;
}

/**
 * Calibration point for calibration charts
 */
export interface CalibrationDataPoint extends ChartDataPoint {
  x_start: number;
  x_center: number;
  x_end: number;
  y_center: number;
  count: number;
}

/**
 * Time-based bar chart data point
 */
export interface TimeBarDataPoint extends ChartDataPoint {
  startXDate: string;
  startXMs: number;
  endXDate: string;
  endXMs: number;
  value: number;
  count: number;
}

/**
 * Options specific to box plot charts
 */
export interface BoxPlotOptions extends BaseChartOptions {
  type: 'boxplot';
  scoreType?: string;
  plotRangeStart?: number;
  plotRangeEnd?: number;
}

/**
 * Options specific to histogram charts
 */
export interface HistogramOptions extends BaseChartOptions {
  type: 'histogram';
  binCount?: number;
  rangeStart?: number;
  rangeEnd?: number;
}

/**
 * Options specific to calibration charts
 */
export interface CalibrationOptions extends BaseChartOptions {
  type: 'calibration';
  criterion?: string;
  weight?: string;
}

/**
 * Options specific to time bar charts
 */
export interface TimeBarOptions extends BaseChartOptions {
  type: 'timebar';
  groupBy?: 'day' | 'week' | 'month' | 'year';
  cumulative?: boolean;
  dateField?: string;
  subtitle?: string;
}

/**
 * Union type for all chart options
 */
export type ChartOptions = BoxPlotOptions | HistogramOptions | CalibrationOptions | TimeBarOptions;

/**
 * Factory function to create a selector option with standard structure
 */
export interface ChartDataSelector<T, R> {
  (data: T[], context: ChartContext): R[];
}

/**
 * Interface for chart selector options
 */
export interface ChartSelectorOption<T, R> {
  id: string;
  icon?: string | null;
  description: string;
  dataSelector: ChartDataSelector<T, R>;
  config?: Partial<ChartOptions>;
  count?: number;
}

/**
 * Data transformer interface for creating data processing pipelines
 */
export interface DataTransformer<T, U> {
  transform(data: T, context?: any): U;
}

/**
 * Filter transformer implementation
 */
export class FilterTransformer<T> implements DataTransformer<T[], T[]> {
  constructor(private predicate: (item: T) => boolean) {}
  
  transform(data: T[]): T[] {
    return data.filter(this.predicate);
  }
}

/**
 * Group transformer implementation
 */
export class GroupTransformer<T> implements DataTransformer<T[], Record<string, T[]>> {
  constructor(private keyFn: (item: T) => string) {}
  
  transform(data: T[]): Record<string, T[]> {
    return data.reduce((acc, item) => {
      const key = this.keyFn(item);
      if (!acc[key]) acc[key] = [];
      acc[key].push(item);
      return acc;
    }, {} as Record<string, T[]>);
  }
}

/**
 * Data processing pipeline
 */
export class Pipeline<T, U> {
  constructor(private transformers: DataTransformer<any, any>[]) {}
  
  process(data: T): U {
    return this.transformers.reduce(
      (result, transformer) => transformer.transform(result),
      data as any
    ) as U;
  }
}

/**
 * Chart data processing utilities
 */
export const ChartDataUtils = {
  /**
   * Creates a filter for market data based on common criteria
   */
  createMarketFilter(minTraders?: number, minVolume?: number, minDurationDays?: number): (m: MarketDetails) => boolean {
    return (market: MarketDetails): boolean => {
      return (
        ((minTraders === undefined || (market.traders_count !== null && market.traders_count >= minTraders)) &&
        (minVolume === undefined || (market.volume_usd !== null && market.volume_usd >= minVolume))) &&
        (minDurationDays === undefined || market.duration_days >= minDurationDays)
      );
    };
  },
  
  /**
   * Creates a filter for market score data based on score type
   */
  createScoreFilter(scoreType: string): (ms: MarketScoreDetails) => boolean {
    return (score: MarketScoreDetails): boolean => {
      return score.score_type === scoreType;
    };
  },
  
  /**
   * Filter markets by date range
   */
  filterByDateRange(markets: MarketDetails[], monthsAgo?: number): MarketDetails[] {
    if (!monthsAgo) return markets;
    
    const cutoffDate = new Date();
    cutoffDate.setMonth(cutoffDate.getMonth() - monthsAgo);
    const cutoffTime = cutoffDate.getTime();
    
    return markets.filter(
      (market) => new Date(market.close_datetime).getTime() >= cutoffTime
    );
  },
  
  /**
   * Filter markets by category
   */
  filterByCategory(markets: MarketDetails[], categorySlug?: string): MarketDetails[] {
    if (!categorySlug) return markets;
    return markets.filter((market) => market.category_slug === categorySlug);
  },
  
  /**
   * Filter markets by resolution value
   */
  filterByResolution(markets: MarketDetails[], resolutionValue?: number | [number, number]): MarketDetails[] {
    if (resolutionValue === undefined) return markets;
    
    if (Array.isArray(resolutionValue)) {
      const [min, max] = resolutionValue;
      return markets.filter((market) => market.resolution >= min && market.resolution <= max);
    } else {
      return markets.filter((market) => market.resolution === resolutionValue);
    }
  }
};