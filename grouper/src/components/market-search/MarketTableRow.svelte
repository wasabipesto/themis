<script lang="ts">
  import type { MarketDetails } from "@types";
  import MarketBadge from "./MarketBadge.svelte";
  import MarketStats from "./MarketStats.svelte";
  import ActionButtons from "./ActionButtons.svelte";

  export let market: MarketDetails;
  export let onDismiss: (id: string, level: number) => void;
</script>

<tr class="hover:bg-base-dark">
  <td class="px-6 py-4 text-sm">
    {market.platform_name}
  </td>
  <td class="px-6 py-4 text-sm">
    {market.title}
    <br />
    <MarketBadge resolution={market.resolution} />
  </td>
  <td class="px-6 py-4 w-50 text-sm">
    <MarketStats
      volumeUsd={market.volume_usd}
      tradersCount={market.traders_count}
      durationDays={market.duration_days}
      closeDateTime={market.close_datetime}
    />
  </td>
  <td class="px-6 py-4 w-60 text-sm font-medium actions">
    <ActionButtons marketId={market.id} {onDismiss} />
  </td>
</tr>
