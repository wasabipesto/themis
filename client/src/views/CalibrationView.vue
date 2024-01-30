<script setup>
import { ref, onMounted } from 'vue'

const loading = ref(true)
const option_drawers_visible = ref(true)
const sidebar_data = ref([])
const options_selected = ref({
  bin_method: 'prob_at_midpoint',
  weight_attribute: 'none',
  min_open_days: 0,
  min_num_traders: 0,
  min_volume_usd: 0,
  title_contains: '',
  category_contains: ''
})
const options = ref({
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
})

onMounted(() => {
  input.value.focus()
})
</script>

<template>
  <v-navigation-drawer :width="400" v-model="option_drawers_visible">
    <v-expansion-panels>
      <v-expansion-panel>
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-ruler-square-compass</v-icon>
          X-Axis Bin Method: <br />
          {{ getOptionLabel('bin_method', options_selected.bin_method) }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            The binning method determines where on the x-axis each market is placed. This metric
            should represent the market's true belief or predicted value.
          </p>
          <v-radio-group v-model="options_selected.bin_method">
            <v-radio v-for="(v, k) in options.bin_method" :key="k" :value="k">
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
      <v-expansion-panel>
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-globe-model</v-icon>
          Y-Axis (Resolution) Weighting: <br />
          {{ getOptionLabel('weight_attribute', options_selected.weight_attribute) }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            With no weighting, the true resolution value of all markets in each bin are averaged
            evenly. Weighting gives more importance to markets meeting certain criteria.
          </p>
          <v-radio-group v-model="options_selected.weight_attribute">
            <v-radio
              v-for="(v, k) in options.weight_attribute"
              :key="k"
              :value="k"
              :label="v.label"
            ></v-radio>
          </v-radio-group>
        </v-expansion-panel-text>
      </v-expansion-panel>
      <v-expansion-panel>
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-calendar</v-icon>
          Minimum Open Length: {{ options_selected.min_open_days }} days
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            Filter the markets in the sample to only those open longer than a certain number of
            days. Useful to filter out markets that were quickly or fradulently resolved.
          </p>
          <v-slider
            v-model="options_selected.min_open_days"
            :min="options.min_open_days.range[0]"
            :max="options.min_open_days.range[1]"
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
          <v-icon class="mr-3">mdi-account-group-outline</v-icon>
          Minimum Unique Traders: {{ options_selected.min_num_traders }} traders
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            Filter the markets in the sample to only those with at least a certain number of unique
            traders. Useful to filter out personal markets with no wider interest.
          </p>
          <v-slider
            v-model="options_selected.min_num_traders"
            :min="options.min_num_traders.range[0]"
            :max="options.min_num_traders.range[1]"
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
          Minimum Market Volume: ${{ options_selected.min_volume_usd }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            Filter the markets in the sample to only those with at least a certain amount of money
            in volume. Useful to isolate only the high-profile markets. This metric is measured in
            USD.
          </p>
          <v-slider
            v-model="options_selected.min_volume_usd"
            :min="options.min_volume_usd.range[0]"
            :max="options.min_volume_usd.range[1]"
            step="10"
            class="pt-8"
            density="compact"
            thumb-label="always"
          >
          </v-slider>
        </v-expansion-panel-text>
      </v-expansion-panel>
      <v-expansion-panel>
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-message-outline</v-icon>
          Title Contains: {{ options_selected.title_contains || 'None' }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">
            Filter for markets that contain a specific term in their title. Note that different
            sites will have different naming conventions.
          </p>
          <v-text-field
            clearable
            v-model="options_selected.title_contains"
            label="Title Contains"
          ></v-text-field>
        </v-expansion-panel-text>
      </v-expansion-panel>
      <v-expansion-panel>
        <v-expansion-panel-title>
          <v-icon class="mr-3">mdi-group</v-icon>
          Category: {{ options_selected.category_contains || 'None' }}
        </v-expansion-panel-title>
        <v-expansion-panel-text>
          <p class="my-2">Filter the markets in the sample to only those in a certain category.</p>
          <v-chip-group
            v-model="options_selected.category_contains"
            selected-class="text-deep-purple-accent-4"
            filter
          >
            <v-chip v-for="cat in options.categories" :key="cat" :value="cat">
              {{ cat }}
            </v-chip>
          </v-chip-group>
        </v-expansion-panel-text>
      </v-expansion-panel>
    </v-expansion-panels>
  </v-navigation-drawer>

  <v-snackbar v-model="loading">
    <v-progress-circular indeterminate color="red"></v-progress-circular>
    <span class="mx-5">Loading data...</span>
  </v-snackbar>

  <v-main>
    <v-container>
      <v-card elevation="16">
        <v-card-text>
          <div id="graph"></div>
        </v-card-text>
      </v-card>
    </v-container>
  </v-main>
</template>
