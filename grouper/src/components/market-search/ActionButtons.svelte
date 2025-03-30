<script lang="ts">
  export let marketId: string;
  export let onDismiss: (id: string, level: number) => void;
</script>

<div>
  <a
    href={`/markets/edit?id=${marketId}`}
    class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-blue/50 hover:bg-blue"
  >
    View
  </a>
  <button
    on:click={() => navigator.clipboard.writeText(marketId)}
    class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-teal/50 hover:bg-teal"
  >
    ID
  </button>
  <button
    on:click={() => onDismiss(marketId, 1)}
    class="inline-flex items-center px-3 py-2 text-sm font-medium rounded-md text-white bg-red/50 hover:bg-red"
  >
    Dismiss
  </button>
</div>
