<script lang="ts">
  import type { MarketDetails } from "@types";
  import MarketTableRow from "./MarketTableRow.svelte";

  export let markets: MarketDetails[] = [];
  export let onDismiss: (id: string, level: number) => void;
</script>

<table class="w-full divide-y divide-subtext bg-crust rounded-lg shadow">
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
      <MarketTableRow {market} {onDismiss} />
    {/each}
  </tbody>
</table>
