<script>
  import { onMount } from "svelte";
  import {
    getMarkets,
    getCategories,
    getPlatformsLite,
    getQuestions,
    deleteItem,
  } from "@lib/api";

  // Props
  export let headers = [];
  export let endpoint = "";

  // State
  let items = [];
  let loading = true;
  let error = null;

  onMount(loadTableData);

  async function loadTableData() {
    try {
      if (endpoint === "markets") {
        items = await getMarkets();
      } else if (endpoint === "categories") {
        items = await getCategories();
      } else if (endpoint === "platforms") {
        items = await getPlatformsLite();
      } else if (endpoint === "questions") {
        items = await getQuestions();
      } else {
        error = "Invalid endpoint";
      }
      error = items.length === 0 ? "No items found." : null;
    } catch (err) {
      error = `Error loading data: ${err.message}`;
      console.error("Error loading table data:", err);
    } finally {
      loading = false;
    }
  }

  async function handleDelete(item) {
    const identifier = getItemIdentifier(item);

    if (!identifier) {
      alert("No item ID or slug found.");
      return;
    }

    if (!confirm("Are you sure you want to delete this item?")) {
      return;
    }

    try {
      const { attr, value } = identifier;
      await deleteItem(endpoint, attr, value);
      // Remove item from the list for instant UI update
      items = items.filter((i) => i !== item);
    } catch (err) {
      alert("Error deleting item: " + err.message);
    }
  }

  function getItemIdentifier(item) {
    if (item.id) return { attr: "id", value: item.id };
    if (item.slug) return { attr: "slug", value: item.slug };
    return null;
  }

  function getEditUrl(item) {
    const identifier = getItemIdentifier(item);
    if (!identifier) return "";
    const { attr, value } = identifier;
    return `/${endpoint}/edit?${attr}=${value}`;
  }
</script>

<div class="overflow-x-auto">
  {#if loading}
    <div class="flex justify-center p-4">
      <div
        class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue"
      ></div>
    </div>
  {:else if error}
    <div class="text-red p-4 text-center">{error}</div>
  {:else}
    <table class="divide-y divide-subtext bg-crust rounded-lg shadow">
      <thead>
        <tr>
          {#each headers as header}
            <th
              class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider"
            >
              {header.label}
            </th>
          {/each}
          <th
            class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider"
          >
            Actions
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-subtext">
        {#each items as item}
          <tr class="hover:bg-base-dark">
            {#each headers as header}
              <td class="px-6 py-4 whitespace-nowrap text-sm">
                {item[header.key] || ""}
              </td>
            {/each}
            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium actions">
              <a
                href={getEditUrl(item)}
                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue mr-2"
              >
                Edit
              </a>
              <button
                on:click={() => handleDelete(item)}
                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-red/50 hover:bg-red"
              >
                Delete
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>
