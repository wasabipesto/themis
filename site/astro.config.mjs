// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import icon from "astro-icon";

// https://astro.build/config
export default defineConfig({
  site: "https://predictionmetrics.org",
  vite: {
    plugins: [tailwindcss()],
  },
  experimental: {
    svg: true,
  },
  integrations: [icon()],
});
