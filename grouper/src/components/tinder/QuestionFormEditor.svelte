<script lang="ts">
  import type { NewQuestion, CategoryDetails } from "@types";
  import { LoadingSpinnerSmall } from "../market-view";
  import {
    llmGetCategory,
    llmSlugify,
    llmSummarizeDescriptions,
  } from "@lib/ai";

  // Props
  export let questionForm: NewQuestion;
  export let categories: CategoryDetails[] = [];
  export let markets: any[] = []; // For AI generation purposes
  
  // State for AI generation buttons
  let generatingTitle = false;
  let generatingSlug = false;
  let generatingDescription = false;
  let generatingCategory = false;

  // Auto-generate slug when title changes
  $: if (questionForm.title && !questionForm.slug) {
    generateSlug();
  }

  async function generateTitle() {
    if (!markets || markets.length === 0) return;
    
    generatingTitle = true;
    try {
      // Use the first market's title as a base
      questionForm.title = markets[0].title;
    } catch (err) {
      console.error("Error generating title:", err);
    } finally {
      generatingTitle = false;
    }
  }

  async function generateSlug() {
    if (!questionForm.title) return;
    
    generatingSlug = true;
    try {
      if (markets && markets.length > 0) {
        questionForm.slug = await llmSlugify(markets[0]);
      } else {
        // Simple slug generation if no markets or AI fails
        questionForm.slug = questionForm.title
          .toLowerCase()
          .replace(/[^a-z0-9]+/g, "-")
          .replace(/^-|-$/g, "");
      }
    } catch (err) {
      console.error("Error generating slug:", err);
      // Fallback to simple slugification
      questionForm.slug = questionForm.title
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
    } finally {
      generatingSlug = false;
    }
  }

  async function generateDescription() {
    if (!markets || markets.length === 0 || !questionForm.title) return;
    
    generatingDescription = true;
    try {
      questionForm.description = await llmSummarizeDescriptions(
        { title: questionForm.title },
        markets
      );
    } catch (err) {
      console.error("Error generating description:", err);
      questionForm.description = "Generated from similar markets";
    } finally {
      generatingDescription = false;
    }
  }

  async function generateCategory() {
    if (!questionForm.title) return;
    
    generatingCategory = true;
    try {
      const suggestedCategory = await llmGetCategory(questionForm.title);
      if (suggestedCategory) {
        questionForm.category_slug = suggestedCategory;
      }
    } catch (err) {
      console.error("Error generating category:", err);
      // If we have markets with categories, use the first one with a category
      for (const market of markets) {
        if (market.category_slug) {
          questionForm.category_slug = market.category_slug;
          break;
        }
      }
    } finally {
      generatingCategory = false;
    }
  }
</script>

<div class="bg-crust p-6 rounded-lg shadow-md mb-4">
  <h2 class="text-xl font-semibold mb-4">Question Details</h2>
  
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <label for="title" class="font-medium">Title</label>
      <button
        on:click={generateTitle}
        disabled={generatingTitle || markets.length === 0}
        class="text-xs px-2 py-1 bg-blue hover:bg-blue/80 text-crust rounded-md disabled:opacity-50"
      >
        {#if generatingTitle}
          <LoadingSpinnerSmall />
        {:else}
          AI Generate
        {/if}
      </button>
    </div>
    <input
      id="title"
      type="text"
      bind:value={questionForm.title}
      class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
      placeholder="What is the main question these markets are asking?"
    />
  </div>
  
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <label for="slug" class="font-medium">Slug</label>
      <button
        on:click={generateSlug}
        disabled={generatingSlug || !questionForm.title}
        class="text-xs px-2 py-1 bg-blue hover:bg-blue/80 text-crust rounded-md disabled:opacity-50"
      >
        {#if generatingSlug}
          <LoadingSpinnerSmall />
        {:else}
          Generate from Title
        {/if}
      </button>
    </div>
    <input
      id="slug"
      type="text"
      bind:value={questionForm.slug}
      class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
      placeholder="url-friendly-question-slug"
    />
  </div>
  
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <label for="category" class="font-medium">Category</label>
      <button
        on:click={generateCategory}
        disabled={generatingCategory || !questionForm.title}
        class="text-xs px-2 py-1 bg-blue hover:bg-blue/80 text-crust rounded-md disabled:opacity-50"
      >
        {#if generatingCategory}
          <LoadingSpinnerSmall />
        {:else}
          AI Suggest
        {/if}
      </button>
    </div>
    <select
      id="category"
      bind:value={questionForm.category_slug}
      class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
    >
      {#each categories as category}
        <option value={category.slug}>{category.name}</option>
      {/each}
    </select>
  </div>
  
  <div class="mb-4">
    <div class="flex items-center justify-between mb-2">
      <label for="description" class="font-medium">Description</label>
      <button
        on:click={generateDescription}
        disabled={generatingDescription || !questionForm.title || markets.length === 0}
        class="text-xs px-2 py-1 bg-blue hover:bg-blue/80 text-crust rounded-md disabled:opacity-50"
      >
        {#if generatingDescription}
          <LoadingSpinnerSmall />
        {:else}
          AI Generate
        {/if}
      </button>
    </div>
    <textarea
      id="description"
      bind:value={questionForm.description}
      rows="5"
      class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
      placeholder="Describe what this question is about and how it should be resolved..."
    ></textarea>
  </div>
  
  <!-- Optional date overrides - simplified for now, could be expanded -->
  <div class="grid grid-cols-2 gap-4 mb-2">
    <div>
      <label for="start_date" class="block mb-2 font-medium">Start Date Override (Optional)</label>
      <input
        id="start_date"
        type="date"
        bind:value={questionForm.start_date_override}
        class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
      />
    </div>
    <div>
      <label for="end_date" class="block mb-2 font-medium">End Date Override (Optional)</label>
      <input
        id="end_date"
        type="date"
        bind:value={questionForm.end_date_override}
        class="w-full px-4 py-2 bg-mantle rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
      />
    </div>
  </div>
</div>