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
import { saveToCache, loadFromCache } from "./cache";

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
  timeout: number = 10_000,
): Promise<T> {
  if (!PGRST_URL) {
    throw new Error("API URL is not configured");
  }

  const url = `${PGRST_URL}/${endpoint.startsWith("/") ? endpoint.slice(1) : endpoint}`;

  // Try the request with optional retry
  return await makeRequest(url, options, timeout, false);
}

// Helper function to make the request with retry capability
async function makeRequest<T>(
  url: string,
  options: RequestInit,
  timeout: number,
  isRetry: boolean,
): Promise<T> {
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
      // If this is already a retry, don't retry again
      if (isRetry) {
        throw new APIError(`Request timeout after retry`, 0, url);
      }

      // First timeout - wait and retry once
      console.log(
        `Request timeout for ${url} after ${timeout}ms, retrying once after 5s wait...`,
      );
      await new Promise((resolve) => setTimeout(resolve, 5_000));

      // Retry the request
      return await makeRequest<T>(url, options, timeout, true);
    }

    // Re-throw API errors
    if (error instanceof APIError) {
      throw error;
    }

    // Handle other errors
    const errorMessage =
      error instanceof Error
        ? error.message
        : isRetry
          ? "Unknown error occurred during retry"
          : "Unknown error occurred";
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

let cachedMarkets: MarketDetails[] | null = null;
export async function getMarkets(): Promise<MarketDetails[]> {
  // Return memory cache if existing
  if (cachedMarkets) {
    return cachedMarkets;
  }

  // Try to load from disk cache
  const diskCache = loadFromCache<MarketDetails[]>("markets");
  if (diskCache) {
    cachedMarkets = diskCache;
    return diskCache;
  }

  console.log("Refreshing market cache.");
  const batchSize = 100_000;
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
  console.log(`Finished downloading markets, ${allMarkets.length} items`);
  cachedMarkets = allMarkets;

  // Save to disk cache
  saveToCache("markets", allMarkets);

  return allMarkets;
}

let cachedCriterionProbs: Map<string, CriterionProbability> = new Map();
let cachedCriterionProbsLoading = false;
let cachedCriterionProbsLoaded = false;
export async function getCriterionProb(
  market_id: string,
  criterion_type: string,
): Promise<CriterionProbability | null> {
  if (cachedCriterionProbsLoading) {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    return getCriterionProb(market_id, criterion_type);
  }
  const key = `${market_id}/${criterion_type}`;
  if (cachedCriterionProbsLoaded) {
    return cachedCriterionProbs.get(key) || null;
  }

  // Try to load from disk cache
  const cachedData =
    loadFromCache<Map<string, CriterionProbability>>("criterion_probs");
  if (cachedData) {
    // Use the Map
    cachedCriterionProbs = cachedData;
    cachedCriterionProbsLoaded = true;
    return cachedCriterionProbs.get(key) || null;
  }

  console.log("Refreshing criterion probability cache.");
  cachedCriterionProbsLoading = true;
  const batchSize = 100_000;
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

  // Pre-filter and cache data into maps for quick access
  const filteredMap: Map<string, CriterionProbability> = new Map();
  allCriterionProbs.forEach((prob) => {
    const criterionKey = `${prob.market_id}/${prob.criterion_type}`;
    filteredMap.set(criterionKey, prob);
  });

  // Cache all pre-filtered results
  filteredMap.forEach((prob, key) => {
    cachedCriterionProbs.set(key, prob);
  });

  console.log(
    `Finished downloading criterion probabilities, ${allCriterionProbs.length} items`,
  );
  cachedCriterionProbsLoading = false;
  cachedCriterionProbsLoaded = true;

  // Save to disk cache
  saveToCache("criterion_probs", cachedCriterionProbs);

  return cachedCriterionProbs.get(key) || null;
}

let cachedMarketScores: MarketScoreDetails[] | null = null;
export async function getMarketScores(): Promise<MarketScoreDetails[]> {
  if (cachedMarketScores) {
    return cachedMarketScores;
  }

  // Try to load from disk cache
  const diskCache = loadFromCache<MarketScoreDetails[]>("market_scores");
  if (diskCache) {
    cachedMarketScores = diskCache;
    return diskCache;
  }

  console.log("Refreshing market scores cache.");
  const batchSize = 100_000;
  let allMarketScores: MarketScoreDetails[] = [];
  let offset = 0;
  let hasMoreResults = true;
  while (hasMoreResults) {
    let url = `/market_scores_details?order=market_id,score_type&limit=${batchSize}&offset=${offset}`;
    const batch = await fetchFromAPI<MarketScoreDetails[]>(url);
    allMarketScores = [...allMarketScores, ...batch];
    offset += batchSize;
    if (batch.length < batchSize) {
      hasMoreResults = false;
    }
  }

  console.log(
    `Finished downloading market scores, ${allMarketScores.length} items`,
  );
  cachedMarketScores = allMarketScores;
  let isMap = false;

  // Save to disk cache
  saveToCache("market_scores", allMarketScores);

  return allMarketScores;
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

let cachedDailyProbabilities: DailyProbabilityDetails[] | null = null;
let cachedDailyProbabilitiesLoading = false;
export async function getAllDailyProbabilities(): Promise<
  DailyProbabilityDetails[]
> {
  // Return memory cache if existing
  if (cachedDailyProbabilities) {
    return cachedDailyProbabilities;
  }

  // If already loading, wait for completion
  if (cachedDailyProbabilitiesLoading) {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    return getAllDailyProbabilities();
  }

  // Try to load from disk cache
  const diskCache = loadFromCache<DailyProbabilityDetails[]>(
    "daily_probabilities",
  );
  if (diskCache) {
    cachedDailyProbabilities = diskCache;
    return diskCache;
  }

  console.log("Refreshing daily probabilities cache.");
  cachedDailyProbabilitiesLoading = true;
  const batchSize = 100_000;
  let allDailyProbabilities: DailyProbabilityDetails[] = [];
  let offset = 0;
  let hasMoreResults = true;

  try {
    while (hasMoreResults) {
      let url = `/daily_probability_details?order=market_id,date&limit=${batchSize}&offset=${offset}`;
      const batch = await fetchFromAPI<DailyProbabilityDetails[]>(url);
      allDailyProbabilities = [...allDailyProbabilities, ...batch];
      offset += batchSize;
      if (batch.length < batchSize) {
        hasMoreResults = false;
      }
    }

    console.log(
      `Finished downloading daily probabilities, ${allDailyProbabilities.length} items`,
    );
    cachedDailyProbabilities = allDailyProbabilities;

    // Save to disk cache
    saveToCache("daily_probabilities", allDailyProbabilities);

    return allDailyProbabilities;
  } catch (error) {
    console.error(`Error refreshing daily probabilities cache: ${error}`);
    throw error;
  } finally {
    cachedDailyProbabilitiesLoading = false;
  }
}

export async function getDailyProbabilitiesByQuestion(
  question_id: number,
  start_date_override: string | null,
  end_date_override: string | null,
): Promise<DailyProbabilityDetails[]> {
  // First try to get all daily probabilities from cache
  const allProbabilities = await getAllDailyProbabilities();

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
