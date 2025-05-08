// @ts-check
import { defineConfig } from "astro/config";
import alpinejs from "@astrojs/alpinejs";
import icon from "astro-icon";
import tailwindcss from "@tailwindcss/vite";

// https://astro.build/config
export default defineConfig({
  site: "https://predictionmetrics.org",
  vite: {
    plugins: [tailwindcss()],
  },
  experimental: {},
  integrations: [alpinejs(), icon()],
});
