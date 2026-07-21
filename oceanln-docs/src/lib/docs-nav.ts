export type DocLink = { slug: string; title: string; summary: string };
export type DocSection = { title: string; links: DocLink[] };

/**
 * Documentation table of contents. `slug` is the route under /docs
 * ('' is the index). Drives the sidebar, the index cards, and prev/next.
 */
export const docsNav: DocSection[] = [
  {
    title: 'Overview',
    links: [
      {
        slug: '',
        title: 'Introduction',
        summary: 'What OCEAN Lightning is, the workspace layout, and how the pieces fit together.'
      },
      {
        slug: 'getting-started',
        title: 'Getting started',
        summary: 'Install the tooling, build the workspace, and run your first end-to-end payout.'
      }
    ]
  },
  {
    title: 'Guides',
    links: [
      {
        slug: 'cli',
        title: 'CLI reference',
        summary: 'The oceanln command: generate, init, offer, payout, and seed resolution.'
      },
      {
        slug: 'http-api',
        title: 'HTTP API',
        summary: 'The oceanln-httpd loopback server: endpoints, auth, and its security model.'
      }
    ]
  },
  {
    title: 'Frontends',
    links: [
      {
        slug: 'wizard',
        title: 'Web wizard',
        summary: 'The Svelte onboarding wizard, live payout dashboard, and MCP panel.'
      },
      {
        slug: 'desktop',
        title: 'Desktop app',
        summary: 'The Tauri desktop shell, Linux packages, and the Lexe sidecar.'
      }
    ]
  }
];

export const docsFlat: DocLink[] = docsNav.flatMap((s) => s.links);

export function neighbours(slug: string): { prev: DocLink | null; next: DocLink | null } {
  const i = docsFlat.findIndex((l) => l.slug === slug);
  if (i === -1) return { prev: null, next: null };
  return {
    prev: i > 0 ? docsFlat[i - 1] : null,
    next: i < docsFlat.length - 1 ? docsFlat[i + 1] : null
  };
}
