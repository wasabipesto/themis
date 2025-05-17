import type {
  CategoryDetails,
  DailyProbabilityDetails,
  CriterionProbability,
  MarketDetails,
  MarketScoreDetails,
  OtherScoreDetails,
  PlatformCategoryScoreDetails,
  PlatformDetails,
  QuestionDetails,
  SimilarQuestions,
} from "@types";
import { getOrFetchData } from "./cache";

const PGRST_URL = import.meta.env.PGRST_URL;

class APIError extends Error {
  status: number;
  url: string;

  constructor(message: string, status: number, url: string) {
    const formattedMessage = `\n\nMessage: ${message}\nStatus: ${status}\nURL: ${url}`;
    super(formattedMessage);

    this.name = "APIError";
    this.status = status;
    this.url = url;
  }
}

export async function fetchFromAPI<T>(
  endpoint: string,
  options: RequestInit = {},
  timeout: number = 10_000,
): Promise<T> {
  if (!PGRST_URL) {
    throw new Error("API URL is not configured");
  }

  const url = `${PGRST_URL}/${endpoint.startsWith("/") ? endpoint.slice(1) : endpoint}`;

  // Try the request with retries and exponential backoff
  return await makeRequest(url, options, timeout, 0);
}

export async function fetchAllPaginatedResults<T>(
  endpoint: string,
  options: {
    orderBy?: string;
    batchSize?: number;
    additionalParams?: Record<string, string>;
  } = {},
): Promise<T[]> {
  const {
    orderBy = "id",
    batchSize = 100_000,
    additionalParams = {},
  } = options;
  let allItems: T[] = [];
  let offset = 0;
  let hasMoreResults = true;

  while (hasMoreResults) {
    // Build the query parameters
    // Handle orderBy parameter separately to prevent comma encoding
    const queryParams = new URLSearchParams({
      order: orderBy,
      limit: batchSize.toString(),
      offset: offset.toString(),
      ...additionalParams,
    });

    let url = `/${endpoint}?order=${orderBy}&${queryParams.toString()}`;
    const batch = await fetchFromAPI<T[]>(url);
    allItems = [...allItems, ...batch];
    offset += batchSize;
    if (batch.length < batchSize) {
      hasMoreResults = false;
    }
  }

  return allItems;
}

// Helper function to make the request with retry capability
async function makeRequest<T>(
  url: string,
  options: RequestInit,
  timeout: number,
  retryCount: number,
  maxRetries: number = 5,
  baseDelayMs: number = 1000,
): Promise<T> {
  // Create an AbortController for timeout handling
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    // console.log(`Fetching ${url}`);
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

    // Determine if we should retry (either AbortError or network errors with status 0)
    const isAbortError = error instanceof Error && error.name === "AbortError";
    const isNetworkError = error instanceof APIError && error.status === 0;

    if (isAbortError || isNetworkError) {
      // Check if we've reached the max retry count
      if (retryCount >= maxRetries) {
        const errorType = isAbortError ? "timeout" : "network failure";
        throw new APIError(
          `Request failed due to ${errorType} after ${maxRetries} retries`,
          0,
          url,
        );
      }

      // Calculate exponential backoff delay: baseDelay * 2^retryCount
      const delayMs = baseDelayMs * Math.pow(2, retryCount);

      // Log the retry attempt
      const errorType = isAbortError ? "timeout" : "network failure";
      console.log(
        `Request failed for ${url} due to ${errorType}, retry ${retryCount + 1}/${maxRetries} after ${delayMs}ms wait...`,
      );

      // Wait with exponential backoff
      await new Promise((resolve) => setTimeout(resolve, delayMs));

      // Retry the request with incremented retry count
      return await makeRequest<T>(
        url,
        options,
        timeout,
        retryCount + 1,
        maxRetries,
        baseDelayMs,
      );
    }

    // Re-throw API errors
    if (error instanceof APIError) {
      throw error;
    }

    // Handle other errors
    const errorMessage =
      error instanceof Error
        ? error.message
        : `Unknown error occurred during retry ${retryCount}`;
    throw new APIError(errorMessage, 0, url);
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
  categorySlug?: string,
): Promise<QuestionDetails[]> {
  const url = `/question_details?order=hotness_score.desc&limit=${limit}`;
  if (categorySlug) {
    return fetchFromAPI<QuestionDetails[]>(
      `${url}&category_slug=eq.${categorySlug}`,
    );
  }
  return fetchFromAPI<QuestionDetails[]>(url);
}

export async function getTopQuestionsForPlatform(
  limit: number,
  platformSlug: string,
): Promise<QuestionDetails[]> {
  const scores = await fetchFromAPI<MarketScoreDetails[]>(
    `/market_scores_details?limit=${limit}&platform_slug=eq.${platformSlug}&score_type=eq.brier-relative&order=score.asc`,
  );
  return await fetchFromAPI<QuestionDetails[]>(
    `/question_details?order=hotness_score.desc&id=in.(${scores.map((s) => s.question_id).join(",")})`,
  );
}

export async function getSimilarQuestions(
  questionId: number,
  limit: number,
): Promise<QuestionDetails[]> {
  const similarItems = await fetchFromAPI<SimilarQuestions[]>(
    `/rpc/find_similar_questions_by_id?target_question_id=${questionId}&threshold=1&limit=${limit}`,
  );
  let result: QuestionDetails[] = [];
  for (const item of similarItems) {
    const details = await fetchFromAPI<QuestionDetails[]>(
      `/question_details?id=eq.${item.question_id}`,
    );
    result.push(details[0]);
  }
  return result;
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
  const allMarkets = await getMarkets();
  const filteredMarkets = allMarkets.filter(
    (market) => market.question_id === question_id,
  );
  if (filteredMarkets.length === 0) {
    throw new Error(`No markets found for question ID ${question_id}`);
  }
  return filteredMarkets;
}

export async function getMarkets(): Promise<MarketDetails[]> {
  return getOrFetchData<MarketDetails[]>("markets", async () => {
    return fetchAllPaginatedResults<MarketDetails>("market_details", {
      orderBy: "id",
      batchSize: 100_000,
    });
  });
}

export async function getCriterionProb(
  market_id: string,
  criterion_type: string,
): Promise<CriterionProbability | null> {
  // Get the map of all criterion probabilities
  const criterionProbsMap = await getOrFetchData<
    Map<string, CriterionProbability>
  >("criterion_probs", async () => {
    // Download all items
    const items = await fetchAllPaginatedResults<CriterionProbability>(
      "criterion_probabilities",
      {
        orderBy: "market_id",
        batchSize: 100_000,
      },
    );
    // Convert to cache for easier lookup
    const map: Map<string, CriterionProbability> = new Map();
    items.forEach((i) => {
      const key = `${i.market_id}/${i.criterion_type}`;
      map.set(key, i);
    });
    return map;
  });

  const key = `${market_id}/${criterion_type}`;
  return criterionProbsMap.get(key) || null;
}

export async function getMarketScores(): Promise<MarketScoreDetails[]> {
  return getOrFetchData<MarketScoreDetails[]>("market_scores", async () => {
    return fetchAllPaginatedResults<MarketScoreDetails>(
      "market_scores_details",
      {
        orderBy: "market_id,score_type",
        batchSize: 100_000,
      },
    );
  });
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

export async function getAllDailyProbabilities(): Promise<
  DailyProbabilityDetails[]
> {
  return getOrFetchData<DailyProbabilityDetails[]>(
    "daily_probabilities",
    async () => {
      return fetchAllPaginatedResults<DailyProbabilityDetails>(
        "daily_probability_details",
        {
          orderBy: "market_id,date",
          batchSize: 100_000,
        },
      );
    },
  );
}

export async function getAllDailyProbabilitiesLinked(): Promise<
  DailyProbabilityDetails[]
> {
  return getOrFetchData<DailyProbabilityDetails[]>(
    "daily_probabilities_linked",
    async () => {
      const allDailyProbabilities = await getAllDailyProbabilities();
      const linkedProbabilities = allDailyProbabilities.filter(
        (prob) => prob.question_id !== null,
      );
      return linkedProbabilities;
    },
  );
}

export async function getDailyProbabilitiesByQuestion(
  question_id: number,
  start_date_override: string | null,
  end_date_override: string | null,
): Promise<DailyProbabilityDetails[]> {
  // Get linked probabilities, from cache or database as necessary
  const allProbabilities = await getAllDailyProbabilitiesLinked();

  // Filter by question_id
  let filteredProbabilities = allProbabilities.filter(
    (prob) => prob.question_id === question_id,
  );

  // Apply date filters if provided
  if (start_date_override) {
    filteredProbabilities = filteredProbabilities.filter(
      (prob) => prob.date >= start_date_override,
    );
  }

  if (end_date_override) {
    filteredProbabilities = filteredProbabilities.filter(
      (prob) => prob.date <= end_date_override,
    );
  }

  // If we have results from cache, return them
  if (filteredProbabilities.length > 0) {
    return filteredProbabilities;
  } else {
    throw new Error(
      "No matching data found in daily_probability_details cache",
    );
  }
}

export async function getQuestionStats(): Promise<{
  numQuestions: number;
  numLinkedMarkets: number;
}> {
  const numQuestions = await fetchFromAPI<[{ count: number }]>(
    "/question_details?select=count",
  );
  const numLinkedMarkets = await fetchFromAPI<[{ count: number }]>(
    "/market_details?question_id=not.is.null&select=count",
  );

  return {
    numQuestions: numQuestions[0].count,
    numLinkedMarkets: numLinkedMarkets[0].count,
  };
}
