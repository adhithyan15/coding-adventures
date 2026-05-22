/**
 * render-shell.ts — main `renderPageShell` entry.
 *
 * Composes the four render helpers (sidebar, breadcrumbs, toc,
 * the head metadata) into the final `{ head, body }` chunks.
 *
 * =============================================================================
 * THE SHELL STRUCTURE
 * =============================================================================
 *
 * head:
 *   <meta charset="utf-8">
 *   <meta name="viewport" content="width=device-width, initial-scale=1">
 *   <title>{escaped page.title} | {escaped site.title}</title>
 *   [<meta name="description" content="{escaped page.description}">]
 *   [{options.headExtra raw}]
 *
 * body:
 *   <header class="site-header">
 *     <a class="brand" href="{safe site.homeUrl}">{escaped site.title}</a>
 *     [<input class="search" placeholder="..."> if options.searchPlaceholder]
 *     [<a class="github" href="{safe site.githubUrl}">GitHub</a>]
 *   </header>
 *   <div class="layout">
 *     {sidebar nav}
 *     <main>
 *       [{breadcrumbs ol}]
 *       <article>
 *         <h1>{escaped page.title}</h1>
 *         {page.body raw}
 *       </article>
 *       [{toc aside}]
 *     </main>
 *   </div>
 *   <footer class="site-footer">
 *     [<span>v{escaped site.version}</span>]
 *     [<a href="{safe page.editUrl}">Edit this page</a>]
 *     [<span>{escaped site.copyright}</span>]
 *   </footer>
 *
 * Square brackets mark conditional blocks (rendered only when
 * the underlying field is present and non-empty).
 *
 * @module render-shell
 */

import { escapeHtml, escapeAttr, safeHref } from "./escape.js";
import { renderBreadcrumbs } from "./render-breadcrumbs.js";
import { renderSidebar } from "./render-sidebar.js";
import { renderToc } from "./render-toc.js";
import type { PageShellInput, PageShellOutput } from "./types.js";

/**
 * Render a `{ head, body }` pair for a single page.
 *
 * @param input - `{ site, page, sidebar, options? }`.
 * @returns `{ head, body }` ready to wrap with `<!DOCTYPE html>
 *          <html><head>…</head><body>…</body></html>`.
 */
export function renderPageShell(input: PageShellInput): PageShellOutput {
  const options = input.options ?? {};
  return {
    head: renderHead(input),
    body: renderBody(input, options),
  };
}

// ─────────────────────────────────────────────────────────────────────
// <head> chunk
// ─────────────────────────────────────────────────────────────────────

function renderHead(input: PageShellInput): string {
  const { site, page, options } = input;
  const opts = options ?? {};
  const title = `${escapeHtml(page.title)} | ${escapeHtml(site.title)}`;
  let head =
    `<meta charset="utf-8">` +
    `<meta name="viewport" content="width=device-width, initial-scale=1">` +
    `<title>${title}</title>`;
  if (page.description !== undefined && page.description.length > 0) {
    head += `<meta name="description" content="${escapeAttr(page.description)}">`;
  }
  // headExtra is the documented escape hatch — caller's
  // responsibility to escape.  We pass it through verbatim.
  if (opts.headExtra !== undefined && opts.headExtra.length > 0) {
    head += opts.headExtra;
  }
  return head;
}

// ─────────────────────────────────────────────────────────────────────
// <body> chunk
// ─────────────────────────────────────────────────────────────────────

function renderBody(
  input: PageShellInput,
  options: NonNullable<PageShellInput["options"]>,
): string {
  return (
    renderHeader(input) +
    `<div class="layout">` +
    renderSidebar(input.sidebar, options.currentPath) +
    renderMain(input) +
    `</div>` +
    renderFooter(input)
  );
}

function renderHeader(input: PageShellInput): string {
  const { site, options } = input;
  const opts = options ?? {};
  const homeHref = safeHref(site.homeUrl ?? "/");
  let header =
    `<header class="site-header">` +
    `<a class="brand" href="${homeHref}">${escapeHtml(site.title)}</a>`;
  if (opts.searchPlaceholder !== undefined && opts.searchPlaceholder.length > 0) {
    header +=
      `<input class="search" type="search" ` +
      `placeholder="${escapeAttr(opts.searchPlaceholder)}" ` +
      `aria-label="${escapeAttr(opts.searchPlaceholder)}">`;
  }
  if (site.githubUrl !== undefined && site.githubUrl.length > 0) {
    header +=
      `<a class="github" href="${safeHref(site.githubUrl)}" ` +
      `aria-label="GitHub repository">GitHub</a>`;
  }
  header += `</header>`;
  return header;
}

function renderMain(input: PageShellInput): string {
  const { page } = input;
  const breadcrumbs = page.breadcrumbs ?? [];
  const toc = page.toc ?? [];
  // The page body is the documented TRUSTED field — passed
  // through verbatim.  The page title gets a leading <h1> so the
  // article always has a recognisable top heading even if the
  // markdown body doesn't start with one.
  return (
    `<main>` +
    renderBreadcrumbs(breadcrumbs) +
    `<article>` +
    `<h1>${escapeHtml(page.title)}</h1>` +
    page.body +
    `</article>` +
    renderToc(toc) +
    `</main>`
  );
}

function renderFooter(input: PageShellInput): string {
  const { site, page } = input;
  const parts: string[] = [];
  if (site.version !== undefined && site.version.length > 0) {
    parts.push(`<span class="version">v${escapeHtml(site.version)}</span>`);
  }
  if (page.editUrl !== undefined && page.editUrl.length > 0) {
    parts.push(`<a class="edit" href="${safeHref(page.editUrl)}">Edit this page</a>`);
  }
  if (site.copyright !== undefined && site.copyright.length > 0) {
    parts.push(`<span class="copyright">${escapeHtml(site.copyright)}</span>`);
  }
  if (parts.length === 0) {
    // Still emit the footer element so themes have somewhere to
    // hook on; just leave it empty.
    return `<footer class="site-footer"></footer>`;
  }
  return `<footer class="site-footer">${parts.join("")}</footer>`;
}
