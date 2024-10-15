import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import wasmPack from 'vite-plugin-wasm-pack'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    wasmPack('./blinkybot-ui-wasm'),
  ],
  optimizeDeps: {
    exclude: ['@rollup/browser'],
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  }
})
