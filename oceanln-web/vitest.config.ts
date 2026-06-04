import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";

// Vitest config (kept separate from vite.config.ts so svelte-check type-checks
// the build config cleanly). Not included in tsconfig.
// `svelteTesting()` sets the browser resolve condition so component mounting
// (client build) works under jsdom, and registers auto-cleanup.
export default defineConfig({
  plugins: [svelte({ hot: false }), svelteTesting()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./vitest.setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,js}"],
  },
});
