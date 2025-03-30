<script lang="ts">
  import type { MarketDetails } from "@types";
  import MarketBadge from "./MarketBadge.svelte";
  import MarketStats from "./MarketStats.svelte";

  export let platformName: string = "";
  export let markets: MarketDetails[] = [];
</script>

<div class="w-full rounded-lg shadow bg-crust">
  <table class="w-full divide-y divide-subtext">
    <thead>
      <tr>
        <th
          class="px-6 py-2 text-left text-xs font-medium uppercase tracking-wider"
        >
          {platformName} Results
        </th>
        <th
          class="px-6 py-2 text-left text-xs font-medium uppercase tracking-wider w-40"
        >
          Stats
        </th>
        <th
          class="px-6 py-2 text-left text-xs font-medium uppercase tracking-wider w-40"
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
            <td class="px-6 py-2 w-40 text-sm font-medium actions">
              <a
                href={`/markets/edit?id=${market.id}`}
                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue"
              >
                View
              </a>
              <button
                on:click={() => navigator.clipboard.writeText(market.id)}
                class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-teal/50 hover:bg-teal"
              >
                ID
              </button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
