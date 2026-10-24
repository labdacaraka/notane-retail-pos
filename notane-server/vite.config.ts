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
    port: 5176,
    strictPort: true,
    proxy: {
      '/api/v1/system': {
        target: 'http://127.0.0.1:18081',
        changeOrigin: true,
      },
      '/api': {
        target: 'http://127.0.0.1:18080',
        changeOrigin: true,
      },
      '/uploads': {
        target: 'http://127.0.0.1:18080',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://127.0.0.1:18080',
        ws: true,
      },
    },
  },
  clearScreen: false,
})
