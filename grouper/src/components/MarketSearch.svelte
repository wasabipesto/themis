<script>
    import { onMount } from "svelte";
    import { getMarkets, getItemsSorted } from "@lib/api";

    // State
    let items = [];
    let platforms = [];
    let loading = true;
    let error = null;
    let searchQuery = "";
    let selectedPlatform = "";

    onMount(async () => {
        // Load platforms for the dropdown
        try {
            platforms = await getItemsSorted("platforms");
        } catch (err) {
            console.error("Error loading platforms:", err);
        }

        // Initial market loading
        await loadTableData();
    });

    async function loadTableData(query = "", platform = "") {
        loading = true;
        try {
            // Base query parameters
            let params = "order=volume_usd.desc.nullslast";

            // Append search query if it exists
            if (query) {
                params += `&title=ilike.*${query}*`;
            }

            // Append platform filter if selected
            if (platform) {
                params += `&platform_slug=eq.${platform}`;
            }

            items = await getMarkets(params);
            error = items.length === 0 ? "No items found." : null;
        } catch (err) {
            error = `Error loading data: ${err.message}`;
            console.error("Error loading table data:", err);
        } finally {
            loading = false;
        }
    }

    function handleSearch() {
        loadTableData(searchQuery, selectedPlatform);
    }

    function handleKeyDown(event) {
        if (event.key === "Enter") {
            handleSearch();
        }
    }

    function handlePlatformChange() {
        loadTableData(searchQuery, selectedPlatform);
    }
</script>

<div class="mb-4">
    <div class="flex gap-2 my-2">
        <input
            type="text"
            placeholder="Search markets..."
            class="w-full px-4 py-2 pl-4 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
            bind:value={searchQuery}
            on:keydown={handleKeyDown}
        />
        <button
            class="px-4 py-2 bg-blue hover:bg-blue/80 text-white rounded-md"
            on:click={handleSearch}
        >
            Search
        </button>
    </div>

    <div class="mt-2">
        <select
            class="w-full px-4 py-2 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
            bind:value={selectedPlatform}
            on:change={handlePlatformChange}
        >
            <option value="">All Platforms</option>
            {#each platforms as platform}
                <option value={platform.slug}>{platform.name}</option>
            {/each}
        </select>
    </div>
</div>

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
                    <th
                        class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider"
                    >
                        Platform
                    </th>
                    <th
                        class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider"
                    >
                        Title
                    </th>
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
                        <td class="px-6 py-4 whitespace-nowrap text-sm">
                            {item.platform_name}
                        </td>
                        <td class="px-6 py-4 whitespace-nowrap text-sm">
                            {item.title}
                        </td>
                        <td
                            class="px-6 py-4 whitespace-nowrap text-sm font-medium actions"
                        >
                            <a
                                href={`/markets/edit?id=${item.id}`}
                                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue mr-2"
                            >
                                View
                            </a>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
