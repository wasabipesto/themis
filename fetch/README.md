# themis-fetch

This is the backend service that downloads data from each platform and saves it to a database for querying later. 

## Usage

```
  -p, --platform <PLATFORM>  Override the default platform list to only pull from one provider
      --id <ID>              Only pull market data for a single market - requires a single platform to be specified
  -o, --output <OUTPUT>      Where to redirect the output [default: database] [possible values: database, stdout]
  -v, --verbose              Show additional output for debugging
  -h, --help                 Print help
  -V, --version              Print version
```

## Platforms

### Stage 1.

These should be platforms that have an established presence and an open API. I would like to have 5 sites for the MVP.

- [x] Kalshi
    - https://kalshi.com
    - API Docs: https://trading-api.readme.io/reference/getting-started
    - Python library: https://github.com/Kalshi/kalshi-python
    - Note: API requires username/password from a verified account with 2FA *disabled*.
- [x] Manifold
    - https://manifold.markets
    - API Docs: https://docs.manifold.markets/api
- [x] Metaculus
    - https://www.metaculus.com
    - API Docs: https://www.metaculus.com/api2/schema/redoc
- [x] Polymarket
    - https://polymarket.com
    - API Docs: https://docs.polymarket.com/#introduction
    - GraphQL (removed): https://gamma-api.polymarket.com
    - CLOB API: https://clob.polymarket.com
    - StrAPI: https://strapi-matic.poly.market/markets
- [ ] PredictIt
    - https://www.predictit.org
    - FAQ Topic: https://predictit.freshdesk.com/support/solutions/articles/12000001878-does-predictit-make-market-data-available-via-an-api-
    - Past project: https://github.com/kiernann/predelect
    - Internal API: https://www.predictit.org/api/Public/GetMarketChartData

### Stage 2.

After the MVP is established we can show off the project to get API access to other sites. These should be sites with larger volumes or different userbases.

- [ ] Betfair
    - https://www.betfair.com
    - Mainly in sports with some UK politics.
    - Secret API endpoint? No documentation.
- [ ] Futuur
    - https://futuur.com
    - https://api.futuur.com/docs
    - Both play money & real money versions.
- [ ] Hypermind
    - https://www.hypermind.com
    - General prediction market and aggregation site.
    - Seems to have good history.
    - Does not appear to have an API.
- [ ] Insight
    - https://insightprediction.com
    - API: https://insightprediction.com/api/markets
    - General market site with high volume.
    - No public API documentation? Taken offline?

### Stage 3.

Same as stage 2 but with lower-volume or otherwise lower-priority sites.

- [ ] Augur
    - https://augur.net
    - Crypto-based smart contracts on Ethereum.
    - Website completely inaccessible from the US.
- [ ] Iowa Electronic Markets
    - https://iem.uiowa.edu/iem
    - Example: https://iemweb.biz.uiowa.edu/iem_market_info/2024-u-s-presidential-winner-takes-all-market
    - Analysis Repo: https://github.com/mickbransfield/IEM
    - Low-volume student exchange.
    - Does not appear to have an API.
- [ ] Infer
    - https://www.infer-pub.com
    - Prediction aggregation like Metaculus.
    - API access upon request.
- [ ] Smarkets
    - https://smarkets.com
    - Lots of sports and some politics.
    - Does not appear to have an API.

### Tentative

Sites that I may add at some point in the future but don't currently seem worth it.

- [ ] FantasySCOTUS
    - https://fantasyscotus.net
    - Forecasting supreme court outcomes from 2017-2020.
    - Site is still up but main contest has ended.
    - Does not appear to have an API.
- [ ] Foretold
    - https://foretold.io
    - Source code: https://github.com/foretold-app/foretold
    - QraphQL Playground: https://api.foretold.io/graphql
    - Uses the guesstimate distribution tool (same author).
    - Site seems unmaintained, most questions are pending resolution.
    - Does not appear to have an API.
- [ ] Good Judgement/GJ Open
    - GJ: https://goodjudgment.io/superforecasts
    - GJ Open: https://www.gjopen.com/questions
    - Very closed down, no real API, but good reputation.
    - Can technically scrape some markets but not enough to be helpful.
- [ ] Hollywood Stock Exchange
    - https://www.hsx.com
    - Stocks for movie box office performance.
    - Does not appear to have an API.
- [ ] Rootclaim
    - https://www.rootclaim.com
    - API: https://live-rootclaim-backend.herokuapp.com/analysis/public-list?limit=1000&offset=0
    - Seems to be an aggregation site? Does not really have a way to get calibration.