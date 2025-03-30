<script lang="ts">
  import type { Platform } from "@types";

  export let platforms: Platform[] = [];
  export let selectedPlatform = "";
  export let selectedSort = "volume_usd.desc.nullslast";
  export let onChange: () => void;

  // Available sorting options
  export const sortOptions = [
    { value: "title.asc", label: "Title A-Z" },
    { value: "title.desc", label: "Title Z-A" },
    { value: "open_datetime.desc", label: "Newest (by open)" },
    { value: "open_datetime.asc", label: "Oldest (by open)" },
    { value: "close_datetime.asc", label: "Newest (by close)" },
    { value: "close_datetime.desc", label: "Oldest (by close)" },
    { value: "traders_count.desc.nullslast", label: "Most traders" },
    { value: "traders_count.asc.nullslast", label: "Least traders" },
    { value: "volume_usd.desc.nullslast", label: "Highest volume" },
    { value: "volume_usd.asc.nullslast", label: "Lowest volume" },
    { value: "duration_days.desc", label: "Longest duration" },
    { value: "duration_days.asc", label: "Shortest duration" },
    { value: "prob_time_avg.desc", label: "Highest average prob" },
    { value: "prob_time_avg.asc", label: "Lowest average prob" },
  ];
</script>

<div class="mt-2 flex gap-2">
  <select
    class="w-1/2 px-4 py-2 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
    bind:value={selectedPlatform}
    on:change={onChange}
  >
    <option value="">All Platforms</option>
    {#each platforms as platform}
      <option value={platform.slug}>{platform.name}</option>
    {/each}
  </select>

  <select
    class="w-1/2 px-4 py-2 bg-crust rounded-lg focus:outline-none focus:ring-1 focus:ring-lavender"
    bind:value={selectedSort}
    on:change={onChange}
  >
    {#each sortOptions as option}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</div>
