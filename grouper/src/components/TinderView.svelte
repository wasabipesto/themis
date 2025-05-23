<script lang="ts">
  import type {
    MarketDetails,
    Platform,
    CategoryDetails,
    DailyProbabilityDetails,
    SimilarMarkets,
    NewQuestion,
  } from "@types";
  import {
    FilterControls,
    LoadingSpinner,
    ErrorMessage,
  } from "./market-search";
  import { MarketProbabilityChart } from "./market-view";
  import { MarketCard, QuestionFormEditor, SimilarityBadge } from "./tinder";
  import { onMount } from "svelte";
  import {
    getMarkets,
    getMarketProbs,
    getPlatformsLite,
    getCategories,
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

  // Constants
  const SIMILARITY_THRESHOLD = 0.75; // Adjust this threshold as needed
  const MIN_SIMILAR_MARKETS = 2;

  // Filter state
  let platforms: Platform[] = [];
  let categories: CategoryDetails[] = [];
  let selectedPlatform = "";
  let selectedCategory = "";
  let selectedSort = "volume_usd.desc.nullslast";
  let defaultFilters = true;

  // Market state
  let candidateMarkets: MarketDetails[] = [];
  let currentMarketIndex = 0;
  let currentMarket: MarketDetails | null = null;
  let similarMarkets: MarketDetails[] = [];
  let allGroupedMarkets: MarketDetails[] = [];
  let similarityScores: Map<string, number> = new Map(); // Store similarity scores by market ID

  // Plot data
  let plotData: DailyProbabilityDetails[] = [];

  // UI state
  let isLoading = true;
  let isFindingSimilar = false;
  let isCreatingQuestion = false;
  let error: string | null = null;
  let noMoreMarkets = false;

  // Question form
  let questionForm: NewQuestion = {
    title: "",
    slug: "",
    description: "",
    category_slug: "",
    start_date_override: null,
    end_date_override: null,
  };

  onMount(async () => {
    try {
      // Load platforms and categories
      platforms = await getPlatformsLite();
      categories = await getCategories();

      // Start the process by loading candidate markets
      await loadCandidateMarkets();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to initialize page";
      isLoading = false;
    }
  });

  async function loadCandidateMarkets() {
    isLoading = true;
    error = null;

    try {
      // Build query parameters
      let params = [];

      // Add platform filter if selected
      if (selectedPlatform) {
        params.push(`platform_slug=eq.${selectedPlatform}`);
      }

      // Add category filter if selected
      if (selectedCategory) {
        params.push(`category_slug=eq.${selectedCategory}`);
      }

      // Add default filters to exclude already linked or dismissed markets
      if (defaultFilters) {
        params.push("question_id=is.null");
        params.push("question_dismissed=eq.0");
      }

      // Add sorting
      params.push(`order=${selectedSort}`);

      // Fetch candidate markets
      candidateMarkets = await getMarkets(params.join("&"));

      // Reset current index
      currentMarketIndex = 0;

      if (candidateMarkets.length === 0) {
        noMoreMarkets = true;
        isLoading = false;
        return;
      }

      // Print status
      console.log(`Found ${candidateMarkets.length} candidate markets`);

      // Process the first market
      await processNextMarket();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load markets";
      isLoading = false;
    }
  }

  async function processNextMarket() {
    if (currentMarketIndex >= candidateMarkets.length) {
      noMoreMarkets = true;
      isLoading = false;
      return;
    }

    currentMarket = candidateMarkets[currentMarketIndex];
    isFindingSimilar = true;
    similarMarkets = [];
    plotData = [];

    try {
      // Get probability data for current market
      const currentMarketProbs = await getMarketProbs(currentMarket.id);
      plotData = [...currentMarketProbs];

      // Find similar markets from other platforms
      await findSimilarMarkets(currentMarket);

      if (similarMarkets.length >= MIN_SIMILAR_MARKETS) {
        // We found enough similar markets
        allGroupedMarkets = [currentMarket, ...similarMarkets];

        // Get probability data for similar markets
        for (const market of similarMarkets) {
          try {
            const probs = await getMarketProbs(market.id);
            plotData = [...plotData, ...probs];
          } catch (err) {
            console.error(
              `Failed to load probability data for market ${market.id}:`,
              err,
            );
          }
        }

        // Pre-fill question form with AI-generated content
        await prefillQuestionForm();

        isFindingSimilar = false;
        isLoading = false;
      } else {
        // Print status
        console.log(
          `Market ${currentMarket.id} (${currentMarket.title}) has ${similarMarkets.length} similar markets, not enough to group.`,
        );

        // Not enough similar markets, try the next candidate
        currentMarketIndex++;
        await processNextMarket();
      }
    } catch (err) {
      console.error("Error processing market:", err);
      // Try the next market instead of showing an error
      currentMarketIndex++;
      await processNextMarket();
    }
  }

  async function findSimilarMarkets(market: MarketDetails) {
    // Get the market's date range for similarity search
    const startDate = market.open_datetime?.split("T")[0] || "";
    const endDate = market.close_datetime?.split("T")[0] || "";

    // Create a set to track platforms we've already added
    const includedPlatforms = new Set<string>([market.platform_slug]);

    // Get a list of all other platforms
    const otherPlatforms = platforms
      .filter((p) => p.slug !== market.platform_slug)
      .map((p) => p.slug);

    // For each platform, find similar markets
    for (const platformSlug of otherPlatforms) {
      try {
        const similarMarketsResults = await getSimilarMarkets(
          market.id,
          platformSlug,
          startDate,
          endDate,
        );

        // Sort by similarity score (best matches first)
        const sortedResults = similarMarketsResults.sort(
          (a, b) => (a.cosine_distance || 1) - (b.cosine_distance || 1),
        );

        // Find the best match that meets our threshold
        for (const sm of sortedResults) {
          if (
            (sm.cosine_distance || 1) <= SIMILARITY_THRESHOLD &&
            !includedPlatforms.has(sm.platform_slug)
          ) {
            // Convert to MarketDetails format
            const marketDetails: MarketDetails = {
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
            };

            // Store the similarity score
            similarityScores.set(sm.id, sm.cosine_distance || 1);

            // Add to our list and mark platform as included
            similarMarkets.push(marketDetails);
            includedPlatforms.add(sm.platform_slug);

            // Break the inner loop - we found a match for this platform
            break;
          }
        }
      } catch (err) {
        console.error(
          `Error getting similar markets for platform ${platformSlug}:`,
          err,
        );
      }
    }
  }

  async function prefillQuestionForm() {
    if (!currentMarket) return;

    try {
      // Generate title (use the main market's title)
      questionForm.title = currentMarket.title;

      // Generate slug
      questionForm.slug = await llmSlugify(currentMarket);

      // Generate description
      questionForm.description = await llmSummarizeDescriptions(
        { title: currentMarket.title },
        allGroupedMarkets,
      );

      // Generate category
      questionForm.category_slug =
        currentMarket.category_slug ||
        (await llmGetCategory(currentMarket.title)) ||
        "politics"; // Default category if none is found
    } catch (err) {
      console.error("Error prefilling question form:", err);
      // If AI fails, just use basic values
      questionForm.title = currentMarket.title;
      questionForm.slug = currentMarket.title
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
      questionForm.description = "Generated from similar markets";
      questionForm.category_slug = currentMarket.category_slug || "politics";
    }
  }

  async function handleApprove() {
    if (!currentMarket || similarMarkets.length < MIN_SIMILAR_MARKETS) {
      return;
    }

    isCreatingQuestion = true;
    error = null;

    try {
      // Create the question
      const createdQuestion = await createQuestionNoRefresh(questionForm);
      const questionId = createdQuestion.id;

      // Link all markets to the question
      const linkPromises = allGroupedMarkets.map((market) =>
        linkMarketNoRefresh(market.id, questionId),
      );
      await Promise.all(linkPromises);

      // Refresh views
      await refreshViewsQuick();

      // Navigate to the question edit page
      window.location.href = `/questions/edit?id=${questionId}`;
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to create question";
      isCreatingQuestion = false;
    }
  }

  function handleSkip() {
    // Move to the next market
    currentMarketIndex++;
    processNextMarket();
  }

  function handleFilterChange() {
    // Reset and reload markets with new filters
    currentMarketIndex = 0;
    similarMarkets = [];
    plotData = [];
    noMoreMarkets = false;
    loadCandidateMarkets();
  }
</script>

<div class="grid grid-cols-1 lg:grid-cols-12 gap-6 w-full">
  <!-- Filters Section -->
  <div class="lg:col-span-12 bg-crust p-6 rounded-lg shadow-md">
    <h2 class="text-xl font-semibold mb-4">Filters</h2>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <!-- Platform filter -->
      <div>
        <label for="platform" class="block mb-2 font-medium"
          >Platform (Optional)</label
        >
        <select
          id="platform"
          bind:value={selectedPlatform}
          class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
        >
          <option value="">All Platforms</option>
          {#each platforms as platform}
            <option value={platform.slug}>{platform.name}</option>
          {/each}
        </select>
      </div>

      <!-- Category filter -->
      <div>
        <label for="category" class="block mb-2 font-medium"
          >Category (Optional)</label
        >
        <select
          id="category"
          bind:value={selectedCategory}
          class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
        >
          <option value="">All Categories</option>
          {#each categories as category}
            <option value={category.slug}>{category.name}</option>
          {/each}
        </select>
      </div>

      <!-- Sort filter -->
      <div>
        <label for="sort" class="block mb-2 font-medium"
          >Sort By (Required)</label
        >
        <select
          id="sort"
          bind:value={selectedSort}
          class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
        >
          <option value="volume_usd.desc.nullslast">Highest Volume</option>
          <option value="volume_usd.asc.nullslast">Lowest Volume</option>
          <option value="traders_count.desc.nullslast">Most Traders</option>
          <option value="traders_count.asc.nullslast">Least Traders</option>
          <option value="open_datetime.desc">Newest (by open)</option>
          <option value="open_datetime.asc">Oldest (by open)</option>
          <option value="close_datetime.desc">Newest (by close)</option>
          <option value="close_datetime.asc">Oldest (by close)</option>
          <option value="title.asc">Title A-Z</option>
          <option value="title.desc">Title Z-A</option>
        </select>
      </div>
    </div>

    <div class="mt-4 flex items-center">
      <label class="flex items-center">
        <input
          type="checkbox"
          bind:checked={defaultFilters}
          class="mr-2 h-4 w-4 rounded bg-mantle border-gray-500 text-lavender focus:ring-lavender focus:ring-1"
        />
        <span>Only show markets without questions/dismissed</span>
      </label>

      <button
        on:click={handleFilterChange}
        class="ml-auto px-6 py-2 bg-blue hover:bg-blue/80 text-crust rounded-md"
      >
        Apply Filters
      </button>
    </div>
  </div>

  <!-- Main Content -->
  {#if isLoading || isFindingSimilar}
    <div class="lg:col-span-12 flex flex-col items-center justify-center py-12">
      <LoadingSpinner />
      <p class="mt-4 text-text">
        {#if isFindingSimilar}
          Finding similar markets...
        {:else}
          Loading markets...
        {/if}
      </p>
    </div>
  {:else if error}
    <div class="lg:col-span-12">
      <ErrorMessage message={error} />
    </div>
  {:else if noMoreMarkets}
    <div class="lg:col-span-12 bg-crust p-6 rounded-lg shadow-md text-center">
      <h2 class="text-xl font-semibold mb-4">No More Markets</h2>
      <p class="text-text mb-4">
        No more markets match your criteria or have enough similar markets on
        other platforms.
      </p>
      <button
        on:click={handleFilterChange}
        class="px-6 py-2 bg-blue hover:bg-blue/80 text-crust rounded-md"
      >
        Try Different Filters
      </button>
    </div>
  {:else if currentMarket && similarMarkets.length >= MIN_SIMILAR_MARKETS}
    <!-- Markets Display -->
    <div class="lg:col-span-6">
      <div class="bg-crust p-6 rounded-lg shadow-md mb-4">
        <h2 class="text-xl font-semibold mb-4">Primary Market</h2>
        {#if currentMarket}
          <MarketCard market={currentMarket} isMain={true} />
        {/if}
      </div>

      <div class="bg-crust p-6 rounded-lg shadow-md">
        <h2 class="text-xl font-semibold mb-4">
          Similar Markets ({similarMarkets.length})
        </h2>
        {#each similarMarkets as market (market.id)}
          <MarketCard
            {market}
            similarity={similarityScores.get(market.id) || null}
          />
        {/each}
      </div>
    </div>

    <!-- Question Form and Chart -->
    <div class="lg:col-span-6">
      <!-- Chart -->
      <div class="mb-4">
        <MarketProbabilityChart {plotData} />
      </div>

      <!-- Question Form -->
      <QuestionFormEditor
        bind:questionForm
        {categories}
        markets={allGroupedMarkets}
      />

      <!-- Action Buttons -->
      <div class="flex justify-between">
        <button
          on:click={handleSkip}
          class="px-6 py-3 bg-crust hover:bg-crust/80 text-text border border-overlay0 rounded-md"
          disabled={isCreatingQuestion}
        >
          Skip
        </button>

        <button
          on:click={handleApprove}
          class="px-6 py-3 bg-green hover:bg-green/80 text-crust rounded-md"
          disabled={isCreatingQuestion}
        >
          {#if isCreatingQuestion}
            <span class="flex items-center">
              <LoadingSpinner /> Creating...
            </span>
          {:else}
            Approve and Create Question
          {/if}
        </button>
      </div>
    </div>
  {/if}
</div>
