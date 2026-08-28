/**
 * theme.ts — the theme-agnostic HTML5 page template.
 *
 * @module theme
 */

/**
 * HTML-escape characters that have meaning in an attribute / text
 * context.  Used for `<title>` and meta tags — body content is
 * already-escaped HTML from `document-ast-to-html`.
 *
 * Deliberately narrow set: ampersand, angle brackets, both quotes.
 * That's enough for the contexts we use it in (attribute values +
 * element text content); HTML's full attribute-escaping ruleset is
 * overkill for slug-derived titles.
 */
export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * Build the full HTML5 document string.  Header + footer text is
 * optional — `siteTitle` empty means no header; we don't synthesise
 * a placeholder, the result is just a body with no chrome.
 *
 * The output ends with a trailing newline so the emitted file matches
 * common Unix conventions (`cat`, `wc -l`, editors that flag missing
 * final newlines).
 */
export interface RenderPageOptions {
  /** The page title — used in <title> and the optional header. */
  readonly title: string;
  /** Site title for the header; falsy → no header. */
  readonly siteTitle: string;
  /** Link target for the site header. Defaults to `/`. */
  readonly siteHref?: string;
  /** Trusted, already-escaped tags to append to `<head>`. */
  readonly headHtml?: string;
  /** Per-page CSS produced by the Forme AOT slicer. */
  readonly styleCss?: string;
  /** Advertise native browser light/dark form-control rendering. */
  readonly supportsDarkMode?: boolean;
  /** Already-rendered body HTML (output of document-ast-to-html). */
  readonly bodyHtml: string;
}

export function renderHtmlDocument(opts: RenderPageOptions): string {
  const titleEscaped = escapeHtml(opts.title);
  const siteTitleEscaped = opts.siteTitle ? escapeHtml(opts.siteTitle) : "";
  const siteHrefEscaped = escapeHtml(opts.siteHref ?? "/");
  const header = opts.siteTitle
    ? `<header><a href="${siteHrefEscaped}">${siteTitleEscaped}</a></header>\n`
    : "";
  const extraHead = opts.headHtml ? `${opts.headHtml}\n` : "";
  const colorScheme = opts.supportsDarkMode
    ? '<meta name="color-scheme" content="light dark">\n'
    : "";
  const style = opts.styleCss === undefined || opts.styleCss.length === 0
    ? ""
    : `<style>\n${opts.styleCss}\n</style>\n`;
  // Year is rendered without a current-time read — that lives on the
  // clock facility in StageContext, but this shell stays pure
  // (deterministic) and let the caller pass the year in via opts if
  // they want one.  An undated footer is fine.
  const footer = "";
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${titleEscaped}</title>
${extraHead}${colorScheme}${style}</head>
<body>
${header}<main>
${opts.bodyHtml}</main>
${footer}</body>
</html>
`;
}
