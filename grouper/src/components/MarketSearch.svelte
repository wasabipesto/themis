<script lang="ts">
  import type { MarketDetails, Platform } from "@types";
  import { onMount } from "svelte";
  import { getMarkets, dismissMarket, getPlatformsLite } from "@lib/api";
  import {
    SearchBar,
    FilterControls,
    LoadingSpinner,
    ErrorMessage,
    MarketTable,
    updateUrl,
    assembleParamString,
  } from "./market-search";

  // Initial state
  let markets: MarketDetails[] = [];
  let platforms: Platform[] = [];
  let loading = true;
  let error: string | null = null;
  let searchQuery = "";
  let selectedPlatformSlug = "";
  let selectedSort = "volume_usd.desc.nullslast";
  let defaultFilters = true;

  onMount(async () => {
    // Get initial values from URL
    const urlParams = new URLSearchParams(window.location.search);
    searchQuery = urlParams.get("q") || "";
    selectedPlatformSlug = urlParams.get("platform") || "";
    selectedSort = urlParams.get("sort") || "volume_usd.desc.nullslast";

    // Load platforms for the dropdown
    try {
      platforms = await getPlatformsLite();
      await loadTableData();
    } catch (err) {
      console.error("Error loading platforms:", err);
    }
  });

  async function loadTableData(
    query = searchQuery,
    platformSlug = selectedPlatformSlug,
    sort = selectedSort,
  ) {
    loading = true;

    // Update URL with current search parameters
    updateUrl(query, platformSlug, sort);

    // Base query parameters
    console.log([platformSlug]);
    let params = assembleParamString(
      defaultFilters,
      query,
      [platformSlug],
      sort,
    );

    try {
      markets = await getMarkets(params);
      error = markets.length === 0 ? "No items found." : null;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Error loading table data.";
    } finally {
      loading = false;
    }
  }

  function handleSearch() {
    loadTableData(searchQuery, selectedPlatformSlug, selectedSort);
  }

  async function handleDismiss(marketId: string, level: number) {
    // Store the original markets state to restore in case of error
    const originalMarkets = [...markets];

    try {
      // Optimistically update the UI first
      markets = markets.filter((item) => item.id !== marketId);
      if (markets.length === 0) {
        error = "No items found.";
      }

      // Then perform the actual API call
      await dismissMarket(marketId, level);
      // If successful, we don't need to do anything else since UI is already updated
    } catch (err: unknown) {
      // If the API call fails, revert to the original state
      markets = originalMarkets;
      error = err instanceof Error ? err.message : "Error dismissing market";
    }
  }
</script>

<div class="w-full mb-4 mx-auto">
  <SearchBar bind:searchQuery onSearch={handleSearch} />
  <FilterControls
    {platforms}
    bind:selectedPlatform={selectedPlatformSlug}
    bind:selectedSort
    bind:defaultFilters
    onChange={handleSearch}
  />
</div>

<div class="w-6xl mx-auto">
  {#if loading}
    <LoadingSpinner />
  {:else if error}
    <ErrorMessage message={error} />
  {:else}
    <MarketTable {markets} onDismiss={handleDismiss} />
  {/if}
</div>
