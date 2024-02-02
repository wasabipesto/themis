<script setup>
import { ref, onMounted, watch } from 'vue'
import axios from 'axios'
import CommonFilters from '@/components/CommonFilters.vue'
import { state } from '@/modules/CommonState.js'
import { debounce } from 'lodash'
import * as d3 from 'd3';

const loading = ref(true)
async function updateGraph() {
  loading.value = true

  let data
  try {
    const response = await axios.get('https://beta-api.calibration.city/calibration_plot', {
      params: state.query_selected
    })
    data = response.data
  } catch (error) {
    console.error('Error fetching data:', error)
  }
  console.log(data)

  // Declare the chart dimensions and margins.
  const width = 1200;
  const height = 600;
  const marginTop = 20;
  const marginRight = 20;
  const marginBottom = 30;
  const marginLeft = 40;

  const percent_scale = d3.scaleLinear().domain([0, 1])

  // Declare the x (horizontal position) scale.
  const x_axis = d3.scaleLinear()
      .domain([0, 1])
      .range([marginLeft, width - marginRight]);

  // Declare the y (vertical position) scale.
  const y_axis = d3.scaleLinear()
      .domain([0, 1])
      .range([height - marginBottom, marginTop]);

  // Create the SVG container.
  const svg = d3.select("svg")
      .attr("width", width)
      .attr("height", height);

  // Add the x-axis.
  svg.append("g")
      .attr("transform", `translate(0,${height - marginBottom})`)
      .call(d3.axisBottom(x_axis).tickFormat(d3.format('~%')));

  // Add the y-axis.
  svg.append("g")
      .attr("transform", `translate(${marginLeft},0)`)
      .call(d3.axisLeft(y_axis).tickFormat(d3.format('~%')));

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
  <v-main>
    <v-card elevation="16">
      <v-card-text>
        <div id="graph">
          <svg>

          </svg>
        </div>
      </v-card-text>
    </v-card>
  </v-main>

  <v-snackbar v-model="loading">
    <v-progress-circular indeterminate color="red"></v-progress-circular>
    <span class="mx-5">Loading data...</span>
  </v-snackbar>
</template>