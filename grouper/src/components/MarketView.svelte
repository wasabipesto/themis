<script lang="ts">
  import type {
    NewQuestion,
    Question,
    MarketDetails,
    DailyProbabilityDetails,
    SimilarMarkets,
  } from "@types";
  import {
    LoadingSpinnerSmall,
    MarketDetailsCard,
    MarketProbabilityChart,
    QuestionLinkCard,
    StagedMarketsList,
    slugify,
  } from "./market-view";
  import {
    SearchBar,
    LoadingSpinner,
    ErrorMessage,
    MarketTableLite,
    assembleParamString,
    getOtherPlatforms,
    type HardcodedPlatform,
  } from "./market-search";
  import { onMount } from "svelte";
  import {
    getMarket,
    getQuestion,
    getMarketProbs,
    getMarkets,
    getSimilarMarkets,
    createQuestionNoRefresh,
    linkMarketNoRefresh,
    refreshViewsQuick,
  } from "@lib/api";
  import {
    llmGetKeywords,
    llmGetCategory,
    llmSlugify,
    llmSummarizeDescriptions,
  } from "@lib/ai";

  // Market view
  let marketId: string | null = null;
  let market: MarketDetails | null = null;
  let newQuestion: NewQuestion | null = null;
  let question: Question | null = null;
  let marketLoading = true;
  let error: string | null = null;

  // Plot data
  let plotData: DailyProbabilityDetails[] = [];
  let plotLoading = true;
  let plotRendered = false;

  // Search sidebar
  let otherPlatforms: HardcodedPlatform[] = [];
  let searchQuery = "";
  let searchLoading = true;
  let keywordsLoading = false;
  let searchMode: "keyword" | "similar" = "similar"; // Toggle between keyword and similar markets search

  // Separate results for each platform
  let platformResults: Map<
    string,
    {
      loading: boolean;
      error: string | null;
      markets: MarketDetails[];
    }
  > = new Map();

  // Staged markets feature
  let stagedMarkets: MarketDetails[] = [];
  let creatingQuestion = false;

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

      // Add to staging
      stagedMarkets.push(market);

      // Fetch question data
      if (market.question_id) {
        question = await getQuestion(market.question_id.toString());
      }

      // Fetch market probability data for plotting
      try {
        plotData = await getMarketProbs(marketId);
      } catch (err) {
        console.error("Failed to load probability data:", err);
      }

      // Set up platforms for search
      try {
        otherPlatforms = getOtherPlatforms(market.platform_slug);

        // Initialize platform results map
        otherPlatforms.forEach((platform) => {
          platformResults.set(platform.slug, {
            loading: true,
            error: null,
            markets: [],
          });
        });

        // Load data for each platform
        await loadAllPlatformData(searchQuery);
      } catch (err) {
        console.error("Error loading platforms:", err);
      }

      marketLoading = false;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to load market data";
      marketLoading = false;
    }
  });

  async function loadPlatformData(
    query: string | null,
    platformSlug: string,
    sort: string,
  ) {
    // Update the loading state for this platform
    platformResults.set(platformSlug, {
      loading: true,
      error: null,
      markets: [],
    });
    platformResults = platformResults; // Trigger reactivity

    try {
      let results;

      if (searchMode === "similar" && market) {
        const startDate = market.open_datetime?.split("T")[0];
        const endDate = market.close_datetime?.split("T")[0];
        // Get similar markets
        const similarMarketsResults = await getSimilarMarkets(
          market.id,
          platformSlug,
          startDate,
          endDate,
        );
        // Each result in similarMarkets contains all the fields we need, but we need to convert it to MarketDetails format
        results = similarMarketsResults.map((sm) => ({
          id: sm.id,
          title: sm.title,
          url: sm.url,
          platform_slug: sm.platform_slug,
          platform_name: sm.platform_name,
          category_slug: sm.category_slug,
          category_name: sm.category_name,
          question_id: sm.question_id,
          question_invert: sm.question_invert,
          question_dismissed: sm.question_dismissed,
          open_datetime: sm.open_datetime,
          close_datetime: sm.close_datetime,
          traders_count: sm.traders_count,
          volume_usd: sm.volume_usd,
          duration_days: sm.duration_days,
          resolution: sm.resolution,
          description: "",
          question_slug: "",
          question_title: "",
        }));
      } else {
        // For keyword search, use the existing approach
        let params = assembleParamString(true, query, [platformSlug], sort);
        results = await getMarkets(params);
      }

      platformResults.set(platformSlug, {
        loading: false,
        error: results.length === 0 ? `${platformSlug}: No items found.` : null,
        markets: results,
      });
    } catch (err: unknown) {
      platformResults.set(platformSlug, {
        loading: false,
        error:
          err instanceof Error
            ? err.message
            : `${platformSlug}: Error loading table data.`,
        markets: [],
      });
    }

    // Update the map to trigger reactivity
    platformResults = platformResults;
  }

  async function loadAllPlatformData(query: string | null) {
    searchLoading = true;

    // Fetch data for each platform in parallel
    const promises = otherPlatforms.map((platform) =>
      loadPlatformData(query, platform.slug, platform.sort),
    );

    await Promise.all(promises);
    searchLoading = false;
  }

  function handleSearch() {
    // In similar mode, we don't need the search query
    if (searchMode === "similar") {
      loadAllPlatformData(null); // Pass null to ignore the query
    } else {
      loadAllPlatformData(searchQuery);
    }
  }

  async function generateKeywords() {
    if (!market?.title) return;

    keywordsLoading = true;
    try {
      const keywords = await llmGetKeywords(market.title);
      searchQuery = keywords;
      // After setting keywords, trigger the search
      handleSearch();
    } catch (err) {
      console.error("Error generating keywords:", err);
      alert(
        "Failed to generate keywords: " +
          (err instanceof Error ? err.message : String(err)),
      );
    } finally {
      keywordsLoading = false;
    }
  }

  // Functions for staging markets
  function toggleStageMarket(market: MarketDetails) {
    // Check if the market is already staged
    if (!stagedMarkets.some((m) => m.id === market.id)) {
      stageMarket(market);
    } else {
      unstageMarket(market.id);
    }
  }

  async function stageMarket(market: MarketDetails) {
    // Add the market to the staged list
    stagedMarkets = [...stagedMarkets, market];

    console.log(plotData.length);
    // Fetch and add probability data for the staged market
    try {
      const newProbs = await getMarketProbs(market.id);
      plotData = [...plotData, ...newProbs];
    } catch (err) {
      console.error(
        `Failed to load probability data for market ${market.id}:`,
        err,
      );
    }
    console.log(plotData.length);
  }

  function unstageMarket(marketId: string) {
    stagedMarkets = stagedMarkets.filter((m) => m.id !== marketId);

    // Remove probability data for the unstaged market
    plotData = plotData.filter((dataPoint) => dataPoint.market_id !== marketId);
  }

  async function createQuestionFromStaged() {
    if (stagedMarkets.length === 0) {
      alert("Please stage at least one market first");
      return;
    }

    creatingQuestion = true;
    try {
      // Use the first staged market for the question details
      const firstMarket = stagedMarkets[0];

      // Create a new Question object
      const newQuestion: NewQuestion = {
        title: firstMarket.title,
        slug: await llmSlugify(firstMarket),
        description: await llmSummarizeDescriptions(
          { title: firstMarket.title },
          stagedMarkets,
        ),
        category_slug:
          firstMarket.category_slug ||
          (await llmGetCategory(firstMarket.title)) ||
          "politics",
        start_date_override: null,
        end_date_override: null,
      };

      // Create the question
      const createdQuestion = await createQuestionNoRefresh(newQuestion);
      const questionId = createdQuestion.id;

      // Link all staged markets to the question
      const linkPromises = stagedMarkets.map((market) =>
        linkMarketNoRefresh(market.id, questionId),
      );
      await Promise.all(linkPromises);

      // Refresh views
      await refreshViewsQuick();

      // Navigate to the question edit page
      window.location.href = `/questions/edit?id=${questionId}`;
    } catch (err) {
      console.error("Error creating question:", err);
      alert(
        "Failed to create question: " +
          (err instanceof Error ? err.message : String(err)),
      );
      creatingQuestion = false;
    }
  }
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-4 w-full">
  {#if marketLoading}
    <LoadingSpinner />
  {:else if error}
    <ErrorMessage message={error} />
  {:else if market}
    <div class="max-w-4xl mx-auto w-full">
      <MarketDetailsCard {market} />

      {#if question}
        <QuestionLinkCard {question} />
      {:else}
        <StagedMarketsList
          {stagedMarkets}
          {creatingQuestion}
          onUnstage={unstageMarket}
          onCreateQuestion={createQuestionFromStaged}
        />
      {/if}
      <MarketProbabilityChart {plotData} />
    </div>

    <!-- Search Sidebar -->
    <div class="max-w-4xl mx-auto w-full">
      <div class="mb-4 flex items-center gap-2">
        <div class="flex-grow">
          <SearchBar bind:searchQuery onSearch={handleSearch} />
        </div>
        <button
          on:click={generateKeywords}
          class="px-4 py-2 bg-blue hover:bg-blue/80 text-crust rounded-md"
          disabled={keywordsLoading}
        >
          {#if keywordsLoading}
            <LoadingSpinnerSmall />
          {:else}
            Keywords
          {/if}
        </button>
        <button
          on:click={() => {
            searchMode = searchMode === "keyword" ? "similar" : "keyword";
            handleSearch();
          }}
          class="px-4 py-2 hover:opacity-80 text-crust rounded-md"
          class:bg-green={searchMode === "similar"}
          class:bg-blue={searchMode === "keyword"}
        >
          {searchMode === "keyword" ? "Use Similar" : "Use Keywords"}
        </button>
      </div>

      {#if searchLoading}
        <LoadingSpinner />
      {:else}
        <div class="mb-4 p-2 bg-base-light text-crust rounded-md">
          <p class="font-medium">
            {#if searchMode === "similar"}
              Showing similar markets to "{market?.title}"
            {:else}
              Search results for "{searchQuery || "all markets"}"
            {/if}
          </p>
        </div>
        <!-- Display results from each platform separately -->
        {#each otherPlatforms as platform}
          {#if platformResults.has(platform.slug)}
            {@const platformResult = platformResults.get(platform.slug)}
            <div class="mb-4">
              {#if platformResult?.loading}
                <LoadingSpinner />
              {:else if platformResult?.error}
                <ErrorMessage message={platformResult.error} />
              {:else}
                <MarketTableLite
                  platformName={platform.name}
                  markets={platformResult?.markets}
                  {stagedMarkets}
                  onStage={toggleStageMarket}
                />
              {/if}
            </div>
          {/if}
        {/each}
      {/if}
    </div>

    <!-- Error Message -->
  {:else}
    <ErrorMessage message="No market found with ID: {marketId}" />
  {/if}
</div>
