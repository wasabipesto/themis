<script setup>
import { ref } from 'vue'
import axios from 'axios'

const loading = ref(true)

const platform_metadata = ref([])
const table_headers = ref([])
const table_data = ref([])

function build_table_row(platform_stats, category) {
  const platforms_in_category = platform_stats.filter((i) => i.category === category)
  var output = { category: category }
  platforms_in_category.forEach((i) => {
    output[i.platform + '_absolute_brier'] = i.platform_absolute_brier
    output[i.platform + '_relative_brier'] = i.platform_relative_brier
    output[i.platform + '_sample_presence'] = i.platform_sample_presence
  })
  return output
}

async function fetchData() {
  loading.value = true

  let platform_stats
  try {
    const response = await axios.get('https://beta-api.calibration.city/group_accuracy')
    platform_metadata.value = response.data.platform_metadata
    platform_stats = response.data.platform_stats
  } catch (error) {
    console.error('Error fetching group view data:', error)
  }

  platform_stats.forEach((i) => {
    i.platform_absolute_brier = i.platform_absolute_brier.toFixed(4)
    i.platform_relative_brier = i.platform_relative_brier.toFixed(4)
    i.platform_sample_presence = i.platform_sample_presence.toFixed(4) * 100 + '%'
  })

  table_data.value = []
  const categories = [...new Set(platform_stats.map((i) => i.category))]
  categories.forEach((category) => {
    table_data.value.push(build_table_row(platform_stats, category))
  })
  console.log(table_data.value)

  table_headers.value = [{ title: 'Category', key: 'category' }]
  platform_metadata.value.forEach((platform) => {
    table_headers.value.push({
      title: platform.name_fmt,
      align: 'center',
      children: [
        { title: 'Absolute Brier', value: platform.name + '_absolute_brier' },
        { title: 'Relative Brier', value: platform.name + '_relative_brier' },
        { title: 'Sample Presence', value: platform.name + '_sample_presence' }
      ]
    })
  })
  console.log(table_headers.value)

  loading.value = false
}

fetchData()
</script>

<template>
  <v-main>
    <v-card flat title="Platform Comparison">
      <v-data-table
        :headers="headers"
        :items="table_data"
        :loading="loading"
        :items-per-page="1000"
      >
      </v-data-table>
    </v-card>
  </v-main>
</template>
