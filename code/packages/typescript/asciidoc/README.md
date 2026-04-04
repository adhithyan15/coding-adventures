# @coding-adventures/asciidoc

AsciiDoc pipeline convenience package. Combines the parser and the HTML
renderer into a single `toHtml()` function.

## What it does

- Wraps `@coding-adventures/asciidoc-parser` (block + inline parsing) and
  `@coding-adventures/document-ast-to-html` (HTML rendering).
- Provides a one-call `toHtml(asciidoc)` function for the common case.
- Also re-exports `parse()` and `render()` for users who need the AST.

## Where it fits

```
AsciiDoc source
    ↓  toHtml()          @coding-adventures/asciidoc  ← you are here
HTML string
```

Under the hood:

```
AsciiDoc source
    ↓  parse()           @coding-adventures/asciidoc-parser
DocumentNode AST
    ↓  render()          @coding-adventures/document-ast-to-html
HTML string
```

## Usage

```typescript
import { toHtml } from "@coding-adventures/asciidoc";

const html = toHtml(`
= My Document

Introduction paragraph with *bold* and _italic_ text.

== Section

[source,typescript]
----
const greeting = "Hello, AsciiDoc!";
----

* Item one
* Item two
`);

console.log(html);
// <h1>My Document</h1>
// <p>Introduction paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
// <h2>Section</h2>
// <pre><code class="language-typescript">const greeting = "Hello, AsciiDoc!";
// </code></pre>
// <ul>
// <li><p>Item one</p></li>
// <li><p>Item two</p></li>
// </ul>
```

## Working with the AST directly

```typescript
import { parse, render } from "@coding-adventures/asciidoc";
import type { DocumentNode } from "@coding-adventures/asciidoc";

const doc: DocumentNode = parse("= Hello\n\nWorld.\n");
// ... inspect or transform the AST ...
const html = render(doc);
```
