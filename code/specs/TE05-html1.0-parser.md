# TE05 — HTML 1.0 Parser

## Overview

The HTML 1.0 lexer (TE04) breaks raw HTML text into tokens — start tags, end
tags, text, comments, doctypes. But tokens alone are flat. A sequence of tokens
like `StartTag("ul"), StartTag("li"), Text("A"), EndTag("li"), EndTag("ul")` does
not convey that `"A"` is *inside* a list item that is *inside* an unordered list.
That hierarchical structure — the tree — is what the parser builds.

This package is the **tree builder**. It consumes the flat token stream from
`html1.0-lexer` (TE04) and produces a `DocumentNode` tree using the node types
defined in `document-ast` (TE00). It understands tag semantics: which tags can
nest inside which, which tags implicitly close their predecessors, and how each
HTML element maps to a DocumentAST node.

### Historical Context

HTML 1.0 was born in the Mosaic era (1993). Marc Andreessen's NCSA Mosaic was
the first graphical web browser to reach a wide audience, and it had one
overriding design principle: **never crash on bad input**. The HTML that Mosaic
encountered was written by physicists, not programmers. Tags were unclosed,
nesting was wrong, `<body>` was often missing entirely. Mosaic rendered all of
it, because a browser that refused to display a page was a browser nobody used.

This permissiveness was later codified as **Postel's Law** (RFC 793, originally
about TCP): *"Be conservative in what you send, be liberal in what you accept."*
The HTML 1.0 parser embodies this principle. It never fails. It never returns an
error. Given any string — valid HTML, garbled HTML, or no HTML at all — it
produces a well-formed DocumentAST tree.

### The Construction Foreman Analogy

Think of the parser as a construction foreman building a house from a delivery
of materials (tokens). The lexer is the truck driver who delivers labelled boxes:
"start wall", "add window", "end wall", "start roof". The foreman's job is to
assemble these into a standing structure.

But the delivery manifest is often wrong. Sometimes the "end wall" box never
arrives — the foreman closes the wall anyway when the roof materials show up,
because a roof cannot sit inside a wall. Sometimes a "start wall" box arrives
when there is already an open wall — the foreman finishes the first wall before
starting the second. Sometimes boxes arrive with labels the foreman does not
recognise — they get set aside, but the contents (text, images) are still used.

The foreman never sends the truck back. The house always gets built.

### The 22 Tags

HTML 1.0 (the Mosaic subset) defined 22 tags. These are the only tags the parser
understands semantically. Any other tag is treated as unknown — silently ignored,
but its children are still processed.

**Structure tags** (document scaffolding):

    html, head, title, body

**Block-level tags** (things that stack vertically on the page):

    h1, h2, h3, h4, h5, h6, p, ul, ol, li, pre, blockquote, hr, br

**Inline tags** (things that flow within a line of text):

    a, img, b, i, em, strong, code

The distinction between block and inline matters for implicit close rules (see
below). A block element like `<ul>` can contain other blocks and inlines. An
inline element like `<b>` should only contain other inlines and text.

---

## Where It Fits

```
html1.0-lexer (TE04) → html1.0-parser (TE05) → document-ast (TE00)
                             ↑ THIS PACKAGE         ↓
                                              document-ast-to-layout (UI06)
                                                     ↓
                                              layout-block → layout-to-paint → PaintVM
```

**Depends on:** `html1.0-lexer` (TE04) for tokenization, `document-ast` (TE00)
for the output tree types.

**Depended on by:** The Venture browser (BR01), which uses this parser to turn
downloaded HTML pages into renderable document trees.

**Follows the pattern of:** `commonmark-parser` in this repo. Both parsers
produce the same output type (`DocumentNode` from TE00) but consume different
input formats. CommonMark parses Markdown syntax; this parser parses HTML syntax.
The shared output type means downstream consumers (layout, rendering, export) do
not need to know which input format produced the tree.

---

## Concepts

### What Is a Parser?

A parser is the second stage of a language processor, after the lexer. Where the
lexer answers "what are the words?", the parser answers "what is the sentence
structure?" In linguistic terms, the lexer performs *lexical analysis* and the
parser performs *syntactic analysis*.

Consider this analogy: you receive a telegram (remember, it is 1993). The
telegraph operator has already decoded the Morse code into letters and grouped
them into words — that is the lexer's job. Now you, the reader, must understand
the sentence structure. "MEETING CANCELLED STOP ARRIVE TUESDAY STOP" — two
sentences, separated by STOP markers. The parser groups words into sentences and
sentences into a message.

For HTML, the parser groups tokens into a tree:

```
Input tokens:
  StartTag("html")
  StartTag("body")
  StartTag("p")
  Text("Hello, ")
  StartTag("b")
  Text("world")
  EndTag("b")
  EndTag("p")
  EndTag("body")
  EndTag("html")

Parser output (tree):
  DocumentNode
  └── ParagraphNode
      ├── TextNode("Hello, ")
      └── StrongNode
          └── TextNode("world")
```

Notice that the structure tags (`html`, `body`) do not appear in the output tree.
They are scaffolding — the parser uses them to understand document structure, but
the DocumentAST represents *content*, not HTML syntax.

### Tag → DocumentAST Mapping

This is the core of the parser. Each HTML tag maps to a specific DocumentAST node
type from TE00. This table is the Rosetta Stone between the HTML world and the
DocumentAST world:

| HTML Tag | DocumentAST Node | Notes |
|----------|-----------------|-------|
| `h1`–`h6` | `HeadingNode { level: 1..6 }` | Level extracted from tag name |
| `p` | `ParagraphNode` | |
| `ul` | `ListNode { ordered: false, tight: false }` | Unordered list |
| `ol` | `ListNode { ordered: true, tight: false }` | Ordered list |
| `li` | `ListItemNode` | |
| `pre` | `CodeBlockNode { language: None }` | Whitespace is preserved verbatim |
| `blockquote` | `BlockQuoteNode` | |
| `hr` | `ThematicBreakNode` | Self-closing, no children |
| `br` | `LineBreakNode` | Self-closing, no children |
| `a` | `LinkNode { destination: href, title: None }` | `href` attribute becomes destination |
| `img` | `ImageNode { destination: src, alt: alt }` | `src` → destination, `alt` → description |
| `b` | `StrongNode` | Alias for `strong` |
| `strong` | `StrongNode` | Semantic strong emphasis |
| `i` | `EmphasisNode` | Alias for `em` |
| `em` | `EmphasisNode` | Semantic emphasis |
| `code` | `CodeSpanNode` | Inline code |
| *(text)* | `TextNode` | Plain character data |

A few things to notice:

1. **`<b>` and `<strong>` produce the same node.** In 1993, `<b>` meant "bold"
   (a visual instruction) and `<strong>` meant "strong emphasis" (a semantic
   instruction). DocumentAST is semantic, so both map to `StrongNode`. Same for
   `<i>` and `<em>` → `EmphasisNode`.

2. **`<hr>` and `<br>` are self-closing.** They have no children and no end tag.
   The parser must recognise them as void elements and never push them onto the
   open-elements stack.

3. **`<pre>` preserves whitespace.** In all other contexts, the parser may
   collapse runs of whitespace. Inside `<pre>`, every space, tab, and newline
   must be preserved exactly as it appears in the source.

4. **`<a>` extracts the `href` attribute.** The parser must dig into the
   token's attribute list to find the destination URL. Similarly, `<img>` needs
   both `src` and `alt`.

### Implicit Close Rules

This is what makes HTML parsing genuinely interesting — and genuinely different
from parsing a well-structured language like XML or JSON.

In XML, every start tag must have a matching end tag. In HTML 1.0, end tags are
often optional. The parser must infer when elements close based on context. These
are called **implicit close rules**.

Think of it like a conversation. In English, you do not always say "end of
sentence" — the listener infers it from context. When someone says "I went to the
store. Then I went home.", the period implicitly closes the first sentence before
the second one begins. HTML works the same way with certain tags.

The rules for HTML 1.0:

**Rule 1: `<p>` closes any open `<p>`.**

A paragraph cannot contain another paragraph. When the parser encounters a new
`<p>` start tag while a `<p>` is already open, it implicitly closes the open one.

```html
<p>First paragraph
<p>Second paragraph
```

Produces:

```
├── ParagraphNode
│   └── TextNode("First paragraph\n")
├── ParagraphNode
│   └── TextNode("Second paragraph\n")
```

**Rule 2: `<li>` closes any open `<li>`.**

List items behave like paragraphs — a new `<li>` closes the previous one.

```html
<ul>
  <li>Alpha
  <li>Beta
  <li>Gamma
</ul>
```

Produces:

```
ListNode { ordered: false }
├── ListItemNode
│   └── TextNode("Alpha\n  ")
├── ListItemNode
│   └── TextNode("Beta\n  ")
├── ListItemNode
│   └── TextNode("Gamma\n")
```

**Rule 3: `<h1>`–`<h6>` close any open `<p>`.**

A heading is a block element that forces any open paragraph to close.

```html
<p>Some text
<h1>A Heading</h1>
```

Produces:

```
├── ParagraphNode
│   └── TextNode("Some text\n")
├── HeadingNode { level: 1 }
│   └── TextNode("A Heading")
```

**Rule 4: Block elements close any open `<p>`.**

Any block-level start tag (`<ul>`, `<ol>`, `<blockquote>`, `<pre>`, `<hr>`)
implicitly closes an open `<p>`, because block elements cannot nest inside
paragraphs.

```html
<p>Before the list
<ul><li>Item</li></ul>
```

Produces:

```
├── ParagraphNode
│   └── TextNode("Before the list\n")
├── ListNode { ordered: false }
│   └── ListItemNode
│       └── TextNode("Item")
```

**Rule 5: `</body>` and `</html>` close all open elements.**

These end tags signal the end of the document. Any elements still on the open
stack are implicitly closed in reverse order.

### Error Recovery

The parser **never fails**. This is a hard requirement, not a nice-to-have. Any
string of bytes, no matter how malformed, must produce a valid DocumentAST tree.
Here are the specific recovery strategies:

**Unknown tags: silently ignored, children processed.**

```html
<blink>This text is visible</blink>
```

The parser does not recognise `<blink>` (it is not one of the 22 tags). It
ignores the start and end tags but processes the text content normally, appending
it to the current open element.

**Missing `<html>`, `<head>`, `<body>`: implied.**

```html
<h1>Hello</h1>
```

There is no `<html>` or `<body>` wrapper. The parser implies them — all content
is treated as body content. The output is exactly the same as if `<html><body>`
had been present.

**Missing close tags: closed at end of parent or document.**

```html
<ul><li>One<li>Two</ul>
```

The `<li>` elements are never explicitly closed. Rule 2 closes the first `<li>`
when the second starts, and `</ul>` closes the second `<li>` because it closes
its parent.

**Overlapping tags: simplified adoption agency.**

```html
<b><i>bold italic</b> just italic?</i>
```

Strictly, the nesting is wrong — `</b>` closes before `</i>`, but `<i>` was
opened inside `<b>`. The simplified adoption agency algorithm handles this: when
`</b>` is encountered, the parser closes `<i>` first (because it is inner), then
closes `<b>`. The remaining ` just italic?` text is appended to the parent.

**Duplicate `<body>` or `<head>`: ignored, content merged.**

```html
<body><p>First</p></body><body><p>Second</p></body>
```

The second `<body>` tag is ignored. Its contents are merged into the first body.

**Text outside `<body>`: treated as body content.**

```html
<html>Some text<body><p>More text</p></body></html>
```

"Some text" appears before `<body>`. The parser treats it as body content.

---

## Public API

The parser exposes two functions. Both produce DocumentAST types from TE00.

### `parse` — Parse a Complete HTML Document

```rust
/// Parse a complete HTML document into a DocumentAST.
///
/// The input is an HTML string. The function tokenizes it (via html1.0-lexer)
/// and builds the tree in one pass.
///
/// # Examples
///
/// ```
/// use html1_parser::parse;
///
/// let doc = parse("<html><body><p>Hello</p></body></html>");
/// // doc is a DocumentNode containing one ParagraphNode with text "Hello"
/// ```
///
/// The parser never fails. Any input produces a valid tree:
///
/// ```
/// let doc = parse("not even html at all");
/// // doc is a DocumentNode containing a single TextNode
/// ```
pub fn parse(input: &str) -> DocumentNode
```

### `parse_fragment` — Parse an HTML Fragment

```rust
/// Parse an HTML fragment (without <html>/<body> wrapper).
/// Returns the block-level children as a vec.
///
/// This is useful when parsing a snippet of HTML that does not represent
/// a complete document — for example, the body of an HTTP response where
/// the structure tags are missing, or a user-authored HTML chunk.
///
/// # Examples
///
/// ```
/// use html1_parser::parse_fragment;
///
/// let blocks = parse_fragment("<p>One</p><p>Two</p>");
/// // blocks is a Vec<BlockNode> with two ParagraphNodes
/// ```
pub fn parse_fragment(input: &str) -> Vec<BlockNode>
```

The output types — `DocumentNode`, `BlockNode`, `InlineNode`, `HeadingNode`,
`ParagraphNode`, `TextNode`, and all others — are defined in the `document-ast`
crate (TE00). This parser does not define any new AST types. It only builds
trees from existing types.

### Tree-Building Algorithm

The parser uses a single-pass, stack-based algorithm. Here is the procedure in
full:

```
PROCEDURE build_tree(tokens):
    stack ← [DocumentNode]          -- the root is always on the stack
    for each token in tokens:
        match token:
            StartTag(name, attrs):
                apply_implicit_close_rules(name, stack)
                node ← map_tag_to_ast_node(name, attrs)
                if node is a void element (hr, br, img):
                    append node to stack.top()
                else:
                    append node to stack.top()
                    push node onto stack
            EndTag(name):
                pop_until_matching(name, stack)
            Text(content):
                append TextNode(content) to stack.top()
            Comment:
                skip
            Doctype:
                skip
            Eof:
                pop all remaining elements from stack
    return stack[0]                  -- the DocumentNode root

PROCEDURE apply_implicit_close_rules(new_tag, stack):
    if new_tag is "p" and stack.top() is <p>:
        pop stack                    -- Rule 1
    if new_tag is "li" and stack.top() is <li>:
        pop stack                    -- Rule 2
    if new_tag is heading(h1-h6) and stack.top() is <p>:
        pop stack                    -- Rule 3
    if new_tag is block-level and stack.top() is <p>:
        pop stack                    -- Rule 4

PROCEDURE pop_until_matching(name, stack):
    scan stack from top for element matching name
    if found:
        pop all elements above it (closing them)
        pop the matching element itself
    if not found:
        ignore the end tag           -- no matching open element
```

The key insight is that the stack represents the current path from the root to
the deepest open element. When we append a child node to `stack.top()`, we are
adding it to whatever element is currently "open" — i.e., the one whose start
tag we saw most recently without a corresponding end tag.

### Worked Example

Let us trace the algorithm through a complete document:

```html
<html><body><h1>Title</h1><p>Hello <b>world</b></p></body></html>
```

The lexer produces these tokens:

```
StartTag("html"), StartTag("body"), StartTag("h1"), Text("Title"),
EndTag("h1"), StartTag("p"), Text("Hello "), StartTag("b"),
Text("world"), EndTag("b"), EndTag("p"), EndTag("body"), EndTag("html")
```

Trace:

| Step | Token | Stack (tag names) | Action |
|------|-------|-------------------|--------|
| 0 | — | `[Document]` | Initial state |
| 1 | `StartTag("html")` | `[Document]` | Structure tag, no AST node pushed |
| 2 | `StartTag("body")` | `[Document]` | Structure tag, no AST node pushed |
| 3 | `StartTag("h1")` | `[Document, Heading(1)]` | Push HeadingNode |
| 4 | `Text("Title")` | `[Document, Heading(1)]` | Append TextNode to Heading |
| 5 | `EndTag("h1")` | `[Document]` | Pop Heading |
| 6 | `StartTag("p")` | `[Document, Paragraph]` | Push ParagraphNode |
| 7 | `Text("Hello ")` | `[Document, Paragraph]` | Append TextNode to Paragraph |
| 8 | `StartTag("b")` | `[Document, Paragraph, Strong]` | Push StrongNode |
| 9 | `Text("world")` | `[Document, Paragraph, Strong]` | Append TextNode to Strong |
| 10 | `EndTag("b")` | `[Document, Paragraph]` | Pop Strong |
| 11 | `EndTag("p")` | `[Document]` | Pop Paragraph |
| 12 | `EndTag("body")` | `[Document]` | Structure tag, close all |
| 13 | `EndTag("html")` | `[Document]` | Structure tag, close all |

Final tree:

```
DocumentNode
├── HeadingNode { level: 1 }
│   └── TextNode("Title")
└── ParagraphNode
    ├── TextNode("Hello ")
    └── StrongNode
        └── TextNode("world")
```

---

## Testing Strategy

### 1. Basic Document Structure

Parse `<html><body><p>Hello</p></body></html>` and verify the tree has a
DocumentNode root containing a single ParagraphNode with a TextNode child.

### 2. All Heading Levels

Parse `<h1>One</h1>` through `<h6>Six</h6>` and verify each produces a
HeadingNode with the correct `level` field (1 through 6).

### 3. Lists

Parse `<ul><li>A</li><li>B</li></ul>` and verify it produces a
`ListNode { ordered: false }` with two `ListItemNode` children, each containing
a TextNode.

Parse `<ol><li>First</li><li>Second</li></ol>` and verify `ordered: true`.

### 4. Links

Parse `<a href="http://example.com">click</a>` and verify it produces a
`LinkNode { destination: "http://example.com", title: None }` containing a
TextNode.

### 5. Images

Parse `<img src="photo.gif" alt="Photo">` and verify it produces an
`ImageNode { destination: "photo.gif", alt: "Photo" }`. Verify it is treated as
a void element (no children, no end tag needed).

### 6. Inline Formatting Equivalence

Verify that `<b>bold</b>` and `<strong>strong</strong>` both produce a
`StrongNode`. Verify that `<i>italic</i>` and `<em>emphasis</em>` both produce
an `EmphasisNode`.

### 7. Implicit Close — Paragraphs

Parse `<p>one<p>two` (no close tags) and verify the tree contains two separate
ParagraphNodes, not a paragraph nested inside a paragraph.

### 8. Implicit Close — List Items

Parse `<ul><li>A<li>B<li>C</ul>` and verify three separate ListItemNodes.

### 9. Missing Body

Parse `<h1>Hello</h1>` (no `<html>`, no `<body>`) and verify it works identically
to `<html><body><h1>Hello</h1></body></html>`.

### 10. Unknown Tags

Parse `<blink>text</blink>` and verify the text content is preserved (as a
TextNode in the current element) and no error is raised.

### 11. Nested Formatting

Parse `<b><i>bold italic</i></b>` and verify it produces a StrongNode containing
an EmphasisNode containing a TextNode.

### 12. Pre Blocks — Whitespace Preservation

Parse `<pre>  spaces\n  preserved  </pre>` and verify the TextNode child
preserves all whitespace exactly as written.

### 13. Entities in Attributes

Parse `<a href="page?a=1&amp;b=2">link</a>` and verify the destination is
`"page?a=1&b=2"` (entity decoded by the lexer, preserved by the parser).

### 14. Round-Trip Consistency

Parse an HTML document, emit it back to HTML via a `document-ast-to-html`
renderer, then parse the output again. Verify the two DocumentAST trees are
structurally identical.

### 15. Real Mosaic-Era Pages

Parse archived 1993 HTML from `info.cern.ch` (the first web server). These pages
use the exact tag set this parser supports and exercise real-world error recovery
patterns.

### 16. Overlapping Tags

Parse `<b><i>text</b></i>` and verify the parser recovers gracefully — the text
content must not be lost, and the resulting tree must be well-formed.

### 17. Empty Document

Parse `""` (empty string) and verify the result is a DocumentNode with no
children.

### 18. Text-Only Document

Parse `"Just some text, no tags at all."` and verify the result is a DocumentNode
containing a single TextNode.

---

## Scope

### In Scope

- The 22-tag HTML 1.0 / Mosaic subset listed above
- Implicit close rules for `<p>`, `<li>`, headings, and block elements
- Error recovery for all forms of malformed HTML (never fail, never crash)
- Full mapping from HTML tags to DocumentAST node types (TE00)
- Void element handling (`<hr>`, `<br>`, `<img>`)
- Attribute extraction (`href` from `<a>`, `src` and `alt` from `<img>`)
- Whitespace preservation inside `<pre>` blocks
- Structure tag (`<html>`, `<head>`, `<title>`, `<body>`) handling and implication

### Out of Scope

- **HTML 2.0+ tags**: `<table>`, `<form>`, `<input>`, `<select>`, `<textarea>`,
  `<div>`, `<span>`, `<font>`, and all tags introduced after the Mosaic era.
  These will be handled by future parser versions (TE06+).
- **CSS**: Neither `style` attributes nor `<style>` elements. CSS did not exist
  in 1993 (CSS1 was published in 1996).
- **JavaScript**: No `<script>` element processing. JavaScript did not exist
  in 1993 (Brendan Eich created it in 1995).
- **Character encoding detection**: The parser operates on Rust `&str` (UTF-8).
  Encoding detection is a transport-layer concern, not a parsing concern.
- **DOM API**: This parser builds a static, immutable AST. It does not provide
  `getElementById`, `querySelector`, event listeners, or any interactive DOM
  functionality. The output is a data structure, not a live object model.
- **Full HTML5 parsing algorithm**: The HTML5 spec defines an extremely detailed
  tree-building algorithm with dozens of insertion modes. That algorithm handles
  the full complexity of modern HTML. This parser implements only the subset
  relevant to the 22-tag Mosaic era, using simplified versions of the same
  principles.
