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
    items: [
      {
        items: [
          {
            label:
              'To calculate the time-averaged probability, we assume the market opens at 50%. Once the first trade occurs, we track the probability at each trade and the cumulative durations to generate an average.',
            icon: 'mdi-information-outline'
          }
        ]
      }
    ]
  },
  {
    title: 'Kalshi',
    items: [
      {
        label: 'Notes:',
        items: [
          {
            label:
              'We use the YES price from the most recently executed trade as the probability at any point in time.',
            icon: 'mdi-information-outline'
          },
          {
            label: 'The counter for the number of unique traders is currently unimplemented.',
            icon: 'mdi-progress-wrench'
          }
        ]
      },
      {
        label: 'Supported market types:',
        items: [
          {
            label: 'Binary',
            icon: 'mdi-checkbox-marked-circle-outline'
          },
          {
            label: 'Multiple-Choice',
            icon: 'mdi-circle-outline'
          }
        ]
      }
    ]
  },
  {
    title: 'Manifold',
    items: [
      {
        label: 'Supported market types:',
        items: [
          {
            label: 'Binary',
            icon: 'mdi-checkbox-marked-circle-outline'
          },
          {
            label: 'Pseudo-Numeric',
            icon: 'mdi-progress-wrench'
          },
          {
            label: 'Multiple-Choice Unlinked',
            icon: 'mdi-progress-wrench'
          },
          {
            label: 'Multiple-Choice Linked',
            icon: 'mdi-circle-outline'
          },
          {
            label: 'Non-CPMM Markets',
            icon: 'mdi-cancel'
          }
        ]
      }
    ]
  },
  {
    title: 'Metaculus',
    items: [
      {
        label: 'Notes:',
        items: [
          {
            label: 'We use the community prediction (history.x2.avg series) for the probability.',
            icon: 'mdi-information-outline'
          },
          {
            label:
              'Since Metaculus does not have bets, we use the number of forecasts at 10 cents each for the market volume.',
            icon: 'mdi-information-outline'
          }
        ]
      },
      {
        label: 'Supported market types:',
        items: [
          {
            label: 'Binary',
            icon: 'mdi-checkbox-marked-circle-outline'
          },
          {
            label: 'Multiple-Choice',
            icon: 'mdi-circle-outline'
          }
        ]
      }
    ]
  },
  {
    title: 'Polymarket',
    items: [
      {
        label: 'Notes:',
        items: [
          {
            label:
              'We declare a market has started when the first trade occurs and end at the date noted by end_date_iso. This field is optional and markets without it are not counted.',
            icon: 'mdi-information-outline'
          },
          {
            label: 'The counter for the number of unique traders is currently unimplemented.',
            icon: 'mdi-progress-wrench'
          },
          {
            label: 'The counter for market volume is currently unimplemented.',
            icon: 'mdi-progress-wrench'
          }
        ]
      },
      {
        label: 'Supported market types:',
        items: [
          {
            label: 'Binary',
            icon: 'mdi-checkbox-marked-circle-outline'
          },
          {
            label: 'Multiple-Choice',
            icon: 'mdi-circle-outline'
          },
          {
            label: 'Non-CLOB Markets',
            icon: 'mdi-cancel'
          }
        ]
      }
    ]
  },
  {
    title: 'Disclaimer',
    items: [
      {
        items: [
          {
            label:
              'I use Manifold much more than any of the other platforms included in this analysis, and have received bounties from the Manifold team in both mana (play money) and real money. Their contributions did not affect the contents of this site in any way.',
            icon: 'mdi-shield-alert-outline'
          }
        ]
      }
    ]
  }
]
</script>

<template>
  <v-main>
    <h2>Resources</h2>
    <v-container>
      <v-row align="center" justify="center">
        <v-col cols="12" md="6" v-for="res in resources">
          <v-card
            variant="tonal"
            :title="res.title"
            :href="res.link"
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
        <v-col cols="12" md="6" v-for="i in details">
          <v-card variant="outlined" :title="i.title" color="purple-darken-4">
            <v-card-text>
              <div v-for="j in i.items" class="ma-3">
                <v-table density="compact">
                  <thead v-if="j.label">
                    <tr>
                      <th colspan="2">
                        {{ j.label }}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="k in j.items">
                      <td width="50">
                        <v-icon :icon="k.icon"></v-icon>
                      </td>
                      <td class="py-1">{{ k.label }}</td>
                    </tr>
                  </tbody>
                </v-table>
              </div>
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
</style>
