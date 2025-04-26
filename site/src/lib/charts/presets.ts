import type { MarketDetails, MarketScoreDetails, CategoryDetails } from "@types";
import type {
  ChartSelectorOption,
  BoxPlotOptions,
  HistogramOptions,
  CalibrationOptions,
  TimeBarOptions,
} from "./types";
import { ChartDataUtils } from "./types";
import { createCommonFilterOptions } from "./factory";

/**
 * Creates standard filter options for accuracy box plots
 */
export function createStandardAccuracyOptions(
  scores: MarketScoreDetails[],
): ChartSelectorOption<MarketScoreDetails, any>[] {
  return [
    {
      id: 'all',
      icon: 'mdi:all-inclusive-box',
      description: 'Show all resolved markets.',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'brier-midpoint'),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'basic',
      icon: 'mdi:alpha-a-box',
      description: 'Filter to at least 10 traders or $100 in trade volume, and open for at least 2 days.',
      dataSelector: (data) => data.filter(ms => {
        return ms.score_type === 'brier-midpoint' && 
          ((ms.traders_count !== null && ms.traders_count >= 10) ||
          (ms.volume_usd !== null && ms.volume_usd >= 100)) &&
          ms.duration_days >= 2;
      }),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'medium',
      icon: 'mdi:alpha-b-box',
      description: 'Filter to at least 100 traders or $1000 in trade volume, and open for at least 14 days.',
      dataSelector: (data) => data.filter(ms => {
        return ms.score_type === 'brier-midpoint' && 
          ((ms.traders_count !== null && ms.traders_count >= 100) ||
          (ms.volume_usd !== null && ms.volume_usd >= 1000)) &&
          ms.duration_days >= 14;
      }),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'high',
      icon: 'mdi:alpha-c-box',
      description: 'Filter to at least 1000 traders or $10,000 in trade volume, and open for at least 30 days.',
      dataSelector: (data) => data.filter(ms => {
        return ms.score_type === 'brier-midpoint' && 
          ((ms.traders_count !== null && ms.traders_count >= 1000) ||
          (ms.volume_usd !== null && ms.volume_usd >= 10000)) &&
          ms.duration_days >= 30;
      }),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'recent',
      icon: 'mdi:calendar-check',
      description: 'Filter to markets resolved in the past 12 months.',
      dataSelector: (data) => data.filter(ms => {
        return ms.score_type === 'brier-midpoint' && 
          new Date(ms.close_datetime).getTime() >= new Date().getTime() - 365 * 24 * 60 * 60 * 1000;
      }),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'linked',
      icon: 'mdi:check-decagram',
      description: 'Filter to markets that have been linked in questions.',
      dataSelector: (data) => data.filter(ms => {
        return ms.score_type === 'brier-midpoint' && ms.question_id;
      }),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Market score', range: [0, 0.8] }
        }
      }
    },
  ];
}

/**
 * Creates score type selector options for accuracy charts
 */
export function createScoreTypeOptions(
  scores: MarketScoreDetails[],
): ChartSelectorOption<MarketScoreDetails, any>[] {
  return [
    {
      id: 'brier-midpoint',
      description: 'Market Brier score, calculated with midpoint probability',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'brier-midpoint'),
      config: {
        type: 'boxplot',
        scoreType: 'brier-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Midpoint Brier score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'brier-average',
      description: 'Market Brier score, averaged over the entire market',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'brier-average'),
      config: {
        type: 'boxplot',
        scoreType: 'brier-average',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Average Brier score', range: [0, 0.8] }
        }
      }
    },
    {
      id: 'brier-relative',
      description: 'Relative Brier score, calculated for linked markets',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'brier-relative'),
      config: {
        type: 'boxplot',
        scoreType: 'brier-relative',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Relative Brier score', range: [-0.08, 0.08] }
        }
      }
    },
    {
      id: 'logarithmic-midpoint',
      description: 'Market log score, calculated with midpoint probability',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'logarithmic-midpoint'),
      config: {
        type: 'boxplot',
        scoreType: 'logarithmic-midpoint',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Midpoint log score', range: [-2, 0] }
        }
      }
    },
    {
      id: 'logarithmic-average',
      description: 'Market log score, averaged over the entire market',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'logarithmic-average'),
      config: {
        type: 'boxplot',
        scoreType: 'logarithmic-average',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Average log score', range: [-2, 0] }
        }
      }
    },
    {
      id: 'logarithmic-relative',
      description: 'Relative log score, calculated for linked markets',
      dataSelector: (data) => data.filter(ms => ms.score_type === 'logarithmic-relative'),
      config: {
        type: 'boxplot',
        scoreType: 'logarithmic-relative',
        axisConfig: {
          x: { title: 'Platform' },
          y: { title: 'Relative log score', range: [-0.2, 0.2] }
        }
      }
    },
  ];
}

/**
 * Creates calibration criterion options
 */
export function createCalibrationCriterionOptions(
  markets: MarketDetails[],
): ChartSelectorOption<MarketDetails, any>[] {
  return [
    {
      id: 'midpoint',
      icon: 'mdi:format-horizontal-align-center',
      description: 'Probability at midpoint (default)',
      dataSelector: (data) => data,
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'time-average',
      icon: 'mdi:timer-outline',
      description: 'Time-weighted average probability',
      dataSelector: (data) => data,
      config: {
        type: 'calibration',
        criterion: 'time-average',
        axisConfig: {
          x: { title: 'Prediction (average)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'before-close-hours-24',
      icon: 'mdi:timer-sand-complete',
      description: 'Probability 24 hours before resolution',
      dataSelector: (data) => data.filter(market => market.duration_days > 2),
      config: {
        type: 'calibration',
        criterion: 'before-close-hours-24',
        axisConfig: {
          x: { title: 'Prediction (24h before resolution)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'before-close-days-30',
      icon: 'mdi:timer-sand-complete',
      description: 'Probability 30 days before resolution',
      dataSelector: (data) => data.filter(market => market.duration_days > 31),
      config: {
        type: 'calibration',
        criterion: 'before-close-days-30',
        axisConfig: {
          x: { title: 'Prediction (30d before resolution)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'before-close-days-90',
      icon: 'mdi:timer-sand-complete',
      description: 'Probability 90 days before resolution',
      dataSelector: (data) => data.filter(market => market.duration_days > 91),
      config: {
        type: 'calibration',
        criterion: 'before-close-days-90',
        axisConfig: {
          x: { title: 'Prediction (90d before resolution)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'before-close-days-365',
      icon: 'mdi:timer-sand-complete',
      description: 'Probability 365 days before resolution',
      dataSelector: (data) => data.filter(market => market.duration_days > 366),
      config: {
        type: 'calibration',
        criterion: 'before-close-days-365',
        axisConfig: {
          x: { title: 'Prediction (one year before resolution)' },
          y: { title: 'Resolution' }
        }
      }
    },
    {
      id: 'after-start-hours-24',
      icon: 'mdi:timer-sand',
      description: 'Probability 24 hours after open',
      dataSelector: (data) => data.filter(market => market.duration_days > 2),
      config: {
        type: 'calibration',
        criterion: 'after-start-hours-24',
        axisConfig: {
          x: { title: 'Prediction (24h after open)' },
          y: { title: 'Resolution' }
        }
      }
    },
  ];
}

/**
 * Creates calibration weight options
 */
export function createCalibrationWeightOptions(
  markets: MarketDetails[],
): ChartSelectorOption<MarketDetails, any>[] {
  return [
    {
      id: 'unweighted',
      icon: 'mdi:border-none-variant',
      description: 'Unweighted',
      dataSelector: (data) => data,
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        weight: undefined,
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution, unweighted' }
        }
      }
    },
    {
      id: 'volume',
      icon: 'mdi:cash',
      description: 'Weight market by trade volume',
      dataSelector: (data) => data.filter(market => market.volume_usd && market.volume_usd > 0),
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        weight: 'volume_usd',
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution, weighted by volume' }
        }
      }
    },
    {
      id: 'traders',
      icon: 'mdi:account-multiple',
      description: 'Weight market by number of traders',
      dataSelector: (data) => data.filter(market => market.traders_count && market.traders_count > 0),
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        weight: 'traders_count',
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution, weighted by traders' }
        }
      }
    },
    {
      id: 'duration',
      icon: 'mdi:calendar-blank',
      description: 'Weight market by market duration',
      dataSelector: (data) => data.filter(market => market.duration_days > 0),
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        weight: 'duration_days',
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution, weighted by duration' }
        }
      }
    },
    {
      id: 'recency',
      icon: 'mdi:calendar-clock',
      description: 'Weight market by recency',
      dataSelector: (data) => data,
      config: {
        type: 'calibration',
        criterion: 'midpoint',
        weight: 'recency',
        axisConfig: {
          x: { title: 'Prediction (midpoint)' },
          y: { title: 'Resolution, weighted by recency' }
        }
      }
    },
  ];
}

/**
 * Creates category filter options
 */
export function createCategoryFilterOptions(
  data: MarketDetails[] | MarketScoreDetails[],
  categories: CategoryDetails[],
  createOptionFunc: (id: string, description: string, icon: string | null, filter: (data: any[]) => any[]) => ChartSelectorOption<any, any>
): ChartSelectorOption<any, any>[] {
  const options: ChartSelectorOption<any, any>[] = [
    createOptionFunc(
      'all',
      'All categories',
      'mdi:compass-rose',
      (data) => data
    )
  ];
  
  categories.forEach(category => {
    options.push(
      createOptionFunc(
        category.slug,
        category.name,
        category.icon,
        (data) => data.filter(item => item.category_slug === category.slug)
      )
    );
  });
  
  return options;
}

/**
 * Creates histogram options for market attributes
 */
export function createMarketHistogramOptions(
  markets: MarketDetails[],
): ChartSelectorOption<MarketDetails, any>[] {
  return [
    {
      id: 'volume',
      description: 'Market distribution by trade volume.',
      dataSelector: (data) => data
        .filter(mkt => mkt.volume_usd && mkt.volume_usd > 0)
        .map(mkt => ({
          value: mkt.volume_usd || 0,
          platformSlug: mkt.platform_slug,
        })),
      config: {
        type: 'histogram',
        binCount: 20,
        title: 'Histogram of Volume',
        axisConfig: {
          x: { title: 'Volume (USD)' },
          y: { title: 'Percent' }
        }
      }
    },
    {
      id: 'traders',
      description: 'Market distribution by number of traders.',
      dataSelector: (data) => data
        .filter(mkt => mkt.traders_count && mkt.traders_count > 0)
        .map(mkt => ({
          value: mkt.traders_count || 0,
          platformSlug: mkt.platform_slug,
        })),
      config: {
        type: 'histogram',
        binCount: 20,
        title: 'Histogram of Traders',
        axisConfig: {
          x: { title: 'Trader count' },
          y: { title: 'Percent' }
        }
      }
    },
    {
      id: 'duration',
      description: 'Market distribution by total duration.',
      dataSelector: (data) => data.map(mkt => ({
        value: mkt.duration_days,
        platformSlug: mkt.platform_slug,
      })),
      config: {
        type: 'histogram',
        binCount: 20,
        title: 'Histogram of Duration',
        axisConfig: {
          x: { title: 'Duration (days)' },
          y: { title: 'Percent' }
        }
      }
    },
    {
      id: 'title_length',
      description: 'Market distribution by title length.',
      dataSelector: (data) => data.map(mkt => ({
        value: mkt.title.length,
        platformSlug: mkt.platform_slug,
      })),
      config: {
        type: 'histogram',
        binCount: 20,
        rangeEnd: 200,
        title: 'Histogram of Title Length',
        axisConfig: {
          x: { title: 'Length (characters)' },
          y: { title: 'Percent' }
        }
      }
    },
    {
      id: 'description_length',
      description: 'Market distribution by total description length.',
      dataSelector: (data) => data.map(mkt => ({
        value: mkt.description.length,
        platformSlug: mkt.platform_slug,
      })),
      config: {
        type: 'histogram',
        binCount: 20,
        title: 'Histogram of Description Length',
        axisConfig: {
          x: { title: 'Length (characters)' },
          y: { title: 'Percent' }
        }
      }
    },
    {
      id: 'resolution',
      description: 'Market distribution by resolution value.',
      dataSelector: (data) => data.map(mkt => ({
        value: mkt.resolution,
        platformSlug: mkt.platform_slug,
      })),
      config: {
        type: 'histogram',
        binCount: 20,
        rangeEnd: 1,
        title: 'Histogram of Resolution Value',
        axisConfig: {
          x: { title: 'Probability' },
          y: { title: 'Percent' }
        }
      }
    },
  ];
}

/**
 * Creates resolution filter options
 */
export function createResolutionFilterOptions(
  data: MarketDetails[] | MarketScoreDetails[],
  createOptionFunc: (id: string, description: string, icon: string | null, filter: (data: any[]) => any[]) => ChartSelectorOption<any, any>
): ChartSelectorOption<any, any>[] {
  return [
    createOptionFunc(
      'all',
      'All markets, regardless of resolution',
      'mdi:all-inclusive-box',
      (data) => data
    ),
    createOptionFunc(
      'yes',
      'Only markets that resolved YES',
      'mdi:numeric-1-circle',
      (data) => data.filter(item => item.resolution == 1)
    ),
    createOptionFunc(
      'no',
      'Only markets that resolved NO',
      'mdi:numeric-0-circle',
      (data) => data.filter(item => item.resolution == 0)
    ),
    createOptionFunc(
      'partial',
      'Only markets that resolved to something inbetween',
      'mdi:help-circle',
      (data) => data.filter(item => item.resolution != 0 && item.resolution != 1)
    ),
  ];
}