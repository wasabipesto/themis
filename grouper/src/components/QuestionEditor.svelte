<script lang="ts">
  import type {
    Question,
    CategoryDetails,
    DailyProbabilityDetails,
    MarketDetails,
    QuestionDetails,
  } from "@types";
  import { onMount } from "svelte";
  import { MarketStats } from "./market-search";
  import {
    getQuestion,
    createQuestion,
    updateQuestion,
    linkMarket,
    unlinkMarket,
    invertMarketLink,
    getMarketsByQuestion,
    getMarketProbs,
    getCategories,
    deleteItem,
  } from "@lib/api";
  import { llmSummarizeDescriptions } from "@lib/ai";
  import * as Plot from "@observablehq/plot";

  // Question editor state
  let question: QuestionDetails | null = null;
  let categories: CategoryDetails[] = [];
  let loading = true;
  let error: string | null = null;
  let isNew = false;
  let errorMessage = "";
  let formLoading = false;

  // Market assigner state
  let markets: MarketDetails[] = [];
  let questionId: string | null = null;
  let newMarketId = "";
  let linkError: string | null = null;
  let linkSuccess = false;
  let linkLoading = false;
  let marketsLoading = false;

  // Chart state
  let plotData: DailyProbabilityDetails[] = [];

  onMount(async () => {
    try {
      // Load categories first
      categories = await getCategories();

      const urlParams = new URLSearchParams(window.location.search);
      questionId = urlParams.get("id");

      if (!questionId) {
        isNew = true;

        // Check if there's cloned question data in localStorage
        const clonedQuestionData = localStorage.getItem("clonedQuestion");
        if (clonedQuestionData) {
          question = JSON.parse(clonedQuestionData);
          // Clear the data after using it
          localStorage.removeItem("clonedQuestion");
        } else {
          question = {} as QuestionDetails;
        }

        loading = false;
        return;
      }

      // Load question data
      question = await getQuestion(questionId);

      // Load markets associated with this question
      marketsLoading = true;
      markets = await getMarketsByQuestion(question.id);
      marketsLoading = false;

      await loadMarketProbabilities();

      loading = false;
    } catch (err: unknown) {
      error =
        err instanceof Error
          ? err.message
          : "Failed to load question, markets, or categories";
      loading = false;
    }
  });

  async function loadMarketProbabilities() {
    if (!question || !markets) return;

    // Fetch market probability data for plotting
    plotData = [];
    for (const market of markets) {
      try {
        var marketProbs = await getMarketProbs(market.id);
        if (market.question_invert) {
          marketProbs = marketProbs.map((p) => ({
            ...p,
            prob: 1 - p.prob,
          }));
        }
        plotData.push(...marketProbs);
      } catch (error) {
        console.error(`Failed to fetch data for market ${market.id}:`, error);
      }
    }
    renderPlot();
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    formLoading = true;
    errorMessage = "";

    try {
      const form = event.target as HTMLFormElement;
      const formData = new FormData(form);
      const questionData = Object.fromEntries(
        formData.entries(),
      ) as unknown as Question;

      for (const key in questionData) {
        if (questionData[key as keyof Question] === "") {
          (questionData as any)[key] = null;
        }
      }

      if (!isNew) {
        // Submit the new data
        await updateQuestion(questionData);
        // Refresh question data
        if (questionId) {
          question = await getQuestion(questionId);
        }
      } else {
        // For new questions: submit new data
        const newQuestion = await createQuestion(questionData);
        console.log(newQuestion);
        // If this is a new question, redirect to the edit page
        if (newQuestion && newQuestion.id) {
          window.location.href = `?id=${newQuestion.id}`;
          return;
        } else {
          errorMessage = "An unknown error occurred";
        }
      }
    } catch (err: unknown) {
      errorMessage =
        err instanceof Error ? err.message : "An unknown error occurred";
    }
    renderPlot();
    formLoading = false;
  }

  async function handleLinkMarket() {
    if (!newMarketId.trim()) {
      linkError = "Please enter a market ID";
      return;
    }

    linkError = null;
    linkSuccess = false;
    linkLoading = true;

    try {
      // Check if question exists before accessing its properties
      if (!question || !question.id) {
        throw new Error("Question not found or question ID is missing");
      }

      // Pass the market ID and question ID to the linkMarket function
      await linkMarket(newMarketId.trim(), question.id);

      // Fetch updated list of markets
      markets = await getMarketsByQuestion(question.id);

      // Reload market probability data
      await loadMarketProbabilities();

      // Clear the input and show success message
      newMarketId = "";
      linkSuccess = true;
      // Clear success message after 3 seconds
      setTimeout(() => (linkSuccess = false), 3000);
    } catch (err: unknown) {
      linkError = err instanceof Error ? err.message : "Failed to link market";
    } finally {
      linkLoading = false;
    }
  }

  async function handleRemoveMarket(market: MarketDetails) {
    if (
      !confirm("Are you sure you want to remove this market from the question?")
    ) {
      return;
    }

    try {
      await unlinkMarket(market.id);
      // Remove item from the list for instant UI update
      markets = markets.filter((m) => m.id !== market.id);

      // Reload market probability data
      await loadMarketProbabilities();
    } catch (err: unknown) {
      alert(
        err instanceof Error
          ? `Failed to remove market: ${err.message}`
          : "Failed to remove market due to an unknown error",
      );
    }
  }

  async function handleInvertMarketLink(market: MarketDetails) {
    try {
      // Call API to invert the market link
      await invertMarketLink(market.id, !market.question_invert);

      // Update the market in our local array
      markets = markets.map((m) => {
        if (m.id === market.id) {
          return { ...m, question_invert: !m.question_invert };
        }
        return m;
      });

      // Reload market probability data and rerender the chart
      await loadMarketProbabilities();
    } catch (err: unknown) {
      alert(
        err instanceof Error
          ? `Failed to invert market link: ${err.message}`
          : "Failed to invert market link due to an unknown error",
      );
    }
  }

  async function handleDeleteQuestion() {
    // Confirm deletion
    if (!question || !question.id) {
      throw new Error("Question ID not found");
    }
    if (
      !confirm(
        "Are you sure you want to delete this question? This will also unlink all associated markets.",
      )
    ) {
      return;
    }
    // Double-check with another confirmation
    if (!confirm("This action cannot be undone. Are you REALLY sure?")) {
      return;
    }

    try {
      loading = true;
      markets.forEach(async (market) => {
        await unlinkMarket(market.id);
      });
      await deleteItem("questions", "id", question.id.toString());
      window.location.href = "/questions";
    } catch (err: unknown) {
      alert(
        err instanceof Error
          ? `Failed to delete question: ${err.message}`
          : "Failed to delete question due to an unknown error",
      );
    }
  }

  function renderPlot() {
    // Make sure items are loaded and DOM is ready
    if (!question || !markets || !plotData || plotData.length === 0) return;

    // Use setTimeout to ensure DOM is ready
    setTimeout(() => {
      const plotElement = document.querySelector("#plot");
      if (!plotElement) return;

      try {
        const plot = Plot.plot({
          width: plotElement.clientWidth || 600,
          height: 300,
          x: { type: "utc" },
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
              stroke: "platform_name",
              curve: "step",
              tip: {
                fill: "black",
              },
            }),
            Plot.ruleY([0]),
            Plot.ruleX([question?.start_date_override]),
            Plot.ruleX([question?.end_date_override]),
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
</script>

{#if loading}
  <div class="flex justify-center items-center p-12">
    <div
      class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue"
    ></div>
  </div>
{:else if error}
  <div class="w-2xl mx-auto bg-red/20 p-6 rounded-lg shadow-md text-center">
    <p class="text-red font-bold">Error</p>
    <p>{error}</p>
    <a
      href="/questions"
      class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
    >
      Back to Questions
    </a>
  </div>
{:else}
  <!-- Question Editor Section -->
  <form
    id="primaryForm"
    class="mx-auto bg-crust p-6 rounded-lg shadow-md mb-8"
    on:submit={handleSubmit}
  >
    <h2 class="text-2xl font-semibold mb-4">
      {isNew ? "Create Question" : `Edit Question #${question?.id || ""}`}
    </h2>

    {#if question?.id}
      <input type="text" id="id" name="id" value={question?.id || ""} hidden />
    {/if}

    <div class="mb-4">
      <label
        for="title"
        class="min-w-200 block text-sm font-medium text-text mb-1"
      >
        Title
      </label>
      <input
        type="text"
        id="title"
        name="title"
        value={question?.title || ""}
        required
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label for="slug" class="block text-sm font-medium text-text mb-1">
        Slug
      </label>
      <input
        type="text"
        id="slug"
        name="slug"
        value={question?.slug || ""}
        required
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label for="description" class="block text-sm font-medium text-text mb-1">
        Description
      </label>
      <textarea
        id="description"
        name="description"
        rows="7"
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        >{question?.description || ""}</textarea
      >
    </div>

    <div class="mb-4">
      <label
        for="category_slug"
        class="block text-sm font-medium text-text mb-1"
      >
        Category
      </label>
      <select
        id="category_slug"
        name="category_slug"
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      >
        <option value=""></option>
        {#each categories as category}
          <option
            value={category.slug}
            selected={question?.category_slug === category.slug}
          >
            {category.name}
          </option>
        {/each}
      </select>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="mb-4">
        <label
          for="start_date_override"
          class="block text-sm font-medium text-text mb-1"
        >
          Start Date Override
        </label>
        <input
          type="date"
          id="start_date_override"
          name="start_date_override"
          value={question?.start_date_override || ""}
          class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        />
      </div>

      <div class="mb-4">
        <label
          for="end_date_override"
          class="block text-sm font-medium text-text mb-1"
        >
          End Date Override
        </label>
        <input
          type="date"
          id="end_date_override"
          name="end_date_override"
          value={question?.end_date_override || ""}
          class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        />
      </div>
    </div>

    {#if errorMessage}
      <div
        class="max-w-full mx-auto mb-4 p-4 bg-red/20 border border-red text-red rounded-lg"
      >
        <p class="font-medium">Error:</p>
        <p>{errorMessage}</p>
      </div>
    {/if}

    <div class="flex justify-end space-x-4 mt-6">
      <button
        type="button"
        on:click={async () => {
          if (question) {
            let newDescription = await llmSummarizeDescriptions(
              question,
              markets,
            );
            question.description = newDescription;
          }
        }}
        class="px-4 py-2 bg-pink/50 text-text rounded-md hover:bg-blue transition-colors"
      >
        Generate Description
      </button>
      {#if !isNew}
        <button
          type="button"
          on:click={() => {
            // Create a new URL without the ID parameter
            const newUrl = new URL(window.location.href);
            newUrl.searchParams.delete("id");
            // Copy the current question data to localStorage
            localStorage.setItem(
              "clonedQuestion",
              JSON.stringify({
                title: question?.title,
                slug: question?.slug,
                description: question?.description,
                category_slug: question?.category_slug,
                start_date_override: question?.start_date_override,
                end_date_override: question?.end_date_override,
              }),
            );
            // Navigate to the create form
            window.location.href = newUrl.toString();
          }}
          class="px-4 py-2 bg-blue/50 text-white rounded-md hover:bg-blue transition-colors"
        >
          Clone Question
        </button>
        <button
          type="button"
          on:click={handleDeleteQuestion}
          class="px-4 py-2 bg-red/50 text-white rounded-md hover:bg-red transition-colors"
        >
          Delete Question
        </button>
      {/if}
      <button
        type="submit"
        disabled={formLoading}
        class="px-4 py-2 bg-green/50 text-white rounded-md hover:bg-green transition-colors"
      >
        {formLoading
          ? "Saving..."
          : question && Object.keys(question).length > 0 && !isNew
            ? "Save"
            : "Create"} Question
      </button>
    </div>
  </form>

  <!-- Market Assigner Section (Only visible when editing existing question) -->
  {#if !isNew && questionId}
    <div class="max-w-4xl mx-auto my-6">
      <h2 class="text-2xl font-semibold mb-4">Linked Markets</h2>

      {#if marketsLoading}
        <div class="flex justify-center items-center p-8">
          <div
            class="animate-spin rounded-full h-10 w-10 border-t-2 border-b-2 border-blue"
          ></div>
        </div>
      {:else if markets.length === 0}
        <div class="bg-blue/10 p-6 mb-6 rounded-lg shadow-md text-center">
          <p class="text-text/70">No markets assigned to this question.</p>
        </div>
      {:else}
        <div class="bg-crust p-6 mb-6 rounded-lg shadow-md">
          <table class="w-full">
            <thead>
              <tr class="border-b border-blue/20">
                <th class="text-left py-2 px-3 text-text/70"> Links </th>
                <th class="text-left py-2 px-3 text-text/70"> Platform </th>
                <th class="text-left py-2 px-3 text-text/70"> Title </th>
                <th class="text-left py-2 px-3 text-text/70 w-25"> Stats </th>
                <th class="text-center py-2 px-3 text-text/70"> Actions </th>
              </tr>
            </thead>
            <tbody>
              {#each markets as market}
                <tr class="border-b border-blue/10 hover:bg-blue/5 text-sm">
                  <td class="py-3 px-3">
                    <a href={`/markets/edit?id=${market.id}`} target="_blank">
                      <div
                        class="px-3 py-1 m-0.5 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
                      >
                        Internal
                      </div></a
                    >
                    <a
                      href={market.url}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <div
                        class="px-3 py-1 m-0.5 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
                      >
                        External
                      </div>
                    </a>
                  </td>
                  <td class="py-3 px-3">{market.platform_name}</td>
                  <td class="py-3 px-3">
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
                  </td>
                  <td class="py-3 px-3 w-30 text-left">
                    <MarketStats
                      volumeUsd={market.volume_usd}
                      tradersCount={market.traders_count}
                      durationDays={market.duration_days}
                      closeDateTime={market.close_datetime}
                    />
                  </td>
                  <td class="py-3 px-3 text-center">
                    {#if market.question_invert}
                      <button
                        on:click={() => handleInvertMarketLink(market)}
                        class="px-2 py-1 m-0.5 bg-lavender/50 text-text rounded-md hover:bg-lavender transition-colors"
                      >
                        Inverted
                      </button>
                    {:else}
                      <button
                        on:click={() => handleInvertMarketLink(market)}
                        class="px-2 py-1 m-0.5 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
                      >
                        Straight
                      </button>
                    {/if}
                    <button
                      on:click={() => handleRemoveMarket(market)}
                      class="px-2 py-1 m-0.5 bg-red/50 text-text rounded-md hover:bg-red transition-colors"
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="bg-crust p-6 mb-6 rounded-lg shadow-md">
          <h2 class="text-xl font-semibold mb-4">Market Probabilities</h2>
          <div id="plot"></div>
        </div>
      {/if}

      <!-- Add market form -->
      <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
        <h2 class="text-xl font-semibold mb-4">Link Market to Question</h2>
        <div class="flex flex-col md:flex-row gap-4">
          <div class="flex-grow">
            <input
              type="text"
              bind:value={newMarketId}
              placeholder="Enter Market ID"
              class="w-full px-4 py-2 rounded-md bg-mantle border border-blue/30 focus:border-blue focus:outline-none"
            />
          </div>
          <button
            on:click={handleLinkMarket}
            disabled={linkLoading}
            class="px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {#if linkLoading}
              <span
                class="inline-block w-4 h-4 border-2 border-text border-t-transparent rounded-full animate-spin mr-2"
              ></span>
              Linking...
            {:else}
              Link Market
            {/if}
          </button>
        </div>

        {#if linkError}
          <p class="text-red mt-2">{linkError}</p>
        {/if}

        {#if linkSuccess}
          <p class="text-green-400 mt-2">Market successfully linked!</p>
        {/if}
      </div>
    </div>
  {/if}
{/if}
