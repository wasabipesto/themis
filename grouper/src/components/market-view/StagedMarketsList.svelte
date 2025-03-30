<script lang="ts">
  import type { MarketDetails } from "@types";

  export let stagedMarkets: MarketDetails[] = [];
  export let creatingQuestion: boolean = false;
  export let onUnstage: (marketId: string) => void;
  export let onCreateQuestion: () => Promise<void>;
</script>

<div class="bg-crust p-6 rounded-lg shadow-md mb-6">
  <div class="flex justify-between items-center mb-2">
    <h2 class="text-xl font-semibold mb-2">
      Staged Markets ({stagedMarkets.length})
    </h2>
    <button
      on:click={onCreateQuestion}
      disabled={creatingQuestion}
      class="inline-flex items-center px-4 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {creatingQuestion ? "Creating..." : "Create Question"}
    </button>
  </div>
  <div class="space-y-2 max-h-60 overflow-y-auto">
    {#each stagedMarkets as stagedMarket}
      <div class="flex justify-between items-center p-2 bg-base rounded">
        <span class="text-sm truncate flex-1">
          {stagedMarket.platform_name} | {stagedMarket.title}
        </span>
        <a
          href={`/markets/edit?id=${stagedMarket.id}`}
          target="_blank"
          class="mx-1 px-2 py-1 text-sm rounded-md text-white bg-blue/50 hover:bg-blue"
        >
          View
        </a>
        <button
          on:click={() => onUnstage(stagedMarket.id)}
          class="mx-1 px-2 py-1 text-sm rounded-md text-white bg-red/50 hover:bg-red"
        >
          Unstage
        </button>
      </div>
    {/each}
  </div>
</div>
