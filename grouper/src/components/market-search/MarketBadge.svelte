<script lang="ts">
  export let resolution: number | null;
</script>

{#if resolution === 1.0}
  <span class="mx-1 px-2 rounded-md bg-green/20">YES</span>
{:else if resolution === 0.0}
  <span class="mx-1 px-2 rounded-md bg-red/20">NO</span>
{:else if resolution !== null}
  <span class="mx-1 px-2 rounded-md bg-teal/20">{resolution}</span>
{/if}
