/// <reference types="svelte" />
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_OCEANLN_BASE?: string;
  readonly VITE_OCEANLN_TOKEN?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}
