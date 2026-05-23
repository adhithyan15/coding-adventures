# coding-adventures-jsdoc-comment-extractor

Pulls `/** ... */` JSDoc block comments out of raw JavaScript source,
strips the markers and per-line `* ` continuation prefixes, and returns
each one as a `BlockComment { span, inner, anchor_byte }` ready to feed
into [`coding-adventures-jsdoc-lexer`](../jsdoc-lexer). Per
[CLOC05 §"jsdoc-comment-extractor"](../../../specs/CLOC05-jsdoc-sub-pipeline.md).

## What's here (v1)

- `extract_block_comments(source: &str) -> Vec<BlockComment>` — pure
  byte scan over the source string. Returns comments in source order
  with non-overlapping spans.
- `BlockComment { span: Span, inner: String, anchor_byte: u32 }` —
  span covers `/**` through `*/`; inner is the cleaned body;
  `anchor_byte` equals `span.start` for now (becomes "anchored AST
  node start byte" once the AST integration follow-up lands).

## What's deferred

- **`javascript-ast::Program` integration.** v1 takes raw source so it's
  testable without the JS pipeline. The follow-up takes a `Program` and
  anchors each comment to the next anchorable declaration.
- **String-literal awareness.** A `/** */` *inside* a JS string is
  currently reported as a real comment. Documented in the
  `block_comment_inside_string_is_a_false_positive_v1` test. The
  AST-driven version doesn't have this problem.
- **Line/column anchors.** v1 reports byte offsets only.

## Dependency whitelist

- `coding-adventures-javascript-tokens` — for `Span`.

Nothing else for v1. No regex crate, no serde, no `correlation-vector`
(that integration happens via the future AST-driven extractor that
emits sidecar records with full CV plumbing).
