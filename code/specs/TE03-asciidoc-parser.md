# TE03 — AsciiDoc Parser

## Overview

The AsciiDoc parser converts AsciiDoc source text into the Document AST IR
(spec TE00). It is the AsciiDoc front-end in the document-processing pipeline:

```
AsciiDoc source
    ↓  asciidoc_parser.parse()
DocumentNode  [coding_adventures_document_ast]
    ↓  document_ast_to_html.to_html()
HTML string
```

The thin `asciidoc` convenience package chains those two steps into a single
`to_html(text)` call, mirroring the `commonmark` package (TE02).

## Packages

| Layer | Package name | Exports |
|-------|-------------|---------|
| Parser | `coding-adventures-asciidoc-parser` (Python) / `coding_adventures_asciidoc_parser` (Ruby) | `parse(text)` |
| Pipeline | `coding-adventures-asciidoc` (Python) / `coding_adventures_asciidoc` (Ruby) | `to_html(text)` |

## AsciiDoc Subset Supported

This implementation covers the most commonly used AsciiDoc constructs:

### Block-level constructs

| AsciiDoc syntax | Document AST node |
|----------------|-------------------|
| `= Heading` | `HeadingNode { level: 1 }` |
| `== Heading` | `HeadingNode { level: 2 }` |
| `=== Heading` through `====== Heading` | `HeadingNode { level: 3..6 }` |
| `'''` (three or more single-quotes) | `ThematicBreakNode` |
| `----` fence (four or more dashes) | `CodeBlockNode` |
| `....` fence (four or more dots) | `CodeBlockNode { language: nil }` (literal block) |
| `++++` fence | `RawBlockNode { format: "html" }` (passthrough) |
| `____` fence (four or more underscores) | `BlockquoteNode` (quote block, recursively parsed) |
| `* text` / `** text` / … | `ListNode { ordered: false }` |
| `. text` / `.. text` / … | `ListNode { ordered: true }` |
| `[source,lang]` (attribute line) | sets language on next code block |
| `// comment` | skipped (not included in AST) |
| blank line | paragraph separator |
| any other text | `ParagraphNode` |

### Inline constructs

| AsciiDoc syntax | Document AST node |
|----------------|-------------------|
| `` `code` `` | `CodeSpanNode` (verbatim — no nested parsing) |
| `**bold**` | `StrongNode` (unconstrained) |
| `__italic__` | `EmphasisNode` (unconstrained) |
| `*bold*` | `StrongNode` (constrained) |
| `_italic_` | `EmphasisNode` (constrained) |
| `link:url[text]` | `LinkNode { destination: url }` |
| `image:url[alt]` | `ImageNode { destination: url, alt: alt }` |
| `<<anchor,text>>` / `<<anchor>>` | `LinkNode { destination: "#anchor" }` |
| `https://url[text]` / `http://url[text]` | `LinkNode` with explicit text |
| bare `https://…` / `http://…` URL | `AutolinkNode` |
| two trailing spaces + newline | `HardBreakNode` |
| `\` + newline | `HardBreakNode` |
| bare newline within paragraph | `SoftBreakNode` |

## Parsing Algorithm

### Phase 1 — Block Parser (state machine)

The block parser reads the source line by line using a state machine.

**States:**

- `normal` — between blocks; dispatching each new line
- `paragraph` — accumulating a paragraph until a blank line
- `code_block` — inside a `----` fenced block
- `literal_block` — inside a `....` fenced block
- `passthrough_block` — inside a `++++` fenced block
- `quote_block` — inside a `____` fenced block
- `unordered_list` — accumulating `*` list items
- `ordered_list` — accumulating `.` list items

**Line dispatch in `normal` state:**

```
blank line        → stay normal (flush any pending list)
// comment        → skip (do not include in AST)
[source,lang]     → record pending_language for next code block
= text            → HeadingNode(level=1, inline_parse(text))
== text           → HeadingNode(level=2, ...)
... up to ======  → HeadingNode(level=6, ...)
''' (≥3)          → ThematicBreakNode
---- (≥4)         → enter code_block state
.... (≥4)         → enter literal_block state
++++ (≥4)         → enter passthrough_block state
____ (≥4)         → enter quote_block state
* text            → enter/continue unordered_list (level 1)
** text           → enter/continue unordered_list (level 2) etc.
. text            → enter/continue ordered_list (level 1)
.. text           → enter/continue ordered_list (level 2) etc.
other text        → enter paragraph state
```

**Paragraph accumulation:** lines accumulate until a blank line or a line that
starts a new block construct. The accumulated text is inline-parsed.

**Delimited blocks** (`----`, `....`, `++++`, `____`): accumulate lines
verbatim (no inline parsing) until the matching closing delimiter. The closing
delimiter must be the same token as the opening delimiter.

**Quote blocks:** the accumulated content inside `____` is recursively
re-parsed by the block parser. The result becomes the children of a
`BlockquoteNode`.

**List nesting:** `*` = level 1, `** ` = level 2, `***` = level 3, etc. (count
the leading `*` characters). Likewise for ordered lists (count `.` characters).
When emitting the list, items are grouped into nested `ListNode` / `ListItemNode`
trees following the level hierarchy.

**Attribute lines** (`[source,lang]`): if a line matches `[source,LANG]` or
`[source, LANG]`, record `pending_language = LANG`. The next `----` block
consumes that language.

### Phase 2 — Inline Parser (left-to-right character scanner)

The inline parser processes a string left-to-right, character by character.
It returns a list of inline AST nodes.

**Priority order** (checked in this sequence to avoid ambiguity):

1. Two trailing spaces followed by `\n` → `HardBreakNode`
2. `\` + `\n` → `HardBreakNode`
3. `\n` → `SoftBreakNode`
4. `` ` `` → scan for closing `` ` ``; content is verbatim → `CodeSpanNode`
5. `**` → scan for closing `**` → `StrongNode` (unconstrained)
6. `__` → scan for closing `__` → `EmphasisNode` (unconstrained)
7. `*` → scan for closing `*` → `StrongNode` (constrained)
8. `_` → scan for closing `_` → `EmphasisNode` (constrained)
9. `link:` → parse `link:URL[text]` → `LinkNode`
10. `image:` → parse `image:URL[alt]` → `ImageNode`
11. `<<` → parse `<<anchor>>` or `<<anchor,text>>` → `LinkNode { destination: "#anchor" }`
12. `https://` or `http://` followed by `[text]` → `LinkNode`; bare URL → `AutolinkNode`
13. Any other character → accumulate into a `TextNode`

**Constrained vs. unconstrained markup:**

- *Unconstrained* (`**`, `__`) — can appear anywhere in a word
- *Constrained* (`*`, `_`) — in this implementation, treated the same as
  unconstrained for simplicity; the content between the delimiters is parsed

**Code span verbatim rule:** content between backticks is never inline-parsed.
It is stored as-is in `CodeSpanNode.value`.

## Document AST Mapping

The parser produces the following node types from `coding_adventures_document_ast`:

**Block nodes:** `DocumentNode`, `HeadingNode`, `ParagraphNode`, `CodeBlockNode`,
`BlockquoteNode`, `ListNode`, `ListItemNode`, `ThematicBreakNode`, `RawBlockNode`

**Inline nodes:** `TextNode`, `EmphasisNode`, `StrongNode`, `CodeSpanNode`,
`LinkNode`, `ImageNode`, `AutolinkNode`, `HardBreakNode`, `SoftBreakNode`

## Package Interface

### Python

```python
from coding_adventures_asciidoc_parser import parse

doc = parse("= Hello\n\nWorld *with* strong.\n")
doc["type"]                  # "document"
doc["children"][0]["type"]   # "heading"
doc["children"][0]["level"]  # 1
```

```python
from coding_adventures_asciidoc import to_html

html = to_html("= Hello\n\nWorld\n")
# "<h1>Hello</h1>\n<p>World</p>\n"
```

### Ruby

```ruby
require "coding_adventures_asciidoc_parser"

doc = CodingAdventures::AsciidocParser.parse("= Hello\n\nWorld *with* strong.\n")
doc.type               # => "document"
doc.children[0].type   # => "heading"
doc.children[0].level  # => 1
```

```ruby
require "coding_adventures_asciidoc"

html = CodingAdventures::Asciidoc.to_html("= Hello\n\nWorld\n")
# => "<h1>Hello</h1>\n<p>World</p>\n"
```

## Error Handling

- Unclosed delimited blocks: treat all accumulated content up to EOF as the
  block content (lenient parsing).
- Unclosed inline markup: treat the opening delimiter as a literal text
  character and continue parsing.
- Heading levels beyond 6: clamp to 6.

## Test Coverage Requirements

- Minimum 80% line coverage for all packages.
- Parsers must have ≥ 30 test cases covering all block and inline forms.
- Pipeline packages must test round-trip (AsciiDoc → HTML) for the most
  common constructs.
