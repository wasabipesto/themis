import type {
  CategoryDetails,
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
  endpoint: string;

  constructor(message: string, status: number, endpoint: string) {
    super(message);
    this.name = "APIError";
    this.status = status;
    this.endpoint = endpoint;
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

      throw new APIError(errorMessage, response.status, endpoint);
    }

    // Handle empty responses
    if (response.headers.get("Content-Length") === "0") {
      throw new APIError(`JSON returned was empty`, response.status, endpoint);
    }

    // Verify content type
    const contentType = response.headers.get("Content-Type") || "";
    if (!contentType.includes("application/json")) {
      throw new APIError(
        `Expected JSON but got ${contentType}`,
        response.status,
        endpoint,
      );
    }

    // Parse JSON
    const data = await response.json();
    return data as T;
  } catch (error: unknown) {
    // Clear timeout if there was an error
    clearTimeout(timeoutId);

    // Handle abort errors (timeouts)
    if (error instanceof Error && error.name === "AbortError") {
      throw new APIError(`Request timeout after ${timeout}ms`, 0, endpoint);
    }

    // Re-throw API errors
    if (error instanceof APIError) {
      throw error;
    }

    // Handle other errors
    const errorMessage =
      error instanceof Error ? error.message : "Unknown error occurred";
    throw new APIError(errorMessage, 0, endpoint);
  }
}

export async function getPlatforms(): Promise<PlatformDetails[]> {
  return fetchFromAPI<PlatformDetails[]>("/platform_details?order=slug");
}

export async function getCategories(): Promise<CategoryDetails[]> {
  return fetchFromAPI<CategoryDetails[]>("/category_details?order=slug");
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
    `/other_scores?item_type=eq.platform?order=platform_slug`,
  );
}

export async function getCategoryOverallScores(): Promise<OtherScoreDetails[]> {
  return fetchFromAPI<OtherScoreDetails[]>(
    `/other_scores?item_type=eq.category?order=category_slug`,
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
    `/market_details?question_id=eq.${question_id}`,
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

export async function getDailyProbabilities(
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
