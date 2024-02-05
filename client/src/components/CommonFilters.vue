<script setup>
import { toRefs } from 'vue'
import { state } from '@/modules/CommonState.js'

let { query_selected, show_sidebar_toggle } = toRefs(state)
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
  ]
}

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

function get_numeric_label(min, max, prefix, suffix) {
  let less_word
  if (suffix == '') {
    less_word = 'less'
  } else {
    less_word = 'fewer'
  }
  if (min == null) {
    if (max == null) {
      return 'Any'
    } else {
      return prefix + max + ' or ' + less_word + ' ' + suffix
    }
  } else {
    if (max == null) {
      return prefix + min + ' or more ' + suffix
    } else {
      return prefix + min + ' to ' + max + ' ' + suffix
    }
  }
}
</script>

<template>
  <v-expansion-panel>
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
  <v-expansion-panel>
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
  <v-expansion-panel>
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
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-account-group-outline</v-icon>
      Unique Traders:
      {{
        get_numeric_label(
          query_selected.num_traders_min,
          query_selected.num_traders_max,
          '',
          'traders'
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
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar</v-icon>
      Open Length:
      {{
        get_numeric_label(query_selected.open_days_min, query_selected.open_days_max, '', 'days')
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
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
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
      </v-container>
    </v-expansion-panel-text>
  </v-expansion-panel>
  <v-expansion-panel>
    <v-expansion-panel-title>
      <v-icon class="mr-3">mdi-calendar-import</v-icon>
      Midpoint Probability:
      {{
        get_numeric_label(
          query_selected.prob_at_midpoint_min,
          query_selected.prob_at_midpoint_max,
          '',
          ''
        )
      }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those with a midpoint probability in a certain
        range. Must be a decimal between 0 and 1.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="query_selected.prob_at_midpoint_min"
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
              v-model="query_selected.prob_at_midpoint_max"
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
      <v-icon class="mr-3">mdi-calendar-end</v-icon>
      Probability at Close:
      {{
        get_numeric_label(
          query_selected.prob_at_close_min,
          query_selected.prob_at_close_max,
          '',
          ''
        )
      }}
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
              v-model="query_selected.prob_at_close_min"
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
              v-model="query_selected.prob_at_close_max"
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
      <v-icon class="mr-3">mdi-calendar-start</v-icon>
      Time-Averaged Probability:
      {{
        get_numeric_label(
          query_selected.prob_time_avg_min,
          query_selected.prob_time_avg_max,
          '',
          ''
        )
      }}
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
              v-model="query_selected.prob_time_avg_min"
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
              v-model="query_selected.prob_time_avg_max"
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
      <v-icon class="mr-3">mdi-seal</v-icon>
      Resolution Probability:
      {{ get_numeric_label(query_selected.resolution_min, query_selected.resolution_max, '', '') }}
    </v-expansion-panel-title>
    <v-expansion-panel-text>
      <p class="my-2">
        Filter the markets in the sample to only those that resolved with a certain value. Useful
        for isolating markets that resolves YES (1) or NO (0). Must be a decimal between 0 and 1.
      </p>
      <v-container>
        <v-row>
          <v-col>
            <v-text-field
              label="Minimum"
              v-model="query_selected.resolution_min"
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
              v-model="query_selected.resolution_max"
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
</template>
