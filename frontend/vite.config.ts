import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  // The Rust server serves the build from "/". Set VITE_BASE when hosting the
  // static files under a sub-path (e.g. "/grade-calculator/" on GitHub Pages).
  base: process.env.VITE_BASE ?? '/',
  plugins: [react()],
  server: {
    // `npm run dev` forwards API calls to the Rust server (`cargo run` in backend/).
    proxy: {
      '/api': process.env.API_PROXY_TARGET ?? 'http://localhost:3000',
    },
  },
})
