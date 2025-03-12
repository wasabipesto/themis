<script>
    import { onMount } from "svelte";
    import { getMarkets } from "@lib/api";

    // State
    let items = [];
    let loading = true;
    let error = null;

    onMount(loadTableData);

    async function loadTableData() {
        try {
            items = await getMarkets(
                "order=volume_usd.desc.nullslast&platform_slug=eq.manifold",
            );
            error = items.length === 0 ? "No items found." : null;
        } catch (err) {
            error = `Error loading data: ${err.message}`;
            console.error("Error loading table data:", err);
        } finally {
            loading = false;
        }
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
