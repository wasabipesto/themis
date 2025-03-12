<script>
    import { onMount } from "svelte";
    import { getMarket, getQuestion } from "@lib/api";

    let market = null;
    let question = null;
    let loading = true;
    let error = null;
    let marketId = null;

    onMount(async () => {
        try {
            const urlParams = new URLSearchParams(window.location.search);
            marketId = urlParams.get("id");

            if (!marketId) {
                error = "No market ID provided";
                loading = false;
                return;
            }

            // Fetch market data
            market = await getMarket(marketId);

            // Fetch question data
            if (market.question_id) {
                question = await getQuestion(market.question_id);
            }

            loading = false;
        } catch (err) {
            error = err.message || "Failed to load market data";
            loading = false;
        }
    });

    function formatDate(dateString) {
        if (!dateString) return "N/A";
        return new Date(dateString).toLocaleDateString("en-US", {
            year: "numeric",
            month: "long",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    function formatProbability(prob) {
        if (prob === null || prob === undefined) return "N/A";
        return `${(prob * 100).toFixed(1)}%`;
    }
</script>

<div class="max-w-4xl mx-auto mb-6">
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
                href="/markets"
                class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
            >
                Back to Markets
            </a>
        </div>
    {:else if market}
        <div class="bg-crust p-6 rounded-lg shadow-md mb-6">
            <div class="flex justify-between items-start mb-2">
                <h1 class="text-2xl font-bold">{market.title}</h1>
            </div>
            <div class="flex justify-between items-start mb-2">
                <h1 class="text-xs">{market.id}</h1>
            </div>

            <div class="mb-6">
                {#if market.category}
                    <span
                        class="text-sm bg-rosewater/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
                    >
                        {market.category}
                    </span>
                {/if}
                {#if market.volume_usd > 1000}
                    <span
                        class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
                    >
                        High Volume
                    </span>
                {/if}
                {#if market.volume_usd > 100}
                    <span
                        class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
                    >
                        High Traders
                    </span>
                {/if}
                {#if market.duration_days > 100}
                    <span
                        class="text-sm bg-green/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
                    >
                        High Duration
                    </span>
                {/if}
                <span
                    class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2"
                >
                    <a
                        href={market.url}
                        target="_blank"
                        rel="noopener noreferrer"
                    >
                        View on {market.platform_name} →
                    </a>
                </span>
            </div>

            <div class="mb-6">
                <h2 class="text-xl font-semibold mb-2">Description</h2>
                <div class="bg-mantle p-4 rounded-md">
                    <p class="whitespace-pre-line">{market.description}</p>
                </div>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
                <div>
                    <h2 class="text-xl font-semibold mb-2">Market Details</h2>
                    <div class="bg-mantle p-4 rounded-md">
                        <dl>
                            <dt class="text-text/70">Open Date</dt>
                            <dd class="mb-2">
                                {formatDate(market.open_datetime)}
                            </dd>

                            <dt class="text-text/70">Close Date</dt>
                            <dd class="mb-2">
                                {formatDate(market.close_datetime)}
                            </dd>

                            <dt class="text-text/70">Probability (Average)</dt>
                            <dd>{formatProbability(market.prob_time_avg)}</dd>

                            <dt class="text-text/70">Resolution</dt>
                            <dd class="mb-2">
                                {#if market.resolution === null || market.resolution === undefined}
                                    Unresolved
                                {:else if market.resolution === 1}
                                    Yes (1)
                                {:else if market.resolution === 0}
                                    No (0)
                                {:else}
                                    Prob ({market.resolution})
                                {/if}
                            </dd>
                        </dl>
                    </div>
                </div>

                <div>
                    <h2 class="text-xl font-semibold mb-2">
                        Market Statistics
                    </h2>
                    <div class="bg-mantle p-4 rounded-md">
                        <dl>
                            <dt class="text-text/70">Traders</dt>
                            <dd class="mb-2">
                                {market.traders_count?.toLocaleString() ||
                                    "N/A"}
                            </dd>

                            <dt class="text-text/70">Volume (USD)</dt>
                            <dd class="mb-2">
                                ${market.volume_usd?.toLocaleString() || "N/A"}
                            </dd>

                            <dt class="text-text/70">Duration (days)</dt>
                            <dd class="mb-2">
                                {market.duration_days?.toLocaleString() ||
                                    "N/A"}
                            </dd>
                        </dl>
                    </div>
                </div>
            </div>

            {#if question}
                <div>
                    <h2 class="text-xl font-semibold mb-2">Question Link</h2>
                    <div class="bg-mantle p-4 rounded-md">
                        <p>
                            This market is linked to question:
                            <a
                                href={`/questions/edit?id=${question.id}`}
                                class="text-blue hover:underline"
                            >
                                {question.title}
                            </a>
                        </p>
                        {#if market.question_invert}
                            <p class="mt-2 text-yellow">
                                Note: This market is inverted relative to the
                                question.
                            </p>
                        {/if}
                    </div>
                </div>
            {/if}
        </div>
    {:else}
        <div class="bg-yellow/20 p-6 rounded-lg shadow-md text-center">
            <p>No market found with ID: {marketId}</p>
            <a
                href="/markets"
                class="mt-4 inline-block px-4 py-2 bg-blue/50 text-text rounded-md hover:bg-blue transition-colors"
            >
                Back to Markets
            </a>
        </div>
    {/if}
</div>
