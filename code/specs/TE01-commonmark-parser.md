# TE01 — CommonMark Parser

## Overview

CommonMark is a precise, unambiguous specification of Markdown, published in 2014 by
John MacFarlane and collaborators who were frustrated by a decade of incompatible
Markdown implementations. Before CommonMark, every tool — GitHub, Pandoc, Python-Markdown,
Discount — would render the same source text differently at edge cases. CommonMark defines
652 canonical examples and an exact algorithm for resolving every ambiguity.

This package, `commonmark`, implements a CommonMark 0.31.2 compliant parser that converts
a Markdown string into a typed Abstract Syntax Tree (AST). The AST is the foundational
data structure for the thunderegg note-taking application and all downstream tools
(renderers, search indexers, backlink analyzers, syntax highlighters).

### Why no formal grammar?

Most programming languages have a context-free grammar expressible in BNF or EBNF that
you can feed into a parser generator like yacc or ANTLR. Markdown does not. It is
context-sensitive in three important ways:

**1. Two-pass structure.** Block-level structure (headings, lists, code fences) must be
fully resolved before inline content (emphasis, links, code spans) can be parsed within
each block. You cannot interleave the two passes.

**2. Indentation is structural.** Whether a line continues a list item, starts a new
block, or is indented code depends on the column position of its first non-space
character — not just the sequence of tokens.

**3. Emphasis is context-sensitive.** Whether `*` opens or closes emphasis depends on
surrounding whitespace and punctuation on both sides of the delimiter. CommonMark
Appendix A defines 17 rules for delimiter matching. No regular expression captures this.

Because of these properties, CommonMark parsers are hand-rolled in two phases:

```
Phase 1: Block structure
  Input text → scan line by line → identify block containers and leaf blocks
  Output: a tree of block tokens with raw inline content strings

Phase 2: Inline content
  For each leaf block's raw content → scan character by character
  Resolve emphasis, links, images, code spans using a delimiter stack
  Output: inline node trees that replace raw content strings
```

This spec defines the AST produced by both phases. The parsing algorithm and its
connection to the state-machine library (`@coding-adventures/state-machine`) are
specified in TE02. The HTML renderer is specified in TE03.

---

## Layer Position

```
┌─────────────────────────────────────────────────────────────┐
│                      thunderegg app                          │
│              (editor, file index, sync layer)                │
└──────────────────────┬──────────────────────────────────────┘
                       │ uses
┌──────────────────────▼──────────────────────────────────────┐
│                   commonmark (this package)                  │
│  parse(input) → DocumentNode                                 │
│  toHtml(node) → string                                       │
└──────────────────────┬──────────────────────────────────────┘
                       │ uses
┌──────────────────────▼──────────────────────────────────────┐
│              @coding-adventures/state-machine                 │
│  ModalStateMachine — block tokenizer                         │
│  PushdownAutomaton — inline delimiter stack                  │
└─────────────────────────────────────────────────────────────┘
```

---

## The AST

An Abstract Syntax Tree (AST) is a tree-shaped data structure that represents the
semantic structure of a document — not the raw characters, but *what they mean*.

Consider this Markdown:

```markdown
## Hello, *world*

This is a [link](https://example.com).
```

The AST for this document is:

```
DocumentNode
├── HeadingNode (level: 2)
│   ├── TextNode ("Hello, ")
│   └── EmphasisNode
│       └── TextNode ("world")
└── ParagraphNode
    ├── TextNode ("This is a ")
    ├── LinkNode (destination: "https://example.com", title: null)
    │   └── TextNode ("link")
    └── TextNode (".")
```

The tree has two kinds of nodes:

- **Block nodes** — structural containers: document, heading, paragraph, list, blockquote,
  code block, thematic break, HTML block, link definition.
- **Inline nodes** — content within a block: text, emphasis, strong, code span, link,
  image, autolink, HTML inline, hard break, soft break.

Block nodes can contain other block nodes (e.g., a blockquote contains paragraphs) or
inline nodes (e.g., a heading contains text and emphasis). Inline nodes can contain
other inline nodes (e.g., a link contains text). This creates a well-typed tree.

---

## Block Node Types

### DocumentNode

The root of every parsed document. Contains zero or more block-level children.

```typescript
interface DocumentNode {
  readonly type: "document";
  readonly children: readonly BlockNode[];
}
```

A document always has exactly one `DocumentNode` as its root. Even an empty string
produces a `DocumentNode` with an empty `children` array.

---

### HeadingNode

ATX headings (`#` through `######`) and setext headings (underlined with `=` or `-`).
The `level` field is the heading depth: 1 is `<h1>`, 6 is `<h6>`.

```typescript
interface HeadingNode {
  readonly type: "heading";
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly InlineNode[];
}
```

**Examples:**

```markdown
# Level 1        → HeadingNode { level: 1 }
## Level 2       → HeadingNode { level: 2 }
###### Level 6   → HeadingNode { level: 6 }

Setext Level 1   → HeadingNode { level: 1 }
==============

Setext Level 2   → HeadingNode { level: 2 }
--------------
```

The `children` array contains inline nodes parsed from the heading text. A heading
like `## Hello, *world*` produces:

```typescript
{
  type: "heading",
  level: 2,
  children: [
    { type: "text", value: "Hello, " },
    { type: "emphasis", children: [{ type: "text", value: "world" }] },
  ],
}
```

---

### ParagraphNode

A sequence of non-blank lines that does not begin a more specific block structure.
Paragraphs are the most common block node — any text that is not a heading, list,
code block, or other container becomes a paragraph.

```typescript
interface ParagraphNode {
  readonly type: "paragraph";
  readonly children: readonly InlineNode[];
}
```

Multiple lines within a single paragraph are joined with soft breaks:

```markdown
This is line one.
This is line two.
```

produces:

```typescript
{
  type: "paragraph",
  children: [
    { type: "text", value: "This is line one." },
    { type: "soft_break" },
    { type: "text", value: "This is line two." },
  ],
}
```

---

### CodeBlockNode

Fenced code blocks (``` ``` ``` or `~~~`) and indented code blocks (4+ spaces).
The `language` field comes from the info string of a fenced code block (`null` for
indented code blocks and fenced blocks with no info string). The `value` field contains
the raw code text with no further processing.

```typescript
interface CodeBlockNode {
  readonly type: "code_block";
  readonly language: string | null;
  readonly value: string;
}
```

**Examples:**

````markdown
```typescript
const x = 1;
```
````

produces `CodeBlockNode { language: "typescript", value: "const x = 1;\n" }`.

````markdown
    indented code
````

produces `CodeBlockNode { language: null, value: "indented code\n" }`.

The info string in a fenced block may contain spaces and additional text after the
language name (e.g., ` ```typescript title="example" `). Only the first word is used
as the language. The rest is discarded in CommonMark (extensions may preserve it).

---

### BlockquoteNode

A blockquote is a block container introduced by `>` at the start of each line.
It can contain any block nodes, including other blockquotes (nested blockquotes).

```typescript
interface BlockquoteNode {
  readonly type: "blockquote";
  readonly children: readonly BlockNode[];
}
```

**Example:**

```markdown
> This is a quote.
>
> > Nested quote.
```

produces:

```typescript
{
  type: "blockquote",
  children: [
    { type: "paragraph", children: [{ type: "text", value: "This is a quote." }] },
    {
      type: "blockquote",
      children: [
        { type: "paragraph", children: [{ type: "text", value: "Nested quote." }] },
      ],
    },
  ],
}
```

---

### ListNode and ListItemNode

Lists are either ordered (`1.`, `2.`, `3.` or `1)`, `2)`, `3)`) or unordered
(`-`, `*`, or `+`). A `ListNode` contains one or more `ListItemNode` children.
Each `ListItemNode` contains block nodes (the item's content).

```typescript
interface ListNode {
  readonly type: "list";
  readonly ordered: boolean;
  readonly start: number | null;  // start number for ordered lists; null for unordered
  readonly tight: boolean;        // tight = no blank lines between items
  readonly children: readonly ListItemNode[];
}

interface ListItemNode {
  readonly type: "list_item";
  readonly children: readonly BlockNode[];
}
```

**Tight vs. loose lists.** A list is *tight* if none of its constituent list items
are separated by blank lines and no list item contains a blank line. A tight list
renders its items' paragraph content without `<p>` tags in HTML. A loose list wraps
each item's paragraph in `<p>` tags.

```markdown
- apple      ← tight list (no blank lines between items)
- banana
- cherry

- apple      ← loose list (blank lines between items)

- banana

- cherry
```

**Ordered list start.** The first number in an ordered list sets the `start` value.
`1. item` has `start: 1`. `42. item` has `start: 42`.

```markdown
42. forty-two   → ListNode { ordered: true, start: 42 }
43. forty-three
```

---

### ThematicBreakNode

A horizontal rule: three or more `*`, `-`, or `_` characters on a line (optionally
separated by spaces). Produces no children — it is a leaf node.

```typescript
interface ThematicBreakNode {
  readonly type: "thematic_break";
}
```

**Examples:** `---`, `***`, `___`, `- - -`, `* * *` all produce `ThematicBreakNode`.

Note: `---` is a thematic break only when not immediately following a paragraph
(in which case it would be a setext heading marker). The block phase resolves this.

---

### HtmlBlockNode

Raw HTML blocks passthrough verbatim. CommonMark defines seven types of HTML blocks
(e.g., `<script>`, `<!-- comment -->`, `<?processing?>`, `<div>`). Their content is
preserved exactly as written and is not further processed.

```typescript
interface HtmlBlockNode {
  readonly type: "html_block";
  readonly value: string;
}
```

**Example:**

```markdown
<div class="note">
  Some raw HTML.
</div>
```

produces `HtmlBlockNode { value: "<div class=\"note\">\n  Some raw HTML.\n</div>\n" }`.

---

### LinkDefinitionNode

Link reference definitions are not rendered directly — they define labels that can
be referenced by links and images elsewhere in the document. They are present in the
AST so that consumers can inspect all definitions, build link maps, and detect
broken references.

```typescript
interface LinkDefinitionNode {
  readonly type: "link_definition";
  readonly label: string;       // normalized label (case-folded, whitespace-collapsed)
  readonly destination: string; // URL
  readonly title: string | null;
}
```

**Example:**

```markdown
[example]: https://example.com "Example Site"
```

produces:

```typescript
{
  type: "link_definition",
  label: "example",
  destination: "https://example.com",
  title: "Example Site",
}
```

Labels are normalized: case-folded to lowercase and internal whitespace collapsed to
a single space. `[Example]`, `[EXAMPLE]`, and `[  example  ]` all resolve to the
same label `"example"`.

---

### BlockNode union type

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
  | HtmlBlockNode
  | LinkDefinitionNode;
```

---

## Inline Node Types

Inline nodes live inside block nodes that contain prose: headings, paragraphs, and
list items. They represent spans of formatted text, links, images, and special
characters within a block.

### TextNode

Plain text with no special markup. All characters that do not trigger other inline
constructs are accumulated into `TextNode` values. Entity references and numeric
character references are decoded: `&amp;` → `&`, `&#65;` → `A`, `&#x41;` → `A`.

```typescript
interface TextNode {
  readonly type: "text";
  readonly value: string; // decoded, ready for display
}
```

---

### EmphasisNode

Single `*text*` or `_text_`. Renders as `<em>` in HTML. Contains inline children
(emphasis can span text, code spans, and other inline constructs).

```typescript
interface EmphasisNode {
  readonly type: "emphasis";
  readonly children: readonly InlineNode[];
}
```

The matching rules for `*` and `_` are the most complex part of CommonMark.
A delimiter run can open emphasis if it is left-flanking and either uses `*` or is
not right-flanking. A delimiter run can close emphasis if it is right-flanking and
either uses `*` or is not left-flanking. CommonMark Appendix A defines the full
algorithm; the inline parser implements it exactly.

---

### StrongNode

Double `**text**` or `__text__`. Renders as `<strong>` in HTML. Strong and emphasis
can nest arbitrarily: `***text***` can produce `<em><strong>text</strong></em>` or
`<strong><em>text</em></strong>` depending on context.

```typescript
interface StrongNode {
  readonly type: "strong";
  readonly children: readonly InlineNode[];
}
```

---

### CodeSpanNode

Inline code delimited by backtick strings: `` `code` `` or ` ``code`` `. The content
is the raw text between the delimiters (after stripping one leading/trailing space
if both are present). It is not further processed for HTML entities or Markdown.

```typescript
interface CodeSpanNode {
  readonly type: "code_span";
  readonly value: string; // raw content, not decoded
}
```

**Backtick matching.** The opening backtick string length must equal the closing
backtick string length. `` `foo `` and `` `foo` `` use different lengths and do not
match. This allows backticks inside code spans: `` `foo`bar`baz` `` is ambiguous,
but ` ``foo`bar`` ` is a single code span containing `foo`bar`.

---

### LinkNode

An inline link `[text](destination "title")` or a reference link `[text][label]`.
Reference links are resolved against `LinkDefinitionNode` entries in the document.
After resolution, all links are represented as `LinkNode` with an explicit
`destination` — the reference indirection is not preserved in the AST.

```typescript
interface LinkNode {
  readonly type: "link";
  readonly destination: string;
  readonly title: string | null;
  readonly children: readonly InlineNode[]; // the link text, which may contain inline nodes
}
```

Links cannot contain other links (no nested links). If a `[` opener is found inside
a link's text, it does not start a new link.

---

### ImageNode

An inline image `![alt text](destination "title")` or `![alt text][label]`. Similar
to `LinkNode` but the `alt` field is the plain-text rendering of the alt text content
(all markup stripped). Images cannot nest.

```typescript
interface ImageNode {
  readonly type: "image";
  readonly destination: string;
  readonly title: string | null;
  readonly alt: string; // plain text alternative, markup stripped
}
```

---

### AutolinkNode

A URL or email address enclosed in angle brackets: `<https://example.com>` or
`<user@example.com>`. The `isEmail` field distinguishes email autolinks from URL
autolinks.

```typescript
interface AutolinkNode {
  readonly type: "autolink";
  readonly destination: string; // the URL or email address (without angle brackets)
  readonly isEmail: boolean;
}
```

---

### HtmlInlineNode

Raw HTML tags and character references that appear inline (as opposed to HTML blocks,
which are block-level). Includes open tags, closing tags, comments, processing
instructions, and declarations found within paragraph text.

```typescript
interface HtmlInlineNode {
  readonly type: "html_inline";
  readonly value: string; // the raw HTML tag or entity, verbatim
}
```

---

### HardBreakNode

A hard line break forces a `<br>` in HTML output. Produced by two or more spaces
followed by a newline, or a backslash `\` followed by a newline.

```typescript
interface HardBreakNode {
  readonly type: "hard_break";
}
```

---

### SoftBreakNode

A soft line break: a single newline within a paragraph that is not a hard break.
In HTML output, this is typically rendered as a space or newline (renderers may
choose either). In the AST it is preserved as a distinct node so that renderers
can control the behaviour.

```typescript
interface SoftBreakNode {
  readonly type: "soft_break";
}
```

---

### InlineNode union type

```typescript
type InlineNode =
  | TextNode
  | EmphasisNode
  | StrongNode
  | CodeSpanNode
  | LinkNode
  | ImageNode
  | AutolinkNode
  | HtmlInlineNode
  | HardBreakNode
  | SoftBreakNode;
```

---

## Combined Node type

```typescript
type Node = BlockNode | InlineNode;
```

---

## Node Containment Rules

The following table summarizes which node types can contain which children.
This is the grammar of the AST — violations are a parser bug.

```
Node              May contain
─────────────────────────────────────────────────────────────────
DocumentNode      BlockNode (any except DocumentNode)
HeadingNode       InlineNode
ParagraphNode     InlineNode
CodeBlockNode     (no children — leaf node, content in `value`)
BlockquoteNode    BlockNode (any except DocumentNode)
ListNode          ListItemNode only
ListItemNode      BlockNode (any except DocumentNode)
ThematicBreakNode (no children — leaf node)
HtmlBlockNode     (no children — leaf node, content in `value`)
LinkDefinitionNode(no children — leaf node)

TextNode          (no children — leaf node, content in `value`)
EmphasisNode      InlineNode
StrongNode        InlineNode
CodeSpanNode      (no children — leaf node, content in `value`)
LinkNode          InlineNode (but not LinkNode — no nested links)
ImageNode         (no children — alt is plain text string, not nodes)
AutolinkNode      (no children — leaf node)
HtmlInlineNode    (no children — leaf node, content in `value`)
HardBreakNode     (no children — leaf node)
SoftBreakNode     (no children — leaf node)
```

---

## Complete TypeScript Definitions

All types collected in one place for easy reference in implementations.

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

export interface HtmlBlockNode {
  readonly type: "html_block";
  readonly value: string;
}

export interface LinkDefinitionNode {
  readonly type: "link_definition";
  readonly label: string;
  readonly destination: string;
  readonly title: string | null;
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
  | HtmlBlockNode
  | LinkDefinitionNode;

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

export interface HtmlInlineNode {
  readonly type: "html_inline";
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
  | HtmlInlineNode
  | HardBreakNode
  | SoftBreakNode;

export type Node = BlockNode | InlineNode;
```

---

## Future Specs in This Series

The AST defined in this spec is the stable foundation. Future specs extend it:

| Spec  | Title                        | Extends   | New node types                                      |
|-------|------------------------------|-----------|-----------------------------------------------------|
| TE01  | CommonMark Parser (this)     | —         | All 20 node types above                             |
| TE02  | Parsing Algorithm            | TE01      | None — specifies how to produce TE01 AST            |
| TE03  | HTML Renderer                | TE01      | None — specifies toHtml(DocumentNode)               |
| TE04  | GFM Extensions               | TE01–03   | TableNode, TableRowNode, TableCellNode, StrikeNode, TaskListItemNode, FencedBlockNode |
| TE05  | ThunderEgg Dialect           | TE01–04   | WikiLinkNode, WikiEmbedNode, HighlightNode, MathNode, FrontmatterNode, CalloutNode |

GFM node types (TE04):

```typescript
// Added in TE04 — not part of CommonMark
interface TableNode      { type: "table";      align: ("left"|"right"|"center"|null)[]; children: readonly TableRowNode[] }
interface TableRowNode   { type: "table_row";  isHeader: boolean; children: readonly TableCellNode[] }
interface TableCellNode  { type: "table_cell"; children: readonly InlineNode[] }
interface StrikeNode     { type: "strikethrough"; children: readonly InlineNode[] }
interface TaskItemNode   { type: "task_item";  checked: boolean; children: readonly BlockNode[] }
interface FencedBlockNode { type: "fenced_block"; name: string | null; info: string | null; value: string }
```

ThunderEgg dialect node types (TE05):

```typescript
// Added in TE05 — not part of CommonMark or GFM
interface WikiLinkNode   { type: "wikilink";    target: string; alias: string | null; heading: string | null }
interface WikiEmbedNode  { type: "wiki_embed";  target: string; heading: string | null }
interface HighlightNode  { type: "highlight";   children: readonly InlineNode[] }
interface MathInlineNode { type: "math_inline"; value: string }
interface MathBlockNode  { type: "math_block";  value: string }
interface FrontmatterNode{ type: "frontmatter"; value: string } // raw YAML
interface CalloutNode    { type: "callout";     kind: string; title: string | null; children: readonly BlockNode[] }
```

---

## Multi-Language Port Plan

The AST types defined in this spec are implemented in every language the
coding-adventures repository supports. The TypeScript implementation is the prototype
and the compliance baseline. Once it passes all 652 CommonMark spec tests, the same
AST structure is ported to each language using idiomatic representations.

### Prototype: TypeScript

```
code/packages/typescript/commonmark/
  src/
    types.ts      ← The AST types from this spec
    index.ts      ← Public exports
  tests/
    ast.test.ts   ← Unit tests for AST construction helpers
  package.json    ← { "name": "@coding-adventures/commonmark" }
  BUILD
  README.md
  CHANGELOG.md
```

### Port targets

| Language   | Package name                                           | Location                              |
|------------|--------------------------------------------------------|---------------------------------------|
| Python     | `coding_adventures_commonmark`                         | `code/packages/python/commonmark/`    |
| Go         | `coding-adventures/commonmark`                         | `code/packages/go/commonmark/`        |
| Ruby       | `coding_adventures-commonmark`                         | `code/packages/ruby/commonmark/`      |
| Rust       | `coding_adventures_commonmark`                         | `code/packages/rust/commonmark/`      |
| Elixir     | `CodingAdventures.CommonMark`                          | `code/packages/elixir/commonmark/`    |
| Lua        | `coding_adventures.commonmark`                         | `code/packages/lua/commonmark/`       |
| WASM       | compiled from Rust port                                | `code/packages/wasm/commonmark/`      |

### Language-idiomatic AST representations

**TypeScript:** interfaces with `readonly` fields and discriminated union types
(`type` field as string literal for exhaustive `switch` statements).

**Python:** `dataclasses` with `frozen=True`. Union types expressed as
`Union[TextNode, EmphasisNode, ...]` or a base `Node` class with subclasses.

**Go:** structs with exported fields. Union types expressed as a `Node` interface
with a `Type() string` method; each concrete type implements the interface.

**Ruby:** frozen `Struct` or `Data` (Ruby 3.2+) definitions. Duck typing for
node unions.

**Rust:** `enum` types with variants carrying struct-like fields. `#[derive(Debug,
Clone, PartialEq)]` on all types. The union `Node` is a single enum with all
variants.

**Elixir:** structs (`defstruct`) with `@type` specs. Pattern matching on the
`type` field for dispatch.

**Lua:** tables with a `type` string field. No static typing — the spec serves as
the documentation contract.

---

## Testing Strategy

### CommonMark spec test suite

The authoritative test oracle is the CommonMark 0.31.2 specification test suite,
available as a JSON file at:

```
https://spec.commonmark.org/0.31.2/spec.json
```

Each entry has the shape:

```json
{
  "markdown": "# Hello\n",
  "html":     "<h1>Hello</h1>\n",
  "example":  1,
  "start_line": 1,
  "end_line":   6,
  "section":  "ATX headings"
}
```

The test strategy:

1. Fetch `spec.json` and commit it to `tests/fixtures/commonmark-spec.json`.
2. For each of the 652 examples, run: `toHtml(parse(example.markdown))`.
3. Assert the output equals `example.html` (exact string match).
4. Report which spec sections pass/fail for incremental development visibility.

A parser that passes all 652 examples is considered CommonMark 0.31.2 compliant.

### Unit tests

Beyond the spec suite, each node type has its own unit tests that construct AST
nodes directly and verify their structure. These catch regressions in the AST
type definitions before the full parser is wired up.

```typescript
// Example unit test for HeadingNode shape
it("HeadingNode has correct shape", () => {
  const node: HeadingNode = {
    type: "heading",
    level: 2,
    children: [{ type: "text", value: "Hello" }],
  };
  expect(node.type).toBe("heading");
  expect(node.level).toBe(2);
  expect(node.children).toHaveLength(1);
});
```

### Coverage target

95%+ line coverage across all source files. The CommonMark spec suite provides
broad integration coverage; unit tests fill the gaps for error paths and edge cases.
