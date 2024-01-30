<script setup>
import NavLinks from './NavLinks.vue'
</script>

<template>
  <v-app-bar>
    <template v-slot:prepend>
      <v-app-bar-nav-icon></v-app-bar-nav-icon>
    </template>

    <v-app-bar-title>
      <v-icon class="mx-1">mdi-city</v-icon>
      <span class="mx-1">Calibration City</span>
    </v-app-bar-title>

    <template v-slot:append>
      <NavLinks />
    </template>
  </v-app-bar>
</template>
