export interface Platform {
  name_lowercase: string;
  name_display: string;
  slug: string;
  description: string;
  icon_url: string;
  site_url: string;
  color_primary: string;
  color_accent: string;
}

export interface Category {
  name_display: string;
  slug: string;
  parent_slug: string | null;
  description: string;
  icon_url: string;
}

export interface Question {
  questionId: string;
  title: string;
  slug: string;
  description: string;
  parentCategory: string;
  subCategory: string | null;
  tags: string[];
  totalTraders: number;
  totalVolume: number;
  totalDuration: number;
  startDate: string;
  endDate: string;
  markets: ScoredMarket[];
  overall: ScoreOverall;
}

type ScoredMarket = {
  platform: string;
  marketId: string;
  link: string;
  traders: number | null;
  volume: number | null;
  duration: number;
  grade: string;
  scoreRel: number;
  scoreAbs: number;
};

type ScoreOverall = {
  grade: string;
  scoreRel: number;
  scoreAbs: number;
};

export interface DailyProbability {
  platform: string;
  date: string;
  prob: number;
}

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
