<script lang="ts">
  import type { MarketDetails } from "@types";
  import MarketBadge from "./MarketBadge.svelte";
  import MarketStats from "./MarketStats.svelte";
  import { dismissMarket } from "@lib/api";

  export let platformName: string = "";
  export let markets: MarketDetails[] = [];
  export let stagedMarkets: MarketDetails[] = [];
  export let onStage: (market: MarketDetails) => void;

  // Helper function to check if a market is already staged
  function isMarketStaged(marketId: string): boolean {
    return stagedMarkets.some((m) => m.id === marketId);
  }

  // Market dismiss function
  async function handleDismiss(marketId: string, level: number = 1) {
    try {
      // First, filter out the dismissed market from our local markets array
      markets = markets.filter((market) => market.id !== marketId);
      // Then call the API to dismiss the market
      await dismissMarket(marketId, level);
    } catch (err) {
      console.error("Error dismissing market:", err);
      alert(
        "Failed to dismiss market: " +
          (err instanceof Error ? err.message : String(err)),
      );
    }
  }
</script>

<div class="w-full rounded-lg shadow bg-crust">
  <table class="w-full divide-y divide-subtext">
    <thead>
      <tr>
        <th
          class="px-2 py-2 text-left text-xs font-medium uppercase tracking-wider"
        >
          {platformName} Results
        </th>
        <th
          class="px-2 py-2 w-40 text-center text-xs font-medium uppercase tracking-wider"
        >
          Stats
        </th>
        <th
          class="px-2 py-2 w-40 text-center text-xs font-medium uppercase tracking-wider"
        >
          Actions
        </th>
      </tr>
    </thead>
  </table>

  <div class="max-h-80 overflow-y-auto">
    <table class="w-full divide-y divide-subtext">
      <tbody class="divide-y divide-subtext">
        {#each markets as market}
          <tr class="hover:bg-base-dark">
            <td class="px-6 py-2 text-sm">
              {market.title}
              <MarketBadge resolution={market.resolution} />
            </td>
            <td class="px-6 py-2 w-40 text-sm">
              <MarketStats
                volumeUsd={market.volume_usd}
                tradersCount={market.traders_count}
                durationDays={market.duration_days}
                closeDateTime={market.close_datetime}
              />
            </td>
            <td class="px-2 py-2 w-40 text-sm font-medium actions">
              <div class="flex gap-1 mb-1">
                <a
                  href={`/markets/edit?id=${market.id}`}
                  target="_blank"
                  class="inline-flex items-center px-2 py-1 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue"
                >
                  View
                </a>
                <button
                  on:click={() => onStage(market)}
                  class="inline-flex items-center px-2 py-1 text-sm font-medium rounded-md text-white bg-yellow/50 hover:bg-yellow"
                >
                  {isMarketStaged(market.id) ? "Unstage" : "Stage"}
                </button>
              </div>
              <div class="flex gap-1">
                <button
                  on:click={() => navigator.clipboard.writeText(market.id)}
                  class="inline-flex items-center px-2 py-1 text-sm font-medium rounded-md text-white bg-teal/50 hover:bg-teal"
                >
                  ID
                </button>
                <button
                  on:click={() => handleDismiss(market.id, 1)}
                  class="inline-flex items-center px-2 py-1 text-sm font-medium rounded-md text-white bg-red/50 hover:bg-red"
                >
                  Dismiss
                </button>
              </div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
