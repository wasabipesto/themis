<script setup>
import { ref, toRefs, watch, watchEffect } from 'vue'
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
    open_date: { label: 'Open Date' },
    close_date: { label: 'Close Date' },
    volume_usd: { label: 'Market Volume' },
    open_days: { label: 'Market Length' },
    market_duration: { label: 'Market Duration' },
    num_traders: { label: 'Number of Traders' }
  },
  num_market_points: {
    range: [500, 5000],
    step: 500
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
    },
    tooltip: {
      callbacks: {
        title: function (context) {
          if (context[0].raw.point_title) {
            return context[0].raw.point_title
          }
        },
        label: function (context) {
          if (context.raw.point_label) {
            return context.raw.point_label
          }
        }
      }
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
      max: 1
    }
  }
})
const platform_data = ref([])

const loading = ref(true)
async function updateGraph() {
  loading.value = true

  let response
  try {
    response = await axios.get('https://api.calibration.city/accuracy_plot', {
      params: query_selected.value
    })
  } catch (error) {
    console.error('Error fetching data:', error)
  }
  const response_data = response.data

  var datasets = []
  response_data.traces.forEach((t) =>
    datasets.push(
      {
        type: 'scatter',
        label: t.platform.name_fmt + ' Markets',
        backgroundColor: t.platform.color + '40',
        borderColor: t.platform.color + '80',
        pointRadius: 2,
        data: t.market_points
      },
      {
        type: 'line',
        label: t.platform.name_fmt + ' Accuracy',
        //backgroundColor: 'black',
        borderColor: t.platform.color,
        pointRadius: 4,
        cubicInterpolationMode: 'monotone',
        spanGaps: true,
        //stepped: true,
        data: t.accuracy_line
      }
    )
  )
  chart_data.value = {
    datasets: datasets
  }

  // set chart-level options
  chart_options.value.plugins.title.text = response_data.metadata.title
  chart_options.value.scales.x.title.text = response_data.metadata.x_title
  chart_options.value.scales.x.min = response_data.metadata.x_min
  chart_options.value.scales.x.max = response_data.metadata.x_max
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

function getOptionLabel(option, value) {
  try {
    return query_options[option][value]['label']
  } catch {
    return '¯\\_(ツ)_/¯'
  }
}

const scoring_override = ref(null)
function get_xaxis_attribute_label() {
  if (scoring_override.value == null) {
    return getOptionLabel('scoring_attribute', query_selected.value.scoring_attribute)
  } else {
    return scoring_override.value
  }
}
watchEffect(() => {
  if (query_selected.value.xaxis_attribute == 'market_duration') {
    scoring_override.value = 'Probability at X Percent'
  } else {
    scoring_override.value = null
  }
})
</script>

<template>
  <v-navigation-drawer :width="400" v-model="state.left_sidebar_visible" app>
    <v-expansion-panels v-model="left_sidebar_options_expanded" multiple variant="accordion">
      <v-expansion-panel value="accuracy_scoring_attribute">
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-ruler-square-compass</v-icon>
          Brier Scoring Method: <br />
          {{ get_xaxis_attribute_label() }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">The Brier score for each market is caluclated based on this attribute.</p>
          <v-radio-group
            v-model="query_selected.scoring_attribute"
            :readonly="scoring_override != null"
          >
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
          <v-icon class="mr-3">mdi-ruler</v-icon>
          Compare Against X-Axis: <br />
          {{ getOptionLabel('xaxis_attribute', query_selected.xaxis_attribute) }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            Compare each platform's Brier score against this value. For example, you can investigate
            how each platform's accuracy is affected by market length or number of traders.
          </p>
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
      <v-expansion-panel value="accuracy_num_market_points">
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-plus-circle-outline</v-icon>
          Market Points to Display: {{ query_selected.num_market_points }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            A random selection of markets are displayed on the chart as a scatterplot. Increase this
            to show more markets per platform. Note that this does not affect the accuracy line
            calculations in any way.
          </p>
          <v-alert border="start" border-color="red" elevation="2" density="compact">
            Changing this setting may impact your browser's performance.
          </v-alert>
          <v-slider
            v-model="query_selected.num_market_points"
            :min="query_options.num_market_points.range[0]"
            :max="query_options.num_market_points.range[1]"
            :step="query_options.num_market_points.step"
            class="pt-10"
            show-ticks="always"
            thumb-label="always"
          >
          </v-slider>
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
