# themis-serve

This is the backend service that serves data straight from the database. 

## Routes

### `GET /calibration_plot`

This route returns a list of plots, one per platform. If filters are applied, any platforms with 0 valid markets are excluded.

Query parameters:

- `bin_method`: which attribute to use as the bin selector:
    - Possible values: `prob_at_midpoint`, `prob_at_close`, `prob_time_weighted` (default)
- `bin_size`: the size of each bin, should be in `(0, 0.5)` and evenly divide the space
- `weight_attribute`: which attribute to use as the weighting modifier:
    - Possible values: `open_days`, `num_traders`, `volume_usd` (default), `count`
- Filters, which constrain the sampled markets:
    - `min_open_days`: minimum market duration in days, can be a decimal
    - `min_num_traders`: minimum number of unique traders
    - `min_volume_usd`: minimum market volume in usd, can be a decimal
    - `title_contains`: given text occurs in the title
    - `categories`: unimplemented

Each plot contains the following:

- `metadata`:
- `traces`:
    - `platform`: the platform name
    - `x_series`: an evenly-distributed list of x-values
    - `y_series`: the y-value of each point, equal in length to `x_series`
    - `y_series`: the y-value of each point, equal in length to `x_series`

```
{
    "metadata": {
        "title": "Calibration Plot",
        "x_title": "Time-Weighted Probability",
        "y_title": "Resolution, Unweighted"
    },
    "traces": [
        {
            "platform_name_fmt": "Kalshi",
            "platform_description": "A US-regulated exchange with limited real-money contracts.",
            "platform_avatar_url": "images/kalshi.png",
            "platform_color": "#00d298",
            "num_markets": 15047,
            "brier_score": 0.18200922,
            "x_series": [0.025, 0.075, 0.125, 0.175, 0.225, 0.275, 0.325, 0.375, 0.425, 0.475, 0.525, 0.575, 0.625, 0.675, 0.725, 0.775, 0.825, 0.875, 0.925, 0.975],
            "y_series": [0.010714286, 0.018838305, 0.03638814, 0.060747664, 0.08781127, 0.17166212, 0.17530864, 0.120704845, 0.23296703, 0.25637183, 0.75416666, 0.8493724, 0.9309211, 0.85470086, 0.7887324, 0.8783784, 0.93421054, 0.97727275, 0.97727275, 1.0],
            "point_sizes": [8.948745, 10.236586, 10.615362, 11.026605, 10.691116, 10.586502, 10.860664, 12.033068, 11.221404, 32.0, 9.670224, 8.800841, 9.035322, 8.36074, 8.194799, 8.205622, 8.212836, 8.256125, 8.0974, 8.0]
        },
        {
            "platform_name_fmt": "Manifold",
            "platform_description": "A play-money platform where anyone can make any market.",
            "platform_avatar_url": "images/manifold.svg",
            "platform_color": "#4337c9",
            "num_markets": 45814,
            "brier_score": 0.15229039,
            "x_series": [0.025, 0.075, 0.125, 0.175, 0.225, 0.275, 0.325, 0.375, 0.425, 0.475, 0.525, 0.575, 0.625, 0.675, 0.725, 0.775, 0.825, 0.875, 0.925, 0.975],
            "y_series": [0.0045753634, 0.027342444, 0.04666939, 0.085330896, 0.121051334, 0.1602442, 0.21819338, 0.25855398, 0.35085803, 0.42242613, 0.54722965, 0.62253165, 0.6797988, 0.74774045, 0.8091675, 0.84415066, 0.8949309, 0.936361, 0.95164835, 0.978495],
            "point_sizes": [13.60659, 20.865898, 22.075783, 23.087975, 22.455355, 21.656673, 22.241844, 23.491268, 25.017464, 32.0, 31.675783, 27.366062, 24.716969, 22.2972, 0.138386, 19.197365, 17.639538, 16.113344, 14.065239, 8.0]
        }
    ]
}
```