# Project Themis

This is a work-in-progress rewrite of [Calibration City](https://github.com/wasabipesto/calibration-site), a prediction market scoring site. Project Themis includes a number of upgrades and changes to the original concept, such as:

- Orders of magnitude faster, thanks to a complete rewrite in rust! API calls to the v1 backend would take up to 15 seconds, while the new version handles hundreds of thousands of markets in under half a second.
- Supports multiple prediction platforms! Now you can have a direct comparison across platforms with simultaneous filters and weights to modify the calibration plots and Brier scores.
- Much nicer user interface! Upgraded from an HTML form with no documentation, now each option has an explanation and live-updating selectors.

## How it works

We have three subprojects here: `fetch`, `serve`, and `client`. Fetch and serve are written in rust, and they save markets to or query markets from the database respectively. Client is a simple webpage to hit the serve API and show the results.

### Fetch

In `fetch`, we have a library `lib` that holds the basic logic and `main` that serves it as a CLI with intelligent argument parsing. The valid arguments can be found in the serve README or `themis-fetch --help`. The `platform` module has type definitions, reused functions, and default trait implementations. Each sub-module has platform-specific download and parsing functions. Adding new platforms is fairly straightforward as long as the trait implementations are satisfied.

In the default flow we spin up a group of async tasks to recursively download all markets on each platform and then sub-tasks to get additional information like bets and market history. Each platform has an independent ratelimit and deserializes immediately into typed structs upon response. We insert markets into the database or update if they already exist, printing errors to the console but largely skipping recoverable or row-specific faults.

### Serve

Subproject `serve` queries an external Postgres database with saved markets, calculates calibration points and Brier score, and ships it off to the client. Different user-selectable options are deserialized immediately and applied to the SQL filter. A paginated endpoint allows third parties to query markets for their own analysis. You can find the API schema in the serve README.

### Client

The `client` subproject is a Vue project that is built upon deployment to be served behind nginx. There are only a few reused components, with most items being integrated directly into the view to allow for more flexibility. We use Veutify for basic components and the ChartJS library for plotting the visualizations. 

## Roadmap

These are the things I plan to add in the near future.

### Fetch

#### Kalshi
- Investigate getting the number of unique traders
- Investigate additional market types

#### Manifold
- Investigate including linked and unlinked multiple choice

#### Metaculus
- Investigate additional market types

#### Polymarket
- Investigate getting market volume
- Investigate getting the number of unique traders
- Investigate including multiple choice

### Serve
- Compute log score (maybe with transformation)
- Add optional KDE smoothing to calibration

### Client
- Add filters for open & close times
- Add a way to share a link to the current view with weights & filters

### Other
- Investigate PredictIt
- Add an x-method for a random point along the market duration
- Add an x-method with time-based probability spread
- Set up docker container for client and a sample compose file
- Investigate a standardized corpus of questions across platforms

## Disclaimer
I use Manifold much more than any of the other platforms included in this analysis, and have received bounties from the Manifold team in both mana (play money) and real money. Their contributions did not affect the contents of this site in any way.