# Changelog

## [0.1.0] - 2026-07-28

### Added

- Initial `axiom-runtime` crate (MA-13d, front2 Wave 7): evaluates the
  `axiom-parser` (MA-13c) `GrammarASTNode` CST as a tree-walking interpreter,
  delegating every arithmetic/comparison sub-expression to
  `symbolic_vm::VM` over the shared `SymbolicBackend` — unchanged, no custom
  `Backend` — and adding `axiom-runtime`'s own new layer for `:`/`::`/`has`
  (MA13 §2/§3): a fixed, non-extensible `AxiomDomain`/`AxiomCategory` table.
- `domains` module: `AxiomDomain` (`Boolean`, `Integer`, `PositiveInteger`,
  `NonNegativeInteger`, `Float`, `String`, `Fraction(Integer)`,
  `Polynomial(Integer)`, `List(T)`) and `AxiomCategory` (`Ring`,
  `OrderedSet`) enums; `resolve_domain`/`resolve_category` (constructor
  arity/argument-domain validation, rejecting e.g. `Polynomial(String)`
  exactly as the book's own worked example does); the fixed
  `domain_has_category` membership table (confirmed:
  `Polynomial(Integer) has Ring` → `true`, `List(Integer) has Ring` →
  `false`); `coerce_value` (the subdomain-predicate-plus-representation-
  conversion function both `::` and `:`-declared `:=` consult — `Float` is
  the one domain that actually converts representation, `Integer`/
  `Rational` → `Float`).
- `builtins` module: reads a parsed `type_expr` node (both the explicit-
  parens and paren-optional-shorthand forms) into a generic `TypeSpec`,
  independent of which built-in names are valid.
- `value` module: `AxiomValue` (an `IRNode` paired with an `Option
  <AxiomDomain>`); `infer_domain` (structural domain inference from a
  value's own evaluated shape — including the book's own confirmed
  "unresolved arithmetic over symbols is `Polynomial(Integer)`" example, for
  the un-cancelled case); `print_axiom` (Axiom surface notation: infix
  arithmetic, `~=` not-equal, `[a, b, c]` lists, lowercase `true`/`false`).
- `eval` module: the interpreter. Evaluates eagerly (not a two-phase lower-
  then-evaluate pass, unlike every sibling CAS-family runtime here) because
  `::`/`:`/`has` have no `IRNode` representation and `::` can nest anywhere
  inside ordinary arithmetic. Handles `if`/`:=`/`:`/`::`/`has`/comparison/
  arithmetic/function-call/list/block; folds a flat arithmetic chain
  iteratively (one `VM::eval` call per step) rather than building one deep
  nested tree first, sidestepping the "flat chain folds into a deep tree"
  DoS vector by construction. Function bodies are lowered structurally
  (`lower_pure_body`, never evaluated at definition time) and registered via
  the shared VM's own `Define`/user-function-call mechanism, reused
  unchanged — restricted to the arithmetic/comparison/`if`/call/list subset
  (no `:=`/`:`/`::`/`has`/blocks inside a body), a real, disclosed
  narrowing matching MA13 §4's own single confirmed function-definition
  example.
- `AxiomSession`/`eval`: a string-in/string-out facade. `axiom.grammar`'s own
  `program = expr` means one `feed` call is always exactly one statement
  (unlike Derive's/Reduce's own multi-statement-per-call `feed`), displayed
  with real Axiom's own numbered-prompt convention (`(n)`, MA13 §5) plus its
  inferred/declared domain when known (`(1) 5 : PositiveInteger`).
- Robustness: `MAX_INPUT_LEN` (64 KiB) bounds total input;
  `MAX_STATEMENT_TOKENS` (2000, measured against the real `axiom-lexer`
  token stream) exists as defense-in-depth (the iterative-fold evaluation
  strategy above already closes the main "flat chain -> deep tree" vector
  by construction, unlike sibling runtimes which need this cap as their
  primary mitigation). Evaluation runs on a 512 MiB-stack worker thread
  inside `catch_unwind`, rebuilding the session (VM environment *and*
  declared-domain table) after any caught panic.
- 97+ unit tests across `domains`/`builtins`/`value`/the top-level session,
  covering domain inference, every built-in domain/category `has` pair
  (including both of the book's own confirmed true/false examples),
  subdomain predicates, the coercion/declaration-mismatch error shape,
  arithmetic/comparison delegation, `:=`/`==`/`if`/block evaluation
  (including recursion through a user-defined function), the function-body
  restriction, and both robustness guards.
