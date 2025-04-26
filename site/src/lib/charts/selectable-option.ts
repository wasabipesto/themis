/**
 * Standardized chart selection utilities
 */
import * as Plot from "@observablehq/plot";
import type { ChartOptionBase } from "./types";

/**
 * Base interface for selectable options
 * Extends the ChartOptionBase to ensure compatibility
 */
export interface SelectableOption extends ChartOptionBase {
  [key: string]: any;
}

/**
 * Configuration for a selectable chart
 */
export interface SelectableChartConfig<T extends SelectableOption> {
  /** Unique identifier for the chart selection */
  plotId: string;
  /** CSS class for the chart element */
  chartClass: string;
  /** CSS class prefix for the option elements */
  optionClass: string;
  /** Options for selection */
  options: T[];
  /** Function to call when selection changes */
  updateFn: (element: HTMLElement, option: T) => void;
}

/**
 * Get the currently selected option
 */
export function getSelectedOption<T extends SelectableOption>(
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
 * Setup a selectable chart
 */
export function setupSelectableChart<T extends SelectableOption>(
  config: SelectableChartConfig<T>
): void {
  const { plotId, chartClass, optionClass, options, updateFn } = config;
  
  // Get the chart element
  const chartElement = document.getElementById(`${chartClass}-${plotId}`);
  if (!chartElement) {
    console.warn(`Chart element not found: ${chartClass}-${plotId}`);
    return;
  }
  
  // Find the options that match the plot ID
  const radios = [
    ...document.querySelectorAll(`.${optionClass}-${plotId}`),
  ] as HTMLInputElement[];
  
  // Set the first radio button to checked by default
  if (radios.length > 0 && !radios.some(r => r.checked)) {
    radios[0].checked = true;
  }
  
  // Get the initially selected option
  const selectedOption = getSelectedOption(options, radios);
  
  // Update the chart with initial values
  updateFn(chartElement, selectedOption);
  
  // Set up event listeners for selection changes
  radios.forEach((radio, index) => {
    radio.addEventListener("change", () => {
      if (radio.checked) {
        updateFn(chartElement, options[index]);
      }
    });
  });
}

/**
 * Initialize all charts matching a certain pattern
 */
export function initSelectableCharts<T extends SelectableOption>(
  chartClass: string,
  optionClass: string,
  updateFn: (element: HTMLElement, option: T) => void,
  getOptions: (element: HTMLElement) => T[]
): void {
  // Get all chart elements
  const chartElements = document.querySelectorAll(`.${chartClass}`);
  
  chartElements.forEach(element => {
    // Extract plot ID from element ID
    const idMatch = element.id.match(new RegExp(`${chartClass}-(.*)$`));
    if (!idMatch || !idMatch[1]) {
      console.warn(`Invalid chart ID format: ${element.id}`);
      return;
    }
    
    const plotId = idMatch[1];
    const options = getOptions(element as HTMLElement);
    
    // Set up the selectable chart
    setupSelectableChart({
      plotId,
      chartClass,
      optionClass,
      options,
      updateFn
    });
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