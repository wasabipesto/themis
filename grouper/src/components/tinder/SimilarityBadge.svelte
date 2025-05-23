<script lang="ts">
  // Similarity score (lower is better in this context)
  export let score: number | null = null;
  
  // Calculate color based on score
  $: color = getColorClass();
  
  function getColorClass() {
    if (score === null) return "bg-overlay0";
    
    // Lower score means higher similarity
    if (score <= 0.5) return "bg-green";
    if (score <= 0.65) return "bg-teal";
    if (score <= 0.75) return "bg-blue";
    if (score <= 0.85) return "bg-yellow";
    return "bg-red";
  }
  
  // Format the score for display
  $: displayScore = score === null ? "N/A" : Math.round(score * 100) / 100;
</script>

<div class="inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium {color} text-crust">
  <span class="mr-1">Similarity:</span>
  <span class="font-bold">{displayScore}</span>
</div>