<script>
  import { onMount } from 'svelte';
  import { fetchFromAPI } from "@lib/api";

  // Props
  export let headers = [];
  export let endpoint = '';

  // State
  let items = [];
  let loading = true;
  let error = null;

  onMount(async () => {
    await loadTableData();
  });

  async function loadTableData() {
    try {
      items = await fetchFromAPI(endpoint);
      loading = false;

      if (items.length === 0) {
        error = "No items found.";
      }
    } catch (err) {
      error = `Error loading data: ${err.message}`;
      loading = false;
      console.error("Error loading table data:", err);
    }
  }

  async function handleDelete(item) {
    // Determine which identifier to use (id or slug)
    let attr = null;
    let value = null;

    if (item.id) {
      attr = "id";
      value = item.id;
    } else if (item.slug) {
      attr = "slug";
      value = item.slug;
    } else {
      alert("No item ID or slug found.");
      return;
    }

    // Confirm deletion
    if (confirm("Are you sure you want to delete this item?")) {
      try {
        await fetchFromAPI(`${endpoint}?${attr}=eq.${value}`, {
          method: "DELETE",
          headers: {
            Prefer: "return=representation",
          },
        });
        // Remove item from the list for instant UI update
        items = items.filter(i => i !== item);
      } catch (err) {
        alert("Error deleting item: " + err.message);
      }
    }
  }
</script>

<div class="overflow-x-auto">
  {#if loading}
    <!-- Loading spinner -->
    <div class="flex justify-center p-4">
      <div class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue"></div>
    </div>
  {:else if error}
    <!-- Error message -->
    <div class="text-red p-4 text-center">{error}</div>
  {:else}
    <!-- Data table -->
    <table class="divide-y divide-subtext bg-crust rounded-lg shadow">
      <thead>
        <tr>
          {#each headers as header}
            <th class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider">{header.label}</th>
          {/each}
          <th class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider">Actions</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-subtext">
        {#each items as item}
          <tr class="hover:bg-base-dark">
            {#each headers as header}
              <td class="px-6 py-4 whitespace-nowrap text-sm">{item[header.key] || ''}</td>
            {/each}
            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium actions">
              <a
                href="/{endpoint}/{item.id || item.slug}"
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
