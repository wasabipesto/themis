<script lang="ts">
  import type { MarketDetails } from "@types";

  export let market: MarketDetails;

  function formatDate(dateString: string) {
    if (!dateString) return "N/A";
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
    });
  }
</script>

<div class="bg-crust p-6 rounded-lg shadow-md mb-6">
  <!-- Market Title & Resolution Chip -->
  <div class="flex justify-between items-start mb-2">
    <h1 class="text-2xl font-bold">
      {market.title}
      {#if market.resolution == 1.0}
        <span class="px-2 rounded-md bg-green/20"> YES </span>
      {:else if market.resolution == 0.0}
        <span class="px-2 rounded-md bg-red/20"> NO </span>
      {:else}
        <span class="px-2 rounded-md bg-teal/20">
          {market.resolution.toFixed(2)}
        </span>
      {/if}
    </h1>
  </div>

  <!-- Market ID -->
  <div class="flex justify-between items-start mb-2">
    <h1 class="text-xs break-all">{market.id}</h1>
  </div>

  <!-- Action Chips -->
  <div class="mb-0">
    <button
      on:click={() => navigator.clipboard.writeText(market?.id || "")}
      class="inline-flex items-center px-3 py-1 mr-2 mb-2 text-sm rounded-md text-white bg-teal/50 hover:bg-teal"
    >
      Copy ID
    </button>
    <span class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2">
      <a href={market.url} target="_blank" rel="noopener noreferrer">
        View on {market.platform_name} →
      </a>
    </span>
  </div>

  <!-- Info Chips -->
  <div class="mb-6">
    {#if market.category_name}
      <span class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2">
        {market.category_name}
      </span>
    {/if}
    {#if market.volume_usd}
      <span class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2">
        ${market.volume_usd
          ? Math.round(market.volume_usd).toLocaleString()
          : "N/A"} Volume
      </span>
    {/if}
    {#if market.traders_count}
      <span class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2">
        {market.traders_count?.toLocaleString() || "N/A"}
      </span>
    {/if}
    <span class="text-sm bg-blue/20 text-text px-4 py-1 rounded-md mr-2 mb-2">
      {formatDate(market.open_datetime)} to {formatDate(market.close_datetime)} ({market.duration_days}d)
    </span>
  </div>

  <div class="mb-0">
    <h2 class="text-xl font-semibold mb-2">Description</h2>
    <div class="bg-mantle p-4 rounded-md">
      <p class="whitespace-pre-line max-h-60 overflow-y-auto pr-2">
        {market.description}
      </p>
    </div>
  </div>
</div>
