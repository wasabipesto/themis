<script lang="ts">
  import { onMount } from "svelte";
  import { getCategory, createCategory, updateCategory } from "@lib/api";
  import type { Category } from "@types";

  // Initialize with required properties to satisfy TypeScript
  let category: Category = {
    slug: "",
    name: "",
    description: "",
    icon: "",
  };
  let loading = true;
  let error: string | null = null;
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
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to load category";
      loading = false;
    }
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    formLoading = true;
    errorMessage = "";

    try {
      const form = event.target as HTMLFormElement;
      const formData = new FormData(form);
      // Use type assertion to tell TypeScript this will be a valid Category
      const categoryData = Object.fromEntries(
        formData.entries(),
      ) as unknown as Category;

      for (const key in categoryData) {
        if (categoryData[key as keyof Category] === "") {
          (categoryData as any)[key] = null;
        }
      }

      if (Object.keys(category).length > 0 && !isNew) {
        await updateCategory(categoryData);
      } else {
        await createCategory(categoryData);
      }

      window.location.href = "/categories";
    } catch (err: unknown) {
      errorMessage =
        err instanceof Error ? err.message : "An unknown error occurred";
    } finally {
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
      <label
        for="name"
        class="min-w-200 block text-sm font-medium text-text mb-1"
      >
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
        >{category.description || ""}</textarea
      >
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
