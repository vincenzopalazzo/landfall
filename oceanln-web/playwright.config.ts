import { defineConfig } from "@playwright/test";

// Headless browser QA of the wizard (scripts/qa/web-e2e.sh drives this).
// Specs are `e2e/*.e2e.ts` so vitest's `src/**/*.{test,spec}.ts` glob never
// picks them up. The script starts `oceanln-httpd --mock-wallet` and
// `vite preview` itself and passes their addresses through the environment:
//   OCEANLN_QA_WEB       the preview origin (default http://localhost:4173)
//   OCEANLN_QA_OUT       where the create spec writes the signed artefacts
//   OCEANLN_QA_CHROMIUM  optional executable path (a pre-installed Chromium)
export default defineConfig({
  testDir: "e2e",
  testMatch: /.*\.e2e\.ts/,
  workers: 1,
  retries: 0,
  timeout: 60_000,
  reporter: process.env.CI ? [["list"], ["github"]] : "list",
  use: {
    baseURL: process.env.OCEANLN_QA_WEB ?? "http://localhost:4173",
    trace: "retain-on-failure",
    launchOptions: process.env.OCEANLN_QA_CHROMIUM
      ? { executablePath: process.env.OCEANLN_QA_CHROMIUM }
      : {},
  },
});
