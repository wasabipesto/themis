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
  start_date_actual: string;
  end_date_actual: string;
  market_count: number;
  total_traders: number;
  total_volume: number;
  total_duration: number;
  hotness_score: number;
}
export interface SimilarQuestions {
  question_id: number;
  cosine_distance: number;
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
  resolution: number;
}
export interface SimilarMarkets {
  id: string;
  title: string;
  url: string;
  platform_slug: string;
  platform_name: string;
  category_slug: string | null;
  category_name: string | null;
  question_id: number | null;
  question_invert: boolean;
  question_dismissed: number;
  open_datetime: string;
  close_datetime: string;
  traders_count: number | null;
  volume_usd: number | null;
  duration_days: number;
  resolution: number;
  cosine_distance: number;
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

/** Score data for a market's performance within a question. */
export interface MarketScoreDetails {
  score_type: string;
  market_id: string;
  market_title: string;
  market_url: string;
  platform_slug: string;
  platform_name: string;
  category_slug: string | null;
  question_id: number | null;
  open_datetime: string;
  close_datetime: string;
  traders_count: number;
  volume_usd: number;
  duration_days: number;
  question_invert: boolean;
  resolution: number;
  score: number;
  grade: string;
}

/** Score data for a platform's performance within a category. */
export interface PlatformCategoryScoreDetails {
  platform_slug: string;
  platform_name: string;
  category_slug: string;
  category_name: string;
  score_type: string;
  num_markets: number;
  score: number | null;
  grade: string | null;
}

/** Score data for platforms, categories, or questions. */
export interface OtherScoreDetails {
  item_type: string;
  item_id: string;
  score_type: string;
  num_markets: number;
  score: number | null;
  grade: string | null;
}

/** Single point on a daily probability plot. */
export interface DailyProbabilityDetails {
  market_id: string;
  market_title: string;
  platform_slug: string;
  platform_name: string;
  question_id: number | null;
  question_invert: boolean;
  date: string; // ISO datetime, noon UTC on day of sample
  prob: number;
}

/** Probability point used for calibration plot binning. **/
export interface CriterionProbability {
  market_id: string;
  criterion_type: string;
  prob: number;
}

/** A user signed up for the newsletter. **/
export interface NewsletterSignup {
  email: string;
  date: string; // ISO datetime
}

/** Feedback submitted through the website form. **/
export interface FeedbackItem {
  email: string;
  feedback_type: string;
  feedback: string;
  date: string; // ISO datetime
}
