import { defineCollection } from "astro:content";
import { fetchFromAPI } from "@lib/api";
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

const platforms = defineCollection({
  loader: async () => {
    const response = await fetchFromAPI<PlatformDetails[]>(
      "/platform_details?order=slug",
    );
    return response.map((i) => ({
      id: i.slug,
      ...i,
    }));
  },
});

const categories = defineCollection({
  loader: async () => {
    const response = await fetchFromAPI<CategoryDetails[]>(
      "/category_details?order=slug",
    );
    return response.map((i) => ({
      id: i.slug,
      ...i,
    }));
  },
});

const questions = defineCollection({
  loader: async () => {
    const response = await fetchFromAPI<QuestionDetails[]>(
      "/question_details?order=id",
    );
    return response.map((i) => ({
      ...i,
      id: i.slug, // ID has to be a string >:(
      question_id: i.id,
    }));
  },
});

const markets = defineCollection({
  loader: async () => {
    console.log("Markets loading...");
    const batchSize = 10_000;
    let responses: MarketDetails[] = [];
    let offset = 0;
    let hasMoreResults = true;
    while (hasMoreResults) {
      let url = `/market_details?order=id&limit=${batchSize}&offset=${offset}`;
      const batch = await fetchFromAPI<MarketDetails[]>(url);
      responses = [...responses, ...batch];
      offset += batchSize;
      if (batch.length < batchSize) {
        hasMoreResults = false;
      }
    }
    console.log(`Markets loaded (${responses.length} items).`);
    return responses;
  },
});

const marketScores = defineCollection({
  loader: async () => {
    console.log("Market scores loading...");
    const batchSize = 100_000;
    let responses: MarketScoreDetails[] = [];
    let offset = 0;
    let hasMoreResults = true;
    while (hasMoreResults) {
      let url = `/market_scores_details?order=platform_slug&limit=${batchSize}&offset=${offset}`;
      const batch = await fetchFromAPI<MarketScoreDetails[]>(url);
      responses = [...responses, ...batch];
      offset += batchSize;
      if (batch.length < batchSize) {
        hasMoreResults = false;
      }
    }
    console.log(`Market scores loaded (${responses.length} items).`);
    return responses.map((i) => ({
      id: `${i.market_id}/${i.score_type}`,
    }));
  },
});

const criterionProbs = defineCollection({
  loader: async () => {
    console.log("Criterion probs loading...");
    const batchSize = 100_000;
    let responses: CriterionProbability[] = [];
    let offset = 0;
    let hasMoreResults = true;
    while (hasMoreResults) {
      let url = `/criterion_probabilities?order=market_id&limit=${batchSize}&offset=${offset}`;
      const batch = await fetchFromAPI<CriterionProbability[]>(url);
      responses = [...responses, ...batch];
      offset += batchSize;
      if (batch.length < batchSize) {
        hasMoreResults = false;
      }
    }
    console.log(`Criterion probs loaded (${responses.length} items).`);
    return responses.map((i) => ({
      id: `${i.market_id}/${i.criterion_type}`,
    }));
  },
});

export const collections = {
  platforms,
  categories,
  questions,
  markets,
  marketScores,
  criterionProbs,
};
