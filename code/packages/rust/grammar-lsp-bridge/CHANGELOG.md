# Changelog — `grammar-lsp-bridge`

## 0.3.0 — 2026-07-13

### Fixed — recursion-depth guard against native stack overflow (DoS)

`GrammarLanguageBridge::parse` built its `GrammarParser` with no
recursion-depth cap, even though `basic-lsp-bridge`/`nib-lsp-bridge`/
`oct-lsp-bridge`/`twig-lsp-bridge` all compile whatever file is open in the
editor being debugged/edited through this one shared code path — a real,
not theoretical, attack surface for all four. Deeply-nested input
(`((((...))))`) would recurse until it overflowed the native thread stack —
an uncatchable process abort — before `parse` ever got a chance to turn a
parse failure into a `Diagnostic`.

Unlike a dedicated parser crate, this bridge builds a `GrammarParser` from
*whichever* `LanguageSpec` its caller supplies, so a single cap here must be
safe for every grammar any current consumer's spec carries. Measured
directly (binary search, uncapped parser, default-stack worker thread,
debug build) against all four grammars this bridge currently serves:
`nib`/`oct` (via their own dedicated parser crates, which lower through the
same compiled grammar this bridge parses at runtime) bind the constraint at
27/30 real nesting levels (285/290 in rule-frame terms); `twig` and `basic`
(Dartmouth BASIC) both tolerate far deeper nesting before crashing.

- Added `MAX_RULE_DEPTH: usize = 200` (about 30% below the binding
  285-rule-frame floor, matching `nib-parser`/`oct-parser`'s own
  independently-measured value) and wired it into `parse` via
  `.with_max_depth(...)`.
- 3 new regression tests against this crate's own `toy` test grammar: deep
  adversarial input on an enlarged-stack thread produces a clean
  `Diagnostic` (never a crash), input at the measured real-nesting boundary
  (98 levels for `toy`'s simpler `form -> list -> {form}` shape) still
  parses one level past it doesn't, and the cap trips before the native
  stack would overflow even on a default-stack thread.
- Verified no regression across all four real consumers
  (`basic-lsp-bridge`/`nib-lsp-bridge`/`oct-lsp-bridge`/`twig-lsp-bridge`'s
  own test suites, 61 tests total, all pass unchanged).

A future `LanguageSpec` with an even lower native-stack floor than
`nib`/`oct`'s would need this constant re-measured — see the `MAX_RULE_DEPTH`
doc comment in `bridge.rs`.

## 0.2.0 — 2026-05-04

**LS02 PR A — Full `GrammarLanguageBridge` implementation.**

`GrammarLanguageBridge` now fully implements `ls00::LanguageBridge` for any
language that supplies a `.tokens` + `.grammar` file pair.  The eight
LSP features are:

1. **`tokenize`** — delegates to `lexer::grammar_lexer::GrammarLexer`; strips
   EOF tokens; converts 1-based lexer positions to `ls00::Token`.
2. **`parse`** — delegates to `parser::grammar_parser::GrammarParser`; wraps
   the `GrammarASTNode` tree in `Box<dyn Any + Send + Sync>` for the ls00
   cache; converts parse errors to `Diagnostic` objects with source positions.
3. **`semantic_tokens`** — walks all leaf tokens, filters via `token_kind_map`,
   converts 1-based positions to 0-based LSP `SemanticToken` objects.
4. **`document_symbols`** — uses `find_nodes` to locate top-level nodes whose
   `rule_name` is in `declaration_rules`; extracts the first `NAME` token
   child as the symbol name; returns flat `DocumentSymbol` list.
5. **`folding_ranges`** — `collect_folding()` recursively walks the AST and
   emits a `FoldingRange` for every node that spans more than one line;
   positions converted 1-based → 0-based.
6. **`hover`** — `node_at_pos()` finds the innermost `GrammarASTNode` at the
   cursor; returns the grammar rule name and, if the cursor is on a keyword,
   marks it as such.
7. **`completion`** — emits `keyword` completions for all `keyword_names` plus
   `function` completions for every declaration found in the live AST.
8. **`format`** — delegates to `spec.format_fn` when set; returns `None`
   (no-op) when `format_fn` is `None`.

All eight capability flags (`supports_*`) are gated on the relevant spec field
(format only when `format_fn.is_some()`; rest always `true`).

**Test suite — 20 unit tests** covering all eight features, error paths, and
edge cases:
- `new_parses_grammars_without_panic`
- `tokenize_basic`, `tokenize_eof_excluded`
- `parse_valid_source`, `parse_invalid_source_returns_diagnostic`
- `semantic_tokens_basic`, `semantic_tokens_unknown_kind_excluded`
- `document_symbols_finds_declarations`, `document_symbols_no_decls`
- `folding_ranges_single_form`, `folding_ranges_multiline`
- `hover_on_keyword`, `hover_on_rule_node`, `hover_no_node`
- `completion_returns_keywords`, `completion_returns_declarations`,
  `completion_deduplicated`
- `format_no_fn`, `format_with_fn`
- `capabilities_reported_correctly`

**Structural change:** the eight stub modules (`tokenize.rs`, `parse.rs`,
`semantic_tokens.rs`, `symbols.rs`, `folding.rs`, `hover.rs`,
`completion.rs`, `format.rs`) have been removed; all logic lives in
`bridge.rs` for cohesion and discoverability.

**`Cargo.toml`**: added `lexer` and `parser` workspace dependencies; version
bumped `0.1.0 → 0.2.0`.

## 0.1.0 — 2026-05-04

Initial skeleton. Spec, types, and module structure committed.
Implementation stubs in place with detailed inline TODO guides.
See spec `LS02-grammar-driven-language-server.md` / `LS03-dap-adapter-core.md`.
