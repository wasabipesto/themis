<script lang="ts">
  export let resolution: number | null;
</script>

{#if resolution === 1.0}
  <span class="px-2 rounded-md bg-green/20">Resolved YES</span>
{:else if resolution === 0.0}
  <span class="px-2 rounded-md bg-red/20">Resolved NO</span>
{:else if resolution !== null}
  <span class="px-2 rounded-md bg-teal/20">Resolved {resolution}</span>
{/if}
