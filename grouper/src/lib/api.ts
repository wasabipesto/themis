import type {
  Category,
  CategoryDetails,
  DailyProbabilityDetails,
  MarketDetails,
  Platform,
  PlatformDetails,
  Question,
  QuestionDetails,
  MarketQuestionLink,
  MarketDismissStatus,
} from "@types";

const PGRST_URL = import.meta.env.PUBLIC_PGRST_URL;
const PGRST_APIKEY = import.meta.env.PUBLIC_PGRST_APIKEY;

interface FetchOptions extends RequestInit {
  headers?: Record<string, string>;
}

export async function fetchFromAPI(
  endpoint: string,
  options: FetchOptions = {},
) {
  const url = `${PGRST_URL}/${endpoint}`;

  // Create a deep copy of options to avoid modifying the original
  const fetchOptions: FetchOptions = { ...options };

  // Initialize headers properly
  fetchOptions.headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${PGRST_APIKEY}`,
    ...(options.headers || {}),
  };

  const response = await fetch(url, fetchOptions);

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || "API request failed");
  }

  // Check if response has content
  const contentType = response.headers.get("Content-Type");
  const contentLength = response.headers.get("Content-Length");

  // If content length is 0 or content type is not JSON, return empty object
  if (contentLength === "0" || !contentType?.includes("application/json")) {
    return {};
  }

  try {
    return await response.json();
  } catch (error) {
    console.error("Failed to parse JSON:", error);
    return {};
  }
}

export async function refreshViewsAll(): Promise<any> {
  return fetchFromAPI(`rpc/refresh_all_materialized_views`, {
    method: "POST",
  });
}

export async function refreshViewsQuick(): Promise<any> {
  return fetchFromAPI(`rpc/refresh_quick_materialized_views`, {
    method: "POST",
  });
}

export async function deleteItem(
  endpoint: string,
  attr: "ID" | "slug",
  value: string,
): Promise<any> {
  await fetchFromAPI(`${endpoint}?${attr}=eq.${value}`, {
    method: "DELETE",
  });
  return refreshViewsQuick();
}

export async function getPlatformsLite(): Promise<Platform[]> {
  const limit = 100;
  const order = "slug.asc";
  return fetchFromAPI(`platforms?limit=${limit}&order=${order}`);
}

export async function getPlatformsDetailed(): Promise<PlatformDetails[]> {
  const limit = 100;
  const order = "slug.asc";
  return fetchFromAPI(`platform_details?limit=${limit}&order=${order}`);
}

export async function getPlatform(slug: string): Promise<PlatformDetails> {
  return fetchFromAPI(`platform_details?slug=eq.${slug}`).then(
    (data) => data[0] || null,
  );
}

export async function createPlatform(data: Platform): Promise<Platform> {
  await fetchFromAPI("platforms", {
    method: "POST",
    body: JSON.stringify(data),
  });
  await refreshViewsAll();
  return await getPlatform(data.slug);
}

export async function updatePlatform(data: Platform): Promise<Platform> {
  await fetchFromAPI(`platforms?slug=eq.${data.slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      "On-Conflict-Update": "*",
    },
  });
  await refreshViewsAll();
  return await getPlatform(data.slug);
}

export async function getCategories(): Promise<CategoryDetails[]> {
  const limit = 100;
  const order = "slug.asc";
  return fetchFromAPI(`category_details?limit=${limit}&order=${order}`);
}

export async function getCategory(slug: string): Promise<CategoryDetails> {
  return fetchFromAPI(`category_details?slug=eq.${slug}`).then(
    (data) => data[0] || null,
  );
}

export async function createCategory(data: Category): Promise<Category> {
  await fetchFromAPI("categories", {
    method: "POST",
    body: JSON.stringify(data),
  });
  await refreshViewsQuick();
  return await getCategory(data.slug);
}

export async function updateCategory(data: Category): Promise<Category> {
  await fetchFromAPI(`categories?slug=eq.${data.slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
  await refreshViewsQuick();
  return await getCategory(data.slug);
}

export async function getQuestions(): Promise<QuestionDetails[]> {
  const limit = 100;
  const order = "slug.asc";
  return fetchFromAPI(`question_details?limit=${limit}&order=${order}`);
}

export async function getQuestion(id: string): Promise<QuestionDetails> {
  return fetchFromAPI(`question_details?id=eq.${id}`).then(
    (data) => data[0] || null,
  );
}

export async function getQuestionBySlug(
  slug: string,
): Promise<QuestionDetails> {
  return fetchFromAPI(`question_details?slug=eq.${slug}`).then(
    (data) => data[0] || null,
  );
}

export async function createQuestion(data: Question): Promise<Question> {
  await fetchFromAPI("questions", {
    method: "POST",
    body: JSON.stringify(data),
  });
  await refreshViewsQuick();
  return await getQuestionBySlug(data.slug);
}

export async function updateQuestion(data: Question): Promise<Question> {
  await fetchFromAPI(`questions?id=eq.${data.id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  });
  await refreshViewsQuick();
  return await getQuestion(data.id.toString());
}

export async function getMarkets(params: string): Promise<MarketDetails[]> {
  const limit = 100;
  return fetchFromAPI(`market_details?limit=${limit}&${params}`);
}

export async function getMarketsByQuestion(
  questionId: number,
): Promise<MarketDetails[]> {
  const order = "platform_slug.asc";
  return fetchFromAPI(
    `market_details?question_id=eq.${questionId}&order=${order}`,
  );
}

export async function getMarket(id: string): Promise<MarketDetails> {
  return fetchFromAPI(`market_details?id=eq.${id}`).then(
    (data) => data[0] || null,
  );
}

export async function dismissMarket(
  marketId: string,
  status: number,
): Promise<any> {
  await fetchFromAPI(`market_dismissals?id=eq.${marketId}`, {
    method: "POST",
    body: JSON.stringify({ market_id: marketId, dismissed_status: status }),
  });
  await refreshViewsQuick();
  return await getMarket(marketId);
}

export async function getMarketProbs(
  marketId: string,
): Promise<DailyProbabilityDetails[]> {
  const order = "date.asc";
  return fetchFromAPI(
    `daily_probability_details?market_id=eq.${marketId}&order=${order}`,
  );
}

export async function linkMarket(
  marketId: string,
  questionId: number,
): Promise<MarketDetails> {
  await fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "POST",
    body: JSON.stringify({ market_id: marketId, question_id: questionId }),
  });
  await refreshViewsQuick();
  return getMarket(marketId);
}

export async function unlinkMarket(marketId: string): Promise<MarketDetails> {
  await fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "DELETE",
  });
  await refreshViewsQuick();
  return getMarket(marketId);
}

export async function invertMarketLink(
  marketId: string,
  invert: boolean,
): Promise<MarketDetails> {
  await fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "PATCH",
    body: JSON.stringify({ question_invert: invert }),
  });
  await refreshViewsQuick();
  return getMarket(marketId);
}
