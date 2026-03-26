# coding_adventures_document_ast_sanitizer

Policy-driven AST sanitization for the Document IR pipeline (TE02, stage 1).

## What it does

This crate performs a **pure, immutable tree transformation** on a `DocumentNode`
value, producing a sanitized copy according to a caller-supplied
`SanitizationPolicy`. The input is never mutated.

It sits between the parser and renderer in the document pipeline:

```text
parse(markdown)          ← TE01 — CommonMark Parser
      ↓
sanitize(doc, policy)    ← TE02 — document-ast-sanitizer (this crate)
      ↓
to_html(doc)             ← TE00 — document-ast-to-html
      ↓
final output
```

## Key features

- **Pure and immutable** — returns a new `DocumentNode`, never mutates input
- **Complete** — every node type in the Document AST is handled explicitly
- **Empty-child pruning** — container nodes with no surviving children are dropped
- **Link promotion** — when `drop_links: true`, link text children are promoted to the parent
- **URL scheme sanitization** — strips C0 control chars and zero-width chars before scheme detection to prevent bypass attacks like `java\x00script:`

## Quick start

```rust
use coding_adventures_document_ast_sanitizer::{sanitize, strict};
use document_ast::{DocumentNode, BlockNode, ParagraphNode, InlineNode, LinkNode, TextNode};

let doc = DocumentNode {
    children: vec![
        BlockNode::Paragraph(ParagraphNode {
            children: vec![
                InlineNode::Link(LinkNode {
                    destination: "javascript:alert(1)".to_string(),
                    title: None,
                    children: vec![InlineNode::Text(TextNode {
                        value: "click me".to_string(),
                    })],
                }),
            ],
        }),
    ],
};

let safe = sanitize(&doc, &strict());
// safe.children[0] is a ParagraphNode with a Link whose destination is ""
```

## Named presets

| Preset          | Use case                                         |
|-----------------|--------------------------------------------------|
| `strict()`      | User-generated content (comments, forum posts)   |
| `relaxed()`     | Authenticated users / internal wikis             |
| `passthrough()` | Fully trusted content (docs, static sites)       |

## Policy customisation

```rust
use coding_adventures_document_ast_sanitizer::policy::{SanitizationPolicy, relaxed};

let custom = SanitizationPolicy {
    min_heading_level: 2,  // reserve h1 for the page title
    ..relaxed()
};
```

## Dependencies

- `document-ast` — the Document IR node types

## Development

```bash
cargo build --manifest-path Cargo.toml
cargo test --manifest-path Cargo.toml -- --nocapture
cargo clippy
```
