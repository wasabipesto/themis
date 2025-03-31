<script lang="ts">
  export let volumeUsd: number | null;
  export let tradersCount: number | null;
  export let durationDays: number | null;
  export let closeDateTime: string;
</script>

<div>
  {#if volumeUsd}
    ${Math.round(volumeUsd).toLocaleString()}
    <br />
  {/if}
  {#if tradersCount}
    {tradersCount.toLocaleString() || "N/A"}
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      height={16}
      fill="currentColor"
      class="inline mx-0.5 mr-1"
    >
      <title>People</title>
      <path
        d="M16 17V19H2V17S2 13 9 13 16 17 16 17M12.5 7.5A3.5 3.5 0 1 0 9 11A3.5 3.5 0 0 0 12.5 7.5M15.94 13A5.32 5.32 0 0 1 18 17V19H22V17S22 13.37 15.94 13M15 4A3.39 3.39 0 0 0 13.07 4.59A5 5 0 0 1 13.07 10.41A3.39 3.39 0 0 0 15 11A3.5 3.5 0 0 0 15 4Z"
      />
    </svg>
  {/if}
  {#if durationDays && durationDays > 999}<br />{/if}
  {durationDays?.toLocaleString() || "N/A"}
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    height={16}
    fill="currentColor"
    class="inline mx-0.5 mr-1"
  >
    <title>Days</title>
    <path
      d="M19,19H5V8H19M16,1V3H8V1H6V3H5C3.89,3 3,3.89 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V5C21,3.89 20.1,3 19,3H18V1"
    />
  </svg>
  <br />
  {closeDateTime.split("T")[0]}
</div>
