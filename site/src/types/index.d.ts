/** All data for a prediction market platform. */
export interface Platform {
  slug: string;
  name: string;
  description: string;
  long_description: string;
  icon_url: string;
  site_url: string;
  wikipedia_url: string;
  color_primary: string;
  color_accent: string;
  total_markets: number;
  total_traders: number;
  total_volume: number;
}

/** Score data for a platform's performance within a category, or overall. */
export interface PlatformScore {
  platform_slug: string;
  platform_name: string;
  category_slug: string;
  category_name: string;
  num_markets: number;
  grade: string;
  brier_score_rel: number;
  brier_score_abs: number;
}

/**
 * All data for a category (a collection of questions).
 * Can be a parent or a child category.
 */
export interface Category {
  slug: string;
  name: string;
  description: string;
  parent_slug: string | null;
  is_parent: boolean;
  icon: string;
  total_markets: number;
  total_traders: number;
  total_volume: number;
}

/** All data for a question (a group of markets). */
export interface Question {
  id: number;
  title: string;
  slug: string;
  description: string;
  category_slug: string;
  category_name: string;
  parent_category_slug: string | null;
  parent_category_name: string | null;
  start_date_override: string | null;
  end_date_override: string | null;
  total_traders: number;
  total_volume: number;
  total_duration: number;
  overall_grade: string;
  overall_brier_score_rel: number;
  overall_brier_score_abs: number;
  markets: QMarketScore[] | null;
}

/** Score data for a market's performance within a question. */
export interface QMarketScore {
  question_id: number;
  platform_slug: string;
  platform_name: string;
  market_id: string;
  market_link: string;
  traders: number;
  volume: number;
  duration: number;
  grade: string;
  brier_score_rel: number;
  brier_score_abs: number;
}

/** Data for an individual market */
export interface Market {
  id: string;
  title: string;
  platform_slug: string;
  platform_name: string;
  description: string;
  question_id: number | null;
  question_invert: boolean;
  question_dismissed: number;
  url: string;
  open_datetime: string;
  close_datetime: string;
  traders_count: number | null;
  volume_usd: number | null;
  duration_days: number;
  category: string | null;
  prob_at_midpoint: number;
  prob_time_avg: number;
  resolution: number;
}

/** Single point on a daily probability plot. */
export interface DailyProbability {
  market_id: string;
  platform_slug: string;
  date: string;
  prob: number;
}

/** Single point on a calibration plot. */
export interface CalibrationPoint {
  platform_slug: string;
  x_start: number | null;
  x_center: number;
  x_end: number | null;
  y_start: number | null;
  y_center: number;
  y_end: number | null;
  count: number | null;
}
