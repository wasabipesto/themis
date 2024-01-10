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

`serve` queries an external Postgres database with saved markets, calculates calibration points and Brier score, and ships it off to the client. Right now it only has a single route but it is extremely fast, usually completing a request in under 250ms. You can find the API schema in the serve README.

### Client

The `client` subproject is actually just a single HTML file that pulls a few scripts via CDN to build the layout and then queries the `serve` API to get data.

## Implementation details

When standardizing things across platforms we ran into some edge cases, I've tried to detail them all here. When in doubt, you can always check the source to see how we compute a specific attribute.

## All

- `prob_time_weighted` is not computed for markets open for less than 60 seconds

## Kalshi

- We use the current YES price for the probability
- `num_traders` is currently unimplemented
- Supported market types:
    - [x] Binary

## Manifold

- `volume` is directly as reported by the API
- Supported market types: 
    - [x] CPMM-1 Binary
    - [ ] CPMM-1 Pseudo-Numeric
    - [ ] CPMM-1 Multiple-Choice Unlinked
    - [ ] CPMM-1 Multiple-Choice Linked
    - [ ] DPM-2 Binary

## Metaculus

- We use the `community_prediction.history.x2.avg` series for the probability
- Since Metaculus does not have bets, we use the number of forecasts at 10 cents each for `volume_usd`
- Supported market types: 
    - [x] Binary

## Roadmap

### Fetch
- Hardcode maps of site categories to standard categories
- Add error importance, hide low importance, eprint on medium, and panic on high

#### Kalshi
- Investigate getting the number of unique traders

#### Manifold
- Call `/market` to get groups
- Investigate including linked and unlinked multiple choice

#### Metaculus
- None

### Serve
- Compute log score (maybe with transformation)
- Add optional KDE smoothing to calibration

### Client
- Add a dedicated explainer page with a walkthough on calibration
- Add links to the github repo, old site, my homepage, etc
- Add a way to share a link to the current view with weights & filters
- Investigate mobile support: dragging slider moves sidebar, can't re-enter sidebar
- Investigate swapping out plotly for a custom D3 component

### Other
- Investigate Polymarket
- Investigate PredictIt
- List all market types on all platforms
- Set up docker container for client and a sample compose file
- Return a list of markets in each sample
- Plot Brier score against any x-axis (closed date, number of traders, market volume)
- Investigate a standardized corpus of questions across platforms

## Disclaimer

I use Manifold much more than any of the other platforms included in this analysis, and have received bounties from the Manifold team in both mana (play money) and real money. Their contributions did not affect the contents of this site in any way.