<script setup>
import { ref, onMounted, watch } from 'vue'
import axios from 'axios'
import CommonFilters from '@/components/CommonFilters.vue'
import { state } from '@/modules/CommonState.js'
import { debounce } from 'lodash'
import { Chart as ChartJS, Tooltip, Legend, PointElement, LinearScale, Title } from 'chart.js'
import { Bubble } from 'vue-chartjs'

ChartJS.register(LinearScale, PointElement, Tooltip, Legend, Title)

const chart_data = ref({
  datasets: []
})
const chart_options = ref({
  responsive: true,
  maintainAspectRatio: false,
  layout: {
    padding: 8
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
      position: 'chartArea',
      align: 'start'
    }
  },
  scales: {
    x: {
      title: {
        display: true,
        text: 'Prediction',
        padding: 12,
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
        padding: 12,
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
const platform_data = ref([])

const loading = ref(true)
async function updateGraph() {
  loading.value = true

  let response_data
  try {
    const response = await axios.get('https://beta-api.calibration.city/calibration_plot', {
      params: state.query_selected
    })
    response_data = response.data
  } catch (error) {
    console.error('Error fetching data:', error)
  }

  var datasets = []
  response_data.traces.forEach((t) =>
    datasets.push({
      label: t.platform.name_fmt,
      backgroundColor: t.platform.color + '80',
      borderColor: t.platform.color,
      data: t.points
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
onMounted(() => {
  updateGraph()
})
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
    <v-expansion-panels multiple variant="accordion"> <CommonFilters /> </v-expansion-panels>
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
      <v-row>
        <v-col class="v-col-3" v-for="p in platform_data">
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
