// Where the landfall-httpd server lives and the bearer token to reach it.
// In local dev both come from Vite env (.env.local) or are entered in the UI.
// In a future Tauri shell the same-process server makes this seamless.

export const DEFAULT_BASE =
  import.meta.env.VITE_LANDFALL_BASE ?? "http://127.0.0.1:7762";

// DEV ONLY. `import.meta.env.VITE_LANDFALL_TOKEN` is inlined at build time, so a
// production `npm run build` with it set would embed the bearer token in the
// static bundle (readable by anyone with the assets). Use it only for local
// dev; in production leave it unset and enter the token via the in-app settings
// panel (or rely on a same-process Tauri host later).
export const DEFAULT_TOKEN = import.meta.env.VITE_LANDFALL_TOKEN ?? "";
