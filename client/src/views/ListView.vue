<script setup>
import { ref } from 'vue'
import axios from 'axios'

const ListAPI = {
  fetch: async ({ page, itemsPerPage, sortBy }) => {
    return new Promise(async (resolve) => {
      const limit = itemsPerPage
      const offset = (page - 1) * limit

      let sortAttr, sortDesc
      if (sortBy.length) {
        sortAttr = sortBy[0].key
        if (sortBy[0].order == 'desc') {
          sortDesc = true
        } else {
          sortDesc = false
        }
      } else {
        sortAttr = 'volume_usd'
        sortDesc = true
      }

      let items
      try {
        const response = await axios.get('https://beta-api.calibration.city/list_markets', {
          params: { limit, offset, sort_attribute: sortAttr, sort_desc: sortDesc }
        })
        items = response.data.markets
      } catch (error) {
        console.error('Error fetching market list:', error)
      }

      items.forEach((obj) => {
        obj.platform = obj.platform.charAt(0).toUpperCase() + obj.platform.slice(1)
      })

      const attributes_to_round = ['open_days', 'volume_usd']
      items.forEach((obj) => {
        attributes_to_round.forEach((attribute) => {
          obj[attribute] = Math.round(obj[attribute] * 100) / 100
        })
      })

      const attributes_to_usd = ['volume_usd']
      items.forEach((obj) => {
        attributes_to_usd.forEach((attribute) => {
          obj[attribute] = '$' + obj[attribute]
        })
      })

      const attributes_to_pct = ['prob_at_midpoint', 'prob_at_close', 'prob_time_avg', 'resolution']
      items.forEach((obj) => {
        attributes_to_pct.forEach((attribute) => {
          obj[attribute] = (obj[attribute] * 100).toFixed(2) + '%'
        })
      })

      resolve({ items })
    })
  }
}

const search = ref('')
const loading = ref(true)
const itemsPerPage = ref(100)
const itemsPerPageOptions = ref([
  { value: 100, title: '100' },
  { value: 250, title: '250' },
  { value: 500, title: '500' },
  { value: 1000, title: '1000' },
  { value: 10000, title: '10,000' },
  { value: 99999999, title: 'All' }
])
const headers = ref([
  { title: 'Title', key: 'title', align: 'start' },
  { title: 'Platform', key: 'platform', align: 'start' },
  { title: 'Category', key: 'category', align: 'start' },
  //{ title: 'Open Date', key: 'open_dt', align: 'start' },
  //{ title: 'Close Date', key: 'close_dt', align: 'start' },
  { title: 'Traders', key: 'num_traders', align: 'end' },
  { title: 'Days Open', key: 'open_days', align: 'end' },
  { title: 'Volume', key: 'volume_usd', align: 'end' },
  { title: 'Midpoint', key: 'prob_at_midpoint', align: 'end' },
  { title: 'Close', key: 'prob_at_close', align: 'end' },
  { title: 'Time Average', key: 'prob_time_avg', align: 'end' },
  { title: 'Resolution', key: 'resolution', align: 'end' }
])

const responseItems = ref([])
function loadItems({ page, itemsPerPage, sortBy }) {
  loading.value = true
  ListAPI.fetch({ page, itemsPerPage, sortBy }).then(({ items }) => {
    responseItems.value = items
    loading.value = false
  })
}
</script>

<template>
  <v-card flat title="Market List" class="my-5">
    <template v-slot:text>
      <v-text-field
        v-model="search"
        label="Search"
        prepend-inner-icon="mdi-magnify"
        single-line
        variant="outlined"
        density="compact"
        hide-details
      ></v-text-field>
    </template>
    <v-data-table-server
      v-model:items-per-page="itemsPerPage"
      :items-per-page-options="itemsPerPageOptions"
      :headers="headers"
      :items="responseItems"
      :items-length="itemsPerPage"
      :loading="loading"
      :search="search"
      item-value="name"
      hover
      @update:options="loadItems"
    >
      <template #item.title="{ value, item }">
        <a :href="item.url" target="_blank">
          {{ value }}
        </a>
      </template>
    </v-data-table-server>
  </v-card>
</template>
