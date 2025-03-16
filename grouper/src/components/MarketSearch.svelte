<script lang="ts">
  import type { Market, Platform } from "@types";
    import { onMount } from "svelte";
    import { getMarkets, getItemsSorted, dismissMarket } from "@lib/api";

    // Initial state
    let markets: Market[] = [];
    let platforms: Platform[] = [];
    let loading = true;
    let error: string | null = null;
    let searchQuery = "";
    let selectedPlatform = "";
    let selectedSort = "volume_usd.desc.nullslast";

    // Available sorting options
    const sortOptions = [
        { value: "title.asc", label: "Title A-Z" },
        { value: "title.desc", label: "Title Z-A" },
        { value: "open_datetime.desc", label: "Newest (by open)" },
        { value: "open_datetime.asc", label: "Oldest (by open)" },
        { value: "close_datetime.asc", label: "Newest (by close)" },
        { value: "close_datetime.desc", label: "Oldest (by close)" },
        { value: "traders_count.desc.nullslast", label: "Most traders" },
        { value: "traders_count.asc.nullslast", label: "Least traders" },
        { value: "volume_usd.desc.nullslast", label: "Highest volume" },
        { value: "volume_usd.asc.nullslast", label: "Lowest volume" },
        { value: "duration_days.desc", label: "Longest duration" },
        { value: "duration_days.asc", label: "Shortest duration" },
        { value: "prob_time_avg.desc", label: "Highest average prob" },
        { value: "prob_time_avg.asc", label: "Lowest average prob" },
    ];

    onMount(async () => {
        // Get initial values from URL
        const urlParams = new URLSearchParams(window.location.search);
        searchQuery = urlParams.get("q") || "";
        selectedPlatform = urlParams.get("platform") || "";
        selectedSort = urlParams.get("sort") || "volume_usd.desc.nullslast";

        // Load platforms for the dropdown
        try {
            platforms = await getItemsSorted("platforms");
            await loadTableData();
        } catch (err) {
            console.error("Error loading platforms:", err);
        }
    });

    async function loadTableData(
        query = searchQuery,
        platform = selectedPlatform,
        sort = selectedSort,
    ) {
        loading = true;

        // Update URL with current search parameters
        updateUrl(query, platform, sort);

        try {
            // Base query parameters
            let params = `order=${sort}`;
            params += "&question_id=is.null&question_dismissed=eq.0";
            if (query) params += `&or=(id.ilike.*${query}*,title.ilike.*${query}*,url.ilike.*${query}*,description.ilike.*${query}*)`;
            if (platform) params += `&platform_slug=eq.${platform}`;

            markets = await getMarkets(params);
            error = markets.length === 0 ? "No items found." : null;
        } catch (err: unknown) {
            error =
                err instanceof Error
                    ? err.message
                    : "Error loading table data.";
            loading = false;
        }
        loading = false;
    }

    function updateUrl(query: string, platform: string, sort: string) {
        const url = new URL(window.location.href);

        // Update or remove search params based on values
        if (query) url.searchParams.set("q", query);
        else url.searchParams.delete("q");

        if (platform) url.searchParams.set("platform", platform);
        else url.searchParams.delete("platform");

        if (sort) url.searchParams.set("sort", sort);
        else url.searchParams.delete("sort");

        // Update the URL without refreshing the page
        window.history.pushState({}, "", url);
    }

    function handleSearch() {
        loadTableData(searchQuery, selectedPlatform, selectedSort);
    }

        async function handleDismiss(marketId: string, level: number) {
            try {
                await dismissMarket(marketId, level);
                // Optimistically remove the item from the list
                markets = markets.filter(item => item.id !== marketId);
                if (markets.length === 0) {
                    error = "No items found.";
                }
            } catch (err: unknown) {
                error =
                    err instanceof Error
                        ? err.message
                        : "Error dismissing market";
            }
        }
</script>

<div class="w-full mb-4 mx-auto">
    <div class="flex gap-2 my-2">
        <input
            type="text"
            placeholder="Search markets..."
            class="w-full px-4 py-2 pl-4 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
            bind:value={searchQuery}
            on:keydown={(e) => e.key === "Enter" && handleSearch()}
        />
        <button
            class="px-4 py-2 bg-blue hover:bg-blue/80 text-white rounded-md"
            on:click={handleSearch}
        >
            Search
        </button>
    </div>

    <div class="mt-2 flex gap-2">
        <select
            class="w-1/2 px-4 py-2 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
            bind:value={selectedPlatform}
            on:change={handleSearch}
        >
            <option value="">All Platforms</option>
            {#each platforms as platform}
                <option value={platform.slug}>{platform.name}</option>
            {/each}
        </select>

        <select
            class="w-1/2 px-4 py-2 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
            bind:value={selectedSort}
            on:change={handleSearch}
        >
            {#each sortOptions as option}
                <option value={option.value}>{option.label}</option>
            {/each}
        </select>
    </div>
</div>

<div class="w-6xl mx-auto">
    {#if loading}
        <div class="flex justify-center p-4">
            <div
                class="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue"
            ></div>
        </div>
    {:else if error}
        <div class="text-red p-4 text-center">{error}</div>
    {:else}
        <table
            class="w-full divide-y divide-subtext bg-crust rounded-lg shadow"
        >
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
                        Stats
                    </th>
                    <th
                        class="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider"
                    >
                        Actions
                    </th>
                </tr>
            </thead>
            <tbody class="divide-y divide-subtext">
                {#each markets as market}
                    <tr class="hover:bg-base-dark">
                        <td class="px-6 py-4 text-sm">
                            {market.platform_name}
                        </td>
                        <td class="px-6 py-4 text-sm">
                            {market.title}
                            <br />
                            {#if market.resolution == 1.0}
                                <span class="px-2 rounded-md bg-green/20">Resolved YES</span>
                            {:else if market.resolution == 0.0}
                                <span class="px-2 rounded-md bg-red/20">Resolved NO</span>
                            {:else if market.resolution == 0.0}
                                <span class="px-2 rounded-md bg-teal/20">Resolved {market.resolution}</span>
                            {/if}
                        </td>
                        <td class="px-6 py-4 w-50 text-sm">
                            ${market.volume_usd?.toLocaleString() ||
                                "N/A"}
                            <br />
                            {market.traders_count?.toLocaleString() ||
                                "N/A"}
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 24 24"
                                height={16}
                                fill="currentColor"
                                class="inline"
                            >
                                <title>People</title>
                                <path
                                    d="M16 17V19H2V17S2 13 9 13 16 17 16 17M12.5 7.5A3.5 3.5 0 1 0 9 11A3.5 3.5 0 0 0 12.5 7.5M15.94 13A5.32 5.32 0 0 1 18 17V19H22V17S22 13.37 15.94 13M15 4A3.39 3.39 0 0 0 13.07 4.59A5 5 0 0 1 13.07 10.41A3.39 3.39 0 0 0 15 11A3.5 3.5 0 0 0 15 4Z"
                                />
                            </svg>
                        {market.duration_days?.toLocaleString() ||
                            "N/A"}
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            height={16}
                            fill="currentColor"
                            class="inline"
                        >
                            <title>Days</title>
                            <path d="M19,19H5V8H19M16,1V3H8V1H6V3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3H18V1" />
                        </svg>
                        <br />
                        {market.close_datetime.split('T')[0]}
                        </td>
                        <td class="px-6 py-4 w-50 text-sm font-medium actions">
                            <a
                                href={`/markets/edit?id=${market.id}`}
                                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue mr-2"
                            >
                                View
                            </a>
                            <button
                                on:click={() => handleDismiss(market.id, 1)}
                                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-red/50 hover:bg-red"
                            >
                                Dismiss
                            </button>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    {/if}
</div>
