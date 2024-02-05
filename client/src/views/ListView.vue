<script setup>
import { ref, toRefs, watch } from 'vue'
import axios from 'axios'
import CommonFilters from '@/components/CommonFilters.vue'
import { state } from '@/modules/CommonState.js'
import { debounce } from 'lodash'

let { query_selected, left_sidebar_options_expanded } = toRefs(state)

const loading = ref(true)
const responseItems = ref([])
async function updateList() {
  loading.value = true

  if (query_selected.limit == null) {
    query_selected.limit = 100
  }

  let items
  try {
    const response = await axios.get('https://beta-api.calibration.city/list_markets', {
      params: query_selected.value
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

  responseItems.value = items
  loading.value = false
}

const itemsPerPage = ref(100)
const itemsPerPageOptions = ref([
  { value: 100, title: '100' },
  { value: 250, title: '250' },
  { value: 500, title: '500' },
  { value: 1000, title: '1000' }
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

function sendTableDataToQuery({ page, sortBy }) {
  const limit = itemsPerPage.value
  const offset = (page - 1) * limit

  let sort_attribute, sort_desc
  if (sortBy.length) {
    sort_attribute = sortBy[0].key
    if (sortBy[0].order == 'desc') {
      sort_desc = true
    } else {
      sort_desc = false
    }
  } else {
    sort_attribute = 'volume_usd'
    sort_desc = true
  }

  query_selected.value = Object.assign(query_selected.value, {
    limit,
    offset,
    sort_attribute,
    sort_desc
  })
}
watch(
  () => query_selected.value,
  debounce((query_selected) => {
    //console.log(query_selected)
    updateList()
  }, 100),
  { deep: true }
)
</script>

<template>
  <v-navigation-drawer :width="400" v-model="state.left_sidebar_visible" app>
    <v-expansion-panels v-model="left_sidebar_options_expanded" multiple variant="accordion">
      <CommonFilters />
    </v-expansion-panels>
  </v-navigation-drawer>
  <v-main>
    <v-card flat title="Market List" class="my-5">
      <template v-slot:text>
        <v-text-field
          v-model="query_selected.title_contains"
          label="Search"
          prepend-inner-icon="mdi-magnify"
          single-line
          variant="outlined"
          density="compact"
          hide-details
          clearable
        ></v-text-field>
      </template>
      <v-data-table-server
        v-model:items-per-page="itemsPerPage"
        :items-per-page-options="itemsPerPageOptions"
        :headers="headers"
        :items="responseItems"
        :items-length="10000"
        :loading="loading"
        item-value="name"
        hover
        @update:options="sendTableDataToQuery"
      >
        <template #item.title="{ value, item }">
          <a :href="item.url" target="_blank">
            {{ value }}
          </a>
        </template>
      </v-data-table-server>
    </v-card>
  </v-main>
</template>
