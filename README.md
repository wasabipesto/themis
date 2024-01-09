# Project Themis

This is a work-in-progress rewrite of [Calibration City](https://github.com/wasabipesto/calibration-site), a prediction market scoring site. Project Themis includes a number of upgrades and changes to the original concept, such as:

- Orders of magnitude faster, thanks to a complete rewrite in rust! API calls to the v1 backend would take up to 15 seconds, while the new version handles hundreds of thousands of markets in under half a second.
- Supports multiple prediction platforms! Now you can have a direct comparison across platforms with simultaneous filters and weights to modify the calibration plots and Brier scores.
- Much nicer user interface! Upgraded from an HTML form with no documentation, now each option has an explanation and live-updating selectors.

## Roadmap

### Kalshi
- Fix hangup on GPT%
- Verify operation of re-auth
- Investigate getting the number of unique traders

### Manifold
- Call `/market` to get groups, save to MarketFull
- Investigate including linked and unlinked multiple choice

### Metaculus
- Fix hangup on `avg` field

### Client
- Add a dedicated explainer page with a walkthough on calibration
- Add links to the github repo, old site, my homepage, etc
- Add a way to share a link to the current view with weights & filters
- Adjust right platform bar: make title more visible, remove click effect
- Investigate mobile support: dragging slider moves sidebar, can't re-enter sidebar
- Investigate swapping out plotly for a custom D3 component

### Other
- Investigate Polymarket
- Investigate PredictIt
- Hardcode maps of site categories to standard categories
- Set up docker containers for fetch and client with a sample compose file
- Compute log score (maybe with transformation)
- Add optional KDE smoothing to calibration
- Return a list of markets in each sample
- Plot Brier score against any x-axis (closed date, number of traders, market volume)
- Investigate a standardized corpus of questions across platforms