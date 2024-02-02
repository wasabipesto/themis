<script setup>
import { ref } from 'vue'
import { state } from '@/modules/CommonState.js'

state.query_selected = {
  title_contains: null,
  platform_select: null,
  category_select: null,
  open_ts_min: null,
  open_ts_max: null,
  close_ts_min: null,
  close_ts_max: null,
  open_days_min: null,
  open_days_max: null,
  num_traders_min: null,
  num_traders_max: null,
  volume_usd_min: null,
  volume_usd_max: null
}

const query_options = {
  bin_method: {
    prob_at_midpoint: { label: 'Probability at Market Midpoint' },
    prob_at_close: { label: 'Probability at Market Close' },
    prob_time_weighted: {
      label: 'Market Time-Averaged Probability',
      tooltip:
        'For each market, this is the probability averaged over time. <br>\
          Each market is only counted once.'
    }
  },
  weight_attribute: {
    none: { label: 'None' },
    volume_usd: { label: 'Market Volume' },
    open_days: { label: 'Market Length' },
    num_traders: { label: 'Number of Traders' }
  },
  platforms: ['Kalshi', 'Manifold', 'Metaculus', 'Polymarket'],
  open_days_min: {
    range: [0, 90]
  },
  num_traders_min: {
    range: [0, 50]
  },
  volume_usd_min: {
    range: [0, 500]
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
  ]
}
</script>

<template>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-message-outline</v-icon>
      Title Contains: {{ state.query_selected.title_contains || 'None' }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter for markets that contain a specific term in their title. Note that different sites
        will have different naming conventions.
      </p>
      <v-text-field
        clearable
        v-model="state.query_selected.title_contains"
        label="Title Contains"
      ></v-text-field>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-map-marker</v-icon>
      Platform: {{ state.query_selected.platform_select || 'None' }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">Filter the markets in the sample to only those from a certain site.</p>
      <v-chip-group
        v-model="state.query_selected.platform_select"
        selected-class="text-deep-purple-accent-4"
        column
        filter
      >
        <v-chip v-for="item in query_options.platforms" :key="item" :value="item">
          {{ item }}
        </v-chip>
      </v-chip-group>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-group</v-icon>
      Category: {{ state.query_selected.category_select || 'None' }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">Filter the markets in the sample to only those in a certain category.</p>
      <v-chip-group
        v-model="state.query_selected.category_select"
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
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-account-group-outline</v-icon>
      Minimum Unique Traders: {{ state.query_selected.num_traders_min || 0 }} traders
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with at least a certain number of unique
        traders. Useful to filter out personal markets with no wider interest.
      </p>
      <v-slider
        v-model="state.query_selected.num_traders_min"
        :min="query_options.num_traders_min.range[0]"
        :max="query_options.num_traders_min.range[1]"
        step="1"
        class="pt-8"
        density="compact"
        thumb-label="always"
      >
      </v-slider>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar</v-icon>
      Minimum Open Length: {{ state.query_selected.open_days_min || 0 }} days
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those open longer than a certain number of days.
        Useful to filter out markets that were quickly or fradulently resolved.
      </p>
      <v-slider
        v-model="state.query_selected.open_days_min"
        :min="query_options.open_days_min.range[0]"
        :max="query_options.open_days_min.range[1]"
        step="1"
        class="pt-8"
        density="compact"
        thumb-label="always"
      >
      </v-slider>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-cash</v-icon>
      Minimum Market Volume: ${{ state.query_selected.volume_usd_min || 0 }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with at least a certain amount of money in
        volume. Useful to isolate only the high-profile markets. This metric is measured in USD.
      </p>
      <v-slider
        v-model="state.query_selected.volume_usd_min"
        :min="query_options.volume_usd_min.range[0]"
        :max="query_options.volume_usd_min.range[1]"
        step="10"
        class="pt-8"
        density="compact"
        thumb-label="always"
      >
      </v-slider>
    </v-expansion-panel-text>
  </v-expansion-panel>
</template>
