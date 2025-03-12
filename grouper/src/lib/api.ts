import type { Category, Platform, Question } from "@types";

const PGRST_URL = import.meta.env.PUBLIC_PGRST_URL;
const PGRST_APIKEY = import.meta.env.PUBLIC_PGRST_APIKEY;

export async function fetchFromAPI(endpoint: string, options = {}) {
  const url = `${PGRST_URL}/${endpoint}`;

  // Create a deep copy of options to avoid modifying the original
  const fetchOptions = { ...options };

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

export async function getItemsSorted(endpoint: string): Promise<any> {
  return fetchFromAPI(`${endpoint}?order=slug.asc`);
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

export async function getPlatform(slug: string): Promise<Platform> {
  return fetchFromAPI(`platforms?slug=eq.${slug}`).then(
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

export async function getCategories(): Promise<Category[]> {
  return fetchFromAPI(`categories`);
}

export async function getCategory(slug: string): Promise<Category> {
  return fetchFromAPI(`categories?slug=eq.${slug}`).then(
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

export async function getQuestion(id: string): Promise<Question> {
  return fetchFromAPI(`questions?id=eq.${id}`).then((data) => data[0] || null);
}

export async function createQuestion(data: Question): Promise<Question> {
  return fetchFromAPI("questions", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updateQuestion(
  id: string,
  data: Question,
): Promise<Question> {
  return fetchFromAPI(`questions?id=eq.${id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}
