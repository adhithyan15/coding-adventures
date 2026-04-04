# @coding-adventures/asciidoc-parser

AsciiDoc parser that converts AsciiDoc source text into a format-agnostic
Document AST (the same IR produced by the CommonMark parser).

## What it does

- Parses all major AsciiDoc block types: headings, paragraphs, code blocks
  (listing and literal), passthrough blocks, quote blocks, ordered and
  unordered lists (with nesting), thematic breaks, and comments.
- Parses AsciiDoc inline syntax: `*strong*`, `_emphasis_`, `` `code` ``,
  `link:url[text]`, `image:url[alt]`, `<<xref,text>>`, bare URLs, hard/soft
  line breaks.
- Key difference from CommonMark: `*asterisk*` = **strong** (bold) in
  AsciiDoc, not emphasis. `_underscore_` = _emphasis_ (italic).
- Produces a `DocumentNode` from `@coding-adventures/document-ast` — the same
  IR used by all other front-end parsers in this repo.

## Where it fits

```
AsciiDoc source
    ↓  parse()           @coding-adventures/asciidoc-parser
DocumentNode AST
    ↓  render()          @coding-adventures/document-ast-to-html
HTML string
```

## Usage

```typescript
import { parse } from "@coding-adventures/asciidoc-parser";

const doc = parse(`
= Document Title

Introduction paragraph with *bold* and _italic_ text.

== Section One

[source,typescript]
----
const greeting = "Hello, AsciiDoc!";
console.log(greeting);
----

* Item A
* Item B
** Nested item
`);

doc.type;              // "document"
doc.children[0].type; // "heading"  (level 1)
doc.children[1].type; // "paragraph"
doc.children[2].type; // "heading"  (level 2)
doc.children[3].type; // "code_block"
doc.children[4].type; // "list"
```

## AsciiDoc syntax reference

| Block syntax       | Result                |
|--------------------|-----------------------|
| `= Title`          | HeadingNode level 1   |
| `== Section`       | HeadingNode level 2   |
| `'''`              | ThematicBreakNode     |
| `----` ... `----`  | CodeBlockNode         |
| `....` ... `....`  | CodeBlockNode (no lang)|
| `++++` ... `++++`  | RawBlockNode ("html") |
| `____` ... `____`  | BlockquoteNode        |
| `* item`           | Unordered ListNode    |
| `. item`           | Ordered ListNode      |

| Inline syntax      | Result            |
|--------------------|-------------------|
| `*bold*`           | StrongNode        |
| `**bold**`         | StrongNode (unconstrained) |
| `_italic_`         | EmphasisNode      |
| `__italic__`       | EmphasisNode (unconstrained) |
| `` `code` ``       | CodeSpanNode      |
| `link:url[text]`   | LinkNode          |
| `image:url[alt]`   | ImageNode         |
| `<<anchor,text>>`  | LinkNode (xref)   |
| `https://url`      | AutolinkNode      |
| `https://url[text]`| LinkNode          |
