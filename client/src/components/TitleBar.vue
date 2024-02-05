<script setup>
import { ref } from 'vue'
import { state } from '@/modules/CommonState.js'
import { useDisplay } from 'vuetify'

const { mdAndUp } = useDisplay()
const desktop = mdAndUp
const right_sidebar_visible = ref(false)
</script>

<template>
  <v-app-bar>
    <template v-slot:prepend>
      <v-app-bar-nav-icon
        v-if="state.show_sidebar_toggle"
        variant="text"
        @click.stop="state.left_sidebar_visible = !state.left_sidebar_visible"
      >
      </v-app-bar-nav-icon>
    </template>

    <v-app-bar-title>
      <v-icon class="mx-1">mdi-city</v-icon>
      <span class="mx-1">Calibration City</span>
    </v-app-bar-title>

    <template v-slot:append v-if="desktop">
      <nav>
        <RouterLink to="/introduction">Introduction</RouterLink>
        <RouterLink to="/calibration">Calibration</RouterLink>
        <RouterLink to="/accuracy">Accuracy</RouterLink>
        <RouterLink to="/list">List</RouterLink>
        <RouterLink to="/faq">FAQ</RouterLink>
        <a href="https://github.com/wasabipesto/themis" target="_blank">
          <span class="d-flex align-center">
            Source
            <v-icon size="x-small" class="pl-2">mdi-open-in-new</v-icon>
          </span>
        </a>
        <a href="https://wasabipesto.com/" target="_blank">
          <span class="d-flex align-center">
            Made By
            <v-icon size="x-small" class="pl-2">mdi-open-in-new</v-icon>
          </span>
        </a>
      </nav>
    </template>

    <template v-slot:append v-if="!desktop">
      <v-btn icon="mdi-dots-vertical" @click.stop="right_sidebar_visible = !right_sidebar_visible">
      </v-btn>
    </template>
  </v-app-bar>
  <v-navigation-drawer
    :width="200"
    location="right"
    v-if="!desktop"
    v-model="right_sidebar_visible"
  >
    <nav>
      <v-list class="text-center">
        <v-list-item>
          <RouterLink to="/introduction">Introduction</RouterLink>
        </v-list-item>
        <v-list-item>
          <RouterLink to="/calibration">Calibration</RouterLink>
        </v-list-item>
        <v-list-item>
          <RouterLink to="/accuracy">Accuracy</RouterLink>
        </v-list-item>
        <v-list-item>
          <RouterLink to="/list">List</RouterLink>
        </v-list-item>
        <v-list-item>
          <RouterLink to="/faq">FAQ</RouterLink>
        </v-list-item>
        <v-list-item>
          <a href="https://github.com/wasabipesto/themis" target="_blank">
            <span class="d-flex align-center">
              Source
              <v-icon size="x-small" class="pl-2">mdi-open-in-new</v-icon>
            </span>
          </a>
        </v-list-item>
        <v-list-item>
          <a href="https://wasabipesto.com/" target="_blank">
            <span class="d-flex align-center">
              Made By
              <v-icon size="x-small" class="pl-2">mdi-open-in-new</v-icon>
            </span>
          </a>
        </v-list-item>
      </v-list>
    </nav>
  </v-navigation-drawer>
</template>

<style scoped>
nav a.router-link-exact-active {
  color: var(--color-text);
  font-weight: bold;
}
nav a.router-link-exact-active:hover {
  background-color: transparent;
}
nav a {
  display: inline-block;
  padding: 0.25rem 1rem;
  border-left: 1px solid var(--color-border);
}
nav a:first-of-type {
  border: 0;
}
</style>
