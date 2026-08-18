import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@imperium/ui': path.resolve(__dirname, 'packages/ui/src'),
      '@imperium/charts': path.resolve(__dirname, 'packages/charts/src'),
      '@imperium/graph': path.resolve(__dirname, 'packages/graph/src'),
      '@imperium/code': path.resolve(__dirname, 'packages/code/src'),
      '@imperium/voice': path.resolve(__dirname, 'packages/voice/src'),
      '@imperium/sdk': path.resolve(__dirname, 'packages/sdk/src'),
      '@imperium/types': path.resolve(__dirname, 'packages/types/src'),
      '@imperium/workbench': path.resolve(__dirname, 'workbench/src'),
    },
  },
  build: {
    target: 'es2022',
    minify: 'esbuild',
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          'vendor-ui': ['@radix-ui/react-dialog', '@radix-ui/react-dropdown-menu', '@radix-ui/react-tabs', '@radix-ui/react-tooltip'],
          'vendor-charts': ['recharts', 'd3-scale', 'd3-array'],
          'vendor-graph': ['reactflow', 'dagre'],
          'vendor-code': ['monaco-editor'],
          'vendor-voice': ['@ricky0123/porcupine-web', '@ricky0123/kokoro-web'],
        },
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/ws': { target: 'ws://127.0.0.1:8080', ws: true },
    },
  },
})