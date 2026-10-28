import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import pkg from './package.json'
import { codeHardeningPlugin } from '../scripts/obfuscator-plugin.js'

const appVersion = process.env.VITE_APP_VERSION || process.env.APP_VERSION || pkg.version || '1.0.0'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), codeHardeningPlugin()],
  define: {
    '__APP_VERSION__': JSON.stringify(appVersion),
    'import.meta.env.VITE_APP_VERSION': JSON.stringify(appVersion),
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5175,
    strictPort: true,
    proxy: {
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
