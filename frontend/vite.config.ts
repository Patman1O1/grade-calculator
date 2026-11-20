import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  // Served from https://patman1o1.github.io/grade-calculator/ (GitHub Pages project site).
  base: '/grade-calculator/',
  plugins: [react()],
})
