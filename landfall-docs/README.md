# landfall-docs

The OCEAN Lightning documentation website — a static SvelteKit site built on the
OCEAN design system. Its home page is the **Early Access** landing; `/docs`
hosts the CLI, HTTP API, web wizard, and desktop-app documentation.

## Stack

- [SvelteKit](https://svelte.dev/docs/kit) (Svelte 5, runes) + TypeScript
- `@sveltejs/adapter-static` — fully prerendered, deployable to any static host
- OCEAN design tokens (`static/colors_and_type.css`), Inter + Geist Mono fonts

## Develop

```sh
cd landfall-docs
npm install
npm run dev      # http://localhost:5173
```

## Check & build

```sh
npm run check    # svelte-check (type + a11y)
npm run build    # prerender to build/
npm run preview  # serve the built site
```

## CI and hosting

`.github/workflows/docs.yml` type-checks and builds this site on every push or
pull request that touches `landfall-docs/**`. It does not deploy anywhere: the
site is not published while Landfall is a showcase. `build/` is fully static,
so any static host (Cloudflare Pages, GitHub Pages, Netlify) can serve it.

### Hosting under a subpath

If you ever host from a subpath instead (e.g. GitHub Pages under `/<repo>`),
build with `BASE_PATH`:

```sh
BASE_PATH=/<repo-name> npm run build
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
