# Changelog — coding-adventures-document-ast-sanitizer

All notable changes to this package are documented here.

## [0.1.0] — 2026-03-24

### Added

- Initial implementation of `sanitize(doc, policy) → DocumentNode`.
- `SanitizationPolicy` frozen dataclass with full type annotations.
- Named presets: `STRICT`, `RELAXED`, `PASSTHROUGH`.
- `url_utils.py`:
  - `strip_control_chars(url)` — strips C0 control chars and zero-width Unicode.
  - `extract_scheme(url)` — extracts URL scheme, returns `None` for relative URLs.
  - `is_scheme_allowed(url, allowed_schemes)` — complete URL safety check.
- Full truth-table implementation covering all 18 node types from TE00 Document AST:
  - `DocumentNode` — recurse into children (never dropped)
  - `HeadingNode` — clamp level, promote/demote, or drop all
  - `ParagraphNode` — recurse (dropped if empty after sanitization)
  - `CodeBlockNode` — keep or drop per `drop_code_blocks`
  - `BlockquoteNode` — keep/recurse or drop per `drop_blockquotes`
  - `ListNode` / `ListItemNode` — recurse
  - `ThematicBreakNode` — always kept
  - `RawBlockNode` — format-based allowlist
  - `TextNode` — always kept
  - `EmphasisNode` / `StrongNode` — recurse (dropped if empty)
  - `CodeSpanNode` — keep or convert to `TextNode`
  - `LinkNode` — drop+promote children, or sanitize URL
  - `ImageNode` — drop, convert to alt text, or sanitize URL
  - `AutolinkNode` — keep or drop based on URL scheme
  - `RawInlineNode` — format-based allowlist
  - `HardBreakNode` / `SoftBreakNode` — always kept
- Empty container cleanup: containers dropped when all children are dropped.
- Link promotion: `drop_links=True` promotes link children to parent, preserving text.
- Immutability: input `DocumentNode` is never mutated.
- 133 unit tests covering all policy options, all node types, and all XSS vectors from spec.
- `BUILD` and `BUILD_windows` files for the monorepo build system.
- `py.typed` marker for PEP 561 type checking support.

### Implementation notes

- Implemented in Python 3.12 with full type annotations and `typing_extensions` for `TypedDict`.
- `SanitizationPolicy` uses `frozen=True` dataclass for safe sharing across calls.
- URL scheme extraction correctly handles relative URLs (no `:`, or `:` after `/`/`?`).
- `allow_raw_block_formats` and `allow_raw_inline_formats` accept `str` or `tuple[str, ...]`
  at runtime (not a union of literal strings), consistent with Python's type system.
- Tests achieve >90% coverage (target: 95%+).

### Spec compliance

Implements TE02 — Document Sanitization, Stage 1 (AST Sanitizer).
All transformation rules from the truth table are implemented.
