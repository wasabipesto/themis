/**
 * Chart Manager
 * 
 * Provides unified handling of chart initialization, rendering, and selection management.
 */
import * as Plot from "@observablehq/plot";
import type { PlatformDetails } from "@types";

/**
 * Generic option data interface that all chart types should be compatible with
 */
export interface ChartOption {
  description: string;
  [key: string]: any;
}

/**
 * Base chart element interface that all chart types should implement
 */
export interface ChartElement extends HTMLElement {
  dataset: {
    platforms: string;
    options: string;
    [key: string]: string;
  };
}

/**
 * Initialize selection behavior for a chart
 * 
 * @param plotId Unique identifier for the plot
 * @param chartClass CSS class for chart elements
 * @param optionClass CSS class for option elements
 * @param updateFn Function to update the chart when selection changes
 */
export function initializeChartSelection<T extends ChartOption, E extends ChartElement>(
  plotId: string,
  chartClass: string,
  optionClass: string,
  updateFn: (plotElement: E, option: T) => void
): void {
  // Get all matching plot elements
  const plotElements = document.querySelectorAll(
    `.${chartClass}`
  ) as NodeListOf<E>;

  // Set up each plot
  plotElements.forEach((plotElement) => {
    // Skip if this isn't the plot we're looking for
    if (!plotElement.id.includes(plotId)) return;

    // Extract options
    const options = JSON.parse(plotElement.dataset.options) as T[];

    // Find options that match the plot ID
    const radios = [
      ...document.querySelectorAll(`.${optionClass}-${plotId}`),
    ] as HTMLInputElement[];

    // Set the first radio button to checked by default
    if (radios.length > 0) {
      radios[0].checked = true;
    }

    // Update the plot with initial values
    updateFn(plotElement, getSelectedOption(options, radios));

    // Set the callbacks for future changes
    radios.forEach((radio) => {
      radio.addEventListener("change", () => {
        const selectedOption = getSelectedOption(options, radios);
        updateFn(plotElement, selectedOption);
      });
    });
  });
}

/**
 * Get the selected option based on user input
 */
export function getSelectedOption<T>(
  options: T[],
  radios: HTMLInputElement[],
): T {
  for (let i = 0; i < radios.length; i++) {
    if (radios[i].checked) {
      return options[i];
    }
  }
  // Default to first option if none are selected
  return options[0];
}

/**
 * Create a basic plot with standard configuration
 */
export function createBasicPlot(
  title: string,
  element: HTMLElement,
  config: any
): Plot.Plot {
  const plotWidth = parseInt(window.getComputedStyle(element).width);
  
  return Plot.plot({
    title: title,
    width: plotWidth,
    ...config
  });
}

/**
 * Replace the contents of a plot element with a new plot
 */
export function updatePlotElement(
  element: HTMLElement,
  plot: Plot.Plot
): void {
  // Wipe the existing plot and add the new one
  while (element.firstChild) {
    element.removeChild(element.firstChild);
  }
  // Cast to any to avoid TypeScript error with Plot type
  element.appendChild(plot as any);
}

/**
 * Extract plot ID from element ID
 */
export function extractPlotId(elementId: string, prefix: string): string {
  const regex = new RegExp(`${prefix}-(.*)$`);
  const match = elementId.match(regex);
  if (match && match.length > 1) {
    return match[1];
  }
  throw new Error(`Invalid plot ID: ${elementId}`);
}

/**
 * Parse platforms data from element
 */
export function getPlatforms(element: ChartElement): PlatformDetails[] {
  return JSON.parse(element.dataset.platforms) as PlatformDetails[];
}