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

  return response.json();
}

export async function deleteItem(
  endpoint: string,
  attr: "ID" | "slug",
  value: string,
): Promise<any> {
  return fetchFromAPI(`${endpoint}?${attr}=eq.${value}`, {
    method: "DELETE",
    headers: {
      Prefer: "return=representation",
    },
  });
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
  return fetchFromAPI("platforms", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updatePlatform(data: Platform): Promise<Platform> {
  return fetchFromAPI(`platforms?slug=eq.${data.slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
      "On-Conflict-Update": "*",
    },
  });
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
  return fetchFromAPI("categories", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updateCategory(data: Category): Promise<Category> {
  return fetchFromAPI(`categories?slug=eq.${data.slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
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

export async function createQuestion(data: Question): Promise<Question> {
  return fetchFromAPI("questions", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
}

export async function updateQuestion(data: Question): Promise<Question> {
  return fetchFromAPI(`questions?id=eq.${data.id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
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
): Promise<MarketDismissStatus> {
  return fetchFromAPI(`market_dismissals?id=eq.${marketId}`, {
    method: "POST",
    body: JSON.stringify({ market_id: marketId, dismissed_status: status }),
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
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
): Promise<MarketQuestionLink> {
  return fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "POST",
    body: JSON.stringify({ market_id: marketId, question_id: questionId }),
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
}

export async function unlinkMarket(
  marketId: string,
): Promise<MarketQuestionLink> {
  return fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "DELETE",
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
}

export async function invertMarketLink(
  marketId: string,
  invert: boolean,
): Promise<MarketQuestionLink> {
  return fetchFromAPI(`market_questions?market_id=eq.${marketId}`, {
    method: "PATCH",
    body: JSON.stringify({ question_invert: invert }),
    headers: {
      Prefer: "return=representation",
    },
  }).then((data) => data[0] || null);
}
