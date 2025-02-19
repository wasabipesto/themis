/** All data for a prediction market platform. */
export interface Platform {
  slug: string;
  name: string;
  description: string;
  long_description: string;
  wikipedia_url: string;
  icon_url: string;
  site_url: string;
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
  score_rel: number;
  score_abs: number;
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
  icon_url: string;
  total_markets: number;
  total_traders: number;
  total_volume: number;
}

/** All data for a question (a group of markets). */
export interface Question {
  question_id: string;
  title: string;
  slug: string;
  description: string;
  category_slug: string;
  category_name: string;
  parent_category_slug: string | null;
  parent_category_name: string | null;
  tags: string[];
  start_date: string;
  end_date: string;
  total_traders: number;
  total_volume: number;
  total_duration: number;
  overall_grade: string;
  overall_score_rel: number;
  overall_score_abs: number;
  markets: QMarketScore[];
}

/** Score data for a market's performance within a question. */
type QMarketScore = {
  platform_slug: string;
  platform_name: string;
  market_id: string;
  market_link: string;
  traders: number | null;
  volume: number | null;
  duration: number;
  grade: string;
  score_rel: number;
  score_abs: number;
};

/** Single point on a daily probability plot. */
export interface DailyProbability {
  platform: string;
  date: string;
  prob: number;
}

/** Single point on a caliibration plot. */
export interface CalibrationPoint {
  platform: string;
  x_start: number | null;
  x_center: number;
  x_end: number | null;
  y_start: number | null;
  y_center: number;
  y_end: number | null;
  count: number;
}
