# Chart System Documentation

I asked Claude to refactor my charts and give come documentation of the changes. Below are the notes from that migration.

## Overview

This documentation explains how to use the new chart system in Themis. The system is designed to be flexible and extensible, making it easy to create new charts and modify existing ones.

## Architecture

The chart system consists of several main components:

1. **Chart Types** - Different visualizations (box plot, histogram, calibration, time bar)
2. **Data Transformers** - Functions to process raw data into chart-specific formats
3. **Chart Options** - Configuration and customization for each chart
4. **Chart Selectors** - UI components to switch between different data views
5. **Presets** - Common configurations for standard chart types

## File Structure

```
themis/site/src/
├── lib/
│   └── charts/
│       ├── index.ts       # Barrel file for exports
│       ├── types.ts       # Type definitions
│       ├── transformers.ts # Data transformation utilities
│       ├── factory.ts     # Factory functions for creating charts
│       └── presets.ts     # Preset configurations for common charts
└── components/
    └── charts/
        ├── Chart.astro             # Main chart component
        ├── ChartSelector.astro     # UI for selecting chart options
        ├── BoxPlotChart.astro      # Box plot renderer
        ├── HistogramChart.astro    # Histogram renderer
        ├── CalibrationChart.astro  # Calibration plot renderer
        └── TimeBarChart.astro      # Time bar chart renderer
```

## Basic Usage

To create a new chart, you need to provide the data, chart type, and configuration options.

```astro
---
import Chart from "@components/charts/Chart.astro";
import { getPlatforms, getMarkets } from "@lib/api";
import { createStandardAccuracyOptions } from "@lib/charts/presets";

// Get data
const platforms = await getPlatforms();
const markets = await getMarkets();

// Create context
const chartContext = { platforms };

// Create options using presets
const options = createStandardAccuracyOptions(markets);
---

<Chart
  id="my-chart"
  type="boxplot"
  title="My Chart"
  description="Description of the chart"
  data={markets}
  options={options}
  context={chartContext}
/>
```

## Creating Custom Charts

### 1. Define Data Selectors

Data selectors filter and transform raw data for visualization:

```typescript
const myOptions = [
  {
    id: 'option1',
    description: 'My custom option',
    icon: 'mdi:icon-name',
    dataSelector: (data, context) => {
      // Filter and transform data
      return data.filter(item => item.someProperty > 100);
    },
    config: {
      type: 'boxplot',
      axisConfig: {
        x: { title: 'X-Axis' },
        y: { title: 'Y-Axis', range: [0, 100] }
      }
    }
  }
];
```

### 2. Use Data Transformers

For more complex transformations, use the built-in transformers:

```typescript
import { transformToBoxPlotData } from "@lib/charts/transformers";

const boxPlotData = transformToBoxPlotData(marketScores, platforms, 'brier-midpoint');
```

### 3. Create Options with Factory Functions

Use factory functions to create chart options with proper typing:

```typescript
import { createChartOptions } from "@lib/charts/factory";

const options = createChartOptions('boxplot', {
  id: 'my-chart',
  title: 'My Box Plot',
  axisConfig: {
    y: { range: [0, 1] }
  }
});
```

## Chart Types

### Box Plot

Box plots show the distribution of values with quartiles and outliers.

```astro
<Chart
  id="box-plot"
  type="boxplot"
  data={marketScores}
  options={boxPlotOptions}
  context={chartContext}
/>
```

### Histogram

Histograms show the distribution of values in bins.

```astro
<Chart
  id="histogram"
  type="histogram"
  data={histogramData}
  options={histogramOptions}
  context={chartContext}
/>
```

### Calibration Chart

Calibration charts compare predicted probabilities with actual outcomes.

```astro
<Chart
  id="calibration"
  type="calibration"
  data={markets}
  options={calibrationOptions}
  context={chartContext}
/>
```

### Time Bar Chart

Time bar charts show data changes over time.

```astro
<Chart
  id="time-bar"
  type="timebar"
  data={markets}
  options={timeBarOptions}
  context={chartContext}
/>
```

## Using Presets

Presets are pre-configured options for common chart types:

```typescript
import {
  createStandardAccuracyOptions,
  createScoreTypeOptions,
  createCalibrationCriterionOptions,
  createMarketHistogramOptions
} from "@lib/charts/presets";

const accuracyOptions = createStandardAccuracyOptions(marketScores);
const scoreTypeOptions = createScoreTypeOptions(marketScores);
const criterionOptions = createCalibrationCriterionOptions(markets);
const histogramOptions = createMarketHistogramOptions(markets);
```

## Extending the System

### Adding a New Chart Type

1. Define the chart type and options in `types.ts`
2. Create a data transformer in `transformers.ts`
3. Add factory functions in `factory.ts`
4. Create a chart renderer component
5. Add the new chart to `Chart.astro`
6. Create presets in `presets.ts`

### Customizing Existing Charts

To customize an existing chart, modify its options:

```typescript
const customOptions = [
  {
    id: 'custom',
    description: 'My custom view',
    dataSelector: (data) => data.filter(/* custom filter */),
    config: {
      type: 'boxplot',
      // Custom configuration
      axisConfig: {
        x: { title: 'Custom X' },
        y: { title: 'Custom Y', range: [0, 10] }
      }
    }
  }
];
```

## Best Practices

1. **Use TypeScript**: Leverage type safety for chart options and data.
2. **Separate Data Processing**: Keep data processing logic separate from visualization.
3. **Reuse Presets**: Use preset functions when possible to maintain consistency.
4. **Document Options**: Add clear descriptions to option objects.
5. **Consider Performance**: For large datasets, implement pagination or data sampling.
6. **Test Different Screens**: Ensure charts render well on mobile and desktop.

## Troubleshooting

### Chart Not Rendering

- Check that the data format matches what the chart expects
- Verify the chart type is correct
- Check browser console for errors

### Data Not Updating

- Ensure your data selector is correctly processing the data
- Check that event listeners are properly set up

### Performance Issues

- Reduce the amount of data passed to charts
- Consider implementing pagination or filtering
- Use efficient data transformers
