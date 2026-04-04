# coding-adventures-asciidoc

A thin Lua wrapper that combines the AsciiDoc parser and the Document AST
HTML renderer into a single convenience package.

## Architecture

```
AsciiDoc text
    ↓
coding_adventures.asciidoc_parser  →  Document AST
    ↓
coding_adventures.document_ast_to_html  →  HTML string
```

This package follows the same N × M design as `coding_adventures.commonmark`:
multiple front-ends (Markdown, AsciiDoc, …) share a common IR (Document AST)
that can be rendered by multiple back-ends (HTML, PDF, …).

## Usage

```lua
local asciidoc = require("coding_adventures.asciidoc")

-- One-step: AsciiDoc → HTML
local html = asciidoc.to_html("= Hello\n\nWorld\n")
-- → "<h1>Hello</h1>\n<p>World</p>\n"

-- Two-step: parse then render
local doc  = asciidoc.parse("= Hello\n")
local html = asciidoc.to_html_from_ast(doc)

-- render() is an alias for to_html()
local html = asciidoc.render("*bold*\n")
```

## Running tests

```
cd tests && busted . --verbose --pattern=test_
```

## License

MIT
