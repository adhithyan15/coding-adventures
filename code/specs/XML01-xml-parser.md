# XML01 — XML Parser (namespace-aware AST)

## Overview

This is milestone **M1** of the OOXML effort. It builds a Rust crate,
`xml-parser`, that turns XML source text into a typed, **namespace-aware**
Abstract Syntax Tree (AST). The AST is deliberately shaped for the next
milestone: an **OPC** (Open Packaging Conventions) package reader that walks
`[Content_Types].xml` and `.rels` files to discover the parts of a `.docx` /
`.xlsx` / `.pptx` package.

Like every other language front-end in this repo, the parser is **not**
hand-written. It reuses the shared grammar tooling:

- The **`xml-lexer`** crate tokenizes the source using `xml_rust.tokens`.
- A new **`xml.grammar`** file describes how those tokens may nest.
- The generic **`parser::GrammarParser`** turns tokens + grammar into a raw
  `GrammarASTNode` tree.
- A thin, hand-written **AST builder** in this crate walks that raw tree once
  and produces the typed `XmlDocument`, doing the three jobs a context-free
  grammar cannot: end-tag matching, namespace resolution, and entity decoding.

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
xml-parser (this crate) → XmlDocument         (namespaces + entities + typing)
```

## Historical & standards context

XML (Extensible Markup Language, W3C, 1998) is a text format for tree-shaped
data. It descends from SGML (1986) but is far simpler. Two later companion
specs matter here:

- **Namespaces in XML** (1999) — lets documents mix vocabularies without name
  collisions by binding short *prefixes* to long *URIs*.
- **ISO/IEC 29500 (OOXML, 2006)** and its packaging layer **OPC** — the format
  behind modern Word / Excel / PowerPoint files. A `.docx` is a ZIP whose
  entries are XML parts glued together by relationship (`.rels`) files and a
  central `[Content_Types].xml` manifest.

M1 targets exactly the XML subset OPC uses. It has **no DTD support**, so the
only named entities it decodes are the five predefined ones. That is a
deliberate scope cut, not an oversight.

## Why the grammar can't do everything

A context-free grammar (which is what `GrammarParser` consumes) can describe
*nesting* — "a start tag, then content, then an end tag" — but it cannot
express three things that XML requires:

1. **End-tag name matching.** "`<a>` must be closed by `</a>`" is a
   context-*sensitive* constraint (the two names must be *equal*). The grammar
   accepts `<a>...</b>` structurally; the AST builder rejects it.
2. **Namespace resolution.** Binding a prefix to a URI depends on `xmlns`
   declarations that are in scope — runtime state a grammar has no notion of.
3. **Entity decoding.** `&amp;` → `&`, `&#65;` → `A`. The lexer emits entity
   references as separate, *undecoded* tokens on purpose.

So the grammar stays purely structural, and the builder owns everything
stateful. This mirrors how the other front-ends in the repo split "shape"
(grammar) from "meaning" (builder).

## The grammar (`code/grammars/xml.grammar`)

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

Design notes:

- **Distinct first tokens.** Every `content` alternative begins with a unique
  token (`OPEN_TAG_START`, `COMMENT_START`, `CDATA_START`, `PI_START`,
  `CHAR_REF`, `ENTITY_REF`, `TEXT`), so the backtracking packrat parser never
  looks far ahead and never mis-commits.
- **Empty forms parse.** `COMMENT_TEXT`, `CDATA_TEXT`, and `PI_TEXT` are
  optional so `<!---->`, `<![CDATA[]]>`, and `<?t?>` are well-formed.
- **The XML declaration is a PI.** `<?xml version="1.0"?>` is lexed as a `pi`
  whose `PI_TARGET` is `"xml"`. The builder special-cases that target: it lifts
  `version` / `encoding` onto the document instead of storing a PI node.

`src/_grammar.rs` is generated from this file by the Rust `grammar-tools` CLI
(see the regen command in that file's header) and is checked in.

## The AST (`src/ast.rs`)

```rust
struct XmlDocument { root: XmlElement, version: Option<String>, encoding: Option<String> }

struct XmlElement {
    namespace_uri: Option<String>,   // resolved URI, or None = no namespace
    local_name: String,              // name after the prefix
    attributes: Vec<XmlAttribute>,
    children: Vec<XmlNode>,
}

struct XmlAttribute { namespace_uri: Option<String>, local_name: String, value: String }

enum XmlNode {
    Element(Box<XmlElement>),
    Text(String),                    // entity-decoded
    CData(String),                   // verbatim (NOT decoded)
    Comment(String),
    ProcessingInstruction { target: String, text: String },
}

struct ParseError { message: String, line: Option<usize>, column: Option<usize> }
```

`XmlElement` provides the navigation helpers a package reader needs:

| Method | Returns |
|--------|---------|
| `get_child(uri, local)`    | first *direct child element* with that resolved name |
| `get_children(uri, local)` | all such direct children, in order |
| `get_attr(uri, local)`     | that attribute's decoded value |
| `text_content()`           | concatenated descendant text + CDATA |

In every case `uri = None` means "in no namespace" and `uri = Some("…")`
requires that exact URI.

## Namespace resolution rules (`src/namespace.rs`)

A `NamespaceStack` holds one `HashMap<prefix, uri>` frame per open element.
On entering a start tag the builder collects that tag's `xmlns` /
`xmlns:prefix` declarations, pushes a frame, resolves names, builds children,
then pops. Resolution walks frames innermost-first, so an inner declaration
**shadows** an outer one.

The subtle rules, all tested:

- **Default namespace applies to elements, not unprefixed attributes.** Inside
  `<r xmlns="U" a="1">`, the element `r` is in namespace `U`, but attribute `a`
  is in **no** namespace. This asymmetry is in the Namespaces spec and is
  exactly what OPC relies on (its attributes like `PartName`, `Extension`,
  `Target` are all unprefixed and thus namespace-free).
- **URIs are case-sensitive.** `http://x/NS` ≠ `http://x/ns`.
- **Reserved prefixes.** `xml` and `xmlns` are always bound and never declared.
- **Unbound prefix is an error.**

## Entity decoding

`decode_entities` handles the five predefined entities plus numeric character
references:

| Reference | Decodes to |
|-----------|------------|
| `&amp;` `&lt;` `&gt;` `&apos;` `&quot;` | `& < > ' "` |
| `&#N;`  (decimal) | Unicode code point `N` |
| `&#xH;` (hex)     | Unicode code point `0xH` |

Applied to `TEXT` and to attribute values (after quote-stripping). **CDATA is
never decoded** — its purpose is to hold `<` and `&` literally.

## Known limitation inherited from the lexer

The `xml-lexer` skips whitespace that sits *between* tokens in the default
group. As a consequence, a space that immediately follows an entity reference
in mixed text (e.g. the space in `a &amp; b`) is consumed by the lexer before
the following `TEXT` token begins, so it does not reach the parser. Whitespace
*inside* a single text run (`a  b`) is preserved. This is a lexer-layer
behavior, out of scope for M1; the parser faithfully decodes and concatenates
whatever tokens it is handed. OPC part files are element-structured with no
significant mixed-content whitespace, so this does not affect the M1 goal.

## Public API (`src/lib.rs`)

```rust
pub fn create_xml_parser(source: &str) -> GrammarParser;   // raw generic tree
pub fn parse_xml(source: &str) -> Result<XmlDocument, ParseError>;
pub use ast::{ParseError, XmlAttribute, XmlDocument, XmlElement, XmlNode};
pub use namespace::{XML_NAMESPACE, XMLNS_NAMESPACE};
```

`parse_xml` tokenizes through the *fallible* lexer path so a lexer-level
problem (a stray `&`) becomes a `ParseError` with position rather than a
panic. `create_xml_parser` keeps the standard (panicking) glue shape shared by
the repo's other `*-parser` crates, for callers who want the raw tree.

## Robustness: bounded recursion

XML nests without limit (`element → content → element`), and the recursive-
descent parser recurses once per level. A native stack overflow is an
**uncatchable abort** — no `Result` can report it — so on untrusted input (the
whole point of an OOXML reader) an unbounded parser is a one-line denial of
service: a few kilobytes of `<a><a>…` crashes the host process.

Both entry points therefore opt into the parser's recursion-depth cap,
`with_max_depth(DEFAULT_MAX_RULE_DEPTH)` (128 — comfortably below the overflow
point and far above any real document's nesting). Over-deep input surfaces as a
normal `ParseError`. This is the "decode liberally, but never trust depth"
counterpart to the deflate crate's decompression-bomb cap.

## Testing

46 unit tests plus a doc-test, targeting 95%+ coverage of a library:

- structure: simple, self-closing, empty, nested, mixed content;
- attributes: both quote styles, order, absence;
- namespaces: default, prefixed element + prefixed attribute → URI,
  unprefixed-attr-is-namespace-free, nested scoping + shadowing, case
  sensitivity, reserved `xml` prefix, unbound-prefix error;
- entities: named in text and attributes, decimal + hex char refs, unknown
  entity error, bare-`&` error;
- CDATA verbatim (and empty), comments (and empty, and leading);
- XML declaration version + encoding, processing instructions;
- whitespace handling;
- well-formedness errors: mismatched tags, unclosed element, empty input;
- the factory function and `ParseError` `Display`;
- two OOXML-flavoured integration tests: a `[Content_Types].xml` with
  `Default` / `Override` entries in the OPC content-types namespace, and a
  `.rels` document — asserting `get_child(Some(uri), …)` and
  `get_attr(None, …)`.

## Where this sits in the OOXML effort

- **M1 (this):** `xml-parser` — namespace-aware AST. ✅
- **M2 (next):** OPC package reader — unzip a `.docx`, parse
  `[Content_Types].xml` + `.rels` with this crate, resolve parts.
- Beyond: WordprocessingML / SpreadsheetML part models on top of the OPC layer.
