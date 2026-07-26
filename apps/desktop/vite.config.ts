import { fileURLToPath, URL } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// Tauri sets these when it drives Vite. `TAURI_ENV_*` is the v2 naming.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },

  // Tauri owns the terminal output; don't let Vite wipe Rust compiler errors.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    // Only set `hmr` when Tauri gave us a dev host (mobile / LAN). Spread rather
    // than `undefined` so this stays valid under exactOptionalPropertyTypes.
    ...(host ? { hmr: { protocol: 'ws', host, port: 1421 } } : {}),
    watch: {
      // Rust rebuilds are driven by cargo, not Vite.
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // Tauri v2 targets Safari 18 (macOS WKWebView) / Edge WebView2 on Windows.
    target: 'es2022',
    // Vite 8 builds on Rolldown; Oxc is the bundled minifier. ('esbuild' would
    // require installing esbuild separately.)
    minify: process.env.TAURI_ENV_DEBUG ? false : 'oxc',
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
