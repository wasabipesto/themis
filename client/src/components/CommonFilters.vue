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
  num_traders_min: null,
  num_traders_max: null,
  open_days_min: null,
  open_days_max: null,
  volume_usd_min: null,
  volume_usd_max: null
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
  ]
}
</script>

<template>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-message-outline</v-icon>
      Title Contains
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
      Platform
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">Filter the markets in the sample to only those from a certain site.</p>
      <v-chip-group
        v-model="state.query_selected.platform_select"
        selected-class="text-deep-purple-accent-4"
        column
        filter
      >
        <v-chip v-for="(v, k) in query_options.platforms" :key="k" :value="k" :text="v.label">
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
      Unique Traders
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with at least a certain number of unique
        traders. Useful to filter out personal markets with no wider interest.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="state.query_selected.num_traders_min"
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
              v-model="state.query_selected.num_traders_max"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar</v-icon>
      Open Length
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those open longer than a certain number of days.
        Useful to filter out markets that were quickly or fradulently resolved.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="state.query_selected.open_days_min"
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
              v-model="state.query_selected.open_days_max"
              type="number"
              density="compact"
              hide-details
              variant="outlined"
              clearable
            >
            </v-text-field>
          </v-col>
        </v-row>
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-cash</v-icon>
      Market Volume
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with at least a certain amount of money in
        volume. Useful to isolate only the high-profile markets. This metric is measured in USD.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="state.query_selected.volume_usd_min"
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
              v-model="state.query_selected.volume_usd_max"
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
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
</template>
