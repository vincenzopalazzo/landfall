import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // Static site: fully prerendered, deployable to any static host / GitHub Pages.
    adapter: adapter({
      fallback: undefined,
      strict: true
    }),
    paths: {
      // Set BASE_PATH (e.g. "/oceanln-cli") when hosting under a subpath such as GitHub Pages.
      base: process.env.BASE_PATH ?? ''
    }
  }
};

export default config;
