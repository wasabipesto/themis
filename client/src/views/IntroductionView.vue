<script setup>
import { ref } from 'vue'
import {
  Chart as ChartJS,
  Tooltip,
  Legend,
  PointElement,
  LinearScale,
  LineElement,
  Title
} from 'chart.js'
import { Bubble } from 'vue-chartjs'

ChartJS.register(LinearScale, PointElement, LineElement, Tooltip, Legend, Title)

const sample_calibration_chart_data = ref({
  datasets: [
    {
      type: 'line',
      label: 'Reference',
      backgroundColor: '#80808080',
      borderColor: '#80808080',
      data: [
        {
          x: 0,
          y: 0
        },
        {
          x: 1,
          y: 1
        }
      ]
    },
    {
      type: 'bubble',
      label: 'Calibration',
      backgroundColor: '#4337c980',
      borderColor: '#4337c9',
      data: [
        {
          x: 0,
          y: 0.0909,
          r: 11
        },
        {
          x: 0.1,
          y: 0.1176,
          r: 17
        },
        {
          x: 0.2,
          y: 0.28,
          r: 25
        },
        {
          x: 0.3,
          y: 0.3182,
          r: 22
        },
        {
          x: 0.4,
          y: 0.4118,
          r: 34
        },
        {
          x: 0.5,
          y: 0.5135,
          r: 37
        },
        {
          x: 0.6,
          y: 0.6,
          r: 35
        },
        {
          x: 0.7,
          y: 0.6667,
          r: 21
        },
        {
          x: 0.8,
          y: 0.7083,
          r: 24
        },
        {
          x: 0.9,
          y: 0.8125,
          r: 16
        },
        {
          x: 1,
          y: 1,
          r: 9
        }
      ]
    }
  ]
})
const sample_calibration_chart_options = ref({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    intersect: false,
    mode: 'nearest'
  },
  layout: {
    //padding: 4
  },
  plugins: {
    title: {
      display: true,
      text: 'Calibration Plot',
      padding: 16,
      font: {
        size: 16,
        weight: 'bold'
      }
    },
    legend: {
      display: false
    }
  },
  scales: {
    x: {
      title: {
        display: true,
        text: 'Prediction',
        font: {
          size: 14
        }
      },
      ticks: {
        callback: function (value, index, ticks) {
          return value * 100 + '%'
        }
      },
      min: 0,
      max: 1
    },
    y: {
      title: {
        display: true,
        text: 'Resolution',
        font: {
          size: 14
        }
      },
      ticks: {
        callback: function (value, index, ticks) {
          return value * 100 + '%'
        }
      },
      min: 0,
      max: 1
    }
  }
})
</script>

<template>
  <v-main>
    <h2>Introduction</h2>
    <p>Predicting the future is hard, but it's also incredibly important.</p>
    <p>
      Let's say someone starts making predictions about important events. How much should you
      believe them when they say the world will end tomorrow? What about when they say there's a 70%
      chance the world will end in 50 years?
    </p>
    <h2>Quantified Predictions</h2>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text>
          Wait, what does "70%" even mean in this situation? How can you have 70% of an apocalypse?
        </v-card-text>
      </v-card>
    </p>
    <p>
      In this situation the predictor is making a prediction with a certain
      <b>confidence</b>. Rather than just saying "it's likely", they've chosen a number to represent
      how confident they are in that statement.
    </p>
    <p>
      People make predictions every day, but most don't choose a specific number to assign to their
      confidence. This would be wildly impractical for most things! If you're driving and a car in
      front of you slows down, you could make a prediction about what it's going to do. If they turn
      on their turn signal, you could make a pretty confident prediction about what it's going to
      do. You usually don't need to state what potential outcomes you're anticipating, which you
      think is most likely, or what amount of confidence you'd place on each, but you are already
      doing it!
    </p>
    <p>
      Explicit predictions are most useful when trying to communicate about important, uncertain
      events. When you hear the morning news say there's a 70% chance of rain today, they've given
      you a useful data point! You can use that information to make decisions: Should I take an
      umbrella? Should I wear a jacket? Probably!
    </p>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text>
          Why should I care about a specific confidence number? Just say "probably" like everyone
          else!
        </v-card-text>
      </v-card>
    </p>
    <p>
      Predictions, quantified or not, are ultimately only useful as tools that you can use to make
      decisions. If a prediction is not particularly relevant to a decision you're making, or it
      won't affect you much either way, then "probably" is fine! If someone tells you they will
      "probably" be home in twenty minutes, that's usually enough information for any decision you
      need to make.
    </p>
    <p>
      On the other hand, predictions that would affect something significant in your life or require
      you to make a bigger decision should probably be taken more seriously.
    </p>
    <ul>
      <li>Will it rain today? You may have to change your plan to go for a hike.</li>
      <li>How is the economy doing? Should you invest or save?</li>
      <li>Who is going to win the election? Will they pass that law they've been talking about?</li>
      <li>Will COVID cases rise again? Should you stock up on masks or toilet paper?</li>
    </ul>
    <p>
      These are the sorts of questions where it's helpful to have
      <a href="https://en.wikipedia.org/wiki/Probabilistic_classification">
        quantified predictions </a
      >.
    </p>
    <h2>Grading Calibration</h2>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text>
          If these predictions are so important, how do we know who to trust? Just because someone
          is confident in themselves doesn't mean I should be confident in them.
        </v-card-text>
      </v-card>
    </p>
    <p>
      One of the best ways we can measure how good a person is at predicting is by looking at how
      often they were right. If our Nostradamus was wrong about every prediction they've made so
      far, we should probably ignore them. If they have been right every time, we should probably
      take them seriously.
    </p>
    <p>
      To grade simple predictions, we can put all of the YES predictions in one bucket, and all of
      the NO predictions in another. We'll count how many times those predictions came true -
      ideally everything in the NO bucket will resolve NO, and everything in the YES bucket will
      resolve YES.
    </p>
    <v-card variant="outlined" class="middle-box-narrow">
      <v-card-text>
        <v-table density="compact">
          <thead>
            <tr>
              <th>Prediction</th>
              <th>NO</th>
              <th>YES</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Resolved No</td>
              <td>15</td>
              <td>7</td>
            </tr>
            <tr>
              <td>Resolved Yes</td>
              <td>3</td>
              <td>10</td>
            </tr>
            <tr>
              <td>Average Resolution</td>
              <td>3 / 18 = <b>16.7%</b></td>
              <td>10 / 17 = <b>58.8%</b></td>
            </tr>
          </tbody>
        </v-table>
      </v-card-text>
    </v-card>
    <p>
      Well it looks like our Nostradamus was decently accurate whenever he predicted NO - those only
      happened 17% of the time. But his YES predictions weren't so good - they happened about as
      often as chance! It seems like this predictor isn't very calibrated.
    </p>
    <p>
      Anyways, we're more interested in forecasters that don't just say yes or no. We're looking at
      people who assign some sort of probability to their statement. In the example at the top of
      the page, our doomsayer was claiming a 70% chance that the world would end by a specific
      timeframe. How would we judge that after the fact? (Assuming the world did not end, that is.)
    </p>
    <p>
      Instead of two buckets (YES and NO), let's break their predictions up into eleven buckets -
      0%, 10%, 20%, and so on to 100%. If our Nostradamus said there's a 0% chance that the sky will
      fall and a 70% chance there will be a snowy Christmas this year, then we can sort those into
      the right buckets and then evaluate each one.
    </p>
    <v-card variant="outlined" class="middle-box">
      <v-card-text>
        <v-table density="compact">
          <thead>
            <tr>
              <th>Prediction</th>
              <th>0%</th>
              <th>10%</th>
              <th>20%</th>
              <th>30%</th>
              <th>40%</th>
              <th>50%</th>
              <th>60%</th>
              <th>70%</th>
              <th>80%</th>
              <th>90%</th>
              <th>100%</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Resolved No</td>
              <th>10</th>
              <th>15</th>
              <th>18</th>
              <th>15</th>
              <th>20</th>
              <th>18</th>
              <th>14</th>
              <th>7</th>
              <th>7</th>
              <th>3</th>
              <th>0</th>
            </tr>
            <tr>
              <td>Resolved Yes</td>
              <th>1</th>
              <th>2</th>
              <th>7</th>
              <th>7</th>
              <th>14</th>
              <th>19</th>
              <th>21</th>
              <th>14</th>
              <th>17</th>
              <th>13</th>
              <th>9</th>
            </tr>
            <tr>
              <td>Avgerage Resolution</td>
              <th>9.1%</th>
              <th>11.7%</th>
              <th>28.0%</th>
              <th>31.8%</th>
              <th>41.2%</th>
              <th>51.4%</th>
              <th>60.0%</th>
              <th>66.7%</th>
              <th>70.8%</th>
              <th>81.3%</th>
              <th>100.0%</th>
            </tr>
          </tbody>
        </v-table>
      </v-card-text>
    </v-card>
    <p>
      This looks a lot better! Now that we have more granularity, we can differentiate between
      things like "unlikely", "probably not", and "definitely not". When this predictor said
      something has a 10% chance to occur, it actually happened only 11.7% of the time. And when
      they gave something a 60% chance, it actually happened 60% of the time! It seems like this
      predictor has a much better <b>calibration</b>.
    </p>
    <p>
      <v-card variant="tonal" class="right-box">
        <v-card-text>
          If a predictor is <b>calibrated</b> it means that, on average, predictions they make with
          X% confidence occur X% of the time.
        </v-card-text>
      </v-card>
    </p>
    <p>
      Let's plot these on a chart for convenience. Across the bottom we'll have a list of all our
      buckets - 0 to 100%. Along the side we'll have a percentage - how often those predicted events
      came true. If our predictor is well-calibrated, these points should line up in a row from the
      bottom-left to the top-right. We'll call this a
      <a href="https://en.wikipedia.org/wiki/Probabilistic_classification#Probability_calibration">
        calibration plot </a
      >, but it's also known as a reliability diagram.
    </p>
    <p>
      <v-card variant="outlined" class="middle-box">
        <v-card-text>
          <Bubble
            :data="sample_calibration_chart_data"
            :options="sample_calibration_chart_options"
            :width="600"
            :height="400"
          />
        </v-card-text>
      </v-card>
    </p>
    <p>
      This is very good! Now we can see visually where our predictor is calibrated or where they're
      over- or under-confident. If our forecaster keeps making predictions like this, we could
      expect them to be well-calibrated in most cases - especially when they make predictions
      between 30% and 70%.
    </p>
    <h2>Grading Accuracy</h2>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text>
          Those charts are nice and all, but it still doesn't tell me how seriously I should take
          this person.
        </v-card-text>
      </v-card>
    </p>
    <p>
      Good point! Calibration plots can tell you plenty, but they're hard to compare and they don't
      give you a single numeric score. For that, let's look into <b>accuracy</b> scoring. Accuracy
      is an intuitive measure but it has some important caveats.
    </p>
    <p>
      <v-card variant="tonal" class="right-box">
        <v-card-text>
          A predictor is more <b>accurate</b> the closer their predictions are to the resolved
          outcome.
        </v-card-text>
      </v-card>
    </p>
    <p>
      We have a few ways to calculate accuracy, but let's focus on the most popular one:
      <a href="https://en.wikipedia.org/wiki/Brier_score">Brier scores</a>.
    </p>
    <p>
      For each prediction, we take the "distance" it was from the outcome: if we predict 10% but it
      resolved NO, the distance is 0.1 — but if we predict 10% and the answer is YES, the distance
      would be 0.9. We always want this number to be low! Then, for each of these distances, we
      square the number. This has the effect of "forgiving" small errors while punishing larger
      ones.
    </p>
    <p>
      After we have done this for all predictions, we take the average of these scores. This gives
      us the Brier score for the prediction set.
    </p>
    <v-card variant="outlined" class="middle-box">
      <v-card-text>
        <v-table density="compact">
          <thead>
            <tr>
              <th>Prediction</th>
              <th>Resolution</th>
              <th>"Distance"</th>
              <th>Score</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>10%</td>
              <td>NO (0)</td>
              <td>0.10</td>
              <td>0.0100</td>
            </tr>
            <tr>
              <td>35%</td>
              <td>NO (0)</td>
              <td>0.35</td>
              <td>0.1225</td>
            </tr>
            <tr>
              <td>42%</td>
              <td>YES (1)</td>
              <td>0.68</td>
              <td>0.3364</td>
            </tr>
            <tr>
              <td>60%</td>
              <td>NO (0)</td>
              <td>0.60</td>
              <td>0.3600</td>
            </tr>
            <tr>
              <td>75%</td>
              <td>YES (1)</td>
              <td>0.25</td>
              <td>0.0625</td>
            </tr>
            <tr>
              <td>95%</td>
              <td>YES (1)</td>
              <td>0.05</td>
              <td>0.0025</td>
            </tr>
            <tr>
              <td colspan="3"><b>Average Brier Score</b></td>
              <td><b>0.1490</b></td>
            </tr>
          </tbody>
        </v-table>
      </v-card-text>
    </v-card>
    <p>
      The most important thing to note here is the fact that <b>smaller is better</b>! This score is
      actually measuring the amount of error in our predictions, so we want it to be as low as
      possible. In fact, an ideal score in this system is 0 while the worst possible score is 1. If
      you were to guess "50%" on every question, your Brier score would be 0.25. Superforecasters
      tend to fall around 0.15 while aggregated <b>prediction markets</b> generally fall between
      0.10 and 0.20.
    </p>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text> So how is accuracy different than calibration here? </v-card-text>
      </v-card>
    </p>
    <p>
      Calibration is about how good you are at quantifying your own confidence, not always about how
      close you are to the truth. If you make a lot of predictions that are incorrect, but are
      honest about your confidence in those predictions, you can be more well-calibrated than
      someone who makes accurate but over- or under-confident predictions.
    </p>
    <h2>Prediction Markets</h2>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text> What are these prediction markets? How can they be so accurate? </v-card-text>
      </v-card>
    </p>
    <p>
      <a href="https://en.wikipedia.org/wiki/Prediction_market">Prediction markets</a> are based on
      a simple concept: If you're confident about something, you can place a bet on it. If someone
      else disagrees with you, declare terms with them and whoever wins takes the money. By
      aggregating the odds of these trades, you can gain an insight into the "wisdom of the crowds".
      Multiple studies have shown that these markets are often very accurate, even better than
      polling in many cases.
    </p>
    <p>
      Markets give participants a financial incentive to be correct, encouraging researchers and
      skilled forecasters to spend time investigating events. Individuals with insider information
      or niche skills can profit by trading, which also updates the market's probability.
    </p>
    <p>Some popular prediction market platforms include:</p>
    <ul>
      <li>
        <a href="https://en.wikipedia.org/wiki/Iowa_Electronic_Markets">Iowa Electronic Markets</a>
      </li>
      <li><a href="https://en.wikipedia.org/wiki/PredictIt">PredictIt</a></li>
      <li><a href="https://en.wikipedia.org/wiki/Metaculus">Metaculus</a></li>
      <li><a href="https://en.wikipedia.org/wiki/Good_Judgment_Open">Good Judgment Open</a></li>
      <li><a href="https://en.wikipedia.org/wiki/Augur_(software)">Augur</a></li>
    </ul>
    <h2>Caveats</h2>
    <p>
      <v-card variant="tonal" class="left-box">
        <v-card-text>
          It seems like these statistics are pretty easy to game. What's stopping you from
          predicting 100% on a bunch of certain things?
        </v-card-text>
      </v-card>
    </p>
    <p>
      Ultimately, nothing is preventing that! It's very important to check what sorts of predictions
      someone is making to ensure that they're relevant to you. It's especially important when
      looking at user-generated content on prediction market sites, where extremely easy questions
      can be added for profit or calibration manipulation.
    </p>
    <p>
      This is especially relevant when comparing between different predictors or platforms. Just
      because someone has a lower Brier score does not mean that they are inherently better! The
      only way you can directly compare is if the corpus of questions is the same for all
      participants.
    </p>
    <p>
      You can check the individual markets included in this site's data by browsing the markets on
      the
      <RouterLink to="/list">list page</RouterLink>.
    </p>
  </v-main>
</template>

<style scoped>
.v-main {
  max-width: 60rem;
  margin-left: auto;
  margin-right: auto;
}
.left-box {
  max-width: 30rem;
  margin-right: auto;
}
.right-box {
  max-width: 30rem;
  margin-left: auto;
}
.middle-box {
  max-width: 45rem;
  margin-left: auto;
  margin-right: auto;
}
.middle-box-narrow {
  max-width: 30rem;
  margin-left: auto;
  margin-right: auto;
}
p {
  margin: 1rem;
}
th {
  text-align: right !important;
}
td {
  text-align: right;
}
ul {
  margin: 1rem 3rem;
}
</style>
