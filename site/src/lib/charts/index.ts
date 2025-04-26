// Re-export everything from our chart library for easy access
export * from './types';
export * from './transformers';
export * from './factory';
export * from './presets';

// Add a convenient barrel export
export { default as Chart } from '@components/charts/Chart.astro';
export { default as ChartSelector } from '@components/charts/ChartSelector.astro';
export { default as BoxPlotChart } from '@components/charts/BoxPlotChart.astro';
export { default as HistogramChart } from '@components/charts/HistogramChart.astro';
export { default as CalibrationChart } from '@components/charts/CalibrationChart.astro';
export { default as TimeBarChart } from '@components/charts/TimeBarChart.astro';