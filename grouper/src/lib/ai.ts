const OLLAMA_URL = import.meta.env.PUBLIC_OLLAMA_URL;
const OLLAMA_MODEL = import.meta.env.PUBLIC_OLLAMA_MODEL;

import { getCategories, getQuestions } from "@lib/api";
import type { MarketDetails } from "@types";

export async function queryOllama(prompt: string): Promise<string> {
  try {
    const response = await fetch(OLLAMA_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        model: OLLAMA_MODEL,
        prompt: prompt,
        stream: false,
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    const data = await response.json();
    return data.response;
  } catch (error) {
    console.error("Error querying Ollama:", error);
    throw error;
  }
}

export async function llmGetKeywords(input: string): Promise<string> {
  const response = await queryOllama(
    `Extract the most important 2 or 3 keywords from the following text. Return the keywords as a comma-separated list. Do not include any other text. Text input: ${input}`,
  );
  return response.replace(/, /g, " ").replace(/,/g, " ").trim();
}

export async function llmGetCategory(input: string): Promise<string | null> {
  // Get live categories for comparison
  const categories = await getCategories();
  const categorySlugs = categories.map((category) => category.slug);

  // Ask the oracle
  const response = await queryOllama(
    `Categorize the following text into one of the provided categories. Categories: ${categorySlugs.join(", ")}. Text input: ${input}`,
  );

  for (const slug of categorySlugs) {
    // Look for the slug as a whole word using regex
    const regex = new RegExp(`\\b${slug}\\b`, "i"); // case insensitive
    if (regex.test(response)) {
      return slug;
    }
  }

  // If no category slug is found in the response
  return null;
}

export async function llmSlugify(market: MarketDetails): Promise<string> {
  // Get market category
  const category =
    market.category_slug || (await llmGetCategory(market.title)) || null;
  const questionParam = category ? `question_name=eq.${market.title}` : null;

  // Get live slugs for comparison
  const questions = await getQuestions(questionParam, 10, "volume_usd.asc");
  const questionSlugs = questions.map((q) => q.slug);

  const response = await queryOllama(
    `Generate a slug from the provided text similar to the given examples. Examples: ${questionSlugs.join(", ")}. Text input: ${market.title}`,
  );
  return response;
}
