<script setup>
import { ref, toRefs, watchEffect } from 'vue'
import { state } from '@/modules/CommonState.js'
import { useDisplay } from 'vuetify'

let { query_selected, left_sidebar_visible, show_sidebar_toggle } = toRefs(state)

// automatically show sidebar if on desktop
let { mdAndUp } = useDisplay()
if (mdAndUp.value) {
  left_sidebar_visible.value = true
} else {
  left_sidebar_visible.value = false
}

show_sidebar_toggle.value = true

query_selected.value = {
  title_contains: null,
  platform_select: null,
  category_select: null,
  open_ts_min: null,
  open_ts_max: null,
  close_ts_min: null,
  close_ts_max: null,
  num_traders_min: null,
  num_traders_max: null,
  open_days_min: null,
  open_days_max: null,
  volume_usd_min: null,
  volume_usd_max: null,
  prob_at_midpoint_min: null,
  prob_at_midpoint_max: null,
  prob_at_close_min: null,
  prob_at_close_max: null,
  prob_time_avg_min: null,
  prob_time_avg_max: null,
  resolution_min: null,
  resolution_max: null,
  ...query_selected.value
}

const query_options = {
  platforms: {
    kalshi: { label: 'Kalshi' },
    manifold: { label: 'Manifold' },
    metaculus: { label: 'Metaculus' },
    polymarket: { label: 'Polymarket' }
  },
  categories: [
    'AI',
    'Climate',
    'Culture',
    'Crypto',
    'Economics',
    'Politics',
    'Science',
    'Sports',
    'Technology'
  ],
  open_date: { range: [new Date(2000, 0, 1), new Date()] },
  close_date: { range: [new Date(2000, 0, 1), new Date()] }
}

const open_date_range = ref([query_options.open_date.range[0], query_options.open_date.range[1]])
const open_date_range_pickers = ref([false, false])
const close_date_range = ref([query_options.close_date.range[0], query_options.close_date.range[1]])
const close_date_range_pickers = ref([false, false])
const prob_at_midpoint_pct = ref([0, 100])
const prob_at_close_pct = ref([0, 100])
const prob_time_avg_pct = ref([0, 100])
const resolution_pct = ref([0, 100])

function get_text_label(text) {
  if (text == '' || text == null) {
    return 'Any'
  } else {
    return text
  }
}

function get_option_label(option, value) {
  try {
    return query_options[option][value]['label']
  } catch {
    return 'Any'
  }
}

function format_dates(dates) {
  const formatted = []
  dates.forEach((date) =>
    formatted.push([date.getMonth() + 1, date.getDate(), date.getFullYear()].join('/'))
  )
  return formatted[0] + ' to ' + formatted[1]
}

function get_date_label(dates) {
  const dates_formatted = format_dates(dates)
  const default_formatted = format_dates(query_options.open_date.range)
  if (dates_formatted == default_formatted) {
    return 'Any'
  } else {
    return dates_formatted
  }
}

function get_numeric_label(min, max, prefix, suffix) {
  let less_word
  if (suffix == '') {
    less_word = 'less'
  } else {
    less_word = 'fewer'
  }
  if (min == 0 && max == 100 && suffix == '%') {
    return 'Any'
  }
  if (min == null) {
    if (max == null) {
      return 'Any'
    } else {
      return prefix + max + ' or ' + less_word + suffix
    }
  } else {
    if (max == null) {
      return prefix + min + ' or more' + suffix
    } else {
      return prefix + min + ' to ' + max + suffix
    }
  }
}

watchEffect(() => {
  for (let key in query_selected.value) {
    if (query_selected.value[key] === '' || query_selected.value[key] === undefined) {
      query_selected.value[key] = null
    }
    if (query_selected.value.num_traders_min == 0) {
      query_selected.value.num_traders_min = null
    }
    if (query_selected.value.open_days_min == 0) {
      query_selected.value.open_days_min = null
    }
    if (query_selected.value.volume_usd_min == 0) {
      query_selected.value.volume_usd_min = null
    }
  }
})
watchEffect(() => {
  if (prob_at_midpoint_pct.value[0] < 0) {
    prob_at_midpoint_pct.value[0] = 0
  }
  if (prob_at_midpoint_pct.value[1] > 100) {
    prob_at_midpoint_pct.value[1] = 100
  }
  query_selected.value.prob_at_midpoint_min = prob_at_midpoint_pct.value[0] / 100
  query_selected.value.prob_at_midpoint_max = prob_at_midpoint_pct.value[1] / 100
})
watchEffect(() => {
  if (prob_at_close_pct.value[0] < 0) {
    prob_at_close_pct.value[0] = 0
  }
  if (prob_at_close_pct.value[1] > 100) {
    prob_at_close_pct.value[1] = 100
  }
  query_selected.value.prob_at_close_min = prob_at_close_pct.value[0] / 100
  query_selected.value.prob_at_close_max = prob_at_close_pct.value[1] / 100
})
watchEffect(() => {
  if (prob_time_avg_pct.value[0] < 0) {
    prob_time_avg_pct.value[0] = 0
  }
  if (prob_time_avg_pct.value[1] > 100) {
    prob_time_avg_pct.value[1] = 100
  }
  query_selected.value.prob_time_avg_min = prob_time_avg_pct.value[0] / 100
  query_selected.value.prob_time_avg_max = prob_time_avg_pct.value[1] / 100
})
watchEffect(() => {
  if (resolution_pct.value[0] < 0) {
    resolution_pct.value[0] = 0
  }
  if (resolution_pct.value[1] > 100) {
    resolution_pct.value[1] = 100
  }
  query_selected.value.resolution_min = resolution_pct.value[0] / 100
  query_selected.value.resolution_max = resolution_pct.value[1] / 100
})
watchEffect(() => {
  query_selected.value.open_ts_min = Math.floor(open_date_range.value[0].getTime() / 1000)
  query_selected.value.open_ts_max = Math.floor(open_date_range.value[1].getTime() / 1000)
})
watchEffect(() => {
  query_selected.value.close_ts_min = Math.floor(close_date_range.value[0].getTime() / 1000)
  query_selected.value.close_ts_max = Math.floor(close_date_range.value[1].getTime() / 1000)
})
</script>

<template>
  <v-expansion-panel value="title_contains">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-message-outline</v-icon>
      Title Contains: {{ get_text_label(query_selected.title_contains) }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter for markets that contain a specific term in their title. Note that different sites
        will have different naming conventions.
      </p>
      <v-text-field
        clearable
        v-model="query_selected.title_contains"
        prepend-inner-icon="mdi-magnify"
        label="Title Contains"
      ></v-text-field>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="platform_select">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-map-marker</v-icon>
      Platform: {{ get_option_label('platforms', query_selected.platform_select) }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">Filter the markets in the sample to only those from a certain site.</p>
      <v-chip-group
        v-model="query_selected.platform_select"
        selected-class="text-deep-purple-accent-4"
        column
        filter
      >
        <v-chip v-for="(v, k) in query_options.platforms" :key="k" :value="k" :text="v.label">
        </v-chip>
      </v-chip-group>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="category_select">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-group</v-icon>
      Category: {{ query_selected.category_select || 'Any' }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">Filter the markets in the sample to only those in a certain category.</p>
      <v-chip-group
        v-model="query_selected.category_select"
        selected-class="text-deep-purple-accent-4"
        column
        filter
      >
        <v-chip v-for="cat in query_options.categories" :key="cat" :value="cat">
          {{ cat }}
        </v-chip>
      </v-chip-group>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="num_traders">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-account-group-outline</v-icon>
      Unique Traders:
      {{
        get_numeric_label(
          query_selected.num_traders_min,
          query_selected.num_traders_max,
          '',
          ' traders'
        )
      }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a certain number of unique traders.
        Useful to filter out personal markets with no wider interest.
      </p>
      <v-alert border="start" border-color="red" elevation="2" density="compact">
        This filter is not implemented for all platforms.
      </v-alert>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="query_selected.num_traders_min"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="query_selected.num_traders_max"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-slider
              min="0"
              max="500"
              step="10"
              v-model="query_selected.num_traders_min"
              density="compact"
            >
            </v-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="open_days">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar</v-icon>
      Open Length:
      {{
        get_numeric_label(query_selected.open_days_min, query_selected.open_days_max, '', ' days')
      }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those open for a certain length of time. Useful to
        filter out very short or very long markets. This metric is measured in days.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="query_selected.open_days_min"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="query_selected.open_days_max"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-slider
              min="0"
              max="500"
              step="10"
              v-model="query_selected.open_days_min"
              density="compact"
            >
            </v-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="volume_usd">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-cash</v-icon>
      Market Volume:
      {{ get_numeric_label(query_selected.volume_usd_min, query_selected.volume_usd_max, '$', '') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a certain amount of money in volume.
        Useful to isolate only the high-profile markets. This metric is measured in USD.
      </p>
      <v-alert border="start" border-color="red" elevation="2" density="compact">
        This filter is not implemented for all platforms.
      </v-alert>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="query_selected.volume_usd_min"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
              prefix="$"
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="query_selected.volume_usd_max"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
              prefix="$"
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-slider
              min="0"
              max="500"
              step="10"
              v-model="query_selected.volume_usd_min"
              density="compact"
            >
            </v-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="open_date">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-clock-check-outline</v-icon>
      Open Date:
      {{ get_date_label(open_date_range) }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those that opened within a certain date range.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-btn
              @click="open_date_range_pickers[0] = true"
              prepend-icon="mdi-arrow-collapse-left"
            >
              Edit Min
            </v-btn>
            <v-dialog v-model="open_date_range_pickers[0]">
              <v-date-picker label="Start Date" v-model="open_date_range[0]"> </v-date-picker>
            </v-dialog>
          </v-col>
          <v-col>
            <v-btn
              @click="open_date_range_pickers[1] = true"
              append-icon="mdi-arrow-collapse-right"
            >
              Edit Max
            </v-btn>
            <v-dialog v-model="open_date_range_pickers[1]">
              <v-date-picker label="Start Date" v-model="open_date_range[1]"> </v-date-picker>
            </v-dialog>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="close_date">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-clock-alert-outline</v-icon>
      Close Date:
      {{ get_date_label(close_date_range) }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those that closed within a certain date range.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-btn
              @click="close_date_range_pickers[0] = true"
              prepend-icon="mdi-arrow-collapse-left"
            >
              Edit Min
            </v-btn>
            <v-dialog v-model="close_date_range_pickers[0]">
              <v-date-picker label="Start Date" v-model="close_date_range[0]"> </v-date-picker>
            </v-dialog>
          </v-col>
          <v-col>
            <v-btn
              @click="close_date_range_pickers[1] = true"
              append-icon="mdi-arrow-collapse-right"
            >
              Edit Max
            </v-btn>
            <v-dialog v-model="close_date_range_pickers[1]">
              <v-date-picker label="Start Date" v-model="close_date_range[1]"> </v-date-picker>
            </v-dialog>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="prob_at_midpoint">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar-import</v-icon>
      Midpoint Probability:
      {{ get_numeric_label(prob_at_midpoint_pct[0], prob_at_midpoint_pct[1], '', '%') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a midpoint probability in a certain
        range.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="prob_at_midpoint_pct[0]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="prob_at_midpoint_pct[1]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-range-slider
              min="0"
              max="100"
              step="1"
              v-model="prob_at_midpoint_pct"
              density="compact"
            >
            </v-range-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="prob_at_close">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar-end</v-icon>
      Probability at Close:
      {{ get_numeric_label(prob_at_close_pct[0], prob_at_close_pct[1], '', '%') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a closing probability in a certain
        range. Must be a decimal between 0 and 1.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="prob_at_close_pct[0]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="prob_at_close_pct[1]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-range-slider
              min="0"
              max="100"
              step="1"
              v-model="prob_at_close_pct"
              density="compact"
            >
            </v-range-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="prob_time_avg">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar-start</v-icon>
      Time-Averaged Probability:
      {{ get_numeric_label(prob_time_avg_pct[0], prob_time_avg_pct[1], '', '%') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a time-averaged probability in a certain
        range. Must be a decimal between 0 and 1.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="prob_time_avg_pct[0]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="prob_time_avg_pct[1]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-range-slider
              min="0"
              max="100"
              step="1"
              v-model="prob_time_avg_pct"
              density="compact"
            >
            </v-range-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel value="resolution">
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-seal</v-icon>
      Resolution Probability:
      {{ get_numeric_label(resolution_pct[0], resolution_pct[1], '', '%') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those that resolved with a certain value. Useful
        for isolating markets that resolved YES (100%) or NO (0%).
      </p>
      <v-container>
        <v-row class="my-0">
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="resolution_pct[0]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
          <v-col>
            <v-text-field
              label="Maximum"
              v-model="resolution_pct[1]"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
            >
            </v-text-field>
          </v-col>
        </v-row>
        <v-row class="my-0">
          <v-col>
            <v-range-slider min="0" max="100" step="1" v-model="resolution_pct" density="compact">
            </v-range-slider>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
</template>
