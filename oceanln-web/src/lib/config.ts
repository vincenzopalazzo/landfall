// Where the oceanln-httpd server lives and the bearer token to reach it.
// In local dev both come from Vite env (.env.local) or are entered in the UI.
// In a future Tauri shell the same-process server makes this seamless.

export const DEFAULT_BASE =
  import.meta.env.VITE_OCEANLN_BASE ?? "http://127.0.0.1:7762";

export const DEFAULT_TOKEN = import.meta.env.VITE_OCEANLN_TOKEN ?? "";
