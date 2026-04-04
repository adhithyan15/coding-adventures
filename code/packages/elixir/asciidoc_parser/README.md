# asciidoc_parser

AsciiDoc parser for the coding-adventures project. Converts AsciiDoc text into
the shared **Document AST** intermediate representation.

## What it does

`CodingAdventures.AsciidocParser.parse/1` accepts an AsciiDoc string and returns
a `%{type: :document, children: [...]}` Document AST node — the same format
produced by `commonmark_parser` and `gfm_parser`. Any back-end that consumes the
Document AST (e.g., `document_ast_to_html`) works out-of-the-box.

## Architecture

```
AsciiDoc text
     |
     v
BlockParser.parse_blocks/1     (state-machine, line-by-line)
     |
     v
InlineParser.parse/1           (binary pattern-matching scanner)
     |
     v
CodingAdventures.DocumentAst   (IR: document node tree)
```

## AsciiDoc vs CommonMark — key difference

In AsciiDoc, `*bold*` means **strong** (not emphasis) and `_italic_` means
*emphasis*. This is the opposite of how CommonMark processes `*text*`.

## Supported syntax

### Block elements

| Syntax | Result |
|--------|--------|
| `= Title` | Heading level 1 |
| `== Section` ... `====== Level 6` | Headings 2-6 |
| `'''` (3+ single-quotes) | Thematic break |
| `[source,elixir]` + `----` fence | Code block with language |
| `----` (4+ dashes, no attribute) | Code block, no language |
| `....` (4+ dots) | Literal block (code, no language) |
| `++++` | Passthrough block (raw HTML) |
| `____` (4+ underscores) | Block quotation |
| `* item` / `** item` | Unordered list (nested) |
| `. item` / `.. item` | Ordered list (nested) |
| `// comment` | Comment (skipped) |

### Inline elements

| Syntax | Result |
|--------|--------|
| `*bold*` | strong (not emphasis!) |
| `**bold**` | strong (unconstrained) |
| `_italic_` | emphasis |
| `__italic__` | emphasis (unconstrained) |
| backtick code backtick | code_span (verbatim) |
| `link:url[text]` | link |
| `image:url[alt]` | image |
| `<<anchor,text>>` | internal cross-reference link |
| `https://example.com` | autolink |
| two trailing spaces + newline | hard_break |
| single newline inside paragraph | soft_break |

## Usage

```elixir
doc = CodingAdventures.AsciidocParser.parse("""
= My Document

This is a *bold* paragraph with _italic_ text.

== Section

[source,elixir]
----
IO.puts("hello")
----
""")
# => %{type: :document, children: [...]}
```

## Fitting into the stack

- **Depends on:** `document_ast` (node constructors and type specs)
- **Used by:** `asciidoc` (thin `to_html/1` wrapper)

## Testing

```
mix deps.get && mix test --cover
```
