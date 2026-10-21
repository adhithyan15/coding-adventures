# Changelog

## 0.3.0 - Unreleased - walk the parse tree, not the typed AST (LANG84)

LANG84 decision 6: *the IIR compiler walks the parse tree*. No hand-written
code may turn the CST into an owned typed AST before IIR compilation. This
crate broke that rule: `compile_source` called the parser crate's
`parse_javascript_program_division_only`, whose 4,200-line `bridge.rs` built a
`javascript_ast::Program`, and the compiler lowered that.

* **Input is now the `GrammarASTNode` CST.** `compile_source` calls the new
  `coding_adventures_javascript_parser::parse_javascript_cst_division_only`
  (the same tokenizer, exponent-token rejection, ASI parse and spine bound as
  before, minus the bridge) and hands the tree to `compile_ast`. The edition
  is still ES2020.
* **Breaking:** `compile_ast` now takes `&GrammarASTNode` instead of
  `&javascript_ast::Program`. No crate in the workspace called it. A tree from
  either public CST parse (`parse_javascript_cst_division_only`, or the
  regex-aware `parse_javascript_typed`) works.
* **Borrowed views.** `Expr` is a `Copy` handle into the tree: a node, the
  left fold of the first operands of a flat binary node (`Fold`), the prefix
  of a flat suffix chain (`Chain`), or a `.name` token. `classify` says what a
  handle is, one level deep, as a `Shape` that mirrors the typed variants the
  compiler used to match (identifier, number, unary, binary, call, member,
  other). Wrappers and parentheses are crossed in a loop. Left associativity
  is index arithmetic, so no tree is folded or rebuilt (LANG84 C1.6), and
  operators and keywords are recognised by effective token type (C1.5).
* **Same errors.** A new validation pass, `check_cst`, runs over the whole
  program first, as the bridge did, and raises the bridge's errors for the
  constructs the compiler supports, verbatim: a hex/binary/octal literal wider
  than 64 bits (`InternalError in bridge for rule 'primary_expression': failed
  to parse numeric literal '…'`), a destructuring declaration
  (`UnsupportedSyntax: grammar rule 'binding_pattern' at L:C …`), a reserved
  word used as a value, a non-`program` root and other malformed trees. These
  still take precedence over the duplicate-declaration early error and every
  lowering error. The number decoder is the bridge's, rule for rule (checked
  against the typed AST as an oracle in `tests/cst_walk.rs`).
* **Same IIR.** The 0.2.0 and 0.3.0 compilers were run side by side over
  8,346 distinct inputs: every string literal in the Rust sources of the
  `javascript-*`, `closure-*`, `jsdoc-*` and `semantic-ir-to-javascript`
  crates, every repository `.js` file under 70 KB, and a generated and
  hand-written edge corpus (each of ~70 expression snippets in 7 statement
  forms; division, unary and paren chains of 1 to 5,000 terms; 16,383 to
  16,385 statements; the 4,096-declaration budget edge; regex, template,
  ASI, comment and number-spelling cases). `compile_source` results (the
  `Debug` dump of the `IIRModule`, or the error text) were identical for
  8,304 inputs: 2,181 compiled and 6,123 failed with the same message. The
  same held for `compile_ast` over the regex-aware parse (2,159 + 6,144) and
  for `run_source` output and errors (8,304). No input compiled on one side
  and failed on the other, and no compiled module differed.
* **Differences** (42 inputs), all on programs that were and still are
  rejected with no output. In 40, the program holds a construct the pilot
  does not compile (`async`, a class or object method, destructuring in a
  parameter or `for`-`in`/`of` head, `8 /= 2`, …) whose insides the bridge
  rejected with its own `UnsupportedSyntax` text; the compiler now reports
  its own error instead (the unsupported statement or expression, or an
  earlier item's error such as a `var` declaration). In the other 2 the old
  pipeline *panicked*: the bridge's string unquoting slices out of range on a
  string token whose unquoted text is a lone quote (for example
  `console.log('"')`); the compiler never decodes strings, so it now returns
  `unsupported JavaScript expression`.
* **Hostile trees.** `compile_ast` is public, so it also guards trees built by
  hand: a binary node that does not alternate operand/operator, a chain
  suffix with a missing part, a declarator with no name and similar shapes are
  errors, and a tower of 20,000 wrapper nodes compiles on a 256 KiB stack.
* **Tests.** All 24 unit tests pass; the ones that forged typed ASTs now forge
  CSTs from real parse-tree templates. The new `tests/cst_walk.rs` (13 tests)
  adds source checks that `src/lib.rs` never names `javascript_ast`, the
  bridge or `parse_javascript_program*`, imports only CST parses from the
  parser crate, and keeps `javascript-ast` out of `[dependencies]`; plus
  `compile_ast`/`compile_source` agreement, verbatim legacy errors and their
  precedence, the number-decoding oracle, malformed and deep hand-built
  trees, and CST spellings such as `(console).log(1)`.
* Dependencies: `coding-adventures-javascript-ast` moves to
  `[dev-dependencies]` (test oracle only); `lexer` and `parser` are new direct
  dependencies for the `Token` and `GrammarASTNode` types.

## 0.2.0

- Accept top-level numeric `let` and `const` declarations with one plain
  identifier binding and an initializer from the numeric subset, and
  identifier reads in expressions, initializers and `console.log` arguments.
  Each binding lowers directly from the typed AST to a `mov` into register
  `js.<name>`, a namespace disjoint from temporaries (`js<n>`) and runner
  builtins (`js_<name>`). Declarations are accepted both as statements and
  directly at program level in caller-built ASTs.
- Report a duplicate lexical name as the early error
  `SyntaxError: Identifier '<name>' has already been declared`, and `const`
  without an initializer as `SyntaxError: Missing initializer in const
  declaration`, both with no output.
- Raise `ReferenceError: <name> is not defined` for undeclared reads and
  `ReferenceError: Cannot access '<name>' before initialization` for
  temporal-dead-zone reads (including a binding's own initializer) through two
  narrow runner builtins at the read site. Earlier completed console output is
  kept.
- Reject at compile time: `var`, `let` without an initializer, multiple
  declarators, destructuring, non-numeric initializers, assignment
  expressions, reads or bindings of runner global names (ECMA-262 globals,
  `escape`/`unescape`, `Intl`, `console`), and caller-built identifiers that
  are not plain IdentifierName spellings.
- Charge one node per declarator and one per binding name, and apply the node
  and depth budget to initializers of caller-supplied ASTs.

- Select an explicit division-only source parser for the native numeric
  subset. Repeated division now lowers directly to IIR, including combined
  negative-zero, Infinity and NaN console fixtures. Retain regex rejection,
  source/AST/VM bounds and atomic display failure; general JavaScript
  lexical-goal disambiguation remains separate.
- Reject excessive flat operator/member/call spines before typed AST
  construction, avoiding recursive-drop stack overflow on hostile source.
  Preserve the existing 64-level semantic depth boundary and source limits.
- Reject unsupported exponentiation tokens before parsing, preventing stack
  overflow on right-recursive exponent chains before the CST guard can run.

- Accept exactly three numeric `console.log` arguments through direct typed
  AST-to-IIR lowering and Rust vm-core execution. Preserve source order,
  existing argument forms and budgets, and earlier completed output when
  number display fails in any argument; reject four or more arguments.
- Accept exactly two numeric `console.log` arguments through the typed AST
  and Rust VM builtin, formatting them with one space and one newline. Keep
  completed earlier output if a later call fails before appending its text.
- Accept zero-argument `console.log()` through the typed JavaScript AST and
  Rust VM builtin, appending exactly one newline without a numeric operand.
  Preserve that completed newline if a later accepted VM operation fails.
- Preserve completed JavaScript console output when a later VM instruction fails.

## 0.1.0

- Add a bounded direct JavaScript AST to IIR compiler and native VM runner.
- Bound source loading, typed AST lowering, and console output before allocation.
