import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import wasm from 'vite-plugin-wasm';


// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    wasm()
  ],
  build: {
    target: 'esnext'
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8080',
        changeOrigin: true
      }
    }
  }
});
