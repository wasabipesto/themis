const PGRST_URL = import.meta.env.PUBLIC_PGRST_URL;
const PGRST_APIKEY = import.meta.env.PUBLIC_PGRST_APIKEY;

export async function fetchFromAPI(endpoint, options = {}) {
  const url = `${PGRST_URL}/${endpoint}`;
  console.log(PGRST_APIKEY);

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

export async function getPlatforms(params = {}) {
  const queryParams = new URLSearchParams(params).toString();
  return fetchFromAPI(`platforms?${queryParams}`);
}

export async function getPlatform(slug) {
  return fetchFromAPI(`platforms?slug=eq.${slug}`).then(
    (data) => data[0] || null,
  );
}

export async function createPlatform(data) {
  return fetchFromAPI("platforms", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updatePlatform(slug, data) {
  return fetchFromAPI(`platforms?slug=eq.${slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function deletePlatform(slug) {
  return fetchFromAPI(`platforms?slug=eq.${slug}`, {
    method: "DELETE",
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function getCategories(params = {}) {
  const queryParams = new URLSearchParams(params).toString();
  return fetchFromAPI(`categories?${queryParams}`);
}

export async function getCategory(slug) {
  return fetchFromAPI(`categories?slug=eq.${slug}`).then(
    (data) => data[0] || null,
  );
}

export async function createCategory(data) {
  return fetchFromAPI("categories", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updateCategory(slug, data) {
  return fetchFromAPI(`categories?slug=eq.${slug}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function deleteCategory(slug) {
  return fetchFromAPI(`categories?slug=eq.${slug}`, {
    method: "DELETE",
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function getQuestions(params = {}) {
  const queryParams = new URLSearchParams(params).toString();
  return fetchFromAPI(`questions?${queryParams}`);
}

export async function getQuestion(id) {
  return fetchFromAPI(`questions?id=eq.${id}`).then((data) => data[0] || null);
}

export async function createQuestion(data) {
  return fetchFromAPI("questions", {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function updateQuestion(id, data) {
  return fetchFromAPI(`questions?id=eq.${id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
    headers: {
      Prefer: "return=representation",
    },
  });
}

export async function deleteQuestion(id) {
  return fetchFromAPI(`questions?id=eq.${id}`, {
    method: "DELETE",
    headers: {
      Prefer: "return=representation",
    },
  });
}
