# Changelog — twig-parser

## [0.4.0] — 2026-05-14 — LANG48 TW05-A typed syntax

### Added

New AST nodes for the TW05-A bootstrap stage (parse typed Twig syntax;
erase annotations to dynamic IIR; type checking deferred to TW05-B/C).

#### Module metadata

- `TypedMode` enum (`Off`, `Lenient`, `Strict`) — the `(typed …)` clause.
- `ModuleInfo { name, typed_mode, exports, imports }` — extracted from
  `(module name (typed …) (export …) (import …))` preamble.
- `Program::module_info: Option<ModuleInfo>` — populated when source starts
  with a `(module …)` form.

#### Type expressions

- `TypeExpr` enum (`Name(String)`, `Int(i64)`, `List(Vec<TypeExpr>)`) — a
  typed representation of raw type annotation S-expressions, used internally
  by `extract_type_annotation` to pattern-match LANG23 shapes against the
  recursive LANG48 grammar.
- `TypeAnnotation::Opaque(TypeExpr)` — catch-all variant for type expressions
  that don't map to a LANG23 shape.  The compiler erases these to `Any`.
- `TypeAnnotation` and `TypeExpr` both re-exported from `lib.rs`.

#### Record and union type declarations

- `RecordField { name, type_annotation }` — a named typed field.
- `RecordDef { name, fields, line, column }` — `(record Name (field : T) …)`.
  The AST compiler erases this to constructor + accessors + predicate functions.
- `UnionVariant { name, fields }` — one tagged variant.
- `UnionDef { name, variants, line, column }` — `(union Name (Variant …) …)`.
  Each variant gets a zero-based integer tag.
- `Form::RecordDef(RecordDef)` and `Form::UnionDef(UnionDef)` — top-level forms.
- `Form::TypeAlias(TypeAlias)` — `(type Name type_expr)`; no-op in TW05-A.
- All new `Form` variants and struct types re-exported from `lib.rs`.

#### Pattern-matching

- `MatchPat` enum: `Variant { name, bindings }`, `Binding(String)`, `Wildcard`.
- `MatchArm { pat, body }` — one arm in a `(match …)` expression.
- `Match { scrutinee, arms, line, column }` — `(match expr arm+)` expression.
- `Expr::Match(Match)` — new expression variant.
- All re-exported from `lib.rs`.

### Changed

- `extract_type_annotation` rewritten to use a TypeExpr-first approach:
  convert the recursive LANG48 grammar node to a `TypeExpr` tree, then
  pattern-match LANG23 shapes on the typed tree.  This fixes `(Int lo hi)`
  and `(Member int (vals…))` annotations silently falling through to
  `Opaque` under the new recursive grammar.
- Grammar: `type_annotation` made fully recursive (all children are
  `type_annotation` sub-nodes rather than NAME/INTEGER tokens).  See
  `code/grammars/twig.grammar`.
- Grammar: new keywords `typed`, `type`, `record`, `union`, `match` added
  to `code/grammars/twig.tokens` (using `#` comment syntax, not `;`).
- `ast_extract.rs`: adds extractors for module forms, type aliases, record
  defs, union defs, and match expressions.

### Tests added (56 total, up from 31)

- Module form parsing (typed off/lenient/strict, exports, imports).
- Type alias round-trips.
- Record and union definition parsing.
- Match expression parsing (variant, binding, wildcard arms).
- Type annotation round-trips for all LANG23 shapes via the new
  TypeExpr-first path.

---

## [0.3.0] — LS04 spec dump

### Added
- New binary `twig-spec-dump` (`bin/twig_spec_dump.rs`) — emits Twig's
  `LanguageSpec` v1 JSON document for downstream editor tooling
  (VS Code extension generator, treesitter wrappers, syntax-highlight
  generators).
- The binary uses the build-time-compiled lexer and parser grammars
  baked into `twig-lexer` and `twig-parser` rlibs — no runtime file
  I/O, no dependency on `code/grammars/twig.tokens` or
  `code/grammars/twig.grammar` being present at runtime.
- `serde_json` dependency added for pretty-printing JSON output.

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

## [0.1.1] — 2026-05-04

Security hardening — parse-time cap for membership-set cardinality.

### Added

- **`MAX_MEMBERSHIP_INT_VALUES = 256`** public constant.  LANG23 PR 23-E
  introduces `TypeAnnotation::MembershipInt { values }` which is lowered to
  an Or-of-Eq predicate by `constraint-core`.  Each value becomes one CNF
  clause in the SAT tactic; an uncapped list of 10 000+ values would blow
  the CNF budget added in `constraint-core` 0.1.1 and could stress the LIA
  tactic equally.  This constant is the canonical upper limit; PR 23-E's
  `extract_type_annotation` calls `check_membership_int_count` (below) to
  enforce it at parse time, before any lowering occurs.

- **`check_membership_int_count(count, line, column) -> Result<(), TwigParseError>`**
  public helper.  Returns `Err` with a descriptive message when `count`
  exceeds `MAX_MEMBERSHIP_INT_VALUES`.  Re-exported from the crate root
  alongside `MAX_MEMBERSHIP_INT_VALUES`.

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
