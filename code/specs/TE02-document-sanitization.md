# TE02 — Document Sanitization

## Overview

Sanitization is a **separate pipeline concern** from parsing and rendering. A
parser's job is to faithfully convert source text into an AST — it must not
silently discard content the author intended. A renderer's job is to faithfully
convert an AST into output HTML — it must not second-guess the semantic meaning
of nodes. Neither component should be in the business of deciding which content
is "safe" for which audience.

This spec defines two sanitization packages that slot cleanly between the
existing pipeline stages:

```
parse(markdown)          ← TE01 — CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 — document-ast-sanitizer (this spec, stage 1)
      ↓
toHtml(doc)              ← TE00 — document-ast-to-html
      ↓
sanitizeHtml(html, pol)  ← TE02 — document-html-sanitizer (this spec, stage 2)
      ↓
final output
```

Either stage can be used in isolation. Stage 1 (AST sanitization) is the
**preferred approach** because it operates on structured data where intent is
unambiguous. Stage 2 (HTML sanitization) is a safety net for cases where the
AST has already been rendered, or where HTML arrives from external systems.

---

## Why Sanitization Is a Separate Concern

### The Single Responsibility Principle applied to document pipelines

Consider the analogy to a compiler: the C compiler does not refuse to compile
code that calls `memcpy` — that would be overstepping. The OS enforces memory
boundaries at runtime. Each layer enforces only what it understands.

The same applies here:

| Stage            | Responsibility                                      | NOT responsible for                          |
|------------------|-----------------------------------------------------|----------------------------------------------|
| Parser           | Faithfully parse source into an AST                 | Deciding which content is safe               |
| AST Sanitizer    | Transform the AST per caller-defined policy         | Knowing what the caller will do with the AST |
| Renderer         | Faithfully convert AST → HTML                       | Deciding which HTML attributes are safe      |
| HTML Sanitizer   | Strip dangerous HTML from an opaque string          | Understanding document semantics             |

### The `sanitize: boolean` antipattern

The current `toHtml(doc, { sanitize: true })` option in
`@coding-adventures/document-ast-to-html` conflates rendering with policy
enforcement. This creates several problems:

1. **Boolean is too coarse.** A boolean cannot express "allow HTML raw blocks but
   not LaTeX raw blocks", nor "strip images but keep links", nor "clamp headings
   to level 3".

2. **Wrong layer.** The renderer does not know whether the document is destined
   for a trusted editor preview or an untrusted public comment thread. The
   calling code knows this; the renderer should not need to.

3. **Not composable.** A pipeline that uses a non-HTML renderer (PDF, plain text)
   has no way to sanitize the AST before rendering, because sanitization is
   locked inside the HTML renderer.

4. **Not portable.** Every language port of `document-ast-to-html` must
   re-implement the same sanitization logic. Extracting it to a dedicated package
   makes the logic canonical and independently testable.

The `sanitize: boolean` option in `RenderOptions` is deprecated by this spec.
See the Migration Path section.

---

## Package Architecture

```
@coding-adventures/document-ast              ← pure types (TE00)
         ↑ types                                      ↑ types
@coding-adventures/document-ast-sanitizer    @coding-adventures/document-ast-to-html
  sanitize(doc, policy) → DocumentNode         toHtml(doc) → string
         ↑                                              ↑
         └──────────────── compose ────────────────────┘
                               ↓
                   toHtml(sanitize(parse(md), STRICT))

@coding-adventures/document-html-sanitizer
  sanitizeHtml(html, policy) → string
  (operates on rendered output — no document-ast dependency)
```

The two sanitizer packages have **no dependency on each other**. They address
different layers of the pipeline and can be used independently.

---

## Stage 1 — `@coding-adventures/document-ast-sanitizer`

### Concept

The AST sanitizer performs a **policy-driven tree transformation**. It walks the
`DocumentNode` tree and, for each node, applies the policy to decide whether to:

- Keep the node unchanged (pass through)
- Replace the node with a safe equivalent (e.g. convert a dangerous link to plain text)
- Drop the node entirely (omit it from output)
- Clamp a value (e.g. reduce heading level 1 to level 2)

The transform is **pure and immutable**: it never mutates the input `DocumentNode`.
It always returns a freshly constructed `DocumentNode`. Callers can safely pass
the same document through multiple sanitizers with different policies.

The transform is **complete**: every node type in the TE00 Document AST is handled
explicitly. When new node types are added to the AST in future specs, the sanitizer
must be updated to handle them — an unknown node type is never silently passed through.

### Public API

```typescript
import type { DocumentNode } from "@coding-adventures/document-ast";

/**
 * Sanitize a DocumentNode by applying a SanitizationPolicy.
 *
 * Returns a new DocumentNode with all policy violations removed or neutralised.
 * The input is never mutated.
 *
 * @param document  The document to sanitize.
 * @param policy    The sanitization policy to apply.
 * @returns         A new, sanitized DocumentNode.
 *
 * @example
 * // User-generated content — strict policy
 * const safe = sanitize(parse(userMarkdown), STRICT);
 * const html = toHtml(safe);
 *
 * // Documentation — pass through everything
 * const doc = sanitize(parse(trustedMarkdown), PASSTHROUGH);
 * const html = toHtml(doc);
 */
export function sanitize(document: DocumentNode, policy: SanitizationPolicy): DocumentNode;
```

### `SanitizationPolicy` Type

```typescript
/**
 * Policy that controls what the AST sanitizer keeps, transforms, or drops.
 *
 * All fields are optional. Omitting a field uses the PASSTHROUGH default
 * (keep everything). Use the named presets STRICT, RELAXED, or PASSTHROUGH
 * as starting points and spread-override specific fields.
 */
export interface SanitizationPolicy {

  // ─── Raw node handling ──────────────────────────────────────────────────

  /**
   * Controls which RawBlockNode formats are allowed through.
   *
   * - `"drop-all"` — drop every RawBlockNode regardless of format (safest)
   * - `"passthrough"` — keep every RawBlockNode regardless of format
   * - `string[]` — allowlist of format strings to keep; all others dropped
   *
   * Default (when omitted): "passthrough"
   *
   * @example
   * // Allow only HTML raw blocks, drop LaTeX and others
   * allowRawBlockFormats: ["html"]
   *
   * // Drop all raw blocks (recommended for user-generated content)
   * allowRawBlockFormats: "drop-all"
   */
  readonly allowRawBlockFormats?: "drop-all" | "passthrough" | readonly string[];

  /**
   * Controls which RawInlineNode formats are allowed through.
   * Same semantics as allowRawBlockFormats.
   *
   * Default (when omitted): "passthrough"
   */
  readonly allowRawInlineFormats?: "drop-all" | "passthrough" | readonly string[];

  // ─── URL scheme policy ──────────────────────────────────────────────────

  /**
   * Allowlist of URL schemes that are permitted in LinkNode.destination,
   * ImageNode.destination, and AutolinkNode.destination.
   *
   * URLs whose scheme is not in this list are replaced with "" (empty string),
   * making the link/image inert. Relative URLs (no scheme) always pass through.
   *
   * Default (when omitted): ["http", "https", "mailto", "ftp"]
   *
   * Set to `null` to allow any scheme (not recommended for untrusted content).
   */
  readonly allowedUrlSchemes?: readonly string[] | null;

  // ─── Node type policy ───────────────────────────────────────────────────

  /**
   * If true, all LinkNode instances are dropped.
   * The link's text children are promoted to their parent as plain inline nodes.
   * Default: false (links are kept).
   */
  readonly dropLinks?: boolean;

  /**
   * If true, all ImageNode instances are dropped entirely.
   * Default: false (images are kept).
   * Note: dropImages takes precedence over transformImageToText.
   */
  readonly dropImages?: boolean;

  /**
   * If true, ImageNode instances are replaced by a TextNode containing their
   * alt text. This provides a text fallback without completely silencing image
   * references.
   * Default: false.
   */
  readonly transformImageToText?: boolean;

  /**
   * Maximum heading level allowed. Headings deeper than this level are
   * demoted to this level.
   *
   * - `"drop"` — all HeadingNode instances are dropped entirely
   * - 1–6 — headings with level > maxHeadingLevel are clamped down
   *
   * Default (when omitted): 6 (no clamping)
   */
  readonly maxHeadingLevel?: 1 | 2 | 3 | 4 | 5 | 6 | "drop";

  /**
   * Minimum heading level allowed. Headings shallower than this level are
   * promoted (level raised) to this level.
   *
   * Default (when omitted): 1 (no promotion)
   *
   * @example
   * // Reserve h1 for the page title; user content starts at h2
   * minHeadingLevel: 2
   */
  readonly minHeadingLevel?: 1 | 2 | 3 | 4 | 5 | 6;

  /**
   * If true, BlockquoteNode instances are dropped (children are NOT promoted).
   * Default: false.
   */
  readonly dropBlockquotes?: boolean;

  /**
   * If true, CodeBlockNode instances are dropped.
   * Default: false.
   */
  readonly dropCodeBlocks?: boolean;

  /**
   * If true, CodeSpanNode instances are converted to plain TextNode instances.
   * Default: false.
   */
  readonly transformCodeSpanToText?: boolean;
}
```

### Named Presets

```typescript
/**
 * STRICT — for user-generated content (comments, forum posts, chat messages).
 *
 * Drops all raw HTML/format passthrough. Allows only http, https, mailto URLs.
 * Images are converted to alt text. Links are kept but URL-sanitized.
 * Headings are clamped to h2–h6 (h1 is reserved for page title).
 */
export const STRICT: SanitizationPolicy = {
  allowRawBlockFormats: "drop-all",
  allowRawInlineFormats: "drop-all",
  allowedUrlSchemes: ["http", "https", "mailto"],
  dropImages: false,
  transformImageToText: true,
  minHeadingLevel: 2,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};

/**
 * RELAXED — for semi-trusted content (authenticated users, internal wikis).
 *
 * Allows HTML raw blocks (but not other formats). Allows http, https, mailto,
 * ftp. Images pass through unchanged. Headings unrestricted.
 */
export const RELAXED: SanitizationPolicy = {
  allowRawBlockFormats: ["html"],
  allowRawInlineFormats: ["html"],
  allowedUrlSchemes: ["http", "https", "mailto", "ftp"],
  dropImages: false,
  transformImageToText: false,
  minHeadingLevel: 1,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};

/**
 * PASSTHROUGH — for fully trusted content (documentation, static sites).
 *
 * No sanitization. Everything passes through unchanged.
 * Equivalent to not calling sanitize() at all.
 */
export const PASSTHROUGH: SanitizationPolicy = {
  allowRawBlockFormats: "passthrough",
  allowRawInlineFormats: "passthrough",
  allowedUrlSchemes: null,
  dropImages: false,
  transformImageToText: false,
  minHeadingLevel: 1,
  maxHeadingLevel: 6,
  dropLinks: false,
  dropBlockquotes: false,
  dropCodeBlocks: false,
  transformCodeSpanToText: false,
};
```

### Transformation Rules

The sanitizer performs a single recursive descent of the AST, producing a new
tree. The following truth table defines the transformation for each node type:

```
Node type          Condition                            Action
────────────────────────────────────────────────────────────────────────────────
DocumentNode       always                               recurse into children
HeadingNode        maxHeadingLevel === "drop"           drop node
HeadingNode        level < minHeadingLevel              clamp level up to minHeadingLevel
HeadingNode        level > maxHeadingLevel              clamp level down to maxHeadingLevel
HeadingNode        otherwise                            recurse into children
ParagraphNode      always                               recurse into children
CodeBlockNode      dropCodeBlocks === true              drop node
CodeBlockNode      otherwise                            keep as-is (leaf)
BlockquoteNode     dropBlockquotes === true             drop node
BlockquoteNode     otherwise                            recurse into children
ListNode           always                               recurse into children
ListItemNode       always                               recurse into children
ThematicBreakNode  always                               keep as-is (leaf)
RawBlockNode       allowRawBlockFormats="drop-all"      drop node
RawBlockNode       allowRawBlockFormats="passthrough"   keep as-is
RawBlockNode       allowRawBlockFormats=[…]             keep if format in list, else drop

TextNode           always                               keep as-is
EmphasisNode       always                               recurse into children
StrongNode         always                               recurse into children
CodeSpanNode       transformCodeSpanToText === true     convert to TextNode { value }
CodeSpanNode       otherwise                            keep as-is
LinkNode           dropLinks === true                   promote children to parent
LinkNode           URL scheme not allowed               keep node, set destination=""
LinkNode           otherwise                            sanitize URL, recurse into children
ImageNode          dropImages === true                  drop node
ImageNode          transformImageToText === true        TextNode { value: node.alt }
ImageNode          URL scheme not allowed               keep node, set destination=""
ImageNode          otherwise                            sanitize URL, keep as-is
AutolinkNode       URL scheme not allowed               drop node
AutolinkNode       otherwise                            sanitize URL, keep as-is
RawInlineNode      allowRawInlineFormats="drop-all"     drop node
RawInlineNode      allowRawInlineFormats="passthrough"  keep as-is
RawInlineNode      allowRawInlineFormats=[…]            keep if format in list, else drop
HardBreakNode      always                               keep as-is
SoftBreakNode      always                               keep as-is
```

### URL Scheme Sanitization

1. Strip C0 control characters (U+0000–U+001F) and zero-width characters
   (U+200B, U+200C, U+200D, U+2060, U+FEFF) from the URL before scheme
   detection. Browsers silently ignore these characters when parsing URL
   schemes, enabling bypasses like `java\x00script:`.

2. Extract the scheme: everything before the first `:`.

3. If `allowedUrlSchemes` is `null`, all schemes pass through.

4. If `allowedUrlSchemes` is a list, check if the extracted scheme (lowercased)
   is in the list. Relative URLs (no `:` found, or `:` after `/` or `?`) always
   pass through.

5. If the scheme is not allowed, replace the destination with `""`.

### Empty Children After Sanitization

When all children of a container node are dropped (e.g. a `ParagraphNode` whose
only child was a `RawInlineNode` that got dropped), the parent node is itself
dropped from the output. This prevents empty `<p></p>` tags in the rendered HTML.

Exception: `DocumentNode` is never dropped — an empty document is valid and
returns `{ type: "document", children: [] }`.

### Promoting Link Children

When `dropLinks: true`, a `LinkNode { children: [TextNode("click here")] }` is
not simply removed. The children are **promoted** to the parent container as if
the link wrapper did not exist. This preserves the text content while removing
the hyperlink.

---

## Stage 2 — `@coding-adventures/document-html-sanitizer`

### Concept

The HTML sanitizer operates on an **opaque HTML string** with no knowledge of
how it was produced. It is a string → string transformation. This makes it
applicable to:

- HTML rendered by `document-ast-to-html`
- HTML from external APIs (CMS, third-party services)
- HTML pasted by users in rich-text editors

The HTML sanitizer does **not** parse a full DOM by default. It uses
pattern-based string operations for portability across environments (browser,
Node.js, Deno, Go, Python, Ruby, Rust, Elixir, Lua). Where a DOM is available,
callers can supply a `domAdapter` for higher fidelity.

### Public API

```typescript
/**
 * Sanitize an HTML string by stripping dangerous elements and attributes.
 *
 * @example
 * const safe = sanitizeHtml(toHtml(parse(markdown)), HTML_STRICT);
 */
export function sanitizeHtml(html: string, policy: HtmlSanitizationPolicy): string;
```

### `HtmlSanitizationPolicy` Type

```typescript
export interface HtmlSanitizationPolicy {
  /**
   * HTML element names (lowercase) removed entirely, including all content.
   * Default: ["script","style","iframe","object","embed","applet",
   *           "form","input","button","select","textarea",
   *           "noscript","meta","link","base"]
   */
  readonly dropElements?: readonly string[];

  /**
   * Attribute names (lowercase) stripped from every element.
   * Default: all event handler attributes (on*) plus ["srcdoc","formaction"].
   */
  readonly dropAttributes?: readonly string[];

  /**
   * Allowlist of URL schemes for href and src attributes.
   * Default: ["http", "https", "mailto", "ftp"]
   */
  readonly allowedUrlSchemes?: readonly string[] | null;

  /**
   * If true, HTML comments (<!-- … -->) are stripped.
   * Default: true.
   */
  readonly dropComments?: boolean;

  /**
   * If true, style attributes containing expression() or url() with
   * non-http/https arguments are stripped entirely.
   * Default: true.
   */
  readonly sanitizeStyleAttributes?: boolean;

  /**
   * Optional DOM adapter for environments with a real HTML parser.
   * When provided, the sanitizer parses into a DOM, applies the policy,
   * and serialises back to a string.
   */
  readonly domAdapter?: HtmlSanitizerDomAdapter;
}

export interface HtmlSanitizerDomAdapter {
  parse(html: string): unknown;
  walk(dom: unknown, visitor: DomVisitor): void;
  serialize(dom: unknown): string;
}

export interface DomVisitor {
  element(tagName: string, attributes: Map<string, string>): false | Map<string, string>;
  comment(value: string): false | string;
}
```

### Named Presets

```typescript
/** HTML_STRICT — untrusted HTML from external sources. */
export const HTML_STRICT: HtmlSanitizationPolicy = {
  dropElements: [
    "script","style","iframe","object","embed","applet",
    "form","input","button","select","textarea",
    "noscript","meta","link","base",
  ],
  dropAttributes: [], // all on* attributes stripped by default logic
  allowedUrlSchemes: ["http","https","mailto"],
  dropComments: true,
  sanitizeStyleAttributes: true,
};

/** HTML_RELAXED — authenticated users / internal tools. */
export const HTML_RELAXED: HtmlSanitizationPolicy = {
  dropElements: ["script","iframe","object","embed","applet"],
  dropAttributes: [],
  allowedUrlSchemes: ["http","https","mailto","ftp"],
  dropComments: false,
  sanitizeStyleAttributes: true,
};

/** HTML_PASSTHROUGH — no sanitization. */
export const HTML_PASSTHROUGH: HtmlSanitizationPolicy = {
  dropElements: [],
  dropAttributes: [],
  allowedUrlSchemes: null,
  dropComments: false,
  sanitizeStyleAttributes: false,
};
```

### Dangerous Elements Removed by Default

```
Element        Risk
───────────────────────────────────────────────────────────────
<script>       Direct JavaScript execution
<style>        CSS expression() attacks, data exfiltration
<iframe>       Framing attacks, clickjacking
<object>       Plugin execution (Flash, Java applets)
<embed>        Same as <object>
<applet>       Java applet execution (legacy)
<form>         CSRF, credential phishing
<input>        Data capture, autofill attacks
<meta>         Redirect via http-equiv="refresh"
<base>         Base URL hijacking (breaks all relative links)
<link>         CSS import, DNS prefetch exfiltration
<noscript>     Can be abused in certain parser contexts
```

### Dangerous Attributes Stripped by Default

```
Pattern        Risk
───────────────────────────────────────────────────────────────
on*            All event handler attributes (onclick, onload, etc.)
srcdoc         Inline HTML frame content (iframe srcdoc XSS)
formaction     Overrides form action URL
```

### CSS Injection Prevention

When `sanitizeStyleAttributes: true`, any `style` attribute containing
`expression(` (case-insensitive) or `url(` with a non-http/https argument is
stripped entirely. The full `style` attribute is removed rather than attempting
to parse CSS.

---

## Pipeline Integration

### Single-stage usage (recommended)

```typescript
import { parse } from "@coding-adventures/commonmark-parser";
import { sanitize, STRICT } from "@coding-adventures/document-ast-sanitizer";
import { toHtml } from "@coding-adventures/document-ast-to-html";

const html = toHtml(sanitize(parse(userMarkdown), STRICT));

// Custom policy — allow HTML blocks but restrict headings
const html = toHtml(sanitize(parse(editorMarkdown), {
  ...RELAXED,
  minHeadingLevel: 2,
  allowedUrlSchemes: ["http", "https"],
}));
```

### Two-stage usage (belt and suspenders)

```typescript
import { sanitize, STRICT } from "@coding-adventures/document-ast-sanitizer";
import { sanitizeHtml, HTML_STRICT } from "@coding-adventures/document-html-sanitizer";

const safeHtml = sanitizeHtml(
  toHtml(sanitize(parse(userMarkdown), STRICT)),
  HTML_STRICT
);
```

### HTML-only usage (when AST is unavailable)

```typescript
import { sanitizeHtml, HTML_STRICT } from "@coding-adventures/document-html-sanitizer";

const safeHtml = sanitizeHtml(cmsApiResponse.body, HTML_STRICT);
```

---

## Migration Path

### Phase 1 — Deprecate `RenderOptions.sanitize`

Mark `RenderOptions.sanitize` as `@deprecated` in JSDoc:

```typescript
export interface RenderOptions {
  /**
   * @deprecated Use @coding-adventures/document-ast-sanitizer instead.
   *
   * Before: toHtml(doc, { sanitize: true })
   * After:  toHtml(sanitize(doc, STRICT))
   *
   * This option will be removed in v1.0.0.
   */
  readonly sanitize?: boolean;
}
```

### Phase 2 — Remove `RenderOptions.sanitize` (v1.0.0)

Remove the `sanitize` field from `RenderOptions`. The renderer becomes a
pure transformation with no policy logic. Sanitization must be done before
calling `toHtml()`.

The URL sanitization inside `html-renderer.ts` (the `sanitizeUrl` function)
is retained as defence-in-depth for programmatically constructed `DocumentNode`
objects that bypass the sanitizer pipeline.

---

## Package Layout

### `@coding-adventures/document-ast-sanitizer`

```
code/packages/typescript/document-ast-sanitizer/
  src/
    sanitizer.ts      ← sanitize(doc, policy) implementation
    policy.ts         ← SanitizationPolicy type + STRICT/RELAXED/PASSTHROUGH
    url-utils.ts      ← URL scheme extraction and control-char stripping
    index.ts          ← exports
  tests/
    sanitizer.test.ts ← unit tests for every policy option and XSS vector
    xss-vectors.ts    ← shared XSS test fixture
  package.json
  BUILD
  README.md
  CHANGELOG.md
```

**Dependencies:** `@coding-adventures/document-ast` only. No runtime deps beyond types.

### `@coding-adventures/document-html-sanitizer`

```
code/packages/typescript/document-html-sanitizer/
  src/
    html-sanitizer.ts   ← regex/string-based sanitizer
    dom-sanitizer.ts    ← DOM-mode (via domAdapter)
    policy.ts           ← HtmlSanitizationPolicy + presets
    url-utils.ts        ← URL scheme check (independent copy — no shared dep)
    index.ts            ← exports
  tests/
    html-sanitizer.test.ts
    xss-vectors.ts
  package.json
  BUILD
  README.md
  CHANGELOG.md
```

**Dependencies:** None. No dependency on `document-ast` — string in, string out.

---

## Testing Strategy

### XSS Attack Vectors

Every sanitizer test suite must cover the following categories.

#### Script Injection

```
<script>alert(1)</script>
<script src="https://evil.com/xss.js"></script>
<SCRIPT>alert(1)</SCRIPT>
```

#### Event Handler Injection

```
<img onload="alert(1)" src="x.png">
<a onclick="alert(1)">click</a>
<div onfocus="alert(1)" tabindex="0">
<svg onload="alert(1)">
```

#### JavaScript URL Injection (link/image destinations in Markdown)

```markdown
[click me](javascript:alert(1))
[click me](JAVASCRIPT:alert(1))
[click me](java&#x73;cript:alert(1))
[click me](java\x00script:alert(1))
[click me](data:text/html,<script>alert(1)</script>)
[click me](blob:https://origin/some-uuid)
[click me](vbscript:MsgBox(1))
```

#### CSS Expression Injection

```html
<p style="width:expression(alert(1))">
<p style="background:url(javascript:alert(1))">
```

#### HTML Comment Attacks

```html
<!--<img src=x onerror=alert(1)>-->
<!--[if IE]><script>alert(1)</script><![endif]-->
```

#### Control Character URL Bypasses

```
javascript\x00:alert(1)
java\rscript:alert(1)
\u200bjavascript:alert(1)
```

### AST Sanitizer — Specific Test Categories

```typescript
// RawBlockNode handling
sanitize(rawBlockHtml,   { allowRawBlockFormats: "drop-all" })  // → dropped
sanitize(rawBlockHtml,   { allowRawBlockFormats: ["html"]  })  // → kept
sanitize(rawBlockLatex,  { allowRawBlockFormats: ["html"]  })  // → dropped

// URL scheme handling
sanitize(linkJavascript, STRICT)  // → LinkNode { destination: "" }
sanitize(linkHttps,      STRICT)  // → destination unchanged
sanitize(autolinkData,   STRICT)  // → AutolinkNode dropped

// Heading level clamping
sanitize(h1Doc, { minHeadingLevel: 2 })        // → level clamped to 2
sanitize(h1Doc, { maxHeadingLevel: "drop" })   // → HeadingNode removed
sanitize(h5Doc, { maxHeadingLevel: 3 })        // → level clamped to 3

// Image handling
sanitize(imageDoc, { dropImages: true })            // → ImageNode removed
sanitize(imageDoc, { transformImageToText: true })  // → TextNode { value: alt }

// Link promotion
sanitize(linkDoc, { dropLinks: true })  // → children promoted, LinkNode gone

// Empty children cleanup
sanitize(paraWithOnlyRawInline, { allowRawInlineFormats: "drop-all" })
  // → ParagraphNode itself dropped (empty after sanitization)

// Immutability
const original  = parse("...");
const sanitized = sanitize(original, STRICT);
// original must be referentially unchanged

// PASSTHROUGH is effectively identity
deepEqual(sanitize(doc, PASSTHROUGH), doc);
```

### HTML Sanitizer — Specific Test Categories

```typescript
// Script element removal
sanitizeHtml("<p>Safe</p><script>alert(1)</script>", HTML_STRICT)
  // → "<p>Safe</p>"

// Event handler removal
sanitizeHtml('<img src="x.png" onload="alert(1)">', HTML_STRICT)
  // → '<img src="x.png">'

// URL sanitization in attributes
sanitizeHtml('<a href="javascript:alert(1)">click</a>', HTML_STRICT)
  // → '<a href="">click</a>'

// CSS expression stripping
sanitizeHtml('<p style="width:expression(alert(1))">x</p>', HTML_STRICT)
  // → '<p>x</p>'

// Comment stripping
sanitizeHtml('<!-- comment --><p>ok</p>', HTML_STRICT)
  // → '<p>ok</p>'

// Passthrough preserves everything
sanitizeHtml('<script>alert(1)</script>', HTML_PASSTHROUGH)
  // → '<script>alert(1)</script>'
```

---

## Multi-Language Ports

The same two-package split is implemented in every supported language. The
TypeScript implementation is the prototype and compliance baseline.

| Language   | AST Sanitizer package                               | HTML Sanitizer package                               |
|------------|-----------------------------------------------------|------------------------------------------------------|
| TypeScript | `@coding-adventures/document-ast-sanitizer`         | `@coding-adventures/document-html-sanitizer`         |
| Python     | `coding_adventures_document_ast_sanitizer`          | `coding_adventures_document_html_sanitizer`          |
| Go         | `coding-adventures/document-ast-sanitizer`          | `coding-adventures/document-html-sanitizer`          |
| Ruby       | `coding_adventures-document_ast_sanitizer`          | `coding_adventures-document_html_sanitizer`          |
| Rust       | `coding_adventures_document_ast_sanitizer`          | `coding_adventures_document_html_sanitizer`          |
| Elixir     | `CodingAdventures.DocumentAstSanitizer`             | `CodingAdventures.DocumentHtmlSanitizer`             |
| Lua        | `coding_adventures.document_ast_sanitizer`          | `coding_adventures.document_html_sanitizer`          |

Package locations follow the existing repo convention:
`code/packages/<language>/document-ast-sanitizer/` and
`code/packages/<language>/document-html-sanitizer/`.

---

## Architectural Decisions

### Decision 1: Two packages, not one

The AST sanitizer and HTML sanitizer are separate packages with no shared
dependency. The AST sanitizer needs `document-ast` for types. The HTML
sanitizer has **no dependencies at all** — string in, string out. This
makes the HTML sanitizer usable in any context without pulling in the full
document-ast ecosystem.

### Decision 2: Policy objects, not method chaining

Policies are plain data objects. This makes them:
- Easily composable via spread: `{ ...STRICT, maxHeadingLevel: 3 }`
- JSON-serializable (no functions in the policy object)
- Simple to implement in all target languages without OO overhead

### Decision 3: Drop-all vs allowlist for raw formats

`allowRawBlockFormats` uses `"drop-all" | "passthrough" | string[]` rather than
a boolean because "allow HTML but not LaTeX" is a legitimate real-world policy.

### Decision 4: Link children are promoted, not dropped

When `dropLinks: true`, link text is promoted to the parent. Silently removing
link text would confuse users ("click  for more" is worse than "click here for
more" where the anchor text is preserved as plain text).

### Decision 5: No DOM dependency in the HTML sanitizer by default

The HTML sanitizer uses regex/string operations for portability across Go,
Python, Rust, Elixir, Lua, and edge JS runtimes — none of which have a native
DOM. The `domAdapter` escape hatch exists for browser environments where higher
fidelity is needed.

### Decision 6: `sanitize: boolean` deprecated, not immediately removed

Immediate removal would be a breaking change. Deprecation gives callers a
migration window to adopt the dedicated sanitizer package.

### Decision 7: URL sanitization stays in the renderer as defence-in-depth

Even after the AST sanitizer is introduced, `html-renderer.ts` retains its
internal `sanitizeUrl`. This guards against programmatically constructed
`DocumentNode` values that contain dangerous URLs but were never passed through
the sanitizer pipeline.
