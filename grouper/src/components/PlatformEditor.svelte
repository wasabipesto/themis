<script lang="ts">
  import { onMount } from "svelte";
  import { getPlatform, createPlatform, updatePlatform } from "@lib/api";
  import type { Platform } from "@types";

  // Initialize with required properties to satisfy TypeScript
  let platform: Platform = {
    slug: "",
    name: "",
    description: "",
    long_description: "",
    icon_url: "",
    site_url: "",
    wikipedia_url: "",
    color_primary: "",
    color_accent: "",
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

      platform = await getPlatform(slug);
      loading = false;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to load platform";
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
      const platformData = Object.fromEntries(
        formData.entries(),
      ) as unknown as Platform;

      for (const key in platformData) {
        if (platformData[key as keyof Platform] === "") {
          (platformData as any)[key] = null;
        }
      }

      if (Object.keys(platform).length > 0 && !isNew) {
        await updatePlatform(platformData);
      } else {
        await createPlatform(platformData);
      }

      window.location.href = "/platforms";
    } catch (err: unknown) {
      errorMessage =
        err instanceof Error ? err.message : "An unknown error occurred";
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
      href="/platforms"
      class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
    >
      Back to Platforms
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
        value={platform.slug || ""}
        required
        readonly={Object.keys(platform).length > 0 && !isNew}
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
        value={platform.name || ""}
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
        >{platform.description || ""}</textarea
      >
    </div>

    <div class="mb-4">
      <label
        for="long_description"
        class="block text-sm font-medium text-text mb-1"
      >
        Long Description
      </label>
      <textarea
        id="long_description"
        name="long_description"
        rows="10"
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        >{platform.long_description || ""}</textarea
      >
    </div>

    <div class="mb-4">
      <label for="icon_url" class="block text-sm font-medium text-text mb-1">
        Icon URL
      </label>
      <input
        type="text"
        id="icon_url"
        name="icon_url"
        value={platform.icon_url || ""}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label for="site_url" class="block text-sm font-medium text-text mb-1">
        Site URL
      </label>
      <input
        type="url"
        id="site_url"
        name="site_url"
        value={platform.site_url || ""}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="mb-4">
      <label
        for="wikipedia_url"
        class="block text-sm font-medium text-text mb-1"
      >
        Wikipedia URL
      </label>
      <input
        type="url"
        id="wikipedia_url"
        name="wikipedia_url"
        value={platform.wikipedia_url || ""}
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
      />
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="mb-4">
        <label
          for="color_primary"
          class="block text-sm font-medium text-text mb-1"
        >
          Primary Color
        </label>
        <input
          type="text"
          id="color_primary"
          name="color_primary"
          value={platform.color_primary || ""}
          placeholder="#RRGGBB"
          class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue focus:border-indigo-500"
        />
      </div>

      <div class="mb-4">
        <label
          for="color_accent"
          class="block text-sm font-medium text-text mb-1"
        >
          Accent Color
        </label>
        <input
          type="text"
          id="color_accent"
          name="color_accent"
          value={platform.color_accent || ""}
          placeholder="#RRGGBB"
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
        href="/platforms"
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
          : Object.keys(platform).length > 0 && !isNew
            ? "Update"
            : "Create"} Platform
      </button>
    </div>
  </form>
{/if}
