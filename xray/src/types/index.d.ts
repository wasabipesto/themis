// Themis common types - TypeScript definitions
// Generated from Rust types in common/src/lib.rs

// ============================================================================
// Wrapper Types
// ============================================================================

/** URL wrapper type */
export type Url = string;

/** Formatted platform name */
export type PlatformName = string;

/** Slugified platform name */
export type PlatformSlug = string;

/** Disambiguated market identifier */
export type MarketId = string;

/** Market title/question text */
export type MarketTitle = string;

/** Generic label text */
export type Label = string;

/** Probability value constrained to [0.0, 1.0] */
export type Probability = number;

// ============================================================================
// Enums
// ============================================================================

/** A specific market platform */
export enum Platform {
    Kalshi = "Kalshi",
    Manifold = "Manifold",
    Metaculus = "Metaculus",
    Polymarket = "Polymarket",
}

/** Type of prediction market structure */
export enum MarketType {
    Binary = "Binary",
    DiscreteOne = "DiscreteOne",
    DiscreteMulti = "DiscreteMulti",
    Continuous = "Continuous",
    Numeric = "Numeric",
    Date = "Date",
}

/** Current status of a prediction market */
export enum MarketStatus {
    PreOpen = "PreOpen",
    Open = "Open",
    Closed = "Closed",
    Resolved = "Resolved",
    Cancelled = "Cancelled",
}

/** Type of confidence factor impact */
export enum ImpactType {
    Positive = "Positive",
    Negative = "Negative",
    Neutral = "Neutral",
}

// ============================================================================
// Outcome Types
// ============================================================================

/** Binary outcome with a single probability */
export interface OutcomeBinary {
    probability: Probability;
}

/** Discrete outcome with label and probability, usually in sets */
export interface OutcomeDiscrete {
    label: Label;
    probability: Probability;
}

/** Continuous outcome with a probability distribution */
export interface OutcomeContinuous {
    label: Label;
    continuous_min: number;
    continuous_peak: number;
    continuous_max: number;
    probability: Probability;
}

/** Numeric outcome with strike range */
export interface OutcomeNumeric {
    label: Label;
    numerical_strike_low: number;
    numerical_strike_midpoint: number;
    numerical_strike_high: number;
    probability: Probability;
}

/** Date-based outcome with time range */
export interface OutcomeDate {
    label: Label;
    date_strike_low: string; // ISO 8601 datetime string
    date_strike_midpoint: string; // ISO 8601 datetime string
    date_strike_high: string; // ISO 8601 datetime string
    probability: Probability;
}

/** Represents different types of market outcomes */
export type Outcomes =
    | { None: null }
    | { Binary: OutcomeBinary }
    | { Discrete: OutcomeDiscrete[] }
    | { Continuous: OutcomeContinuous }
    | { Numeric: OutcomeNumeric[] }
    | { Date: OutcomeDate[] };

// ============================================================================
// Market Data
// ============================================================================

/** Complete market data including metadata and outcomes */
export interface MarketData {
    platform_name: PlatformName;
    title: MarketTitle;
    url: Url;
    description: string;
    market_status: MarketStatus;
    market_type: MarketType;
    volume_usd: number | null;
    unique_traders: number | null;
    open_datetime: string; // ISO 8601 datetime string
    expected_close_datetime: string; // ISO 8601 datetime string
    last_updated_datetime: string; // ISO 8601 datetime string
    tags: string[];
    outcomes: Outcomes;
}

// ============================================================================
// X-Ray Analysis Types
// ============================================================================

/** Overall assessment summary for X-Ray analysis */
export interface XRayAssessment {
    title: string;
    description: string;
    confidence_percent: number;
}

/** Reference link with context */
export interface Link {
    preface: string | null;
    text: string;
    url: Url;
}

/** Individual factor affecting confidence in market analysis */
export interface XRayConfidenceAspect {
    icon: string;
    title: string;
    aspect_name: string;
    impact_type: ImpactType;
    impact_amount: number;
    description: string;
    links: Link[];
}

/** Single data point in a time series chart */
export interface HistoryChartPoint {
    series_label: Label;
    series_color: string;
    point_datetime: string; // ISO 8601 datetime string
    point_value: number;
}

/** Historical data chart with multiple series */
export interface HistoryChart {
    title: string;
    points: HistoryChartPoint[];
}

/** Complete X-Ray analysis response including market data and insights */
export interface XRayAnalysis {
    market: MarketData;
    assessment: XRayAssessment;
    confidence_aspects: XRayConfidenceAspect[];
    history_chart: HistoryChart;
    similar_markets: MarketData[];
}
