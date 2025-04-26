import type {
  ChartContext,
  ChartOptions,
  BoxPlotOptions,
  HistogramOptions,
  CalibrationOptions,
  TimeBarOptions,
  ChartSelectorOption
} from "./types";
import type { MarketDetails, MarketScoreDetails } from "@types";

/**
 * Default options for each chart type
 */
const defaultOptions = {
  boxplot: {
    type: 'boxplot' as const,
    scoreType: 'brier-midpoint',
    axisConfig: {
      x: { title: 'Platform' },
      y: { title: 'Market score', range: [0, 0.8] }
    }
  },
  histogram: {
    type: 'histogram' as const,
    binCount: 20,
    axisConfig: {
      x: { title: 'Value' },
      y: { title: 'Percent' }
    }
  },
  calibration: {
    type: 'calibration' as const,
    criterion: 'midpoint',
    axisConfig: {
      x: { title: 'Prediction (midpoint)', range: [0, 1] },
      y: { title: 'Resolution', range: [0, 1] }
    }
  },
  timebar: {
    type: 'timebar' as const,
    groupBy: 'month' as const,
    axisConfig: {
      x: { title: 'Date' },
      y: { title: 'Percent' }
    }
  }
};

/**
 * Factory function to create chart options with appropriate defaults
 */
export function createChartOptions<T extends ChartOptions>(
  type: T['type'],
  options: Partial<T>
): T {
  const baseConfig = { id: `chart-${Math.random().toString(36).slice(2, 11)}` };
  
  switch (type) {
    case 'boxplot':
      return { 
        ...baseConfig,
        ...defaultOptions.boxplot, 
        ...options 
      } as T;
    case 'histogram':
      return { 
        ...baseConfig,
        ...defaultOptions.histogram, 
        ...options 
      } as T;
    case 'calibration':
      return { 
        ...baseConfig,
        ...defaultOptions.calibration, 
        ...options 
      } as T;
    case 'timebar':
      return { 
        ...baseConfig,
        ...defaultOptions.timebar, 
        ...options 
      } as T;
    default:
      throw new Error(`Unknown chart type: ${type}`);
  }
}

/**
 * Create a predefined box plot selector option
 */
export function createBoxPlotSelectorOption(
  id: string,
  description: string,
  icon: string | null,
  scoreFilter: (scores: MarketScoreDetails[]) => MarketScoreDetails[],
  config: Partial<BoxPlotOptions> = {}
): ChartSelectorOption<MarketScoreDetails, any> {
  return {
    id,
    description,
    icon,
    dataSelector: (data, context) => scoreFilter(data),
    config: {
      ...config,
      type: 'boxplot',
    }
  };
}

/**
 * Create a predefined histogram selector option
 */
export function createHistogramSelectorOption(
  id: string,
  description: string,
  icon: string | null,
  dataTransformer: (data: any[]) => Array<{ value: number; platformSlug: string }>,
  config: Partial<HistogramOptions> = {}
): ChartSelectorOption<any, any> {
  return {
    id,
    description,
    icon,
    dataSelector: (data, context) => dataTransformer(data),
    config: {
      ...config,
      type: 'histogram',
    }
  };
}

/**
 * Create a predefined calibration selector option
 */
export function createCalibrationSelectorOption(
  id: string,
  description: string,
  icon: string | null,
  marketFilter: (markets: MarketDetails[]) => MarketDetails[],
  config: Partial<CalibrationOptions> = {}
): ChartSelectorOption<MarketDetails, any> {
  return {
    id,
    description,
    icon,
    dataSelector: (data, context) => marketFilter(data),
    config: {
      ...config,
      type: 'calibration',
    }
  };
}

/**
 * Create common filter preset options for market data
 */
export function createCommonFilterOptions(
  markets: MarketDetails[],
  optionCreator: (
    id: string, 
    description: string, 
    icon: string | null, 
    filter: (markets: MarketDetails[]) => MarketDetails[],
    config?: any
  ) => ChartSelectorOption<MarketDetails, any>
): ChartSelectorOption<MarketDetails, any>[] {
  return [
    optionCreator(
      'all',
      'Show all resolved markets.',
      'mdi:all-inclusive-box',
      (data) => data,
    ),
    optionCreator(
      'basic',
      'Filter to at least 10 traders or $100 in trade volume, and open for at least 2 days.',
      'mdi:alpha-a-box',
      (data) => data.filter((market) => {
        return (
          ((market.traders_count !== null && market.traders_count >= 10) ||
            (market.volume_usd !== null && market.volume_usd >= 100)) &&
          market.duration_days >= 2
        );
      }),
    ),
    optionCreator(
      'medium',
      'Filter to at least 100 traders or $1000 in trade volume, and open for at least 14 days.',
      'mdi:alpha-b-box',
      (data) => data.filter((market) => {
        return (
          ((market.traders_count !== null && market.traders_count >= 100) ||
            (market.volume_usd !== null && market.volume_usd >= 1000)) &&
          market.duration_days >= 14
        );
      }),
    ),
    optionCreator(
      'high',
      'Filter to at least 1000 traders or $10,000 in trade volume, and open for at least 30 days.',
      'mdi:alpha-c-box',
      (data) => data.filter((market) => {
        return (
          ((market.traders_count !== null && market.traders_count >= 1000) ||
            (market.volume_usd !== null && market.volume_usd >= 10000)) &&
          market.duration_days >= 30
        );
      }),
    ),
    optionCreator(
      'recent',
      'Filter to markets resolved in the past 12 months.',
      'mdi:calendar-check',
      (data) => data.filter(
        (market) =>
          new Date(market.close_datetime).getTime() >=
          new Date().getTime() - 365 * 24 * 60 * 60 * 1000,
      ),
    ),
    optionCreator(
      'linked',
      'Filter to markets that have been linked in questions.',
      'mdi:check-decagram',
      (data) => data.filter((market) => market.question_id),
    ),
  ];
}