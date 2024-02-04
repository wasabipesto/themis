<template>
  <v-main>
    <h2>Implementation Details</h2>
    <p>
      When standardizing things across platforms we ran into some edge cases, I've tried to detail
      them all here. When in doubt, you can always check the
      <a href="https://github.com/wasabipesto/themis">source</a> to see how we compute a specific
      attribute.
    </p>
    <h3>All</h3>
    <ul>
      <li>
        To calculate the time-averaged probability, we assume the market opens at 50%. Once the
        first trade occurs, we track the probability at each trade and the cumulative durations to
        generate an average.
      </li>
    </ul>
    <h3>Kalshi</h3>
    <ul>
      <li>
        We use the YES price from the most recently executed trade as the probability at any point
        in time.
      </li>
      <li>The counter for the number of unique traders is currently unimplemented.</li>
      <li>Supported market types:</li>
      <li class="indent">Binary</li>
    </ul>
    <h3>Manifold</h3>
    <ul>
      <li>Supported market types:</li>
      <li class="indent">CPMM-1 Binary</li>
    </ul>
    <h3>Metaculus</h3>
    <ul>
      <li>
        We use the <code>community_prediction.history.x2.avg</code> series for the probability.
      </li>
      <li>
        Since Metaculus does not have bets, we use the number of forecasts at 10 cents each for the
        market volume.
      </li>
      <li>Supported market types:</li>
      <li class="indent">Binary</li>
    </ul>
    <h3>Polymarket</h3>
    <ul>
      <li>
        We used to use the Gamma API which had defined start and end dates, but that functionality
        has been removed. We declare a market has started when the first trade occurs and end at the
        date noted by <code>end_date_iso</code>. This field is optional and markets without it are
        not counted.
      </li>
      <li>
        Since Metaculus does not have bets, we use the number of forecasts at 10 cents each for the
        market volume.
      </li>
      <li>The counter for the number of unique traders is currently unimplemented.</li>
      <li>The counter for market volume is currently unimplemented.</li>
      <li>Supported market types:</li>
      <li class="indent">Binary</li>
      <li>
        We also do not support old markets that used a previous order system, only those that use
        the newer CLOB system.
      </li>
    </ul>
    <h2>Disclaimer</h2>
    <p>
      I use Manifold much more than any of the other platforms included in this analysis, and have
      received bounties from the Manifold team in both mana (play money) and real money. Their
      contributions did not affect the contents of this site in any way.
    </p>
  </v-main>
</template>

<style scoped>
.v-main {
  max-width: 60rem;
  margin-left: auto;
  margin-right: auto;
}
p {
  margin: 1rem;
}
code {
  font-size: 0.75rem;
}
ul {
  margin: 1rem 3rem;
}
li.indent {
  margin-left: 1.5rem;
}
</style>
