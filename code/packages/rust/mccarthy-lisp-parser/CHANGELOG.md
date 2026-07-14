# Changelog — mccarthy-lisp-parser

## v0.3.0 — 2026-07-14 — recursion-depth guard on the GrammarParser itself (second layer of defense)

* **Fix (DoS):** `create_mccarthy_parser_from_tokens` built its
  `GrammarParser` with no recursion-depth cap. This crate already has a
  `check_nesting_depth`/`MAX_PAREN_DEPTH` (64) pre-scan that rejects
  excessive combined paren+quote nesting before the parser runs, but
  **only `parse_to_cst`/`parse` call it** —
  `create_mccarthy_parser_from_tokens` is a public entry point
  (documented for editor/LSP integrations that already hold a token
  stream) that bypasses the pre-scan entirely, so a caller invoking it
  directly with adversarial tokens could still hit the same
  native-stack-overflow DoS the pre-scan was meant to close.
* Both of this grammar's independent recursive shapes were measured
  (binary search, uncapped parser, the true default per-test-thread stack
  — no `RUST_MIN_STACK` override, no explicit `Builder::stack_size`,
  bypassing `check_nesting_depth` to measure the parser's own floor):
  list nesting (the *binding*, lower floor) safe through 260 rule-frames,
  crashes at 262; quote-chain safe through 280, crashes at 290. Added
  `MAX_RULE_DEPTH = 180` — about 31% below the binding floor — and wired
  it into `create_mccarthy_parser_from_tokens` via `.with_max_depth(...)`.
* 6 new regression tests (3 per independent recursive shape), calling
  `create_mccarthy_parser_from_tokens` directly (bypassing
  `check_nesting_depth`) so they exercise the new guard specifically: deep
  adversarial input on an enlarged-stack thread returns a clean `Err`,
  input at the measured real-nesting boundary (59 levels for list
  nesting, 88 for quote-chain) still parses while one level past it
  doesn't, and the cap trips before the native stack would overflow even
  on a default-stack thread.

## v0.2.1 — 2026-06-03 — iterative Drop (DoS hardening)

* **Fix (DoS):** added an iterative `Drop` impl for `LispExpr`.  The
  compiler-generated recursive drop unwinds one stack frame per `Cons`
  cell, so dropping a *flat* list `(A A … A)` of N elements — which is
  only paren-depth 1 and therefore slips past the parser's
  `MAX_PAREN_DEPTH` guard — would recurse N frames deep and overflow the
  stack (a cheap single-line DoS on any consumer that builds and drops
  such an AST).  The new `Drop` dismantles the tree using a heap work
  list, so stack usage is O(1).  Found while building the L2a IIR
  compiler; regression-tested with a 100k-element flat list.
* Note: `LispExpr`'s `Display` is still recursive on the cdr-spine, so
  formatting a huge flat list would overflow.  `Display` is not on the
  parse path; consumers must avoid formatting untrusted ASTs (the L2a
  compiler describes expressions by *kind*, never via `Display`).

## v0.2.0 — 2026-06-03 — grammar-driven rewrite (L1)

**Breaking:** replaced the hand-written recursive-descent parser with a
thin wrapper over the shared `GrammarParser` plus a CST → typed-AST
extractor.  The S-expression grammar now lives in
`code/grammars/mccarthy_lisp.grammar`, compiled to Rust at build time
via a `build.rs` (the `twig-parser` pattern).

* The `LispExpr` AST (`Nil`, `Symbol`, `Int`, `Cons`) and its helpers
  (`sym`, `list`, `quote`, `Display`) are **unchanged** — downstream L2
  consumes the same shape.
* Sugar expansions (`'X` → `(QUOTE X)`, list nesting via NIL
  terminator, direct dotted-pair `Cons`) are preserved, now applied in
  the extractor.
* **Structural validation moved into the grammar.**  The bespoke
  `ParseError` variants (`StrayDot`, `MultipleDotsInList`,
  `DotWithoutCdr`, `ExtraAfterDottedTail`, `UnexpectedToken`,
  `UnexpectedEof`, `NestingTooDeep`) are gone; malformed dotted forms
  and unbalanced parens are now rejected by the `GrammarParser`
  itself.  `ParseError` is a flat `{ message, line, column }` struct
  (the `twig-parser` shape).
* **API changes:**
  * `parse(src) -> Result<Vec<LispExpr>, ParseError>` — unchanged
    signature.
  * **Added** `parse_to_cst`, `extract_program`,
    `create_mccarthy_parser_from_tokens`, `mccarthy_grammar`.
  * **Removed** `parse_tokens` (the lexer no longer produces the old
    `TokenWithLoc`; use `parse` or `create_mccarthy_parser_from_tokens`).
* **DoS hardening retained and corrected:** `MAX_PAREN_DEPTH` is now
  **64** (down from 256) because the shared `GrammarParser` uses far
  more stack per nesting level than the old hand-written descent did.
  The pre-parse guard (`check_nesting_depth`) bounds the *combined*
  paren **and** pending-quote nesting depth — an important fix over a
  first draft of this rewrite, which counted only parens: a long quote
  chain (`''''…X`) has paren-depth 0 but unbounded `quoted = QUOTE
  sexpr` recursion, so a ~5 KB all-`'` input would overflow the stack
  and abort the process (the shared `GrammarParser` has no internal
  recursion limit).  Both deep-paren and quote-chain inputs are now
  rejected with a clean `ParseError` (regression-tested).  A second
  `MAX_AST_DEPTH` guard bounds the extractor.  Integer overflow remains
  a `ParseError`, not a panic.
* New deps: `grammar-tools`, `lexer`, `parser` (+ `grammar-tools`
  build-dep); still depends on `mccarthy-lisp-lexer`.

## v0.1.0 — 2026-06-03 — initial release (L1)

McCarthy 1960 Lisp parser.

* `LispExpr` AST: `Nil`, `Symbol(String)`, `Int(i64)`,
  `Cons(Box<LispExpr>, Box<LispExpr>)`.
* `parse(&str)` — re-tokenize + parse.
* `parse_tokens(&[TokenWithLoc])` — parse a pre-tokenized stream.
* Sugar: `'X` → `(QUOTE X)`; standard list nesting via NIL
  terminator; dotted-pair literals parse to `Cons` directly.
* Helpers: `LispExpr::list([a, b, c])` builds the nested-Cons
  encoding, `LispExpr::quote(inner)` wraps in QUOTE.

`ParseError` is structured (`UnexpectedToken`, `UnexpectedEof`,
`StrayDot`, `MultipleDotsInList`, etc.) so downstream
compiler diagnostics can point at the right location.

Tests pin the 1960-paper examples: `(CAR '(A B C))`,
`(LAMBDA (X) X)`, `(LABEL FF (LAMBDA …))`, dotted-pair
literals, and every error path.
