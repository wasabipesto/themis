# themis-serve

This is the backend service that serves data straight from the database. 

The primary url for this service is [https://api.calibration.city/](https://api.calibration.city/).

## Routes

### `/`

The index page returns the server status and a list of routes. When the list returned from this endpoint differs from this README, the endpoint is correct and I just haven't updated the docs yet.

### `/list_platforms`

The platform list takes no parameters and returns all data for all platforms. There is no market data in this table. 

- `name`: the internal identifier for this platform
- `name_fmt`: the display name to use for this platform
- `description`: a brief description, ideally pointing out what makes this platform unique
- `avatar_url`: the path to this site's logo, as appended to `https://calibration.city/`
- `site_url`: the path to the site's homepage
- `color`: the primary color used for the platform, picked from official materials

### Common Filters

All the below endpoints take these optional parameters in addition to the specified endpoint-specific parameters.

- `title_contains`: case-insensitive text matching the title using Postgres ILIKE
- `platform_select`: returns markets matching he selected platform (should match `platform.name`, always lowercase)
- `category_select`: select based on category (matches the text in the UI)
- `open_ts_min`/`open_ts_max`: filter based on min/max open timestamp
- `close_ts_min`/`close_ts_max`: filter based on min/max close timestamp
- `open_days_min`/`open_days_max`: filter based on min/max market length in days
- `volume_usd_min`/`volume_usd_max`: filter based on min/max market volume in USD
- `num_traders_min`/`num_traders_max`: filter based on min/max number of unique traders
- `prob_at_midpoint_min`/`prob_at_midpoint_max`: filter based on min/max market midpoint value
- `prob_at_close_min`/`prob_at_close_max`: filter based on min/max market closing value
- `prob_time_avg_min`/`prob_time_avg_max`: filter based on min/max time-averaged probability
- `resolution_min`/`resolution_max`: filter based on min/max resolution

TODO