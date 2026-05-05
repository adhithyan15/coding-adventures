# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `exists_fileo(path)`, `read_file_to_stringo(path, contents)`, and
  `read_file_to_codeso(path, codes)` for bounded UTF-8 file text I/O from
  bound atom/string paths.
- `openo(path, mode, stream)`, `closeo(stream)`, `read_stringo/3`,
  `read_line_to_stringo/2`, `get_charo/2`, `at_end_of_streamo/1`,
  `writeo/2`, and `nlo/1` for bounded UTF-8 file stream handles.
- `open_optionso/4`, `current_streamo/3`, `stream_propertyo/2`, and
  `flush_outputo/1` for bounded stream aliases, option validation, and
  metadata.
- `set_stream_positiono/2` and `seeko/4` for bounded read-stream cursor
  repositioning.
- `betweeno(low, high, value)` for finite inclusive integer generation and
  validation, matching the common Prolog `between/3` use case.
- `integero(term)` for non-bool integer type checks and `succo(predecessor,
  successor)` for non-negative integer successor generation and validation.
- `labeling_optionso(options, vars)` for a first CLP(FD) labeling option
  subset, including `leftmost`, `ff`, `up`, and `down`.
- `fd_scalar_producto(coeffs, vars, total)` for CLP(FD)-style weighted sum
  constraints over Python sequences or proper logic lists.
- `fd_sum_relationo(vars, op, total)`,
  `fd_scalar_product_relationo(coeffs, vars, op, total)`, and
  `fd_elemento(index, vars, value)` for richer CLP(FD) modeling constraints.
- `fd_reify_relationo(left, op, right, truth)` and boolean FD connectives for
  CLP(FD)-style reification and truth-table modeling.
- variadic `calltermo(term_goal, *extra_args)` for Prolog-style meta-call
  argument extension.
- higher-order list predicates `maplisto/2..5`, `convlisto/3`, `includeo/3`,
  `excludeo/3`, `partitiono/4`, `foldlo/4..7`, and `scanlo/4..7` backed by
  callable-term execution.
- `not_same_termo(left, right)` as the strict non-identity counterpart to
  `same_termo(left, right)`.
- `difo(left, right)` for delayed disequality constraints that block future
  unification instead of failing merely because terms are still open.
- source-level Prolog arithmetic error classes plus strict arithmetic goals
  for `is/2`, `=:=/2`, `=\=/2`, `</2`, `=</2`, `>/2`, and `>=/2` adapters.
- `throwo(ball)` and `catcho(goal, catcher, recovery)` for Prolog-style
  exception control, including catchable structured runtime errors.
- `term_variableso(term, variables)` for collecting unique reified variables
  in first occurrence order.
- `numbervarso(term, start, end)` for binding open variables to `'$VAR'(N)`
  placeholders in first occurrence order.
- `compound_name_argumentso(term, name, arguments)` and
  `compound_name_arityo(term, name, arity)` for compound-only term reflection
  and construction.
- `acyclic_termo(term)` and `cyclic_termo(term)` for standard finite-term and
  rational-tree shape checks.
- `unify_with_occurs_checko(left, right)` and
  `unifiableo(left, right, unifier)` for explicit finite unification and
  non-binding unifier inspection.
- `term_hasho(term, hash)` and `term_hash_boundedo(term, depth, range, hash)`
  for deterministic structural term hashes.
- `current_prolog_flago(name, value)` for enumerating read-only runtime flags
  exposed by the Prolog compatibility layer.
- `set_prolog_flago(name, value)` for branch-local updates to supported
  Prolog runtime flags.
- `variant_termo(left, right)`, `not_variant_termo(left, right)`, and
  `subsumes_termo(general, specific)` for non-binding term generality checks.
- `atom_charso/2`, `atom_codeso/2`, `number_charso/2`, `number_codeso/2`,
  `char_codeo/2`, `string_charso/2`, and `string_codeso/2` for finite text
  conversion relations.
- `atom_concato/3`, `atomic_list_concato/2`,
  `atomic_list_concato_with_separator/3`, and `number_stringo/2` for finite
  atom composition and number/string conversion modes.
- `atom_lengtho/2`, `string_lengtho/2`, `sub_atomo/5`, and `sub_stringo/5`
  for finite atom/string length and slicing relations.

### Fixed

- `bagofo(...)` and `setofo(...)` now implement Prolog-style free-variable
  grouping and `^/2` existential scopes when their goal scope is representable
  as a callable term.
- `iftheno(...)` and `ifthenelseo(...)` now preserve rule-local variable
  freshening when their condition and branches can be represented as callable
  terms.
- `onceo(...)`, `noto(...)`, and `forallo(...)` now preserve rule-local
  variable freshening when their embedded goals can be represented as callable
  terms.
- `findallo(...)`, `bagofo(...)`, and `setofo(...)` now preserve rule-local
  variable freshening when their embedded goals can be represented as callable
  terms.

## [0.13.0] - 2026-04-22

### Added

- `fd_sumo(terms, total)` for CLP(FD)-style n-ary sum constraints over Python
  sequences or proper logic lists
- domain-pruning tests for concrete totals, result variables, empty sums, and
  a resource-allocation example that reads like a small planning model

## [0.12.0] - 2026-04-22

### Added

- end-to-end finite-domain examples for Australia map coloring, a 4x4 Latin
  square, and a simple precedence-constrained task schedule
- tests proving CLP(FD) examples can be solved directly through the Python
  library API before parser integration

### Changed

- `labelingo` now chooses the currently smallest finite domain first, using the
  caller's variable order as a stable tie-breaker, so constrained examples do
  less avoidable search while keeping deterministic answers

## [0.11.0] - 2026-04-22

### Added

- generalized finite-domain residual constraints so binary comparisons,
  arithmetic constraints, and global constraints share one propagation loop
- `fd_addo`, `fd_subo`, and `fd_mulo` for CLP(FD)-style addition,
  subtraction, and multiplication over finite integer domains
- `all_differento` for pairwise-distinct finite-domain terms, including
  duplicate concrete-value checks and singleton-domain pruning
- tests covering arithmetic domain pruning, order-independent arithmetic
  constraints, multiplication, singleton all-different pruning, duplicate
  concrete values, and a tiny Latin-square-style solve

### Changed

- the Unix `BUILD` script removes coverage.py's optional native tracer on
  Darwin after dependency installation so local macOS builds reliably fall back
  to the Python tracer

## [0.10.0] - 2026-04-22

### Added

- finite-domain store types for branch-local CLP(FD)-style constraints
- `fd_ino`, `fd_eqo`, `fd_neqo`, `fd_lto`, `fd_leqo`, `fd_gto`, and
  `fd_geqo` for finite integer domains and binary comparisons
- `labelingo` for deterministic ascending enumeration of finite-domain
  assignments
- tests covering domain formats, domain narrowing, order-independent
  constraints, equality-domain intersection, rollback across disjunctions, and
  explicit labeling

## [0.9.0] - 2026-04-22

### Added

- `cuto()` as the library spelling of Prolog cut backed by engine-level scoped
  choicepoint pruning
- builtin predicate metadata for `cuto/0`
- tests distinguishing `cuto()` from `onceo(...)` and proving it commits the
  surrounding search frame

## [0.8.0] - 2026-04-21

### Added

- dynamic database predicates: `dynamico`, `assertao`, `assertzo`,
  `retracto`, `retractallo`, and `abolisho`
- predicate metadata now observes branch-local dynamic declarations and
  reports `dynamic` properties
- `clauseo` now sees runtime dynamic clauses from the active search state
- tests covering assertion order, rollback across branches, retraction
  bindings, retract-all, abolish, static-predicate protection, dynamic source
  clauses, metadata, and clause introspection

## [0.7.0] - 2026-04-21

### Added

- `calltermo(term_goal)` for executing reified Prolog-shaped goal terms
- standard term-order predicates: `compare_termo`, `termo_lto`,
  `termo_leqo`, `termo_gto`, and `termo_geqo`
- predicate metadata predicates: `current_predicateo` and
  `predicate_propertyo`
- tests proving clause-body round trips, non-binding term comparisons, and
  source/builtin predicate metadata

## [0.6.0] - 2026-04-21

### Added

- `clauseo(head, body)` for Prolog-style clause introspection from inside logic queries
- support for relation-call head arguments, source-order clause enumeration, fact bodies as `true`, rule body term encoding, and standardize-apart behavior
- tests covering head/body filtering, instantiated rule bodies, returned variable freshness, and host-only body skipping

## [0.5.0] - 2026-04-20

### Added

- term metaprogramming predicates: `univo`, `copytermo`, `same_termo`, `atomico`, and `callableo`
- construction-mode `functoro` for atoms and compounds with fresh argument variables
- tests covering Prolog-style term decomposition, construction, variable-refreshing copies, strict identity, and callable/atomic classification

## [0.4.0] - 2026-04-20

### Added

- advanced control predicates: `trueo`, `failo`, `iftheno`, `ifthenelseo`, and `forallo`
- tests covering committed-condition behavior, then-branch backtracking, else-state isolation, and forall binding discipline
- documentation explaining why real Prolog cut is deferred until the solver can prune scoped choicepoints

## [0.3.0] - 2026-04-20

### Added

- collection predicates: `findallo`, `bagofo`, and `setofo`
- deterministic term sorting and duplicate removal for first-pass `setofo`
- tests and examples showing collectors with relation search, arithmetic, and control builtins

## [0.2.0] - 2026-04-20

### Added

- arithmetic expression constructors: `add`, `sub`, `mul`, `div`, `floordiv`, `mod`, and `neg`
- `iso(result, expression)` as the library spelling of Prolog's evaluative `is/2`
- numeric comparison predicates: `numeqo`, `numneqo`, `lto`, `leqo`, `gto`, and `geqo`
- tests and examples showing arithmetic composition with relation search and control builtins

## [0.1.0] - 2026-04-20

### Added

- Prolog-inspired control predicates: `callo`, `onceo`, and `noto`
- term state/type predicates: `groundo`, `varo`, `nonvaro`, `atomo`, `numbero`, `stringo`, and `compoundo`
- first inspection-mode structural predicates: `functoro` and `argo`
