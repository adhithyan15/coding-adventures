/**
 * theme.ts — the v0 classless HTML5 page template.
 *
 * Hard-coded so the v0 blog has zero theme-configuration surface area.
 * When FM04 (Style IR) lands, this whole file gets replaced by a stage
 * that consumes a `StyleDocument` and emits the same shape with the
 * compiled CSS injected.  Until then: one font stack, one max-width
 * container, defensible defaults for headings / code / blockquote.
 *
 * Design choices:
 *   - **System font stack** — no web-font request, fast first paint.
 *   - **Single-column readable measure** — `max-width: 38rem` lands
 *     in the 60–75 character sweet spot at the default 16px / 100%
 *     base font-size.
 *   - **Classless** — selectors target tag names only.  Authors write
 *     plain Markdown; the renderer emits plain HTML; the CSS styles
 *     plain HTML.  No `class="prose"` magic; the source survives a
 *     view-source check unchanged.
 *   - **Light-mode-only for v0.**  Dark mode adds a `prefers-color-
 *     scheme` block — out of scope here, the v0 goal is "is the
 *     pipeline working", not "is the design beautiful".
 *
 * @module theme
 */

/**
 * The CSS injected verbatim inside `<style>` in the page <head>.
 * Kept as one string (rather than a CSS-in-JS soup) because the v0
 * theme is also the *only* theme — there's nothing to compose.
 */
export const CLASSLESS_CSS = `:root { color-scheme: light; }
* { box-sizing: border-box; }
html { font-size: 100%; -webkit-text-size-adjust: 100%; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
               "Helvetica Neue", Arial, sans-serif;
  color: #1f2328;
  background: #ffffff;
  line-height: 1.6;
  margin: 0;
  padding: 2rem 1rem 4rem;
  display: flex;
  flex-direction: column;
  align-items: center;
}
main {
  width: 100%;
  max-width: 38rem;
}
header, footer {
  width: 100%;
  max-width: 38rem;
  color: #57606a;
  font-size: 0.9rem;
}
header { margin-bottom: 2rem; }
footer { margin-top: 4rem; border-top: 1px solid #d0d7de; padding-top: 1rem; }
header a, footer a { color: inherit; }
h1, h2, h3, h4, h5, h6 {
  line-height: 1.25;
  margin: 2rem 0 1rem;
  font-weight: 600;
}
h1 { font-size: 2rem; margin-top: 0; }
h2 { font-size: 1.5rem; border-bottom: 1px solid #d0d7de; padding-bottom: 0.3rem; }
h3 { font-size: 1.25rem; }
h4 { font-size: 1rem; }
p { margin: 0 0 1rem; }
a { color: #0969da; text-decoration: underline; text-underline-offset: 2px; }
a:hover { text-decoration-thickness: 2px; }
strong { font-weight: 600; }
em { font-style: italic; }
code {
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas,
               "Liberation Mono", monospace;
  font-size: 0.9em;
  padding: 0.15em 0.35em;
  background: #f6f8fa;
  border-radius: 4px;
}
pre {
  background: #f6f8fa;
  padding: 1rem;
  border-radius: 6px;
  overflow-x: auto;
  font-size: 0.9rem;
  line-height: 1.5;
}
pre code {
  background: transparent;
  padding: 0;
  border-radius: 0;
  font-size: inherit;
}
blockquote {
  margin: 1rem 0;
  padding: 0.25rem 1rem;
  color: #57606a;
  border-left: 4px solid #d0d7de;
}
ul, ol { padding-left: 1.5rem; margin: 0 0 1rem; }
li { margin: 0.25rem 0; }
hr { border: 0; border-top: 1px solid #d0d7de; margin: 2rem 0; }
img { max-width: 100%; height: auto; }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
th, td { border: 1px solid #d0d7de; padding: 0.4rem 0.7rem; text-align: left; }
th { background: #f6f8fa; }
`;

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
  /** Already-rendered body HTML (output of document-ast-to-html). */
  readonly bodyHtml: string;
}

export function renderHtmlDocument(opts: RenderPageOptions): string {
  const titleEscaped = escapeHtml(opts.title);
  const siteTitleEscaped = opts.siteTitle ? escapeHtml(opts.siteTitle) : "";
  const header = opts.siteTitle
    ? `<header><a href="/">${siteTitleEscaped}</a></header>\n`
    : "";
  // Year is rendered without a current-time read — that lives on the
  // clock facility in StageContext, but for v0 we keep theme.ts pure
  // (deterministic) and let the caller pass the year in via opts if
  // they want one.  An undated footer is fine.
  const footer = "";
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${titleEscaped}</title>
<style>
${CLASSLESS_CSS}</style>
</head>
<body>
${header}<main>
${opts.bodyHtml}</main>
${footer}</body>
</html>
`;
}
