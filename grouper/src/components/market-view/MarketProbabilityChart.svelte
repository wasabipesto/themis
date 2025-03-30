<script lang="ts">
  import type { DailyProbabilityDetails } from "@types";
  import * as Plot from "@observablehq/plot";
  import { onMount, afterUpdate } from "svelte";

  export let plotData: DailyProbabilityDetails[] = [];
  let plotRendered = false;

  afterUpdate(() => {
    if (plotData.length > 0 && !plotRendered) {
      renderPlot();
    }
  });

  function renderPlot() {
    const plotElement = document.querySelector("#plot");
    if (!plotElement) {
      console.error("Error rendering plot: Could not find plot element");
      return;
    }

    try {
      const plot = Plot.plot({
        width: plotElement.clientWidth || 600,
        height: 300,
        x: { type: "utc", label: "Date" },
        y: {
          domain: [0, 100],
          grid: true,
          percent: true,
          label: "Probability",
        },
        marks: [
          Plot.line(plotData, {
            x: "date",
            y: "prob",
            curve: "step",
            tip: {
              fill: "black",
            },
          }),
          Plot.ruleY([0]),
        ],
      });

      // Clear any existing plots first
      while (plotElement.firstChild) {
        plotElement.firstChild.remove();
      }

      plotElement.append(plot);
      plotRendered = true;
    } catch (e) {
      console.error("Error rendering plot:", e);
    }
  }
</script>

<div class="bg-crust p-6 rounded-lg shadow-md mb-4">
  <h2 class="text-xl font-semibold mb-4">Probability History</h2>
  <div id="plot" class="w-full h-[300px]"></div>
</div>
