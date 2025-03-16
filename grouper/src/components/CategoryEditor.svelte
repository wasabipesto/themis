<script>
  import { onMount } from "svelte";
  import { getCategory, createCategory, updateCategory } from "@lib/api";

  let category = {};
  let loading = true;
  let error = null;
  let isNew = false;
  let errorMessage = "";
  let formLoading = false;

  onMount(async () => {
    try {
      const urlParams = new URLSearchParams(window.location.search);
      const slug = urlParams.get("slug");

      if (!slug) {
        isNew = true;
        loading = false;
        return;
      }

      category = await getCategory(slug);
      loading = false;
    } catch (err) {
      error = err.message || "Failed to load category";
      loading = false;
    }
  });

  async function handleSubmit(event) {
    event.preventDefault();
    formLoading = true;
    errorMessage = "";

    try {
      const form = event.target;
      const formData = new FormData(form);
      const categoryData = Object.fromEntries(formData.entries());

      Object.keys(categoryData).forEach((key) => {
        if (categoryData[key] === "") {
          categoryData[key] = null;
        }
      });

      if (Object.keys(category).length > 0 && !isNew) {
        await updateCategory(categoryData);
      } else {
        await createCategory(categoryData);
      }

      window.location.href = "/categories";
    } catch (error) {
      errorMessage = error.message || "An unknown error occurred";
      formLoading = false;
    }
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
      href="/categories"
      class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
    >
      Back to Categories
    </a>
  </div>
{:else}
  <form
    id="primaryForm"
    class="mx-auto bg-crust p-6 rounded-lg shadow-md"
    on:submit={handleSubmit}
  >
    <div class="mb-4">
      <label for="slug" class="block text-sm font-medium text-text mb-1">
        Slug
      </label>
      <input
        type="text"
        id="slug"
        name="slug"
        value={category.slug || ""}
        required
        readonly={Object.keys(category).length > 0 && !isNew}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label for="name" class="block text-sm font-medium text-text mb-1">
        Name
      </label>
      <input
        type="text"
        id="name"
        name="name"
        value={category.name || ""}
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
        rows="3"
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      >
        {category.description || ""}
      </textarea>
    </div>

    <div class="mb-4">
      <label for="parent_slug" class="block text-sm font-medium text-text mb-1">
        Parent Slug
      </label>
      <input
        type="text"
        id="parent_slug"
        name="parent_slug"
        value={category.parent_slug || ""}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label class="flex items-center">
        <input
          type="checkbox"
          id="is_parent"
          name="is_parent"
          checked={category.is_parent || false}
          class="h-4 w-4 text-blue focus:ring-blue border-gray-300 rounded"
        />
        <span class="ml-2 text-sm text-text">Is Parent Category</span>
      </label>
    </div>

    <div class="mb-4">
      <label for="icon" class="block text-sm font-medium text-text mb-1">
        Icon
      </label>
      <input
        type="text"
        id="icon"
        name="icon"
        value={category.icon || ""}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <div class="mb-4">
        <label
          for="total_markets"
          class="block text-sm font-medium text-text mb-1">Total Markets</label
        >
        <input
          type="number"
          id="total_markets"
          name="total_markets"
          value={category.total_markets || ""}
          class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        />
      </div>

      <div class="mb-4">
        <label
          for="total_traders"
          class="block text-sm font-medium text-text mb-1">Total Traders</label
        >
        <input
          type="number"
          id="total_traders"
          name="total_traders"
          value={category.total_traders || ""}
          class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        />
      </div>

      <div class="mb-4">
        <label
          for="total_volume"
          class="block text-sm font-medium text-text mb-1">Total Volume</label
        >
        <input
          type="number"
          id="total_volume"
          name="total_volume"
          step="0.01"
          value={category.total_volume || ""}
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
      <a
        href="/categories"
        class="px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
      >
        Cancel
      </a>
      <button
        type="submit"
        disabled={formLoading}
        class="px-4 py-2 bg-green/50 text-white rounded-md hover:bg-green transition-colors"
      >
        {formLoading
          ? "Saving..."
          : Object.keys(category).length > 0 && !isNew
            ? "Update"
            : "Create"} Category
      </button>
    </div>
  </form>
{/if}
