// Export all components for easier imports
export { default as SearchBar } from "./SearchBar.svelte";
export { default as FilterControls } from "./FilterControls.svelte";
export { default as LoadingSpinner } from "./LoadingSpinner.svelte";
export { default as ErrorMessage } from "./ErrorMessage.svelte";
export { default as MarketTable } from "./MarketTable.svelte";
export { default as MarketTableLite } from "./MarketTableLite.svelte";
export { default as MarketTableRow } from "./MarketTableRow.svelte";
export { default as MarketBadge } from "./MarketBadge.svelte";
export { default as MarketStats } from "./MarketStats.svelte";
export { default as ActionButtons } from "./ActionButtons.svelte";

export interface HardcodedPlatform {
  slug: string;
  name: string;
  sort: string;
}
export const hardcodedPlatforms: HardcodedPlatform[] = [
  { slug: "kalshi", name: "Kalshi", sort: "volume_usd.desc.nullslast" },
  { slug: "manifold", name: "Manifold", sort: "volume_usd.desc.nullslast" },
  {
    slug: "metaculus",
    name: "Metaculus",
    sort: "traders_count.desc.nullslast",
  },
  {
    slug: "polymarket",
    name: "Polymarket",
    sort: "volume_usd.desc.nullslast",
  },
];

export function getOtherPlatforms(inputSlug: string): HardcodedPlatform[] {
  return hardcodedPlatforms.filter((platform) => platform.slug !== inputSlug);
}

function buildSearchQuery(userQuery: string): string {
  // If the query is empty, return a default or empty query
  const cleanQuery = userQuery.trim();
  if (!cleanQuery) {
    return "";
  }

  // Split query into words, filtering out empty strings
  const words = cleanQuery.split(/\s+/).filter((word) => word.length > 0);

  // For each field, create a condition that each field contains ALL the words
  const fields = ["id", "title", "url", "description"];

  // Create conditions for each field where all words must match
  const fieldConditions = fields.map((field) => {
    // For each word, create a condition that the field contains that word
    const wordConditions = words.map((word) => `${field}.ilike.*${word}*`);

    // Join the word conditions with AND to require all words in this field
    return `and(${wordConditions.join(",")})`;
  });

  // Join field conditions with OR (any field can contain all words)
  return `&or=(${fieldConditions.join(",")})`;
}

export function assembleParamString(
  defaultFilters: boolean,
  searchQuery: string | null,
  platformSlugs: string[] | null,
  order: string,
): string {
  // Initialize params with order and default filters
  let params = `order=${order}`;

  // Add default filters if enabled
  if (defaultFilters) {
    params +=
      "&question_id=is.null&question_dismissed=eq.0&duration_days=gte.4";
  }

  // Build the search query if provided
  if (searchQuery) params += buildSearchQuery(searchQuery);

  // Check if platform slugs are provided and not empty
  if (
    platformSlugs &&
    platformSlugs.length > 0 &&
    !(platformSlugs.length === 1 && platformSlugs[0] === "")
  ) {
    params += `&platform_slug=in.(${platformSlugs.join(",")})`;
  }

  return params;
}

export function updateUrl(
  query: string | null,
  platform: string | null,
  sort: string | null,
) {
  const url = new URL(window.location.href);

  // Update or remove search params based on values
  if (query) url.searchParams.set("q", query);
  else url.searchParams.delete("q");

  if (platform) url.searchParams.set("platform", platform);
  else url.searchParams.delete("platform");

  if (sort) url.searchParams.set("sort", sort);
  else url.searchParams.delete("sort");

  // Update the URL without refreshing the page
  window.history.pushState({}, "", url);
}
