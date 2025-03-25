<script lang="ts">
  import type { Question, Market, DailyProbability } from "@types";
  import { onMount } from "svelte";
  import { getMarket, getQuestion, getMarketProbs, linkMarket } from "@lib/api";
  import * as Plot from "@observablehq/plot";

  let market: Market | null = null;
  let question: Question | null = null;
  let loading = true;
  let error: string | null = null;
  let marketId: string | null = null;
  let plotData: DailyProbability[] = [];

  let questionIdInput: number | null = null;
  let linkError: string | null = null;
  let linkingInProgress = false;

  onMount(async () => {
    try {
      const urlParams = new URLSearchParams(window.location.search);
      marketId = urlParams.get("id");

      if (!marketId) {
        error = "No market ID provided";
        loading = false;
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
      } catch (plotErr) {
        console.error("Failed to load probability data:", plotErr);
        // Continue showing the page even if plot fails
      }

      loading = false;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to load market data";
      loading = false;
    }
  });

  async function handleLinkQuestion() {
    if (!marketId || !questionIdInput) {
      linkError = "Please enter a valid Question ID";
      return;
    }

    linkingInProgress = true;
    linkError = null;

    try {
      await linkMarket(marketId, questionIdInput);
      // Redirect on success
      //window.location.href = `/questions/edit?id=${questionIdInput}`;
      question = await getQuestion(questionIdInput.toString());
    } catch (err) {
      linkError =
        err instanceof Error
          ? err.message
          : "Failed to link market to question";
      linkingInProgress = false;
    }
  }

  function renderPlot() {
    // Make sure market is loaded and DOM is ready
    if (!market || !plotData || plotData.length === 0) return;

    // Use setTimeout to ensure DOM is ready
    setTimeout(() => {
      const plotElement = document.querySelector("#plot");
      if (!plotElement) return;

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
      } catch (e) {
        console.error("Error rendering plot:", e);
      }
    }, 0);
  }

  function formatDate(dateString: string) {
    if (!dateString) return "N/A";
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function formatProbability(prob: number) {
    if (prob === null || prob === undefined) return "N/A";
    return `${(prob * 100).toFixed(1)}%`;
  }
</script>

<div class="max-w-4xl mx-auto mb-6">
  {#if loading}
    <div class="flex justify-center items-center p-12">
      <div
        class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue"
      ></div>
    </div>
  {:else if error}
    <div class="bg-red/20 p-6 rounded-lg shadow-md text-center">
      <p class="text-red font-bold">Error</p>
      <p>{error}</p>
      <a
        href="/markets"
        class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
      >
        Back to Markets
      </a>
    </div>
  {:else if market}
    <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
      <div class="flex justify-between items-start mb-2">
        <h1 class="text-2xl font-bold">
          {market.title}
        </h1>
      </div>
      <div class="flex justify-between items-start mb-2">
        <h1 class="text-xs">{market.id}</h1>
      </div>

      <div class="mb-6">
        <button
          on:click={() => navigator.clipboard.writeText(market?.id || "")}
          class="inline-flex items-center px-3 py-1 mr-2 mb-2 text-sm rounded-md text-white bg-teal/50 hover:bg-teal"
        >
          Copy ID
        </button>
        {#if market.category}
          <span
            class="text-sm bg-rosewater/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            {market.category}
          </span>
        {/if}
        {#if market.volume_usd && market.volume_usd > 1000}
          <span
            class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            High Volume
          </span>
        {/if}
        {#if market.traders_count && market.traders_count > 100}
          <span
            class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            High Traders
          </span>
        {/if}
        {#if market.duration_days > 100}
          <span
            class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
          >
            High Duration
          </span>
        {/if}
        <span
          class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
        >
          <a href={market.url} target="_blank" rel="noopener noreferrer">
            View on {market.platform_name} →
          </a>
        </span>
      </div>

      <div class="mb-6">
        <h2 class="text-xl font-semibold mb-2">Description</h2>
        <div class="bg-mantle p-4 rounded-md">
          <p class="whitespace-pre-line">{market.description}</p>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
        <div>
          <h2 class="text-xl font-semibold mb-2">Market Details</h2>
          <div class="bg-mantle p-4 rounded-md">
            <dl>
              <dt class="text-text/70">Open Date</dt>
              <dd class="mb-2">
                {formatDate(market.open_datetime)}
              </dd>

              <dt class="text-text/70">Close Date</dt>
              <dd class="mb-2">
                {formatDate(market.close_datetime)}
              </dd>

              <dt class="text-text/70">Probability (Average)</dt>
              <dd>{formatProbability(market.prob_time_avg)}</dd>

              <dt class="text-text/70">Resolution</dt>
              <dd class="mb-2">
                {#if market.resolution === null || market.resolution === undefined}
                  Unresolved
                {:else if market.resolution === 1}
                  Yes (1)
                {:else if market.resolution === 0}
                  No (0)
                {:else}
                  Prob ({market.resolution})
                {/if}
              </dd>
            </dl>
          </div>
        </div>

        <div>
          <h2 class="text-xl font-semibold mb-2">Market Statistics</h2>
          <div class="bg-mantle p-4 rounded-md">
            <dl>
              <dt class="text-text/70">Traders</dt>
              <dd class="mb-2">
                {market.traders_count?.toLocaleString() || "N/A"}
              </dd>

              <dt class="text-text/70">Volume (USD)</dt>
              <dd class="mb-2">
                ${market.volume_usd
                  ? Math.round(market.volume_usd).toLocaleString()
                  : "N/A"}
              </dd>

              <dt class="text-text/70">Duration (days)</dt>
              <dd class="mb-2">
                {market.duration_days?.toLocaleString() || "N/A"}
              </dd>
            </dl>
          </div>
        </div>
      </div>
    </div>

    <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
      <h2 class="text-xl font-semibold mb-2">Question Link</h2>
      <div class="p-2">
        {#if question}
          <p>
            This market is linked to question:
            <a
              href={`/questions/edit?id=${question.id}`}
              class="text-blue hover:underline"
            >
              {question.title}
            </a>
          </p>
        {:else}
          <div class="flex flex-col space-y-2">
            <div class="flex space-x-2">
              <input
                type="text"
                bind:value={questionIdInput}
                placeholder="Enter Question ID"
                class="px-3 py-2 rounded-md bg-mantle border border-blue/30 focus:border-blue focus:outline-none flex-grow"
              />
              <button
                on:click={handleLinkQuestion}
                class="px-4 py-2 bg-blue text-crust rounded-md hover:bg-blue/80 transition-colors"
                disabled={linkingInProgress}
              >
                {linkingInProgress ? "Linking..." : "Link"}
              </button>
            </div>
            {#if linkError}
              <p class="text-red text-sm">{linkError}</p>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="bg-crust p-6 rounded-lg shadow-md mb-4">
      <h2 class="text-xl font-semibold mb-4">Probability History</h2>
      <div id="plot" class="w-full h-[300px]"></div>
    </div>

    <div class="flex justify-end">
      <a
        href="/markets"
        class="mt-4 px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
      >
        Back to Markets
      </a>
    </div>
  {:else}
    <div class="bg-yellow/20 p-6 rounded-lg shadow-md text-center">
      <p>No market found with ID: {marketId}</p>
      <a
        href="/markets"
        class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
      >
        Back to Markets
      </a>
    </div>
  {/if}
</div>
