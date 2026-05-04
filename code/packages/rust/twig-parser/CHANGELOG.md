# Changelog — twig-parser

## [0.2.0] — 2026-05-04

### Added (LANG23 PR 23-E — refinement type annotation syntax)

- `TypeAnnotation` enum (`src/ast_nodes.rs`): bridges parsed Twig annotation
  syntax to `lang-refined-types::RefinedType`.  Variants:
  - `UnrefinedInt` — bare `int` annotation
  - `UnrefinedBool` — bare `bool` annotation
  - `Any` — bare `any` annotation
  - `RangeInt { lo, hi }` — `(Int lo hi)` ≡ `lo ≤ x < hi`
  - `MembershipInt { values }` — `(Member int (v0 v1 ...))` membership set
- `Lambda` extended with `param_annotations: Vec<Option<TypeAnnotation>>` and
  `return_annotation: Option<TypeAnnotation>` (lockstep with `params`; default
  empty/`None` so all pre-LANG23 callers continue to compile without changes).
- `Define` extended with `type_annotation: Option<TypeAnnotation>` for annotated
  value bindings (`(define x : (Int 0 128) val)`).
- `twig.grammar` extended with three new productions:
  - `name_or_signature` — now supports typed params and optional `ARROW type_annotation`
  - `typed_param` — bare `NAME` or `(NAME COLON type_annotation)`
  - `type_annotation` — `NAME` | `(Int lo hi)` | `(Member int (vals...))`
- `ast_extract.rs` additions:
  - `extract_type_annotation(node)` — lowers a `type_annotation` grammar node
    into a `TypeAnnotation` variant.
  - `extract_fn_signature(sig_node)` — extracts fn name, param names, per-param
    annotations, and optional return annotation from a `name_or_signature` CST node.
  - `extract_typed_param(node)` — handles bare-NAME and annotated params.
  - `extract_define` updated to handle annotated function defines and annotated
    value bindings.
  - `extract_lambda` updated to carry lockstep `param_annotations` (all `None`
    for anonymous lambdas, preserving the invariant that `len(param_annotations)
    == len(params)`).
- `TypeAnnotation` re-exported from `lib.rs`.

### Fixed

- Return-type arrow (`->`) now lexes as a dedicated `ARROW` token (defined in
  `twig.tokens` before the `NAME` pattern).  Previously `->` matched `NAME` and
  was consumed by the `{ typed_param }` repetition before the optional return
  annotation could be parsed, causing "Expected COLON, got '0'" errors on any
  function with a `-> TypeAnnotation` return type.

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig parser (TW00).
- Thin wrapper around the generic [`parser::grammar_parser::GrammarParser`](../parser),
  driven by `code/grammars/twig.grammar` — the canonical Twig parser
  grammar shared with the Python implementation.
- Public entries:
  - `parse(source) -> Result<Program, TwigParseError>` — lex + grammar-parse
    + extract typed AST in one call.
  - `parse_to_ast(source) -> Result<GrammarASTNode, TwigParseError>` —
    stop at the generic AST tree.
  - `create_twig_parser(source) -> GrammarParser` — for callers that
    want the parser object (tracing, alternative entry rules).
  - `create_twig_parser_from_tokens(tokens) -> GrammarParser` — pre-tokenised
    input for LSP-style flows.
- Typed AST: `Program`, `Form`, `Define`, `Expr` (with `IntLit`,
  `BoolLit`, `NilLit`, `SymLit`, `VarRef`, `If`, `Let`, `Begin`,
  `Lambda`, `Apply` variants).
- `ast_extract` module walks the generic `GrammarASTNode` tree → typed
  AST.  Mirrors the Python package's `ast_extract.py`.
- Define-sugar lowering at extraction time: `(define (f x) body+)` →
  `Define { name: "f", expr: Lambda { params: ["x"], body } }`.
- Both quote forms (`'foo` and `(quote foo)`) collapse to a single
  `SymLit { name: "foo" }`.
- Source-position tracking on every AST node (1-indexed `line` /
  `column`), propagated from the underlying tokens.
- `TwigParseError { message, line, column }` with
  `From<GrammarParseError>` so grammar errors propagate transparently.
- **Stack-overflow defence** — `MAX_PAREN_DEPTH = 64` cap pre-scans
  the token stream and rejects deeply-nested untrusted input before
  invoking the recursive `GrammarParser`.  Without this cap a
  pathological source like `(((...)))` with thousands of opens would
  abort the process via OS thread stack-overflow (Rust does not catch
  stack overflow).
- `MAX_AST_DEPTH = 256` cap in the extractor bounds recursion when
  callers bypass `parse()` and feed in a hand-built AST.
- 31 unit tests covering every form, sugar lowering, position
  tracking, depth cap, and error paths.
