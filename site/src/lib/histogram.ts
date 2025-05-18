import type { MarketDetails } from "@types";

export interface DateBarDatapoint {
  platform_name: string;
  date_start: string;
  date_center: string;
  date_end: string;
  count: number;
  percent: number;
}

export function calculateDateBarPoints(
  markets: MarketDetails[],
  dateKey: "open" | "closed",
  minDate?: number,
  binWidth?: number,
): DateBarDatapoint[] {
  // Get or calculate bounds
  const defaultMinDateMs = new Date("2020-01-01").getTime();
  const minDateMs = minDate || defaultMinDateMs;
  const maxDateMs = new Date().getTime();

  // Get or calculate bin config options
  const defaultBinWidthMs = 14 * 24 * 60 * 60 * 1000;
  const binWidthMs = binWidth || defaultBinWidthMs;
  const numBins = Math.ceil((maxDateMs - minDateMs) / binWidthMs);

  // Initialize bins for closed markets
  const bins: { [platform: string]: number[] } = {};
  markets.forEach((mkt) => {
    const platform = mkt.platform_name;

    // Get time value
    let marketTimeMs;
    if (dateKey == "open") {
      marketTimeMs = new Date(mkt.open_datetime).getTime();
    } else {
      marketTimeMs = new Date(mkt.close_datetime).getTime();
    }

    // Crate platform array if not existing
    if (!bins[platform]) {
      bins[platform] = Array(numBins).fill(0);
    }

    // Get index for bin
    const binIndex = Math.min(
      numBins - 1,
      Math.floor((marketTimeMs - minDateMs) / binWidthMs),
    );

    bins[platform][binIndex] += 1;
  });

  // Calculate points for each platform/bin
  const points: DateBarDatapoint[] = [];
  for (const [platform_name, seriesBins] of Object.entries(bins)) {
    const platformNumItems = markets.filter(
      (mkt) => mkt.platform_name === platform_name,
    ).length;

    for (let index = 0; index < numBins; index++) {
      // Get dates
      const startDateMs = minDateMs + index * binWidthMs;
      const centerDateMs = startDateMs + binWidthMs / 2;
      const endDateMs = startDateMs + binWidthMs;

      // Get values
      const count = seriesBins[index];
      const percent = count / platformNumItems;

      points.push({
        platform_name,
        date_start: new Date(startDateMs).toISOString(),
        date_center: new Date(centerDateMs).toISOString(),
        date_end: new Date(endDateMs).toISOString(),
        count,
        percent,
      });
    }
  }
  return points;
}
