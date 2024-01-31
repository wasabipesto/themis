<script setup>
import { RouterLink } from 'vue-router'
</script>

<template>
  <nav>
    <RouterLink to="/calibration">Calibration</RouterLink>
    <RouterLink to="/accuracy">Accuracy</RouterLink>
    <RouterLink to="/list">List</RouterLink>
    <RouterLink to="/about">About</RouterLink>
    <a href="https://github.com/wasabipesto/themis" target="_blank">
      Github
      <!--<v-icon size="x-small">mdi-open-in-new</v-icon>-->
    </a>
  </nav>
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
