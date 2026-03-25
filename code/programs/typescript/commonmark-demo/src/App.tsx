/**
 * CommonMark Live Demo
 *
 * A split-pane editor that renders Markdown to HTML in real time using our
 * own CommonMark pipeline:
 *
 *   Markdown source
 *       ↓  parse()           @coding-adventures/commonmark-parser
 *   DocumentNode AST
 *       ↓  toHtml()          @coding-adventures/document-ast-to-html
 *   HTML string
 *       ↓  dangerouslySetInnerHTML
 *   Live preview
 *
 * The parse + render step runs synchronously on every keystroke. For typical
 * Markdown documents (< 100 KB) this is imperceptibly fast — our parser
 * processes the full CommonMark spec test suite in < 10 ms.
 *
 * Layout:
 *
 *   ┌─────────────────────────────────────────────────────┐
 *   │  header — title, pipeline badge, back link          │
 *   ├──────────────────────────┬──────────────────────────┤
 *   │  Markdown editor         │  HTML preview            │
 *   │  (textarea, monospace)   │  (rendered, styled)      │
 *   ├──────────────────────────┴──────────────────────────┤
 *   │  footer — char count, word count, render time       │
 *   └─────────────────────────────────────────────────────┘
 */

import { useState, useMemo } from "react";
import { parse, toHtml } from "@coding-adventures/commonmark";

// ---------------------------------------------------------------------------
// Default content — showcases most CommonMark features out of the box
// ---------------------------------------------------------------------------

const DEFAULT_MARKDOWN = `# CommonMark Live Demo

Welcome! This is a **live preview** powered by our own CommonMark pipeline,
built entirely from scratch in TypeScript — no third-party parsers.

## The Pipeline

Every keystroke runs through two steps from our library:

\`\`\`typescript
import { parse, toHtml } from "@coding-adventures/commonmark";

// 1. parse() converts Markdown → DocumentNode AST
// 2. toHtml() renders AST → HTML string
const html = toHtml(parse(markdown));
\`\`\`

Try editing the Markdown on the left — the preview updates instantly.

---

## Inline Formatting

- **Bold text** with \`**double asterisks**\`
- *Italic text* with \`*single asterisks*\`
- ***Bold and italic*** combined
- \`Inline code\` with backticks
- Hard line break (two trailing spaces)
  continues here

## Blockquotes

> "The best way to understand a parser is to write one."
>
> Nested blockquotes work too:
>
> > This is doubly quoted.

## Lists

Ordered:

1. **parse(markdown)** — Tokenizes the source and builds a Document AST
2. **toHtml(ast)** — Walks the AST and emits HTML

Unordered:

- Block nodes: headings, paragraphs, code blocks, blockquotes, lists
- Inline nodes: emphasis, strong, code spans, links, images
  - Nested lists work fine
  - Any depth supported

## Links and Autolinks

[Visit the source on GitHub](https://github.com/adhithyan15/coding-adventures "coding-adventures")

Bare autolinks: <https://github.com> and <user@example.com>

## Images

![CommonMark logo](https://commonmark.org/images/markdown-mark.svg "CommonMark")

## Code Blocks

\`\`\`javascript
// The Fibonacci sequence, for illustration
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(fibonacci(10)); // 55
\`\`\`

## Thematic Break

---

## Heading Levels

# H1
## H2
### H3
#### H4
##### H5
###### H6

---

*Edit anything above — or paste in your own Markdown.*
`;

// ---------------------------------------------------------------------------
// Stat helpers
// ---------------------------------------------------------------------------

function countWords(text: string): number {
  return text.trim() === "" ? 0 : text.trim().split(/\s+/).length;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function App() {
  const [markdown, setMarkdown] = useState(DEFAULT_MARKDOWN);

  // Render on every state change. useMemo ensures we only re-render when
  // markdown changes, not on unrelated re-renders.
  const { html, renderMs } = useMemo(() => {
    const t0 = performance.now();
    const rendered = toHtml(parse(markdown));
    const t1 = performance.now();
    return { html: rendered, renderMs: t1 - t0 };
  }, [markdown]);

  const charCount = markdown.length;
  const wordCount = countWords(markdown);

  return (
    <div className="app">
      {/* ── Header ──────────────────────────────────────────────── */}
      <header className="app__header">
        <div className="header__left">
          <a className="header__back" href="/coding-adventures/" aria-label="Back to Coding Adventures">
            ← coding-adventures
          </a>
          <h1 className="header__title">CommonMark Live Demo</h1>
          <p className="header__subtitle">
            Markdown rendered in real time by our own parser and renderer
          </p>
        </div>
        <div className="header__pipeline" aria-label="Rendering pipeline">
          <span className="pipeline__step pipeline__step--input">Markdown</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--fn">parse()</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--ast">DocumentNode</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--fn">toHtml()</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--output">HTML</span>
        </div>
      </header>

      {/* ── Editor / Preview split ───────────────────────────────── */}
      <div className="editor-layout">
        {/* Left pane — Markdown source */}
        <div className="pane pane--editor">
          <div className="pane__label">Markdown</div>
          <textarea
            className="editor"
            value={markdown}
            onChange={(e) => setMarkdown(e.target.value)}
            spellCheck={false}
            aria-label="Markdown editor"
            onKeyDown={(e) => {
              // Insert two spaces on Tab instead of moving focus
              if (e.key === "Tab") {
                e.preventDefault();
                const el = e.currentTarget;
                const start = el.selectionStart;
                const end = el.selectionEnd;
                const next = markdown.slice(0, start) + "  " + markdown.slice(end);
                setMarkdown(next);
                // Restore cursor position after React re-render
                requestAnimationFrame(() => {
                  el.selectionStart = el.selectionEnd = start + 2;
                });
              }
            }}
          />
        </div>

        {/* Divider */}
        <div className="pane-divider" aria-hidden="true" />

        {/* Right pane — rendered HTML */}
        <div className="pane pane--preview">
          <div className="pane__label">Preview</div>
          <div
            className="preview"
            // The HTML is generated by our own toHtml() from user-typed markdown.
            // This is a demo for the user's own content — not for untrusted UGC.
            // eslint-disable-next-line react/no-danger
            dangerouslySetInnerHTML={{ __html: html }}
            aria-label="Rendered HTML preview"
          />
        </div>
      </div>

      {/* ── Footer stats ────────────────────────────────────────── */}
      <footer className="app__footer">
        <span className="stat">
          <span className="stat__value">{charCount.toLocaleString()}</span>
          <span className="stat__label"> chars</span>
        </span>
        <span className="stat__sep">·</span>
        <span className="stat">
          <span className="stat__value">{wordCount.toLocaleString()}</span>
          <span className="stat__label"> words</span>
        </span>
        <span className="stat__sep">·</span>
        <span className="stat">
          <span className="stat__value">{renderMs.toFixed(2)} ms</span>
          <span className="stat__label"> render</span>
        </span>
        <span className="stat__sep">·</span>
        <span className="stat stat--lib">
          @coding-adventures/commonmark
        </span>
      </footer>
    </div>
  );
}
