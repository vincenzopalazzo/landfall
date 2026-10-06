/// <reference types="svelte" />
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_LANDFALL_BASE?: string;
  readonly VITE_LANDFALL_TOKEN?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}
