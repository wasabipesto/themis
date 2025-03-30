<script lang="ts">
  export let searchQuery = "";
  export let onSearch: () => void;
</script>

<div class="flex gap-2 my-2">
  <input
    type="text"
    placeholder="Search markets..."
    class="w-full px-4 py-2 pl-4 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
    bind:value={searchQuery}
    on:keydown={(e) => e.key === "Enter" && onSearch()}
  />
  <button
    class="px-4 py-2 bg-blue hover:bg-blue/80 text-white rounded-md"
    on:click={onSearch}
  >
    Search
  </button>
</div>
