<script setup>
import { RouterView } from 'vue-router'
import TitleBar from './components/TitleBar.vue'
</script>

<template>
  <v-app>
    <TitleBar />
    <RouterView />
  </v-app>
</template>
