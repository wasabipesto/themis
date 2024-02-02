<script setup>
import { RouterView } from 'vue-router'
import TitleBar from './components/TitleBar.vue'
</script>

<template>
  <v-layout class="rounded rounded-md">
    <TitleBar />
    <v-container>
      <RouterView />
    </v-container>
  </v-layout>
</template>
