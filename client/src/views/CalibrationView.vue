<script setup>
import { ref, onMounted, watch } from 'vue'
import axios from 'axios'
import CommonFilters from '@/components/CommonFilters.vue'
import { state } from '@/modules/CommonState.js'
import { debounce } from 'lodash'

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
        <div id="graph"></div>
      </v-card-text>
    </v-card>
  </v-main>

  <v-snackbar v-model="loading">
    <v-progress-circular indeterminate color="red"></v-progress-circular>
    <span class="mx-5">Loading data...</span>
  </v-snackbar>
</template>
