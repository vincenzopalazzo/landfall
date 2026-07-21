<script lang="ts">
  import { base } from '$app/paths';
  import { page } from '$app/stores';
  import Nav from '$lib/components/Nav.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import { docsNav, neighbours } from '$lib/docs-nav';

  let { children } = $props();

  const docsBase = `${base}/docs`;

  // Current slug relative to /docs ('' == index).
  let slug = $derived.by(() => {
    const path = $page.url.pathname.replace(/\/$/, '');
    const rest = path.startsWith(docsBase) ? path.slice(docsBase.length) : '';
    return rest.replace(/^\//, '');
  });

  let nav = $derived(neighbours(slug));

  function hrefFor(s: string): string {
    return s === '' ? docsBase : `${docsBase}/${s}`;
  }
</script>

<Nav
  links={[
    { href: `${base}/#payouts`, label: 'Why Lightning' },
    { href: `${base}/#how`, label: 'How it works' },
    { href: docsBase, label: 'Docs' }
  ]}
/>

<div class="docs-shell wrap">
  <aside class="sidebar" aria-label="Documentation">
    <nav>
      {#each docsNav as section}
        <div class="side-group">
          <h4>{section.title}</h4>
          <ul>
            {#each section.links as link}
              <li>
                <a href={hrefFor(link.slug)} class:active={slug === link.slug} aria-current={slug === link.slug ? 'page' : undefined}>
                  {link.title}
                </a>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </nav>
  </aside>

  <main class="content">
    <article class="doc">
      {@render children()}
    </article>

    {#if nav.prev || nav.next}
      <nav class="prevnext" aria-label="Pagination">
        {#if nav.prev}
          <a class="pn prev" href={hrefFor(nav.prev.slug)}>
            <span class="dir">&#8592; Previous</span>
            <span class="lbl">{nav.prev.title}</span>
          </a>
        {:else}
          <span></span>
        {/if}
        {#if nav.next}
          <a class="pn next" href={hrefFor(nav.next.slug)}>
            <span class="dir">Next &#8594;</span>
            <span class="lbl">{nav.next.title}</span>
          </a>
        {/if}
      </nav>
    {/if}
  </main>
</div>

<Footer />

<style>
  .docs-shell {
    display: grid;
    grid-template-columns: 248px minmax(0, 1fr);
    gap: 56px;
    align-items: start;
    padding-top: 48px;
    padding-bottom: 96px;
  }

  /* ── Sidebar ── */
  .sidebar {
    position: sticky;
    top: 96px;
    align-self: start;
  }
  .side-group {
    margin-bottom: 28px;
  }
  .side-group h4 {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--fg-tertiary);
    margin: 0 0 12px;
  }
  .side-group ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .side-group li {
    margin: 0;
  }
  .side-group a {
    display: block;
    padding: 7px 12px;
    margin: 1px 0;
    border-radius: var(--radius-md);
    color: var(--fg-secondary);
    font-size: 14.5px;
    font-weight: 500;
    border-left: 2px solid transparent;
  }
  .side-group a:hover {
    background: var(--bg-secondary);
    color: var(--fg-primary);
  }
  .side-group a.active {
    color: var(--ocean-blue);
    background: var(--ocean-blue-subtle);
    font-weight: 600;
  }

  /* ── Content column ── */
  .content {
    min-width: 0;
    max-width: 780px;
  }

  /* ── Prev / next ── */
  .prevnext {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    margin-top: 56px;
    padding-top: 28px;
    border-top: 1px solid var(--border);
  }
  .pn {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    min-width: 200px;
    transition:
      border-color var(--dur-base),
      transform var(--dur-base) var(--ease-out);
  }
  .pn:hover {
    border-color: var(--ocean-blue-light);
    transform: translateY(-1px);
  }
  .pn.next {
    text-align: right;
    margin-left: auto;
  }
  .pn .dir {
    font-size: 12px;
    color: var(--fg-tertiary);
    font-weight: 500;
  }
  .pn .lbl {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 600;
    color: var(--fg-primary);
    letter-spacing: -0.2px;
  }

  /* ── Prose (applies to page content rendered into .doc) ── */
  .doc :global(> *:first-child) {
    margin-top: 0;
  }
  .doc :global(.eyebrow) {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 3px;
    text-transform: uppercase;
    color: var(--ocean-blue);
    margin: 0 0 14px;
  }
  .doc :global(h1) {
    font-family: var(--font-display);
    font-size: 42px;
    font-weight: 800;
    letter-spacing: -1.4px;
    line-height: 1.05;
    margin: 0 0 20px;
  }
  .doc :global(h2) {
    font-family: var(--font-display);
    font-size: 27px;
    font-weight: 700;
    letter-spacing: -0.6px;
    line-height: 1.2;
    margin: 52px 0 16px;
    padding-top: 8px;
  }
  .doc :global(h3) {
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 700;
    letter-spacing: -0.3px;
    margin: 34px 0 12px;
  }
  .doc :global(p),
  .doc :global(li) {
    font-size: 16px;
    line-height: 1.7;
    color: var(--fg-secondary);
  }
  .doc :global(.lead) {
    font-size: 19px;
    line-height: 1.6;
    color: var(--fg-secondary);
    margin: 0 0 28px;
  }
  .doc :global(ul),
  .doc :global(ol) {
    padding-left: 24px;
    margin: 16px 0;
  }
  .doc :global(li) {
    margin: 8px 0;
  }
  .doc :global(li)::marker {
    color: var(--fg-muted);
  }
  .doc :global(a) {
    color: var(--ocean-blue);
    font-weight: 500;
    text-decoration: underline;
    text-underline-offset: 2px;
    text-decoration-color: var(--ocean-blue-lighter);
  }
  .doc :global(a:hover) {
    text-decoration-color: var(--ocean-blue);
  }
  .doc :global(strong) {
    color: var(--fg-primary);
    font-weight: 600;
  }
  .doc :global(code) {
    font-family: var(--font-mono);
    font-size: 13.5px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1px 6px;
    color: var(--fg-primary);
  }
  .doc :global(pre) {
    background: #0b0b0d;
    border: 1px solid #1e1e23;
    border-radius: var(--radius-lg);
    padding: 20px 22px;
    overflow-x: auto;
    margin: 20px 0;
    line-height: 1.6;
  }
  .doc :global(pre code) {
    background: none;
    border: none;
    padding: 0;
    color: #e4e4e7;
    font-size: 13.5px;
  }
  .doc :global(blockquote) {
    margin: 20px 0;
    padding: 4px 20px;
    border-left: 3px solid var(--ocean-blue-light);
    background: var(--bg-secondary);
    border-radius: 0 var(--radius-md) var(--radius-md) 0;
  }
  .doc :global(blockquote p) {
    color: var(--fg-secondary);
  }
  .doc :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 20px 0;
    font-size: 14.5px;
    display: block;
    overflow-x: auto;
  }
  .doc :global(th),
  .doc :global(td) {
    text-align: left;
    padding: 11px 14px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }
  .doc :global(th) {
    font-weight: 600;
    color: var(--fg-primary);
    background: var(--bg-secondary);
    white-space: nowrap;
  }
  .doc :global(td) {
    color: var(--fg-secondary);
  }
  .doc :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 44px 0;
  }

  /* ── Callout (use <div class="callout"> in pages) ── */
  .doc :global(.callout) {
    display: flex;
    gap: 14px;
    padding: 18px 20px;
    margin: 24px 0;
    border-radius: var(--radius-lg);
    background: var(--ocean-blue-subtle);
    border: 1px solid var(--ocean-blue-lighter);
  }
  .doc :global(.callout .material-symbols-sharp) {
    color: var(--ocean-blue);
    font-size: 22px;
    flex-shrink: 0;
  }
  .doc :global(.callout p) {
    margin: 0;
    color: var(--ocean-navy-mid);
    font-size: 15px;
  }
  .doc :global(.callout.warn) {
    background: rgba(255, 179, 0, 0.1);
    border-color: rgba(255, 179, 0, 0.4);
  }
  .doc :global(.callout.warn .material-symbols-sharp) {
    color: var(--warning);
  }
  .doc :global(.callout.warn p) {
    color: var(--gray-700);
  }

  @media (max-width: 940px) {
    .docs-shell {
      grid-template-columns: 1fr;
      gap: 24px;
    }
    .sidebar {
      position: static;
      top: auto;
      border-bottom: 1px solid var(--border);
      padding-bottom: 8px;
    }
    .side-group {
      margin-bottom: 16px;
    }
  }
  @media (max-width: 560px) {
    .prevnext {
      flex-direction: column;
    }
    .pn.next {
      margin-left: 0;
    }
  }
</style>
