import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// SPA dev server on :5173 (the origin oceanln-httpd must allow via --allow-origin).
// Builds to dist/ as static assets — ready for a later Tauri shell to bundle.
// Test config lives in vitest.config.ts.
export default defineConfig({
  plugins: [svelte()],
  server: { port: 5173, strictPort: true },
});
