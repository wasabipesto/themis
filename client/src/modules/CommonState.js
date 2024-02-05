import { reactive } from 'vue'

export const state = reactive({
  left_sidebar_visible: true,
  left_sidebar_options_expanded: ['calibration_bin_method', 'calibration_weight_method'],
  show_sidebar_toggle: false,
  query_selected: {}
})
