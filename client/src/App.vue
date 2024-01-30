<script setup>
import { RouterView } from 'vue-router'
import TitleBar from './components/TitleBar.vue'
import SiteFooter from './components/SiteFooter.vue'
</script>

<template>
  <v-layout class="rounded rounded-md">
    <TitleBar />
    <v-container>
      <RouterView />
      <SiteFooter />
    </v-container>
  </v-layout>
</template>
