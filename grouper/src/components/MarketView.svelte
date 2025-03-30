<script lang="ts">
  import type {
    NewQuestion,
    Question,
    MarketDetails,
    DailyProbabilityDetails,
  } from "@types";
  import {
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
    updateUrl,
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
    createQuestionNoRefresh,
    linkMarketNoRefresh,
    refreshViewsQuick,
  } from "@lib/api";

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

    // Assemble query parameters for this specific platform
    let params = assembleParamString(query, [platformSlug], sort);

    try {
      const results = await getMarkets(params);
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

    // Update URL with current search parameters
    updateUrl(query, null, null);

    // Fetch data for each platform in parallel
    const promises = otherPlatforms.map((platform) =>
      loadPlatformData(query, platform.slug, platform.sort),
    );

    await Promise.all(promises);
    searchLoading = false;
  }

  function handleSearch() {
    loadAllPlatformData(searchQuery);
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
        slug: slugify(firstMarket.title),
        description: "",
        category_slug: firstMarket.category_slug || "politics", // TODO
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
      <div class="mb-4">
        <SearchBar bind:searchQuery onSearch={handleSearch} />
      </div>

      {#if searchLoading}
        <LoadingSpinner />
      {:else}
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
