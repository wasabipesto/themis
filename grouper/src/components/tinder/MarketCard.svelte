<script lang="ts">
  import type { MarketDetails } from "@types";
  import { SimilarityBadge } from "./";

  // Props
  export let market: MarketDetails;
  export let similarity: number | null = null;
  export let isMain: boolean = false;

  // Format dates for display
  function formatDate(dateString: string | null | undefined): string {
    if (!dateString) return "N/A";
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  // Format currency for display
  function formatCurrency(amount: number | null | undefined): string {
    if (amount === null || amount === undefined) return "N/A";
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  }
</script>

<div
  class="bg-crust p-4 rounded-lg border {isMain
    ? 'border-green'
    : 'border-overlay0'} shadow-sm mb-3"
>
  <div class="flex justify-between items-start mb-2">
    <div class="flex items-center">
      <div
        class="px-2 py-1 rounded bg-base-light text-crust text-xs font-medium"
      >
        {market.platform_name || market.platform_slug}
      </div>
      {#if market.category_name}
        <div
          class="ml-2 px-2 py-1 rounded bg-blue text-crust text-xs font-medium"
        >
          {market.category_name}
        </div>
      {/if}
      {#if similarity !== null}
        <div class="ml-2">
          <SimilarityBadge score={similarity} />
        </div>
      {/if}
    </div>

    {#if isMain}
      <div class="px-2 py-1 rounded bg-green text-crust text-xs font-medium">
        Primary Market
      </div>
    {/if}
  </div>

  <h3 class="text-lg font-semibold mb-2">{market.title}</h3>

  <div class="grid grid-cols-2 gap-2 text-sm mb-2">
    <div>
      <span class="font-medium text-overlay2">Open:</span>
      <span>{formatDate(market.open_datetime)}</span>
    </div>
    <div>
      <span class="font-medium text-overlay2">Close:</span>
      <span>{formatDate(market.close_datetime)}</span>
    </div>
    <div>
      <span class="font-medium text-overlay2">Volume:</span>
      <span>{formatCurrency(market.volume_usd)}</span>
    </div>
    <div>
      <span class="font-medium text-overlay2">Traders:</span>
      <span>{market.traders_count || "N/A"}</span>
    </div>
  </div>

  {#if market.description && market.description.length > 0}
    <div
      class="text-sm mb-3 text-text/80 border-t border-overlay0 pt-2 line-clamp-2"
    >
      {market.description}
    </div>
  {/if}

  <div class="flex justify-end gap-2">
    <a
      href={`/markets/edit?id=${market.id}`}
      class="text-xs px-2 py-1 bg-lavender hover:bg-lavender/80 text-crust rounded-md"
    >
      View Internally
    </a>
    <a
      href={market.url}
      target="_blank"
      rel="noopener noreferrer"
      class="text-xs px-2 py-1 bg-blue hover:bg-blue/80 text-crust rounded-md"
    >
      View on {market.platform_name}
    </a>
  </div>
</div>
