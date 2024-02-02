<script setup>
import { state } from '@/modules/CommonState.js'
</script>

<template>
  <v-app-bar>
    <template v-slot:prepend>
      <v-app-bar-nav-icon
        variant="text"
        @click.stop="state.left_sidebar_visible = !state.left_sidebar_visible"
      ></v-app-bar-nav-icon>
    </template>

    <v-app-bar-title>
      <v-icon class="mx-1">mdi-city</v-icon>
      <span class="mx-1">Calibration City</span>
    </v-app-bar-title>

    <template v-slot:append>
      <nav>
        <RouterLink to="/introduction">Introduction</RouterLink>
        <RouterLink to="/calibration">Calibration</RouterLink>
        <RouterLink to="/accuracy">Accuracy</RouterLink>
        <RouterLink to="/list">List</RouterLink>
        <RouterLink to="/faq">FAQ</RouterLink>
        <a href="https://github.com/wasabipesto/themis" target="_blank">
          Source
          <v-icon size="x-small">mdi-open-in-new</v-icon>
        </a>
        <a href="https://wasabipesto.com/" target="_blank">
          Made By
          <v-icon size="x-small">mdi-open-in-new</v-icon>
        </a>
      </nav>
    </template>
  </v-app-bar>
</template>

<style scoped>
nav a.router-link-exact-active {
  color: var(--color-text);
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
