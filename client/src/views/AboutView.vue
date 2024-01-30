<template>
  <v-main>
    <h1>aboot</h1>
    <p>
      Prediction markets claim to be accurate, but how can we tell? And under what conditions are
      they reliable? To investigate, we can generate a type of diagram called a
      <b>Calibration Plot</b> which aggregates thousands of markets from several prediction market
      sites on a single graph and shows how often they were right.
    </p>
    <h2>What does the plot mean?</h2>
    <p>
      Across the x-axis the markets are grouped by their estimated probability, and along the y-axis
      the true outcome is shown. If a point is at (0.75,0.80), it means the prediction markets
      thought those events would happen with a <b>75% probaility</b> and they turned out to happen
      <b>80% of the time</b>. That's pretty good! To be the most accurate, you should ecpect the
      dots to line up along the reference line from the bottom-left corner to the top-right - this
      represents perfect calibration.
    </p>
    2
    <h2>What's a Brier score?</h2>
    <p>
      These points are nice, but it's kind of hard to compare the platforms against each other based
      on the plot alone. We can give each platform a numeric score to show how accurate they were
      over all markets, properly penalizing them for incorrect guesses and rewarding them for
      accurate predictions. A Brier score is one such metric, which starts at 0 if you are perfectly
      calibrated and goes up the more incorrect predictions you make. Most prediction markets have a
      Brier score of between 0.1 and 0.2, and lower is always better!
    </p>
  </v-main>
</template>
