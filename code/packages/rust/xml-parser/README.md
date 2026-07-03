# XML Parser

A grammar-driven, **namespace-aware** parser for XML that produces a typed
Abstract Syntax Tree (AST). This is milestone **M1** of the OOXML effort: its
AST is designed to be consumed by an OPC (Open Packaging Conventions) package
reader that walks `[Content_Types].xml` and `.rels` files inside `.docx` /
`.xlsx` / `.pptx` packages.

See the spec: [`code/specs/XML01-xml-parser.md`](../../../specs/XML01-xml-parser.md).

## What it does

It parses XML source text into an [`XmlDocument`] whose elements carry
**resolved** namespace names (a `(namespace_uri, local_name)` pair rather than
a raw `prefix:local`), whose text is **entity-decoded**, and whose XML
declaration's `version` / `encoding` are lifted onto the document.

## How it fits in the stack

```text
Source text  ("<w:p xmlns:w=\"...\">hi</w:p>")
      |
      v
xml-lexer            → Vec<Token>            (grammar-driven tokenizer)
      |
      v
xml.grammar          → ParserGrammar         (structural nesting rules)
      |
      v
parser::GrammarParser → GrammarASTNode tree  (generic rule/token tree)
      |
      v
xml-parser (this)    → XmlDocument            (namespaces + entities + typing)
```

The lexer and grammar do the *structural* work. Everything a context-free
grammar can't express — matching end-tag names to start tags, binding
namespace prefixes to URIs, and decoding entity references — lives in this
crate's `builder` and `namespace` modules.

## Grammar rules

```ebnf
document          = { misc } element { misc } ;
misc              = pi | comment ;
element           = empty_element | container_element ;
empty_element     = OPEN_TAG_START TAG_NAME { attribute } SELF_CLOSE ;
container_element = OPEN_TAG_START TAG_NAME { attribute } TAG_CLOSE
                    { content }
                    CLOSE_TAG_START TAG_NAME TAG_CLOSE ;
attribute         = TAG_NAME ATTR_EQUALS ATTR_VALUE ;
content           = element | comment | cdata | pi | CHAR_REF | ENTITY_REF | TEXT ;
comment           = COMMENT_START [ COMMENT_TEXT ] COMMENT_END ;
cdata             = CDATA_START [ CDATA_TEXT ] CDATA_END ;
pi                = PI_START PI_TARGET [ PI_TEXT ] PI_END ;
```

The grammar lives at `code/grammars/xml.grammar`; the compiled
`src/_grammar.rs` is generated from it by the `grammar-tools` CLI.

## Usage

```rust
use coding_adventures_xml_parser::parse_xml;

let doc = parse_xml(r#"<note lang="en"><to>Alice</to></note>"#).unwrap();
assert_eq!(doc.root.local_name, "note");
assert_eq!(doc.root.get_attr(None, "lang"), Some("en"));

let to = doc.root.get_child(None, "to").unwrap();
assert_eq!(to.text_content(), "Alice");
```

An OOXML-flavoured example (`[Content_Types].xml`):

```rust
use coding_adventures_xml_parser::parse_xml;

const CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

let src = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/...relationships+xml"/>
  <Override PartName="/word/document.xml" ContentType="application/...main+xml"/>
</Types>"#;

let doc = parse_xml(src).unwrap();
assert_eq!(doc.version.as_deref(), Some("1.0"));

// Elements inherit the default namespace...
let over = doc.root.get_child(Some(CT), "Override").unwrap();
// ...but unprefixed attributes are in NO namespace.
assert_eq!(over.get_attr(None, "PartName"), Some("/word/document.xml"));
```

## Namespace rules to remember

- The **default namespace** (`xmlns="…"`) applies to unprefixed **elements**,
  but **not** to unprefixed **attributes** (those are always namespace-free).
- Namespace **URIs are case-sensitive**.
- `xml` and `xmlns` are reserved, always-bound prefixes.
- An unbound prefix is a parse error.

## Entity decoding

`&amp; &lt; &gt; &apos; &quot;` and numeric references `&#N;` (decimal) /
`&#xH;` (hex) are decoded in text and attribute values. **CDATA is verbatim**
— never decoded. There is no DTD support, so no custom named entities exist
(this matches the XML subset OOXML uses).

## Known limitation (lexer-inherited)

The `xml-lexer` skips whitespace *between* tokens in the default group, so a
space immediately following an entity reference in mixed text (the space in
`a &amp; b`) is consumed before the next `TEXT` token and does not reach the
parser. Whitespace inside a single text run (`a  b`) is preserved. OPC part
files are element-structured, so this does not affect the M1 goal.

## Running tests

```bash
cargo test -p coding-adventures-xml-parser -- --nocapture
```
