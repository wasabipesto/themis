import type {
  CategoryDetails,
  CriterionProbability,
  DailyProbabilityDetails,
  MarketDetails,
  MarketScoreDetails,
  OtherScoreDetails,
  PlatformCategoryScoreDetails,
  PlatformDetails,
  QuestionDetails,
} from "@types";

const PGRST_URL = import.meta.env.PGRST_URL;

class APIError extends Error {
  status: number;
  url: string;

  constructor(message: string, status: number, url: string) {
    const formattedMessage = `Message: ${message}\nStatus: ${status}\nURL: ${url}`;
    super(formattedMessage);

    this.name = "APIError";
    this.status = status;
    this.url = url;
  }
}

export async function fetchFromAPI<T>(
  endpoint: string,
  options: RequestInit = {},
  timeout: number = 10000,
): Promise<T> {
  if (!PGRST_URL) {
    throw new Error("API URL is not configured");
  }

  const url = `${PGRST_URL}/${endpoint.startsWith("/") ? endpoint.slice(1) : endpoint}`;

  // Create an AbortController for timeout handling
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const response = await fetch(url, {
      ...options,
      headers: {
        Accept: "application/json",
        ...options.headers,
      },
      signal: controller.signal,
    });

    // Clear timeout since we got a response
    clearTimeout(timeoutId);

    // Handle non-200 responses
    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage;

      try {
        const errorJson = JSON.parse(errorText);
        errorMessage =
          errorJson.message ||
          errorJson.error ||
          `API request failed with status ${response.status}`;
      } catch {
        errorMessage =
          errorText || `API request failed with status ${response.status}`;
      }

      throw new APIError(errorMessage, response.status, url);
    }

    // Handle empty responses
    if (response.headers.get("Content-Length") === "0") {
      throw new APIError(`JSON returned was empty`, response.status, url);
    }

    // Verify content type
    const contentType = response.headers.get("Content-Type") || "";
    if (!contentType.includes("application/json")) {
      throw new APIError(
        `Expected JSON but got ${contentType}`,
        response.status,
        url,
      );
    }

    // Parse JSON
    const data = await response.json();

    // Check if the result is an empty array
    if (Array.isArray(data) && data.length === 0) {
      throw new APIError(`API returned an empty list`, response.status, url);
    }

    return data as T;
  } catch (error: unknown) {
    // Clear timeout if there was an error
    clearTimeout(timeoutId);

    // Handle abort errors (timeouts)
    if (error instanceof Error && error.name === "AbortError") {
      throw new APIError(`Request timeout after ${timeout}ms`, 0, url);
    }

    // Re-throw API errors
    if (error instanceof APIError) {
      throw error;
    }

    // Handle other errors
    const errorMessage =
      error instanceof Error ? error.message : "Unknown error occurred";
    throw new APIError(errorMessage, 0, url);
  }
}

let cachedPlatforms: PlatformDetails[] | null = null;
export async function getPlatforms(): Promise<PlatformDetails[]> {
  if (cachedPlatforms) {
    return cachedPlatforms;
  }
  console.log("Refreshing platform cache.");
  const platforms = await fetchFromAPI<PlatformDetails[]>(
    "/platform_details?order=slug",
  );
  cachedPlatforms = platforms;
  return platforms;
}

let cachedCategories: CategoryDetails[] | null = null;
export async function getCategories(): Promise<CategoryDetails[]> {
  if (cachedCategories) {
    return cachedCategories;
  }
  console.log("Refreshing category cache.");
  const categories = await fetchFromAPI<CategoryDetails[]>(
    "/category_details?order=slug",
  );
  cachedCategories = categories;
  return categories;
}

export async function getQuestions(): Promise<QuestionDetails[]> {
  return fetchFromAPI<QuestionDetails[]>("/question_details?order=id");
}

export async function getFeaturedQuestions(
  limit: number,
): Promise<QuestionDetails[]> {
  return fetchFromAPI<QuestionDetails[]>(
    `/question_details?order=total_volume.desc&limit=${limit}`,
  );
}

export async function getPlatformCategoryScores(
  score_type: string | null,
): Promise<PlatformCategoryScoreDetails[]> {
  let url = "/platform_category_scores_details?order=category_slug";
  if (score_type) {
    url += `&score_type=eq.${score_type}`;
  }
  return fetchFromAPI<PlatformCategoryScoreDetails[]>(url);
}

export async function getPlatformOverallScores(): Promise<OtherScoreDetails[]> {
  return fetchFromAPI<OtherScoreDetails[]>(
    `/other_scores?item_type=eq.platform&order=item_id`,
  );
}

export async function getCategoryOverallScores(): Promise<OtherScoreDetails[]> {
  return fetchFromAPI<OtherScoreDetails[]>(
    `/other_scores?item_type=eq.category&order=item_id`,
  );
}

export async function getQuestionOverallScores(
  question_id: number | null,
): Promise<OtherScoreDetails[]> {
  let url = "/other_scores?item_type=eq.question";
  if (question_id) {
    url += `&item_id=eq.${question_id}`;
  }
  return fetchFromAPI<OtherScoreDetails[]>(url);
}

export async function getMarketsByQuestion(
  question_id: number,
): Promise<MarketDetails[]> {
  return fetchFromAPI<MarketDetails[]>(
    `/market_details?order=platform_slug&question_id=eq.${question_id}`,
  );
}

let cachedMarkets: MarketDetails[] | null = null;
export async function getMarkets(): Promise<MarketDetails[]> {
  // Return cache if existing
  if (cachedMarkets) {
    return cachedMarkets;
  }

  console.log("Refreshing market cache.");
  const batchSize = 10000;
  let allMarkets: MarketDetails[] = [];
  let offset = 0;
  let hasMoreResults = true;

  while (hasMoreResults) {
    let url = `/market_details?order=id&limit=${batchSize}&offset=${offset}`;
    const batch = await fetchFromAPI<MarketDetails[]>(url);
    allMarkets = [...allMarkets, ...batch];
    offset += batchSize;
    if (batch.length < batchSize) {
      hasMoreResults = false;
    }
  }
  cachedMarkets = allMarkets;
  return allMarkets;
}

let cachedCriterionProbsLoading = false;
let cachedCriterionProbs: CriterionProbability[] | null = null;
export async function getCriterionProbs(
  market_id: string | null,
  criterion_type: string | null,
): Promise<CriterionProbability[]> {
  if (cachedCriterionProbsLoading) {
    console.log("Waiting for criterion probability cache to refresh...");
    await new Promise((resolve) => setTimeout(resolve, 100));
    return getCriterionProbs(market_id, criterion_type);
  }
  if (cachedCriterionProbs) {
    console.log("Doing lookups for cachedCriterionProbs...");
    let criterionProbsFiltered = cachedCriterionProbs.filter(
      (p) =>
        (!market_id || p.market_id === market_id) &&
        (!criterion_type || p.criterion_type === criterion_type),
    );
    if (criterionProbsFiltered.length > 0) {
      return criterionProbsFiltered;
    } else {
      throw new Error(
        `Could not find criterion probability for ${market_id}/${criterion_type}`,
      );
    }
  }

  console.log("Refreshing criterion probability cache.");
  cachedCriterionProbsLoading = true;
  const batchSize = 100000;
  let allCriterionProbs: CriterionProbability[] = [];
  let offset = 0;
  let hasMoreResults = true;

  while (hasMoreResults) {
    let url = `/criterion_probabilities?order=market_id&limit=${batchSize}&offset=${offset}`;
    const batch = await fetchFromAPI<CriterionProbability[]>(url);
    allCriterionProbs = [...allCriterionProbs, ...batch];
    offset += batchSize;
    if (batch.length < batchSize) {
      hasMoreResults = false;
    }
  }
  cachedCriterionProbs = allCriterionProbs;
  cachedCriterionProbsLoading = false;
  console.log(
    `Finished downloading cachedCriterionProbs, ${cachedCriterionProbs.length} items`,
  );
  return allCriterionProbs.filter(
    (p) =>
      (!market_id || p.market_id === market_id) &&
      (!criterion_type || p.criterion_type === criterion_type),
  );
}

export async function getMarketScoresByQuestion(
  question_ids: number[],
  score_type: string | null,
): Promise<MarketScoreDetails[]> {
  let url = `/market_scores_details?question_id=in.(${question_ids.join(",")})&order=platform_slug`;
  if (score_type) {
    url += `&score_type=eq.${score_type}`;
  }
  return fetchFromAPI<MarketScoreDetails[]>(url);
}

export async function getDailyProbabilitiesByQuestion(
  question_id: number,
  start_date_override: string | null,
  end_date_override: string | null,
): Promise<DailyProbabilityDetails[]> {
  let url = `/daily_probability_details?order=date.asc&question_id=eq.${question_id}`;
  if (start_date_override) {
    url += `&date=gte.${start_date_override}`;
  }
  if (end_date_override) {
    url += `&date=lte.${end_date_override}`;
  }
  return fetchFromAPI<DailyProbabilityDetails[]>(url);
}
