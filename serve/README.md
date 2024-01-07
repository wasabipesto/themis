# themis-serve

This is the backend service that serves data straight from the database. 

## Routes

### `GET /calibration_plot`

This route returns a list of plots, one per platform. If filters are applied, any platforms with 0 valid markets are excluded.

Query parameters:

- `bin_method`: which attribute to use as the bin selector:
    - Possible values: `prob_at_midpoint`, `prob_at_close`, `prob_time_weighted` (default)
- `bin_size`: the size of each bin, should be in `(0,0.5)` and evenly divide the space
- `weight_attribute`: which attribute to use as the weighting modifier:
    - Possible values: `open_days`, `volume_usd` (default), `count`
- Numeric filters, which constrain the sampled markets:
    - `min_open_days`
    - `min_volume_usd`

Each plot contains the following:

- `platform`: the platform name
- `x_series`: an evenly-distributed list of x-values
- `y_series`: the y-value of each point, equal in length to `x_series`

```
[
    {
        "platform": "manifold",
        "x_series": [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
        "y_series": [0.0023965412, 0.039222192, 0.11597037, 0.23053844, 0.36516556, 0.451152, 0.645353, 0.69496787, 0.8331579, 0.9143641, 0.9958024]
    }
]
```