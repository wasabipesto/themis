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
}
export interface PlatformDetails {
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
export interface PlatformScoreDetails {
  platform_slug: string;
  platform_name: string;
  category_slug: string;
  category_name: string;
  score_type: string;
  num_markets: number;
  score: number;
  grade: string;
}

/**
 * All data for a category (a collection of questions).
 * Can be a parent or a child category.
 */
export interface Category {
  slug: string;
  name: string;
  description: string;
  icon: string;
}
export interface CategoryDetails {
  slug: string;
  name: string;
  description: string;
  icon: string;
  total_markets: number;
  total_traders: number;
  total_volume: number;
}

/** All data for a question (a group of markets). */
export interface NewQuestion {
  title: string;
  slug: string;
  description: string;
  category_slug: string;
  start_date_override: string | null;
  end_date_override: string | null;
}
export interface Question {
  id: number;
  title: string;
  slug: string;
  description: string;
  category_slug: string;
  start_date_override: string | null;
  end_date_override: string | null;
}
export interface QuestionDetails {
  id: number;
  title: string;
  slug: string;
  description: string;
  category_slug: string;
  category_name: string;
  start_date_override: string | null;
  end_date_override: string | null;
  total_traders: number;
  total_volume: number;
  total_duration: number;
}

/** Score data for a market's performance within a question. */
export interface MarketScoreDetails {
  score_type: string;
  market_id: string;
  market_title: string;
  market_url: string;
  platform_slug: string;
  platform_name: string;
  question_id: number | null;
  traders_count: number;
  volume_usd: number;
  duration_days: number;
  question_invert: boolean;
  resolution: number;
  score: number;
  grade: string;
}

/** Data for an individual market */
export interface Market {
  id: string;
  title: string;
  url: string;
  description: string;
  platform_slug: string;
  category_slug: string | null;
  open_datetime: string;
  close_datetime: string;
  traders_count: number | null;
  volume_usd: number | null;
  duration_days: number;
  prob_at_midpoint: number;
  prob_time_avg: number;
  resolution: number;
}
export interface MarketDetails {
  id: string;
  title: string;
  url: string;
  description: string;
  platform_slug: string;
  platform_name: string;
  category_slug: string | null;
  category_name: string | null;
  question_id: number | null;
  question_slug: string | null;
  question_title: string | null;
  question_invert: boolean;
  question_dismissed: number;
  open_datetime: string;
  close_datetime: string;
  traders_count: number | null;
  volume_usd: number | null;
  duration_days: number;
  prob_at_midpoint: number;
  prob_time_avg: number;
  resolution: number;
}

/** Market links to questions. **/
export interface MarketQuestionLink {
  market_id: string;
  question_id: number;
  question_invert: boolean;
}

/** Market question dismiss status. **/
export interface MarketDismissStatus {
  market_id: string;
  dismissed_status: number;
}

/** Single point on a daily probability plot. */
export interface DailyProbabilityDetails {
  market_id: string;
  market_title: string;
  platform_slug: string;
  platform_name: string;
  date: string;
  prob: number;
  question_invert: boolean;
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
