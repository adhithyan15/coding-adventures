/**
 * AsciiDoc Live Demo
 *
 * A split-pane editor that renders AsciiDoc to HTML in real time using our
 * own AsciiDoc pipeline:
 *
 *   AsciiDoc source
 *       ↓  parse()           @coding-adventures/asciidoc-parser
 *   DocumentNode AST
 *       ↓  render()          @coding-adventures/document-ast-to-html
 *   HTML string
 *       ↓  dangerouslySetInnerHTML
 *   Live preview
 *
 * The parse + render step runs synchronously on every keystroke. For typical
 * AsciiDoc documents this is imperceptibly fast.
 *
 * Layout:
 *
 *   ┌─────────────────────────────────────────────────────┐
 *   │  header — title, pipeline badge, back link          │
 *   ├──────────────────────────┬──────────────────────────┤
 *   │  AsciiDoc editor         │  HTML preview            │
 *   │  (textarea, monospace)   │  (rendered, styled)      │
 *   ├──────────────────────────┴──────────────────────────┤
 *   │  footer — char count, word count, render time       │
 *   └─────────────────────────────────────────────────────┘
 */

import { useState, useMemo } from "react";
import { toHtml } from "@coding-adventures/asciidoc";

// ---------------------------------------------------------------------------
// Default content — showcases most AsciiDoc features out of the box
// ---------------------------------------------------------------------------

const DEFAULT_ASCIIDOC = `= AsciiDoc Live Preview

Welcome! This is a *live preview* powered by our own AsciiDoc pipeline,
built entirely from scratch in TypeScript — no third-party parsers.

== The Pipeline

Every keystroke runs through two steps from our library:

[source,typescript]
----
import { toHtml } from "@coding-adventures/asciidoc";

// 1. parse() converts AsciiDoc → DocumentNode AST
// 2. render() converts AST → HTML string
const html = toHtml(asciidoc);
----

Try editing the AsciiDoc on the left — the preview updates instantly.

'''

== Inline Formatting

* *Bold text* with \`*asterisks*\`
* _Italic text_ with \`_underscores_\`
* **Unconstrained bold** works mid-word: **un**constrained
* __Unconstrained italic__ likewise
* \`Inline code\` with backticks

Key difference from Markdown: in AsciiDoc, \`*asterisk*\` = *bold*, not italic!

== Blockquotes

____
"The best way to understand a parser is to write one."

This entire block is a quote — opened and closed with \`____\`.
____

== Lists

Ordered:

. *parse(asciidoc)* — Tokenizes the source and builds a Document AST
. *render(ast)* — Walks the AST and emits HTML

Unordered:

* Block nodes: headings, paragraphs, code blocks, blockquotes, lists
* Inline nodes: strong, emphasis, code spans, links, images
** Nested lists work with \`**\` for level 2
** Any depth supported

== Links and Images

link:https://asciidoc.org[AsciiDoc Official Site]

Cross-reference: see <<the-pipeline,The Pipeline section>> above.

Bare URL: https://asciidoc.org

== Code Blocks

[source,javascript]
----
// The Fibonacci sequence
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(fibonacci(10)); // 55
----

Literal block (no language):

....
  indented
  preformatted
  text
....

== Heading Levels

= H1
== H2
=== H3
==== H4
===== H5
====== H6

'''

_Edit anything above — or paste in your own AsciiDoc._
`;

// ---------------------------------------------------------------------------
// Stat helpers
// ---------------------------------------------------------------------------

function countWords(text: string): number {
  return text.trim() === "" ? 0 : text.trim().split(/\s+/).length;
}

function escapeHtmlText(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function App() {
  const [asciidoc, setAsciidoc] = useState(DEFAULT_ASCIIDOC);

  // Render on every state change. useMemo ensures we only re-render when
  // asciidoc changes, not on unrelated re-renders.
  const { html, renderMs } = useMemo(() => {
    const t0 = performance.now();
    let rendered = "";
    try {
      rendered = toHtml(asciidoc);
    } catch (err) {
      rendered = `<p style="color:#f87171;font-family:monospace">Parse error: ${escapeHtmlText(String(err))}</p>`;
    }
    const t1 = performance.now();
    return { html: rendered, renderMs: t1 - t0 };
  }, [asciidoc]);

  const charCount = asciidoc.length;
  const wordCount = countWords(asciidoc);

  return (
    <div className="app">
      {/* ── Header ──────────────────────────────────────────────── */}
      <header className="app__header">
        <div className="header__left">
          <a className="header__back" href="/coding-adventures/" aria-label="Back to Coding Adventures">
            ← coding-adventures
          </a>
          <h1 className="header__title">AsciiDoc Live Preview</h1>
          <p className="header__subtitle">
            AsciiDoc rendered in real time by our own parser and renderer
          </p>
        </div>
        <div className="header__pipeline" aria-label="Rendering pipeline">
          <span className="pipeline__step pipeline__step--input">AsciiDoc</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--fn">parse()</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--ast">DocumentNode</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--fn">render()</span>
          <span className="pipeline__arrow">→</span>
          <span className="pipeline__step pipeline__step--output">HTML</span>
        </div>
      </header>

      {/* ── Editor / Preview split ───────────────────────────────── */}
      <div className="editor-layout">
        {/* Left pane — AsciiDoc source */}
        <div className="pane pane--editor">
          <div className="pane__label">AsciiDoc</div>
          <textarea
            className="editor"
            value={asciidoc}
            onChange={(e) => setAsciidoc(e.target.value)}
            spellCheck={false}
            aria-label="AsciiDoc editor"
            onKeyDown={(e) => {
              // Insert two spaces on Tab instead of moving focus
              if (e.key === "Tab") {
                e.preventDefault();
                const el = e.currentTarget;
                const start = el.selectionStart;
                const end = el.selectionEnd;
                const next = asciidoc.slice(0, start) + "  " + asciidoc.slice(end);
                setAsciidoc(next);
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
            // The HTML is generated by our own render() from user-typed AsciiDoc.
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
          @coding-adventures/asciidoc
        </span>
      </footer>
    </div>
  );
}
