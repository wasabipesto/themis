<script lang="ts">
  import type {
    Question,
    MarketDetails,
    DailyProbabilityDetails,
  } from "@types";
  import {
    SearchBar,
    LoadingSpinner,
    ErrorMessage,
    MarketTableLite,
    updateUrl,
    assembleParamString,
    getOtherPlatforms,
  } from "./market-search";
  import { onMount, afterUpdate } from "svelte";
  import { getMarket, getQuestion, getMarketProbs, getMarkets } from "@lib/api";
  import * as Plot from "@observablehq/plot";

  // Market view
  let marketId: string | null = null;
  let market: MarketDetails | null = null;
  let question: Question | null = null;
  let marketLoading = true;
  let error: string | null = null;

  // Plot data
  let plotData: DailyProbabilityDetails[] = [];
  let plotLoading = true;
  let plotRendered = false;

  // Search sidebar
  let platformSlugs: string[] = [];
  let platformSort = "volume_usd.desc.nullslast";
  let searchQuery = "";
  let searchLoading = true;
  let searchError: string | null = null;
  let marketSearchResults: MarketDetails[] = [];

  onMount(async () => {
    try {
      // Get initial values from URL
      const urlParams = new URLSearchParams(window.location.search);
      marketId = urlParams.get("id");
      searchQuery = urlParams.get("q") || "";

      if (!marketId) {
        error = "No market ID provided";
        marketLoading = false;
        return;
      }

      // Fetch market data
      market = await getMarket(marketId);

      // Fetch question data
      if (market.question_id) {
        question = await getQuestion(market.question_id.toString());
      }

      // Fetch market probability data for plotting
      try {
        plotData = await getMarketProbs(marketId);
        renderPlot();
      } catch (err) {
        console.error("Failed to load probability data:", err);
      }

      // Fetch initial search results
      try {
        platformSlugs = getOtherPlatforms(market.platform_slug).map(
          (platform) => platform.slug,
        );
        platformSort = getOtherPlatforms(market.platform_slug)[0].sort;
        await loadTableData(searchQuery, platformSlugs, platformSort);
      } catch (err) {
        console.error("Error loading platforms:", err);
      }

      marketLoading = false;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to load market data";
      marketLoading = false;
    }
  });

  afterUpdate(() => {
    if (market && plotData && plotData.length > 0 && !plotRendered) {
      renderPlot();
    }
  });

  async function loadTableData(
    query: string | null,
    platformSlugs: string[] | null,
    sort: string,
  ) {
    searchLoading = true;

    // Update URL with current search parameters
    updateUrl(query, null, null);

    // Base query parameters
    let params = assembleParamString(query, platformSlugs, sort);

    try {
      marketSearchResults = await getMarkets(params);
      searchError = marketSearchResults.length === 0 ? "No items found." : null;
    } catch (err: unknown) {
      searchError =
        err instanceof Error ? err.message : "Error loading table data.";
    } finally {
      searchLoading = false;
    }
  }

  function handleSearch() {
    loadTableData(searchQuery, platformSlugs, platformSort);
  }

  function renderPlot() {
    const plotElement = document.querySelector("#plot");
    if (!plotElement) {
      console.error("Error rendering plot: Could not find plot element");
      return;
    }

    try {
      const plot = Plot.plot({
        width: plotElement.clientWidth || 600,
        height: 300,
        x: { type: "utc", label: "Date" },
        y: {
          domain: [0, 100],
          grid: true,
          percent: true,
          label: "Probability",
        },
        marks: [
          Plot.line(plotData, {
            x: "date",
            y: "prob",
            curve: "step",
            tip: {
              fill: "black",
            },
          }),
          Plot.ruleY([0]),
        ],
      });

      // Clear any existing plots first
      while (plotElement.firstChild) {
        plotElement.firstChild.remove();
      }

      plotElement.append(plot);
      plotRendered = true;
    } catch (e) {
      console.error("Error rendering plot:", e);
    }
  }

  function formatDate(dateString: string) {
    if (!dateString) return "N/A";
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    });
  }
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-4 w-full">
  {#if marketLoading}
    <LoadingSpinner />
  {:else if error}
    <ErrorMessage message={error} />
  {:else if market}
    <div class="max-w-4xl mx-auto w-full">
      <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
        <!-- Market Title & Resolution Chip -->
        <div class="flex justify-between items-start mb-2">
          <h1 class="text-2xl font-bold">
            {market.title}
            {#if market.resolution == 1.0}
              <span class="px-2 rounded-md bg-green/20"> YES </span>
            {:else if market.resolution == 0.0}
              <span class="px-2 rounded-md bg-red/20"> NO </span>
            {:else}
              <span class="px-2 rounded-md bg-teal/20">
                {market.resolution.toFixed(2)}
              </span>
            {/if}
          </h1>
        </div>

        <!-- Market ID -->
        <div class="flex justify-between items-start mb-2">
          <h1 class="text-xs break-all">{market.id}</h1>
        </div>

        <!-- Action Chips -->
        <div class="mb-0">
          <button
            on:click={() => navigator.clipboard.writeText(market?.id || "")}
            class="inline-flex items-center px-3 py-1 mr-2 mb-2 text-sm rounded-md text-white bg-teal/50 hover:bg-teal"
          >
            Copy ID
          </button>
          <span
            class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            <a href={market.url} target="_blank" rel="noopener noreferrer">
              View on {market.platform_name} →
            </a>
          </span>
        </div>

        <!-- Info Chips -->
        <div class="mb-6">
          {#if market.category_name}
            <span
              class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
            >
              {market.category_name}
            </span>
          {/if}
          {#if market.volume_usd}
            <span
              class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
            >
              ${market.volume_usd
                ? Math.round(market.volume_usd).toLocaleString()
                : "N/A"} Volume
            </span>
          {/if}
          {#if market.traders_count}
            <span
              class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
            >
              {market.traders_count?.toLocaleString() || "N/A"}
            </span>
          {/if}
          <span
            class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            {formatDate(market.open_datetime)} to {formatDate(
              market.close_datetime,
            )} ({market.duration_days}d)
          </span>
        </div>

        <div class="mb-0">
          <h2 class="text-xl font-semibold mb-2">Description</h2>
          <div class="bg-mantle p-4 rounded-md">
            <p class="whitespace-pre-line">{market.description}</p>
          </div>
        </div>
      </div>

      <!-- Existing Question Links -->
      {#if question}
        <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
          <h2 class="text-xl font-semibold mb-2">Question Link</h2>
          <div class="p-2">
            <p>
              This market is linked to question:
              <a
                href={`/questions/edit?id=${question.id}`}
                class="text-blue hover:underline"
              >
                {question.title}
              </a>
            </p>
          </div>
        </div>
      {/if}

      <!-- Market Probability Chart -->
      <div class="bg-crust p-6 rounded-lg shadow-md mb-4">
        <h2 class="text-xl font-semibold mb-4">Probability History</h2>
        <div id="plot" class="w-full h-[300px]"></div>
      </div>
    </div>

    <!-- Search Sidebar -->
    <div class="max-w-4xl mx-auto w-full">
      <SearchBar bind:searchQuery onSearch={handleSearch} />
      {#if searchLoading}
        <LoadingSpinner />
      {:else if searchError}
        <ErrorMessage message={searchError} />
      {:else}
        <MarketTableLite
          platformName={platformSlugs[0]}
          markets={marketSearchResults}
        />
      {/if}
    </div>

    <!-- Error Message -->
  {:else}
    <ErrorMessage message="No market found with ID: {marketId}" />
  {/if}
</div>
