// Export all components for easier imports
export { default as MarketDetailsCard } from "./MarketDetailsCard.svelte";
export { default as MarketProbabilityChart } from "./MarketProbabilityChart.svelte";
export { default as QuestionLinkCard } from "./QuestionLinkCard.svelte";

export function slugify(text: string) {
  return text
    .toString()
    .toLowerCase()
    .replace(/\s+/g, "-") // Replace spaces with -
    .replace(/[^\w\-]+/g, "") // Remove all non-word chars
    .replace(/\-\-+/g, "-") // Replace multiple - with single -
    .replace(/^-+/, "") // Trim - from start of text
    .replace(/-+$/, ""); // Trim - from end of text
}
