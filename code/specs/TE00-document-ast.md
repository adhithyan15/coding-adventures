# TE00 — Document AST

## Overview

The Document AST is a **format-agnostic Intermediate Representation (IR)** for
structured documents. It sits between document front-ends (parsers that read
Markdown, reStructuredText, HTML, DOCX, …) and document back-ends (renderers
that produce HTML, PDF, plain text, …).

The relationship is directly analogous to how LLVM IR works in compilers:

```
                         ┌──────────────────────────────────┐
  Markdown ──────────────►                                  ├──► HTML
  reStructuredText ──────►        Document AST (IR)         ├──► PDF
  HTML ──────────────────►                                  ├──► Plain text
  DOCX ──────────────────►                                  ├──► DOCX
  LaTeX ──────────────────►                                 ├──► LaTeX
                         └──────────────────────────────────┘
```

Without a shared IR, every pair of (source format, target format) needs its
own converter. With a shared IR, every front-end produces the IR and every
back-end consumes it. N front-ends × M back-ends requires only N + M
implementations instead of N × M.

---

## Motivation: Why Not Just Use the CommonMark AST?

The CommonMark AST (defined in TE01) is intentionally Markdown-specific.
It preserves artifacts of the Markdown notation that are meaningless outside
of Markdown:

1. **`LinkDefinitionNode`** — a `[ref]: https://…` definition line. This is a
   Markdown-specific mechanism for reusing link destinations. The parser resolves
   all reference links against these definitions and does not need to expose them
   in the IR. A reStructuredText or HTML parser has no equivalent concept.

2. **`HtmlBlockNode` / `HtmlInlineNode`** — these hard-code HTML as the only
   "escape hatch" for raw pass-through content. An RST parser might have a raw
   LaTeX directive; a DOCX reader might have raw XML. The format should be a
   runtime string, not baked into the type name.

The Document AST addresses both:

- **Drops** `LinkDefinitionNode` — links in the IR are always resolved to explicit
  destinations. There is no reference indirection.
- **Generalises** `HtmlBlockNode` → `RawBlockNode { format: "html"; value: string }`
  and `HtmlInlineNode` → `RawInlineNode { format: "html"; value: string }`. Any
  raw passthrough carries a `format` tag naming the target back-end that should
  interpret it.

Everything else — headings, paragraphs, lists, code blocks, blockquotes,
emphasis, strong, links, images, code spans, line breaks — is semantic content
that maps cleanly from any document format to any output format. These nodes are
kept exactly as defined in TE01.

---

## Design Principles

### 1. Semantic, not notational

Nodes carry meaning, not syntax. A `HeadingNode` means "a heading of level N",
not "a line that starts with `#`". The parser is responsible for resolving
notational details (setext vs ATX, `*` vs `_` emphasis) before producing the IR.

### 2. Resolved, not deferred

All cross-references are resolved before the IR is produced. A `LinkNode` always
has an explicit `destination` string. There are no "reference links" in the IR.
The parser resolves `[text][label]` against the link definition map and emits a
`LinkNode { destination: "…" }` directly.

### 3. Format-agnostic, not format-neutral

The IR does not pretend that all formats are equivalent. `RawBlockNode` and
`RawInlineNode` carry a `format` field to identify which back-end should
interpret them. A back-end that does not recognise the format must skip the node
(or optionally emit a warning). This is the correct trade-off: unknown raw blocks
should not silently corrupt the output.

### 4. Immutable and typed

All nodes are fully immutable — no field is ever mutated after creation. In
TypeScript, this is expressed with `readonly` on every field. Node type
discrimination uses a `type` string literal, enabling exhaustive `switch`
statements that the TypeScript compiler can verify.

### 5. Minimal and stable

The IR contains only nodes that represent universal document concepts. Format-
specific extensions (GFM tables, ThunderEgg wikilinks, LaTeX math, RST directives)
are layered on top in derived specs (TE04, TE05, …) and are not part of the core
IR. The core must be stable — once published, its node types do not change.

---

## Package Architecture

The package split that TE00 enables:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  @coding-adventures/document-ast                                          │
│  (this spec)                                                              │
│                                                                           │
│  Pure type definitions: DocumentNode, HeadingNode, ParagraphNode, …     │
│  No runtime code. No dependencies.                                        │
└────────────────────┬─────────────────────────────────────┬───────────────┘
                     │ types                                │ types
          ┌──────────▼──────────┐                ┌─────────▼──────────────┐
          │ @coding-adventures/ │                │  @coding-adventures/   │
          │ commonmark-parser   │                │  document-ast-to-html  │
          │                     │                │                        │
          │  parse(markdown)    │                │  toHtml(doc)           │
          │    → DocumentNode   │                │    → string            │
          └──────────┬──────────┘                └─────────┬──────────────┘
                     │ depends on                          │ depends on
          ┌──────────▼──────────────────────────────────────▼──────────────┐
          │  @coding-adventures/commonmark                                   │
          │                                                                  │
          │  Re-exports: { parse } from commonmark-parser                   │
          │  Re-exports: { toHtml } from document-ast-to-html               │
          │                                                                  │
          │  const html = toHtml(parse(markdownString))                     │
          └─────────────────────────────────────────────────────────────────┘
```

**Why the thin `commonmark` pipeline package?**

The `commonmark` package is the public-facing convenience package. Users who
just want to convert Markdown to HTML import one package and call two functions.
Users who want to plug in a different renderer, work with the AST directly, or
build a Markdown → PDF pipeline import the constituent packages explicitly.

---

## Block Node Types

### DocumentNode

The root of every document. Contains zero or more block-level children.

```typescript
interface DocumentNode {
  readonly type: "document";
  readonly children: readonly BlockNode[];
}
```

Every IR produced by any front-end is a `DocumentNode`. An empty document has
an empty `children` array. The document node is the only node type that cannot
appear as a child of another node.

---

### HeadingNode

A section heading with a depth level. Semantically corresponds to `<h1>`–`<h6>`
in HTML, `=====` / `-----` in RST, `\section{}` in LaTeX, `Heading 1`–`Heading 6`
in DOCX.

```typescript
interface HeadingNode {
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly InlineNode[];
}
```

The `level` range 1–6 is intentionally the same as HTML. Most document formats
support at least six heading levels. If a source format supports more (e.g., RST
has no hard limit), levels beyond 6 are clamped to 6 when converting to this IR.

---

### ParagraphNode

A block of prose. Contains one or more inline nodes, which may include text,
emphasis, links, and soft breaks between the lines of the original source.

```typescript
interface ParagraphNode {
  readonly type: "paragraph";
  readonly children: readonly InlineNode[];
}
```

A paragraph is a universal concept in every document format. Even PDF and DOCX
have paragraph-level objects. Single-line and multi-line paragraphs are both
represented by `ParagraphNode`; the internal line breaks appear as `SoftBreakNode`
children.

---

### CodeBlockNode

A block of literal code or pre-formatted text. The content is raw — it is not
further processed for inline markup or entity references.

```typescript
interface CodeBlockNode {
  readonly type: "code_block";
  readonly language: string | null;  // e.g. "typescript", "python", null
  readonly value: string;            // raw code, including trailing newline
}
```

The `language` field is an optional hint to syntax highlighters. It comes from
the info string in fenced code blocks (Markdown), the `code-block` directive's
`language` option (RST), the `<code class="language-…">` attribute (HTML), etc.
When no language is specified, `language` is `null`.

The `value` field always ends with a `\n`. Renderers should not add additional
newlines.

---

### BlockquoteNode

A block of content attributed to a quotation or aside. Can contain any block
nodes, including nested blockquotes.

```typescript
interface BlockquoteNode {
  readonly type: "blockquote";
  readonly children: readonly BlockNode[];
}
```

In HTML this renders as `<blockquote>…</blockquote>`. In RST, `.. epigraph::`.
In DOCX, a "Quote" style paragraph.

---

### ListNode and ListItemNode

An ordered (numbered) or unordered (bulleted) list. A `ListNode` contains one or
more `ListItemNode` children. Each list item contains block-level content.

```typescript
interface ListNode {
  readonly type: "list";
  readonly ordered: boolean;
  readonly start: number | null;  // first number for ordered lists; null for unordered
  readonly tight: boolean;        // tight = paragraph <p> tags suppressed in HTML
  readonly children: readonly ListItemNode[];
}

interface ListItemNode {
  readonly type: "list_item";
  readonly children: readonly BlockNode[];
}
```

**Tight vs. loose.** The `tight` flag is a rendering hint preserved from the
source format. A tight list is one where list items were written without blank
lines between them; a loose list has blank lines. In HTML, tight lists suppress
`<p>` wrappers around paragraph content. Other back-ends may use this flag
differently or ignore it.

**Ordered list start.** The `start` field records the first list item number
from the source. `1` is the default; `42` means the list begins at forty-two.
`null` for unordered lists.

---

### ThematicBreakNode

A visual separator between sections. No children — it is a leaf node.

```typescript
interface ThematicBreakNode {
  readonly type: "thematic_break";
}
```

In HTML, `<hr />`. In RST, `----`. In plain text, `---`. In DOCX, a horizontal
rule paragraph style.

---

### RawBlockNode

A block of content that should be passed through verbatim to a specific back-end.
The `format` field names the target renderer (e.g. `"html"`, `"latex"`, `"rtf"`).

```typescript
interface RawBlockNode {
  readonly type: "raw_block";
  readonly format: string;  // e.g. "html", "latex", "rtf"
  readonly value: string;   // raw content, verbatim
}
```

**Generalisation of `HtmlBlockNode`.** The CommonMark AST has
`HtmlBlockNode { type: "html_block"; value: string }`. The Document AST
replaces this with `RawBlockNode { type: "raw_block"; format: "html"; value: string }`.
The semantics are identical for Markdown sources, but the type now accommodates
raw blocks from any source format.

**Back-end contract.** A back-end renderer that encounters a `RawBlockNode`
**must** emit the `value` verbatim if `format` matches its own output format, and
**must** skip the node entirely if it does not match. For example, an HTML
back-end renders `RawBlockNode { format: "html" }` verbatim but skips
`RawBlockNode { format: "latex" }`.

```
format field    HTML back-end   LaTeX back-end   Plain-text back-end
──────────────  ─────────────   ──────────────   ───────────────────
"html"          emit verbatim   skip             skip
"latex"         skip            emit verbatim    skip
"rtf"           skip            skip             skip
```

---

## Inline Node Types

Inline nodes live inside block nodes that contain prose content: headings,
paragraphs, and list items. They represent spans of formatted text, links,
images, and structural characters within a line.

### TextNode

Plain text with no markup. All entity references and character references
are decoded before being stored (e.g. `&amp;` → `&`, `&#65;` → `A`). The
value is the decoded Unicode string, ready for display.

```typescript
interface TextNode {
  readonly type: "text";
  readonly value: string;  // decoded Unicode, ready for display
}
```

Adjacent text nodes are automatically merged during inline parsing —
there should never be two consecutive `TextNode` siblings in a well-formed IR.

---

### EmphasisNode

Stressed emphasis. Inline children are the emphasised content.

```typescript
interface EmphasisNode {
  readonly type: "emphasis";
  readonly children: readonly InlineNode[];
}
```

Corresponds to `<em>` in HTML, `*text*` in Markdown, `:emphasis:` in RST,
*italic* text in DOCX. Back-ends render as their conventional "soft emphasis"
form (italic in visual renderers, `_text_` in plain-text renderers).

---

### StrongNode

Strong importance. Inline children are the strongly emphasised content.

```typescript
interface StrongNode {
  readonly type: "strong";
  readonly children: readonly InlineNode[];
}
```

Corresponds to `<strong>` in HTML, `**text**` in Markdown, `**bold**` in RST,
bold text in DOCX. Back-ends render as their conventional "strong emphasis"
form (bold in visual renderers, `**text**` in plain-text renderers).

---

### CodeSpanNode

Inline code. The value is raw text — not decoded for entities or markup.

```typescript
interface CodeSpanNode {
  readonly type: "code_span";
  readonly value: string;  // raw content, not decoded
}
```

Corresponds to `<code>` in HTML, `` `code` `` in Markdown, `` `code` `` in
RST, `code` character style in DOCX. Plain-text back-ends typically render
as backtick-quoted text.

---

### LinkNode

A hyperlink. The `destination` is the fully resolved URL — all reference
indirections have been resolved by the front-end parser. The `title` is an
optional tooltip / hover text. The `children` contain the link's visible text
as inline nodes.

```typescript
interface LinkNode {
  readonly type: "link";
  readonly destination: string;    // fully resolved URL
  readonly title: string | null;   // tooltip, or null
  readonly children: readonly InlineNode[];
}
```

**Resolution contract.** The IR never contains unresolved reference links.
Front-ends that handle reference-style links (Markdown's `[text][label]`)
must resolve them against their link definition maps before emitting a `LinkNode`.
If a reference is unresolvable, the front-end should emit the source text as
`TextNode` children (matching CommonMark's specification behaviour).

Links cannot be nested — a `LinkNode` cannot contain another `LinkNode`.

---

### ImageNode

An embedded image. Like `LinkNode`, `destination` is the fully resolved URL.
The `alt` field is the plain-text alternative text (all inline markup stripped).

```typescript
interface ImageNode {
  readonly type: "image";
  readonly destination: string;    // fully resolved URL
  readonly title: string | null;   // tooltip, or null
  readonly alt: string;            // plain text, markup stripped
}
```

**Alt text.** The `alt` field is a string, not an array of inline nodes,
because `alt` text is by definition a plain-text description for screen
readers and fallback contexts. Markup inside `alt` text is stripped before
the IR is produced. For example, `![**hello**](img.png)` produces
`ImageNode { alt: "hello", … }`.

**Back-end contract.** Back-ends that cannot embed images (plain text,
plain-text email) should render the `alt` text instead.

---

### AutolinkNode

A URL or email address presented as a direct link without custom link text.
The link text in all back-ends is the raw address itself.

```typescript
interface AutolinkNode {
  readonly type: "autolink";
  readonly destination: string;  // the URL or email address, without < >
  readonly isEmail: boolean;     // true → prepend "mailto:" in href
}
```

**Why preserve `isEmail`?** Two reasons:

1. HTML back-ends need to prepend `mailto:` for email autolinks:
   `<https://example.com>` → `<a href="https://example.com">…</a>` but
   `<user@example.com>` → `<a href="mailto:user@example.com">…</a>`.

2. Other back-ends (e.g. PDF, DOCX) may want to format email addresses
   differently from URLs — for example, not underlining email addresses in
   print output.

This distinction is semantically meaningful downstream and would be
unrecoverable if collapsed to a plain `LinkNode`.

---

### RawInlineNode

An inline span of content that should be passed through verbatim to a specific
back-end. The `format` field names the target renderer.

```typescript
interface RawInlineNode {
  readonly type: "raw_inline";
  readonly format: string;  // e.g. "html", "latex"
  readonly value: string;   // raw content, verbatim
}
```

**Generalisation of `HtmlInlineNode`.** The CommonMark AST has
`HtmlInlineNode { type: "html_inline"; value: string }`. The Document AST
replaces this with `RawInlineNode { type: "raw_inline"; format: "html"; value: string }`.

The same back-end contract applies as for `RawBlockNode`: render verbatim if
`format` matches, skip if it does not.

---

### HardBreakNode

A forced line break within a paragraph. Forces `<br>` in HTML, `\newline` in
LaTeX, a literal newline in plain-text renderers.

```typescript
interface HardBreakNode {
  readonly type: "hard_break";
}
```

Produced by two trailing spaces before a newline in Markdown (`\` + newline also
works). RST does not have an equivalent — a `HardBreakNode` in RST-sourced
documents would come from a raw HTML or LaTeX directive.

---

### SoftBreakNode

A soft line break — a newline within a paragraph that is not a hard break. In
HTML, soft breaks render as a single space or a `\n` depending on the renderer.
In plain text, they render as a newline. The back-end controls this.

```typescript
interface SoftBreakNode {
  readonly type: "soft_break";
}
```

The IR preserves soft breaks explicitly so that back-ends that want to control
line-wrapping behaviour can do so. A back-end may also discard soft breaks and
re-wrap paragraphs.

---

## Union Types

```typescript
type BlockNode =
  | DocumentNode
  | HeadingNode
  | ParagraphNode
  | CodeBlockNode
  | BlockquoteNode
  | ListNode
  | ListItemNode
  | ThematicBreakNode
  | RawBlockNode;

type InlineNode =
  | TextNode
  | EmphasisNode
  | StrongNode
  | CodeSpanNode
  | LinkNode
  | ImageNode
  | AutolinkNode
  | RawInlineNode
  | HardBreakNode
  | SoftBreakNode;

type Node = BlockNode | InlineNode;
```

---

## Node Containment Rules

The following table defines the grammar of the AST. A well-formed Document AST
obeys these rules exactly; violations indicate a front-end parser bug.

```
Node               May contain
────────────────────────────────────────────────────────────────────────────
DocumentNode       BlockNode (any except DocumentNode)
HeadingNode        InlineNode
ParagraphNode      InlineNode
CodeBlockNode      (leaf node — content in `value` field)
BlockquoteNode     BlockNode (any except DocumentNode)
ListNode           ListItemNode only
ListItemNode       BlockNode (any except DocumentNode)
ThematicBreakNode  (leaf node)
RawBlockNode       (leaf node — content in `value` field)

TextNode           (leaf node — content in `value` field)
EmphasisNode       InlineNode
StrongNode         InlineNode
CodeSpanNode       (leaf node — content in `value` field)
LinkNode           InlineNode (but not LinkNode — links cannot be nested)
ImageNode          (leaf node — alt text in `alt` string field)
AutolinkNode       (leaf node)
RawInlineNode      (leaf node — content in `value` field)
HardBreakNode      (leaf node)
SoftBreakNode      (leaf node)
```

---

## Differences from the CommonMark AST (TE01)

This table is the precise delta between the Document AST and the CommonMark AST:

| CommonMark AST (TE01)                         | Document AST (TE00)                                   | Reason                                           |
|-----------------------------------------------|-------------------------------------------------------|--------------------------------------------------|
| `LinkDefinitionNode` (block)                  | **Removed**                                           | Markdown parse artifact; links are always resolved in the IR |
| `HtmlBlockNode { type: "html_block" }`        | `RawBlockNode { type: "raw_block"; format: "html" }`  | Generalise to support raw blocks in any target format |
| `HtmlInlineNode { type: "html_inline" }`      | `RawInlineNode { type: "raw_inline"; format: "html" }` | Generalise to support raw inlines in any target format |
| All other node types                          | **Identical**                                         | Semantically universal; no format-specific coupling |

Front-ends that produce the Document AST from CommonMark sources apply a
straightforward mechanical transformation:

```
cm.HtmlBlockNode  → doc.RawBlockNode  { format: "html", value: cm.value }
cm.HtmlInlineNode → doc.RawInlineNode { format: "html", value: cm.value }
cm.LinkDefinitionNode → (resolved internally; not emitted to IR)
```

---

## Complete TypeScript Definitions

All types in one place for implementations to copy.

```typescript
// ─── Block nodes ──────────────────────────────────────────────────────────────

export interface DocumentNode {
  readonly type: "document";
  readonly children: readonly BlockNode[];
}

export interface HeadingNode {
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly InlineNode[];
}

export interface ParagraphNode {
  readonly type: "paragraph";
  readonly children: readonly InlineNode[];
}

export interface CodeBlockNode {
  readonly type: "code_block";
  readonly language: string | null;
  readonly value: string;
}

export interface BlockquoteNode {
  readonly type: "blockquote";
  readonly children: readonly BlockNode[];
}

export interface ListNode {
  readonly type: "list";
  readonly ordered: boolean;
  readonly start: number | null;
  readonly tight: boolean;
  readonly children: readonly ListItemNode[];
}

export interface ListItemNode {
  readonly type: "list_item";
  readonly children: readonly BlockNode[];
}

export interface ThematicBreakNode {
  readonly type: "thematic_break";
}

export interface RawBlockNode {
  readonly type: "raw_block";
  readonly format: string;
  readonly value: string;
}

export type BlockNode =
  | DocumentNode
  | HeadingNode
  | ParagraphNode
  | CodeBlockNode
  | BlockquoteNode
  | ListNode
  | ListItemNode
  | ThematicBreakNode
  | RawBlockNode;

// ─── Inline nodes ─────────────────────────────────────────────────────────────

export interface TextNode {
  readonly type: "text";
  readonly value: string;
}

export interface EmphasisNode {
  readonly type: "emphasis";
  readonly children: readonly InlineNode[];
}

export interface StrongNode {
  readonly type: "strong";
  readonly children: readonly InlineNode[];
}

export interface CodeSpanNode {
  readonly type: "code_span";
  readonly value: string;
}

export interface LinkNode {
  readonly type: "link";
  readonly destination: string;
  readonly title: string | null;
  readonly children: readonly InlineNode[];
}

export interface ImageNode {
  readonly type: "image";
  readonly destination: string;
  readonly title: string | null;
  readonly alt: string;
}

export interface AutolinkNode {
  readonly type: "autolink";
  readonly destination: string;
  readonly isEmail: boolean;
}

export interface RawInlineNode {
  readonly type: "raw_inline";
  readonly format: string;
  readonly value: string;
}

export interface HardBreakNode {
  readonly type: "hard_break";
}

export interface SoftBreakNode {
  readonly type: "soft_break";
}

export type InlineNode =
  | TextNode
  | EmphasisNode
  | StrongNode
  | CodeSpanNode
  | LinkNode
  | ImageNode
  | AutolinkNode
  | RawInlineNode
  | HardBreakNode
  | SoftBreakNode;

export type Node = BlockNode | InlineNode;
```

---

## Package Layout

### `@coding-adventures/document-ast`

Types-only package. No runtime code, no dependencies.

```
code/packages/typescript/document-ast/
  src/
    types.ts        ← The complete TypeScript definitions from this spec
    index.ts        ← Re-exports all types
  package.json      ← { "name": "@coding-adventures/document-ast" }
  BUILD
  README.md
  CHANGELOG.md
```

### `@coding-adventures/commonmark-parser`

Parses a Markdown string and produces a `DocumentNode`.

```
code/packages/typescript/commonmark-parser/
  src/
    block-parser.ts   ← Block phase (from TE01 implementation, adapted)
    inline-parser.ts  ← Inline phase (from TE01 implementation, adapted)
    scanner.ts        ← Cursor-based scanner utility
    entities.ts       ← HTML entity decoding table
    index.ts          ← Exports { parse(markdown: string): DocumentNode }
  tests/
    commonmark.test.ts ← All 652 CommonMark spec examples
  package.json         ← { "name": "@coding-adventures/commonmark-parser" }
  BUILD
  README.md
  CHANGELOG.md
```

### `@coding-adventures/document-ast-to-html`

Renders a `DocumentNode` to an HTML string.

```
code/packages/typescript/document-ast-to-html/
  src/
    html-renderer.ts  ← AST → HTML (from TE01 implementation, adapted)
    index.ts          ← Exports { toHtml(doc: DocumentNode): string }
  tests/
    html-renderer.test.ts
  package.json        ← { "name": "@coding-adventures/document-ast-to-html" }
  BUILD
  README.md
  CHANGELOG.md
```

### `@coding-adventures/commonmark` (pipeline convenience package)

Thin package that pipelines the parser and renderer. No logic of its own.

```
code/packages/typescript/commonmark/
  src/
    index.ts   ← Re-exports { parse } and { toHtml }; exports VERSION
  package.json ← { "name": "@coding-adventures/commonmark" }
  BUILD
  README.md
  CHANGELOG.md
```

---

## Extension Node Types

Extensions to the Document AST are defined in separate specs. They follow the
same pattern: add new node types to the union, never remove or mutate existing
ones.

| Spec  | Title                        | New node types                                                          |
|-------|------------------------------|-------------------------------------------------------------------------|
| TE00  | Document AST (this)          | 19 node types above                                                     |
| TE04  | GFM Extensions               | `TableNode`, `TableRowNode`, `TableCellNode`, `StrikeNode`, `TaskItemNode`, `FencedBlockNode` |
| TE05  | ThunderEgg Dialect           | `WikiLinkNode`, `WikiEmbedNode`, `HighlightNode`, `MathInlineNode`, `MathBlockNode`, `FrontmatterNode`, `CalloutNode` |

GFM node types (TE04):

```typescript
interface TableNode     { readonly type: "table";          readonly align: ReadonlyArray<"left" | "right" | "center" | null>; readonly children: readonly TableRowNode[] }
interface TableRowNode  { readonly type: "table_row";      readonly isHeader: boolean; readonly children: readonly TableCellNode[] }
interface TableCellNode { readonly type: "table_cell";     readonly children: readonly InlineNode[] }
interface StrikeNode    { readonly type: "strikethrough";  readonly children: readonly InlineNode[] }
interface TaskItemNode  { readonly type: "task_item";      readonly checked: boolean; readonly children: readonly BlockNode[] }
interface FencedBlockNode { readonly type: "fenced_block"; readonly name: string | null; readonly info: string | null; readonly value: string }
```

ThunderEgg dialect node types (TE05):

```typescript
interface WikiLinkNode   { readonly type: "wikilink";    readonly target: string; readonly alias: string | null; readonly heading: string | null }
interface WikiEmbedNode  { readonly type: "wiki_embed";  readonly target: string; readonly heading: string | null }
interface HighlightNode  { readonly type: "highlight";   readonly children: readonly InlineNode[] }
interface MathInlineNode { readonly type: "math_inline"; readonly value: string }
interface MathBlockNode  { readonly type: "math_block";  readonly value: string }
interface FrontmatterNode{ readonly type: "frontmatter"; readonly value: string }  // raw YAML
interface CalloutNode    { readonly type: "callout";     readonly kind: string; readonly title: string | null; readonly children: readonly BlockNode[] }
```

---

## Multi-Language Port Plan

The Document AST is implemented in every language that coding-adventures supports.
Each language port provides the type definitions for its language and follows the
same immutable, discriminated-union-style design.

| Language   | Package name                                             | Location                                    |
|------------|----------------------------------------------------------|---------------------------------------------|
| TypeScript | `@coding-adventures/document-ast`                        | `code/packages/typescript/document-ast/`    |
| Python     | `coding_adventures_document_ast`                         | `code/packages/python/document-ast/`        |
| Go         | `coding-adventures/document-ast`                         | `code/packages/go/document-ast/`            |
| Ruby       | `coding_adventures-document_ast`                         | `code/packages/ruby/document-ast/`          |
| Rust       | `coding_adventures_document_ast`                         | `code/packages/rust/document-ast/`          |
| Elixir     | `CodingAdventures.DocumentAST`                           | `code/packages/elixir/document-ast/`        |
| Lua        | `coding_adventures.document_ast`                         | `code/packages/lua/document-ast/`           |

### Language-idiomatic type representations

**TypeScript:** interfaces with `readonly` fields, discriminated union types using
string literal `type` fields. Exhaustive `switch` statements verified by the
TypeScript compiler.

**Python:** `dataclasses` with `frozen=True`. Union types as
`Union[TextNode, EmphasisNode, ...]`. The `type` field is a `Literal["text"]`
string, enabling `match`/`case` dispatch in Python 3.10+.

**Go:** Structs with exported fields. Union types as a `Node` interface with a
`NodeType() string` method; each concrete struct implements the interface.

**Ruby:** `Data` (Ruby 3.2+) for frozen value objects. `type` symbol for
pattern matching dispatch.

**Rust:** `enum` types. All variants carry struct-like fields inline. `#[derive(Debug,
Clone, PartialEq)]` on all types. The top-level `Node` is a single enum.

**Elixir:** Structs (`defstruct`) with `@type` specs. Pattern match on the `type`
atom field. All structs are frozen by convention.

**Lua:** Tables with a `type` string field. No static types — the spec is the
contract.
