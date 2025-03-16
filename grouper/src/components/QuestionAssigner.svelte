<script lang="ts">
    import type { Market } from "@types";
    import { onMount } from "svelte";
    import { unlinkMarket, linkMarket, getAssocMarkets } from "@lib/api";

    let markets: Market[] = [];
    let loading = true;
    let error: string | null = null;
    let questionId: string = "";
    let newMarketId = "";
    let linkError: string | null = null;
    let linkSuccess = false;
    let linkLoading = false;

    onMount(async () => {
        try {
            const urlParams = new URLSearchParams(window.location.search);
            const questionIdGet = urlParams.get("id");

            if (!questionIdGet) {
                loading = false;
                return;
            } else {
                questionId = questionIdGet;
            }

            // Fetch markets associated with this question
            markets = await getAssocMarkets(questionId);

            loading = false;
        } catch (err: unknown) {
            error =
                err instanceof Error
                    ? err.message
                    : "Failed to load question or markets";
            loading = false;
        }
    });

    async function handleRemoveMarket(market: Market) {
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

    async function handleLinkMarket() {
        if (!newMarketId.trim()) {
            linkError = "Please enter a market ID";
            return;
        }

        linkError = null;
        linkSuccess = false;
        linkLoading = true;

        try {
            // Pass the market ID and question ID to the linkMarket function
            await linkMarket(newMarketId.trim(), questionId);

            // Fetch updated list of markets
            markets = await getAssocMarkets(questionId);

            // Clear the input and show success message
            newMarketId = "";
            linkSuccess = true;
            setTimeout(() => (linkSuccess = false), 3000); // Clear success message after 3 seconds
        } catch (err: unknown) {
            linkError =
                err instanceof Error ? err.message : "Failed to link market";
        } finally {
            linkLoading = false;
        }
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
            <h2 class="text-2xl font-semibold mb-4">Linked Markets</h2>

            {#if markets.length === 0}
                <div
                    class="bg-blue/10 p-6 mb-6 rounded-lg shadow-md text-center"
                >
                    <p class="text-text/70">
                        No markets assigned to this question.
                    </p>
                </div>
            {:else}
                <div class="bg-crust p-6 mb-6 rounded-lg shadow-md">
                    <table class="w-full">
                        <thead>
                            <tr class="border-b border-blue/20">
                                <th class="text-left py-2 px-3 text-text/70">
                                    Int.
                                </th>
                                <th class="text-left py-2 px-3 text-text/70">
                                    Ext.
                                </th>
                                <th class="text-left py-2 px-3 text-text/70">
                                    Platform
                                </th>
                                <th class="text-left py-2 px-3 text-text/70">
                                    Title
                                </th>
                                <th class="text-center py-2 px-3 text-text/70">
                                    Invert
                                </th>
                                <th class="text-right py-2 px-3 text-text/70">
                                    Stats
                                </th>
                                <th class="text-center py-2 px-3 text-text/70">
                                    Actions
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each markets as market}
                                <tr
                                    class="border-b border-blue/10 hover:bg-blue/5 text-sm"
                                >
                                    <td class="py-3 px-3">
                                        <a
                                            href={`/markets/edit?id=${market.id}`}
                                            class="text-blue hover:underline"
                                        >
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 24 24"
                                                height={20}
                                                fill="currentColor"
                                                class="ml-1"
                                            >
                                                <title>Link</title>
                                                <path
                                                    d="M14,3V5H17.59L7.76,14.83L9.17,16.24L19,6.41V10H21V3M19,19H5V5H12V3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V12H19V19Z"
                                                />
                                            </svg>
                                        </a>
                                    </td>
                                    <td class="py-3 px-3">
                                        <a
                                            href={market.url}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="text-blue hover:underline"
                                        >
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 24 24"
                                                height={20}
                                                fill="currentColor"
                                                class="ml-1"
                                            >
                                                <title>Link</title>
                                                <path
                                                    d="M14,3V5H17.59L7.76,14.83L9.17,16.24L19,6.41V10H21V3M19,19H5V5H12V3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V12H19V19Z"
                                                />
                                            </svg>
                                        </a>
                                    </td>
                                    <td class="py-3 px-3"
                                        >{market.platform_name}</td
                                    >
                                    <td class="py-3 px-3">{market.title}</td>
                                    <td class="py-3 px-3 text-center">
                                        {#if market.question_invert}
                                            Y
                                        {:else}
                                            N
                                        {/if}
                                    </td>
                                    <td class="py-3 px-3 text-right">
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
                                    </td>
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

            <!-- Add market form -->
            <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
                <h2 class="text-xl font-semibold mb-4">
                    Link Market to Question
                </h2>
                <div class="flex flex-col md:flex-row gap-4">
                    <div class="flex-grow">
                        <input
                            type="text"
                            bind:value={newMarketId}
                            placeholder="Enter Market ID"
                            class="w-full px-4 py-2 rounded-md bg-mantle border border-blue/30 focus:border-blue focus:outline-none"
                        />
                    </div>
                    <button
                        on:click={handleLinkMarket}
                        disabled={linkLoading}
                        class="px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {#if linkLoading}
                            <span
                                class="inline-block w-4 h-4 border-2 border-text border-t-transparent rounded-full animate-spin mr-2"
                            ></span>
                            Linking...
                        {:else}
                            Link Market
                        {/if}
                    </button>
                </div>

                {#if linkError}
                    <p class="text-red mt-2">{linkError}</p>
                {/if}

                {#if linkSuccess}
                    <p class="text-green-400 mt-2">
                        Market successfully linked!
                    </p>
                {/if}
            </div>
        {/if}
    </div>
{/if}
