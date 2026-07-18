import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// Configuração Vite integrada ao Tauri.
// Docs: https://v2.tauri.app/start/frontend/vite/
const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],

  // Evita que o Vite limpe a tela e esconda os logs do Rust/Tauri.
  clearScreen: false,

  server: {
    // Porta fixa: o Tauri (devUrl) espera exatamente esta porta.
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 1421 }
      : undefined,
    watch: {
      // Não observar o backend Rust — o Cargo cuida disso.
      ignored: ['**/src-tauri/**'],
    },
  },

  // Expõe variáveis de ambiente prefixadas com TAURI_ ao frontend.
  envPrefix: ['VITE_', 'TAURI_'],
})
