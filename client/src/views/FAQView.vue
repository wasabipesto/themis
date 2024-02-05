<script setup>
import { toRefs } from 'vue'
import { state } from '@/modules/CommonState.js'

let { show_sidebar_toggle } = toRefs(state)
show_sidebar_toggle.value = false

const resources = [
  {
    title: 'Prediction Market FAQ',
    link: 'https://www.astralcodexten.com/p/prediction-market-faq',
    text: 'Scott Alexander describes the basics of prediction markets, their current state, and novel uses.'
  },
  {
    title: 'Prediction Markets are not Polls',
    link: 'https://outsidetheasylum.blog/prediction-markets-are-not-polls/',
    text: 'Isaac King describes specific advantages prediction markets have over polls.'
  },
  {
    title: 'Wikipedia: Scoring Rule',
    link: 'https://en.wikipedia.org/wiki/Scoring_rule',
    text: "Wikipedia's article on scoring rules such as Brier scores and Logarithmic scores."
  },
  {
    title: 'Metaculus Track Record',
    link: 'https://www.metaculus.com/questions/track-record/',
    text: "Metaculus's own excellent calibration page with lots of options and excellent visualizations."
  },
  {
    title: 'Prediction Market Map',
    link: 'https://saul-munn.notion.site/64b8b93b076a40f598f1788b314039c7?v=472f982a7d834a1185167e48ea1f41e3',
    text: 'Saul Munn lists several prediction market platforms, organizations, tools, community projects, and more.'
  }
]

const details = [
  {
    title: 'All',
    notes: [
      'To calculate the time-averaged probability, we assume the market opens at 50%. Once the first trade occurs, we track the probability at each trade and the cumulative durations to generate an average.'
    ]
  },
  {
    title: 'Kalshi',
    notes: [
      'We use the YES price from the most recently executed trade as the probability at any point in time.',
      'The counter for the number of unique traders is currently unimplemented.'
    ],
    types: [
      {
        label: 'Binary',
        icon: 'mdi-checkbox-marked-outline'
      },
      {
        label: 'Multiple-Choice',
        icon: 'mdi-checkbox-blank-outline'
      }
    ]
  },
  {
    title: 'Manifold',
    notes: [],
    types: [
      {
        label: 'CPMM-1 Binary',
        icon: 'mdi-checkbox-marked-outline'
      },
      {
        label: 'CPMM-1 Pseudo-Numeric',
        icon: 'mdi-checkbox-blank-outline'
      },
      {
        label: 'CPMM-1 Pseudo-Numeric',
        icon: 'mdi-checkbox-blank-outline'
      },
      {
        label: 'CPMM-1 Pseudo-Numeric',
        icon: 'mdi-checkbox-blank-outline'
      },
      {
        label: 'DPM-2 Binary',
        icon: 'mdi-checkbox-blank-off-outline'
      }
    ]
  },
  {
    title: 'Metaculus',
    notes: [
      'We use the community_prediction.history.x2.avg series for the probability.',
      'Since Metaculus does not have bets, we use the number of forecasts at 10 cents each for the market volume.'
    ],
    types: [
      {
        label: 'Binary',
        icon: 'mdi-checkbox-marked-outline'
      },
      {
        label: 'Multiple-Choice',
        icon: 'mdi-checkbox-blank-outline'
      }
    ]
  },
  {
    title: 'Polymarket',
    notes: [
      'We used to use the Gamma API which had defined start and end dates, but that functionality has been removed. We declare a market has started when the first trade occurs and end at the date noted by end_date_iso. This field is optional and markets without it are not counted.',
      'The counter for the number of unique traders is currently unimplemented.',
      'The counter for market volume is currently unimplemented.'
    ],
    types: [
      {
        label: 'Binary',
        icon: 'mdi-checkbox-marked-outline'
      },
      {
        label: 'Multiple-Choice',
        icon: 'mdi-checkbox-blank-outline'
      },
      {
        label: 'Non-CLOB Markets',
        icon: 'mdi-checkbox-blank-off-outline'
      }
    ]
  },
  {
    title: 'Disclaimer',
    notes: [
      'I use Manifold much more than any of the other platforms included in this analysis, and have received bounties from the Manifold team in both mana (play money) and real money. Their contributions did not affect the contents of this site in any way.'
    ]
  }
]
</script>

<template>
  <v-main>
    <h2>Resources</h2>
    <v-container>
      <v-row align="center" justify="center">
        <v-col cols="6" v-for="res in resources">
          <v-card
            variant="tonal"
            :title="res.title"
            :href="res.url"
            :text="res.text"
            color="purple-darken-4"
            target="_blank"
            append-icon="mdi-open-in-new"
            hover
          >
          </v-card>
        </v-col>
      </v-row>
    </v-container>
    <h2>Implementation Details</h2>
    <p>
      When standardizing things across platforms we ran into some edge cases, I've tried to detail
      them all here. When in doubt, you can always check the
      <a href="https://github.com/wasabipesto/themis">source</a> to see how we compute a specific
      attribute.
    </p>
    <v-container>
      <v-row align="start" justify="center">
        <v-col cols="6" v-for="plt in details">
          <v-card variant="outlined" :title="plt.title" color="purple-darken-4">
            <v-card-text>
              <v-table density="compact">
                <tbody>
                  <tr v-for="note in plt.notes">
                    <td>
                      <div class="detail-card-cell">
                        {{ note }}
                      </div>
                    </td>
                  </tr>
                  <tr v-if="'types' in plt">
                    <td>
                      <div class="detail-card-cell">
                        Supported market types:
                        <v-table density="compact">
                          <tbody>
                            <tr v-for="typ in plt.types">
                              <td width="50">
                                <v-icon :icon="typ.icon" size="small"></v-icon>
                              </td>
                              <td>{{ typ.label }}</td>
                            </tr>
                          </tbody>
                        </v-table>
                      </div>
                    </td>
                  </tr>
                </tbody>
              </v-table>
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
    </v-container>
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
.detail-card-cell {
  margin: 0.5rem 0;
}
code {
  font-size: 0.75rem;
}
</style>
