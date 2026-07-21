# oceanln-docs

The OCEAN Lightning documentation website — a static SvelteKit site built on the
OCEAN design system. Its home page is the **Early Access** landing; `/docs`
hosts the CLI, HTTP API, web wizard, and desktop-app documentation.

## Stack

- [SvelteKit](https://svelte.dev/docs/kit) (Svelte 5, runes) + TypeScript
- `@sveltejs/adapter-static` — fully prerendered, deployable to any static host
- OCEAN design tokens (`static/colors_and_type.css`), Inter + Geist Mono fonts

## Develop

```sh
cd oceanln-docs
npm install
npm run dev      # http://localhost:5173
```

## Check & build

```sh
npm run check    # svelte-check (type + a11y)
npm run build    # prerender to build/
npm run preview  # serve the built site
```

## Hosting under a subpath

Set `BASE_PATH` at build time when serving from a subpath (e.g. GitHub Pages):

```sh
BASE_PATH=/oceanln-cli npm run build
```

## Layout

```
src/
  routes/
    +page.svelte            # Early Access landing
    docs/                   # documentation section (sidebar shell + pages)
  lib/
    components/             # Nav, Footer, WaitlistForm, NodeWalletPreview
    docs-nav.ts             # docs table of contents
    waitlist.svelte.ts      # client-side early-access waitlist store
static/
  colors_and_type.css       # OCEAN design tokens
  fonts/  assets/           # Inter/Geist fonts, OCEAN logos
```

The waitlist currently persists to `localStorage` (mirrors the design
prototype). Wiring it to a real backend is a follow-up: swap `waitlist.add()`
for a POST and keep the same public surface.
