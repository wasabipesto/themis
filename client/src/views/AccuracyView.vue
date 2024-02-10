<script setup>
import { ref, toRefs, watch } from 'vue'
import axios from 'axios'
import CommonFilters from '@/components/CommonFilters.vue'
import { state } from '@/modules/CommonState.js'
import { debounce } from 'lodash'
import { Chart as ChartJS, registerables } from 'chart.js'
import { Bubble } from 'vue-chartjs'

ChartJS.register(...registerables)

let { query_selected, left_sidebar_options_expanded } = toRefs(state)

query_selected.value = {
  scoring_attribute: 'prob_at_midpoint',
  //weight_attribute: 'none',
  xaxis_attribute: 'open_days',
  num_market_points: 1000,
  ...query_selected.value
}

const query_options = {
  scoring_attribute: {
    prob_at_midpoint: { label: 'Probability at Market Midpoint' },
    prob_at_close: { label: 'Probability at Market Close' },
    prob_time_avg: {
      label: 'Market Time-Averaged Probability',
      tooltip:
        'For each market, this is the probability averaged over time. <br>\
        Each market is only counted once.'
    }
  },
  xaxis_attribute: {
    volume_usd: { label: 'Market Volume' },
    open_days: { label: 'Market Length' },
    num_traders: { label: 'Number of Traders' }
  }
}

function getOptionLabel(option, value) {
  try {
    return query_options[option][value]['label']
  } catch {
    return '¯\\_(ツ)_/¯'
  }
}

const chart_data = ref({
  datasets: []
})
const chart_options = ref({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    intersect: false,
    mode: 'nearest',
    axis: 'xy'
  },
  layout: {
    padding: 8
  },
  plugins: {
    title: {
      display: true,
      text: 'Accuracy Plot',
      padding: 16,
      font: {
        size: 16,
        weight: 'bold'
      }
    },
    legend: {
      title: {
        display: true,
        text: '',
        padding: {
          top: 42
        },
        font: {
          weight: 'bold'
        }
      },
      position: 'right',
      align: 'start'
    }
  },
  scales: {
    x: {
      title: {
        display: true,
        text: 'Something',
        padding: 12,
        font: {
          size: 14
        }
      }
    },
    y: {
      title: {
        display: true,
        text: 'Brier Score',
        padding: 12,
        font: {
          size: 14
        }
      },
      min: 0,
      max: 0.5
    }
  }
})
const platform_data = ref([])

const loading = ref(true)
async function updateGraph() {
  loading.value = true

  let response
  try {
    response = await axios.get('https://beta-api.calibration.city/accuracy_plot', {
      params: query_selected.value
    })
  } catch (error) {
    console.error('Error fetching data:', error)
  }
  const response_data = response.data

  var datasets = []
  response_data.traces.forEach((t) =>
    datasets.push({
      type: 'scatter',
      label: t.platform.name_fmt,
      backgroundColor: t.platform.color + '80',
      borderColor: t.platform.color,
      data: t.market_points
    })
  )
  chart_data.value = {
    datasets: datasets
  }
  chart_options.value.plugins.title.text = response_data.metadata.title
  chart_options.value.scales.x.title.text = response_data.metadata.x_title
  chart_options.value.scales.y.title.text = response_data.metadata.y_title
  chart_options.value = { ...chart_options.value }

  var platforms = []
  response_data.traces.forEach((t) =>
    platforms.push({
      name: t.platform.name_fmt,
      description: t.platform.description,
      avatar_url: t.platform.avatar_url,
      site_url: t.platform.site_url,
      color: t.platform.color + '40'
    })
  )
  platform_data.value = platforms

  loading.value = false
}
watch(
  () => state.query_selected,
  debounce((query_selected) => {
    //console.log(query_selected)
    updateGraph()
  }, 100),
  { deep: true }
)
</script>

<template>
  <v-navigation-drawer :width="400" v-model="state.left_sidebar_visible" app>
    <v-expansion-panels v-model="left_sidebar_options_expanded" multiple variant="accordion">
      <v-expansion-panel value="accuracy_scoring_attribute">
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-ruler-square-compass</v-icon>
          Brier Scoring Method: <br />
          {{ getOptionLabel('scoring_attribute', query_selected.scoring_attribute) }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">TODO</p>
          <v-radio-group v-model="query_selected.scoring_attribute">
            <v-radio v-for="(v, k) in query_options.scoring_attribute" :key="k" :value="k">
              <template v-slot:label>
                {{ v.label }}
                <v-tooltip v-if="v.tooltip" activator="parent" location="end">
                  <span v-html="v.tooltip"></span>
                </v-tooltip>
              </template>
            </v-radio>
          </v-radio-group>
        </v-expansion-panel-text>
      </v-expansion-panel>
      <v-expansion-panel value="accuracy_xaxis_attribute">
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-globe-model</v-icon>
          X-Axis Attribute: <br />
          {{ getOptionLabel('xaxis_attribute', query_selected.xaxis_attribute) }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">TODO</p>
          <v-radio-group v-model="query_selected.xaxis_attribute">
            <v-radio
              v-for="(v, k) in query_options.xaxis_attribute"
              :key="k"
              :value="k"
              :label="v.label"
            ></v-radio>
          </v-radio-group>
        </v-expansion-panel-text>
      </v-expansion-panel>
      <v-divider :thickness="16"></v-divider>
      <CommonFilters />
    </v-expansion-panels>
  </v-navigation-drawer>

  <v-snackbar v-model="loading">
    <v-progress-circular indeterminate color="red"></v-progress-circular>
    <span class="mx-5">Loading data...</span>
  </v-snackbar>

  <v-main>
    <v-container>
      <v-row>
        <v-col>
          <v-card elevation="10">
            <v-card-text>
              <Bubble :data="chart_data" :options="chart_options" :width="1200" :height="600" />
            </v-card-text>
          </v-card>
        </v-col>
      </v-row>
      <v-row align="center" justify="center">
        <v-col cols="12" sm="6" md="3" v-for="p in platform_data">
          <v-card
            :color="p.color"
            :href="p.site_url"
            target="_blank"
            append-icon="mdi-open-in-new"
            hover
          >
            <template v-slot:title>
              <span class="d-flex align-center">
                <img :src="'../' + p.avatar_url" width="20" class="mr-2" />
                {{ p.name }}
              </span>
            </template>
            <template v-slot:text>
              {{ p.description }}
            </template>
          </v-card>
        </v-col>
      </v-row>
    </v-container>
  </v-main>
</template>
