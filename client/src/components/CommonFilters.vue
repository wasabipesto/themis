<script setup>
import { ref } from 'vue'
import { state } from '@/modules/CommonState.js'

state.query_selected = {
  min_open_days: 0,
  min_num_traders: 0,
  min_volume_usd: 0,
  title_contains: '',
  category_select: ''
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
  min_open_days: {
    range: [0, 90]
  },
  min_num_traders: {
    range: [0, 50]
  },
  min_volume_usd: {
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
      <v-icon class="mr-3">mdi-calendar</v-icon>
      Minimum Open Length: {{ state.query_selected.min_open_days }} days
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those open longer than a certain number of days.
        Useful to filter out markets that were quickly or fradulently resolved.
      </p>
      <v-slider
        v-model="state.query_selected.min_open_days"
        :min="query_options.min_open_days.range[0]"
        :max="query_options.min_open_days.range[1]"
        step="1"
        class="pt-8"
        density="compact"
        thumb-label="always"
      >
      </v-slider>
    </v-expansion-panel-text>
  </v-expansion-panel>
</template>
