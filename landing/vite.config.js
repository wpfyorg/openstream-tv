import { cloudflare } from '@cloudflare/vite-plugin'
import tailwindcss from '@tailwindcss/vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig({
  // GitHub Pages serves this repository from /openstream-tv, where the landing
  // page owns the root and the help centre lives under /docs. Local builds and
  // the Sites deployment stay at the domain root.
  base: process.env.LANDING_BASE || '/',
  plugins: [svelte(), tailwindcss(), cloudflare()],
  server: { port: 5173, strictPort: false },
})
