/**
 * Chart Types - Type definitions for chart configuration and data
 */
import type {
  MarketDetails,
  MarketScoreDetails,
  PlatformDetails,
} from "@types";

/**
 * Re-export CalibrationPoint from @types
 */
export interface CalibrationPoint {
  platform_slug: string;
  x_start: number;
  x_center: number;
  x_end: number;
  y_center: number;
  count: number;
}

/**
 * Base properties for all chart options
 */
export interface ChartOptionBase {
  /** Unique identifier for the option */
  id: string;
  /** Optional icon to display with the option */
  icon?: string | null;
  /** Description of the option shown in the UI */
  description: string;
  /** Number of data points in this option (for display) */
  count?: number;
  /** Backwards compatibility properties */
  [key: string]: any;
}

/**
 * Base chart configuration shared by all charts
 */
export interface ChartConfigBase {
  /** Chart title */
  title?: string;
  /** X-axis title */
  axisTitleX?: string;
  /** Y-axis title */
  axisTitleY?: string;
  /** Whether to show a legend */
  showLegend?: boolean;
  /** Width of the plot (null = responsive) */
  width?: number | null;
  /** Fixed aspect ratio (null = flexible) */
  aspectRatio?: number | null;
}

/**
 * Chart type identifiers
 */
export type ChartType = "histogram" | "calibration" | "boxplot" | "timeseries";

/**
 * Histogram chart specific properties
 */
export interface HistogramConfig extends ChartConfigBase {
  type: "histogram";
  /** Number of bins in the histogram */
  binCount?: number;
  /** Start value for the histogram range */
  rangeStart?: number | null;
  /** End value for the histogram range */
  rangeEnd?: number | null;
}

/**
 * Calibration chart specific properties
 */
export interface CalibrationConfig extends ChartConfigBase {
  type: "calibration";
  /** Criterion to use for calibration (e.g., "midpoint", "time-average") */
  criterion?: string | null;
  /** Property to weight points by (e.g., "volume_usd", "traders_count") */
  weight?: string | null;
  /** Domain for X axis (probability) */
  xDomain?: [number, number];
  /** Domain for Y axis (resolution) */
  yDomain?: [number, number];
}

/**
 * Box plot chart specific properties
 */
export interface BoxPlotConfig extends ChartConfigBase {
  type: "boxplot";
  /** Type of score to use (e.g., "brier-midpoint") */
  scoreType?: string;
  /** Start value for the Y-axis range */
  plotRangeStart?: number | null;
  /** End value for the Y-axis range */
  plotRangeEnd?: number | null;
}

/**
 * Time series chart specific properties
 */
export interface TimeSeriesConfig extends ChartConfigBase {
  type: "timeseries";
  /** Time unit for grouping (day, week, month) */
  timeUnit?: "day" | "week" | "month" | "year";
  /** Start date for the chart */
  dateStart?: string | null;
  /** End date for the chart */
  dateEnd?: string | null;
}

/**
 * Union of all chart configurations
 */
export type ChartConfig =
  | HistogramConfig
  | CalibrationConfig
  | BoxPlotConfig
  | TimeSeriesConfig;

/**
 * Histogram data point (pre-processed for rendering)
 */
export interface HistogramDatapoint {
  platformName: string;
  colorPrimary: string;
  colorAccent: string;
  startX: number;
  endX: number;
  count: number;
  percent: number;
}

/**
 * Histogram option specific data
 */
export interface HistogramOption extends ChartOptionBase {
  /** Input items to bin */
  items?: Array<{
    value: number;
    platformSlug: string;
  }>;
  /** Pre-processed points ready for rendering */
  points?: HistogramDatapoint[];
  /** Configuration for the histogram */
  config?: HistogramConfig;
  /** Plot title - for backwards compatibility */
  plotTitle?: string;
  /** X-axis title - for backwards compatibility */
  axisTitleX?: string;
  /** Y-axis title - for backwards compatibility */
  axisTitleY?: string;
}

/**
 * Calibration option specific data
 */
export interface CalibrationOption extends ChartOptionBase {
  /** Markets to use for calibration */
  markets?: MarketDetails[];
  /** Pre-processed calibration points */
  points?: CalibrationPoint[];
  /** Configuration for the calibration plot */
  config?: CalibrationConfig;
  /** X-axis title - for backwards compatibility */
  axisTitleX?: string;
  /** Y-axis title - for backwards compatibility */
  axisTitleY?: string;
}

/**
 * Box plot option specific data
 */
export interface BoxPlotOption extends ChartOptionBase {
  /** Scores to use for the box plot */
  scores?: MarketScoreDetails[];
  /** Pre-processed box plot points */
  points?: AccuracyBoxDatapoint[];
  /** Configuration for the box plot */
  config?: BoxPlotConfig;
  /** X-axis title - for backwards compatibility */
  axisTitleX?: string;
  /** Y-axis title - for backwards compatibility */
  axisTitleY?: string;
  /** Plot Y range - for backwards compatibility */
  plotRangeY?: number[];
}

/**
 * Box plot data point
 */
export interface AccuracyBoxDatapoint {
  platformName: string;
  colorPrimary: string;
  colorAccent: string;
  whiskerStart: number;
  boxStart: number;
  boxCenter: number;
  boxEnd: number;
  whiskerEnd: number;
}

/**
 * Union of all chart option types
 */
export type ChartOption = HistogramOption | CalibrationOption | BoxPlotOption;

/**
 * Chart context holds shared data needed across chart options
 */
export interface ChartContext {
  /** Available platforms */
  platforms: PlatformDetails[];
  /** Chart element ID */
  plotId: string;
}
