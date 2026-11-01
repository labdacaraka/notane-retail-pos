import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5177,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:19090',
        changeOrigin: true,
      },
    },
  },
  clearScreen: false,
})
