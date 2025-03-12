<script>
    import { onMount } from "svelte";
    import { getQuestion, unlinkMarket, getAssocMarkets } from "@lib/api";

    let question = {};
    let markets = [];
    let loading = true;
    let error = null;
    let questionId = null;

    onMount(async () => {
        try {
            const urlParams = new URLSearchParams(window.location.search);
            questionId = urlParams.get("id");

            if (!questionId) {
                loading = false;
                return;
            }

            // Fetch question data
            question = await getQuestion(questionId);

            // Fetch markets associated with this question
            markets = await getAssocMarkets(questionId);

            loading = false;
        } catch (err) {
            error = err.message || "Failed to load question or markets";
            loading = false;
        }
    });

    async function handleRemoveMarket(market) {
        if (
            !confirm(
                "Are you sure you want to remove this market from the question?",
            )
        ) {
            return;
        }

        await unlinkMarket(market);
        // Remove item from the list for instant UI update
        markets = markets.filter((m) => m.id !== market.id);
    }
</script>

{#if questionId}
    <div class="max-w-4xl mx-auto my-6">
        {#if loading}
            <div class="flex justify-center items-center p-12">
                <div
                    class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue"
                ></div>
            </div>
        {:else if error}
            <div class="bg-red/20 p-6 rounded-lg shadow-md text-center">
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
            {#if markets.length === 0}
                <div class="bg-blue/10 p-6 rounded-lg shadow-md text-center">
                    <p class="text-text/70">
                        No markets assigned to this question.
                    </p>
                </div>
            {:else}
                <div class="bg-crust p-6 rounded-lg shadow-md">
                    <table class="w-full">
                        <thead>
                            <tr class="border-b border-blue/20">
                                <th class="text-left py-2 px-3 text-text/70"
                                    >Market</th
                                >
                                <th class="text-left py-2 px-3 text-text/70"
                                    >Platform</th
                                >
                                <th class="text-right py-2 px-3 text-text/70"
                                    >Volume</th
                                >
                                <th class="text-right py-2 px-3 text-text/70"
                                    >Traders</th
                                >
                                <th class="text-center py-2 px-3 text-text/70"
                                    >Actions</th
                                >
                            </tr>
                        </thead>
                        <tbody>
                            {#each markets as market}
                                <tr
                                    class="border-b border-blue/10 hover:bg-blue/5"
                                >
                                    <td class="py-3 px-3">
                                        <a
                                            href={market.market_link}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="text-blue hover:underline"
                                        >
                                            {market.market_id}
                                        </a>
                                    </td>
                                    <td class="py-3 px-3"
                                        >{market.platform_name}</td
                                    >
                                    <td class="py-3 px-3 text-right"
                                        >${market.volume?.toLocaleString() ||
                                            "0"}</td
                                    >
                                    <td class="py-3 px-3 text-right"
                                        >{market.traders?.toLocaleString() ||
                                            "0"}</td
                                    >
                                    <td class="py-3 px-3 text-center">
                                        <button
                                            on:click={() =>
                                                handleRemoveMarket(market)}
                                            class="px-3 py-1 bg-red/50 text-text rounded-md hover:bg-red transition-colors"
                                        >
                                            Remove
                                        </button>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}

            <div class="mt-6 flex justify-end">
                <a
                    href="/questions"
                    class="px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
                >
                    Back to Questions
                </a>
            </div>
        {/if}
    </div>
{/if}
