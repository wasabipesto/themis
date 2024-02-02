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

function get_title_contains_label() {
  const title_contains = state.query_selected.title_contains
  if (title_contains == '' || title_contains == null) {
    return 'Any'
  } else {
    return title_contains
  }
}

function get_platform_label() {
  const platform = state.query_selected.platform_select
  console.log(query_options)
  if (platform in query_options.platforms) {
    return query_options['platforms'][platform]['label']
  } else {
    return 'Any'
  }
}

function get_numeric_label(min, max) {
  if (min == null) {
    if (max == null) {
      return 'Any'
    } else {
      return max + ' or less'
    }
  } else {
    if (max == null) {
      return min + ' or more'
    } else {
      return min + ' to ' + max
    }
  }
}
</script>

<template>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-message-outline</v-icon>
      Title Contains: {{ get_title_contains_label() }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter for markets that contain a specific term in their title. Note that different sites
        will have different naming conventions.
      </p>
      <v-text-field
        clearable
        v-model="state.query_selected.title_contains"
        prepend-inner-icon="mdi-magnify"
        label="Title Contains"
      ></v-text-field>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-map-marker</v-icon>
      Platform: {{ get_platform_label() }}
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
      Category: {{ state.query_selected.category_select || 'Any' }}
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
      Unique Traders:
      {{
        get_numeric_label(
          state.query_selected.num_traders_min,
          state.query_selected.num_traders_max
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
      Open Length:
      {{
        get_numeric_label(state.query_selected.open_days_min, state.query_selected.open_days_max)
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
      Market Volume:
      {{
        get_numeric_label(state.query_selected.volume_usd_min, state.query_selected.volume_usd_max)
      }}
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
