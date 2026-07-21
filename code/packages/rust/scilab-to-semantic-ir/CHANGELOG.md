# Changelog

## [0.1.0] - 2026-07-20

### Added

- Initial `scilab-to-semantic-ir` frontend crate (MA10 §6, task MA-10e —
  the last item in the Scilab frontend rollout, built alongside
  `scilab-runtime`/`scilab-repl` per `HML01` §2's "every math language
  gets a `-to-semantic-ir` frontend built alongside the runtime"
  convention): `compile`/`compile_source` lowering
  `coding-adventures-scilab-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module` over the shared SIR22 array/matrix domain.
- Built as a close structural mirror of `matlab-to-semantic-ir` (per MA10
  §5's finding that the grammar *shape* is a legitimate MATLAB-family
  inheritance even though the *language* is not): the same
  `compile`/`compile_source` shape, the same `LowerError { message, line,
  column }` error shape (renamed `ScilabLowerError`), and the same
  scalar/array disambiguation heuristic for `+ - * / \` and the same
  "clean error over silent mis-lowering" philosophy for anything outside
  a well-defined first-cut scope.
- Supported: literals (int/float/string — both `'...'` and `"..."`
  spellings lower to the same `Expr::StrLit`, MA10 §3), assignment
  (`LetStarBinding` on first occurrence, `Assign` on re-assignment),
  arithmetic (`+ - * / \ ^` and their dotted elementwise forms),
  comparisons (`== ~= <> < > <= >=` — both not-equal spellings),
  logical `&& || & |`, unary `+ - ~`, ranges (`a:b`, `a:step:b`),
  transpose (`'` and `.'`), matrix literals (`ArrayLit`), indexing (read
  → `IndexGet`, write → `Stmt::IndexSet`) with 1-based → 0-based
  translation at lowering time, `if`/`elseif`/`else` and `while`/
  `for i = a:b` (both accepting the optional `then`/`do` linker keyword
  or a bare comma/newline, MA10 §3's `stmt_sep`), `select`/`case`/`else`
  (desugared into a nested `if`-chain over a hoisted selector
  temporary), the eight `%`-prefixed special constants (constant-folded
  to `IntLit`/`FloatLit`), single- and zero-output function definitions
  (including the explicit `[] = f(...)` and single-name-in-brackets
  `[y] = f(...)` spellings) and calls to them, and `disp` (mapped onto
  the shared SIR `print` builtin).
- Explicit, disclosed scope limits (each rejected with a clear
  `ScilabLowerError`, never silently mis-lowered): `$` (last-index,
  mirrors the MATLAB template's `end`-relative-indexing exclusion — no
  `size`/`shape` builtin is wired up yet to resolve it at lowering time),
  `%i` (complex numbers — `array-runtime` has no representation, mirrors
  `scilab-runtime::builtins::percent_const`'s identical choice),
  multi-output functions (`[a, b] = f(...)`, mirrors the MATLAB
  template's identical exclusion), `break`/`continue` (semantic-ir has no
  early-exit control-flow node at all yet — a whole-IR gap, not specific
  to this frontend), stepped/non-range `for` loops, matrix power and
  right division (`/` mrdivide) between non-scalars, nested function
  definitions, cell arrays/`list`/`tlist`/`mlist`, field access, chained
  assignment, auto-vivification on indexed assignment, and any
  arithmetic/ordering operator over a directly-written string literal
  operand (equality remains in scope).
- One deliberate divergence from the `matlab-to-semantic-ir` template:
  `\`/`.\ ` (left division) are lowered *uniformly* as a broadcast
  reciprocal division regardless of operand scalar-ness, rather than
  mirroring the MATLAB template's asymmetric treatment (which rejects
  bare `\` between non-scalars as an unimplemented matrix solve).
  `scilab-runtime::eval::apply_binop`'s own doc comment confirms this
  repo's ground-truth Scilab interpreter already makes this exact
  simplification for both spellings uniformly, so lowering bare `\` more
  strictly than the language's own shipped interpreter would be a
  regression relative to the actual authority on what Scilab's `\`
  computes, not a safety improvement. See `src/lower.rs`'s module doc
  comment for the full rationale.
- One deliberate addition beyond the `matlab-to-semantic-ir` template:
  every additive/multiplicative/power/ordering-comparison operator
  construction site rejects a directly-written `Expr::StrLit` operand
  with a clean error. MA10 §1 finding 1 — the decisive finding motivating
  this whole language's existence as its own frontend rather than a thin
  MATLAB wrapper — is that Scilab's `+` means concatenation on strings
  where MATLAB's means ASCII-numeric addition; the MATLAB template's own
  `expr_is_known_scalar` heuristic does not special-case a string operand
  at all, so a literal string reaching `+`/`-`/`*`/... there would
  silently take the ordinary array-domain path — precisely the silent
  mis-lowering MA10 §1 finding 1 warns against. This is a syntactic,
  non-evaluating check (a variable merely known to hold a string is not
  caught, since this frontend has no type inference), disclosed plainly
  in `src/lower.rs`'s module doc comment.
- One deliberate improvement beyond the `matlab-to-semantic-ir` template:
  `func_returns` parsing distinguishes `NAME EQ` (single output), the
  explicit-bracket single-name spelling `[y] = f(...)` (also single
  output — the MATLAB template's coarser handling treats any non-empty
  bracket name list as unconditionally multi-output and would incorrectly
  reject this), the explicit empty-bracket zero-output spelling
  `[] = f(...)`, and a genuine multi-name bracket list (rejected, out of
  scope) — mirroring `scilab-runtime::eval::Interpreter::register_function`'s
  own more complete three-way reading of this grammar shape, whose doc
  comment confirms its own handling is "exactly correct."
- 96 tests: 72 unit tests over lowering shapes, every documented scope
  limit's rejection, and the DoS-guard regression set
  (`a_pathologically_long_flat_{additive,multiplicative}_chain_is_
  cleanly_rejected` plus the `elseif`/`case`/transpose-suffix chain
  variants added during security review, see below); 11
  validator/capability-acceptance tests; 12 end-to-end tests that
  actually execute lowered Scilab through `semantic-ir-to-javascript` and
  `node` (gated on `node` availability).
- Marks `scilab-to-semantic-ir` (MA-10e) done in
  `MA10-scilab-language.md` §6 — the last item in the Scilab frontend
  rollout.

### Security

Three rounds of pre-merge security review found and fixed six issues, all
before this crate ever shipped:

- **CRITICAL/HIGH** (round 1): `lower_if`/`lower_select` folded
  `elseif_clause*`/`case_clause*` — flat `{ x }`-repetition CST nodes,
  costing the *parser* zero native stack — into an `Expr::If` chain N
  levels deep via `Box`, with no cap. A source file with 300,000 `elseif`
  clauses compiled successfully, but merely dropping the returned
  `Module` overflowed the native stack and aborted the process
  (uncatchable by `catch_unwind`). Fixed by capping `clauses.len()`
  against the existing `MAX_EXPR_DEPTH` (256) in both functions, before
  any per-clause lowering.
- **CRITICAL/HIGH** (round 2): the identical hazard, found independently
  in `lower_postfix`'s `{ transpose_suffix | call_suffix | ... }` suffix
  fold — the same flat-repetition shape, also unguarded. 300,000 chained
  transpose/call suffixes reproduced the identical uncatchable
  stack-overflow abort. Fixed the same way: a `suffixes.len() >
  MAX_EXPR_DEPTH` cap before the fold.
- **MEDIUM** (round 1): the `select`/`case` hoisted selector temp was
  named `__select_N` — an ordinary, fully legal Scilab identifier a
  program could itself declare, silently colliding with (and producing
  invalid, duplicate-declaration JavaScript from) a same-named user
  variable. Fixed by renaming to `$select_N`: `$` is never part of a
  Scilab `NAME` token, so collision is now structurally impossible.
- **MEDIUM** (round 1): `if`/`else` and `select`/`case`/`else` bodies
  (mutually exclusive) were lowered against a single, still-mutating
  `ctx.locals`, so a name first-assigned in one branch was misclassified
  as a re-assignment by a sibling branch lowered afterward — breaking the
  ordinary idiom of a function assigning its own output variable in
  every branch of an `if`/`select`. Fixed by lowering every branch
  against the same pre-statement `ctx.locals` snapshot (the existing
  `for`-loop scope_mark/scope_rewind mechanism), folding the union of
  every branch's newly-introduced names back in once at the end.
- **MEDIUM** (round 2): the round-1 fix above, and `lower_assignment`'s
  pre-existing first-occurrence check, both tested local-name membership
  via `Vec::contains` — an O(n) scan per check, making a flat sequence of
  n distinct-name assignments (or an if/select's branches collectively
  introducing n names) O(n²) overall. Fixed by adding a `HashSet`-backed
  `FunctionCtx::has_local`/`push_local` pair (kept in sync with the
  existing `Vec` `scope_mark`/`scope_rewind` needs for ordered
  truncation), and backing the branch-fix's own `newly_introduced`
  accumulator with a `HashSet` instead of a `Vec`.
- **HIGH** (round 3, a regression introduced by round 2's own fix): the
  new `push_local` unconditionally appended to `locals`/`locals_set` even
  when the name was ALREADY a known local — the case hit by `lower_for`
  reusing an already-assigned variable as the loop counter (`y = 1; for
  y = 1:3 ... end`, an ordinary Scilab idiom). `scope_rewind` at the
  loop's own scope boundary then drained that duplicate `locals` entry
  and removed the name from `locals_set` entirely — since a `HashSet` has
  no duplicate-count tracking, this erased the variable's pre-existing
  membership too, not just the loop-local copy. Confirmed via `node`:
  the resulting JS had two top-level `let y` declarations in the same
  scope, rejected with "Identifier 'y' has already been declared". Fixed
  by making `push_local` a no-op when the name is already known — it
  needs no scope tracking at all in that case, since it was in scope
  before the push and stays in scope after, regardless of what happens in
  the interim scope.
- **LOW** (round 3): `lower_select` checked only `kids.is_empty()` before
  indexing `kids[0]`/slicing `&kids[2..]`, unlike every sibling function's
  length-based guard (e.g. `lower_if`'s `kids.len() < 3`). A
  directly-constructed malformed `select_stmt` (unreachable via the real
  `scilab-parser`, but `compile()`/`GrammarASTNode` is a public API) with
  exactly one child panicked on the slice instead of erroring cleanly.
  Fixed by checking `kids.len() < 2` instead.

Also separately discovered (not fixed here, tracked as its own follow-up
task): `scilab-parser`'s own `elseif_clause*` parsing appears to scale
worse than linearly at very large clause counts (17s to parse 100,000
`elseif` clauses in `--release`) — a pre-existing, already-merged,
separate crate, out of scope for this PR.

### Notes on divergence from the spec

`MA10-scilab-language.md` §5/§6 anticipated that the `stmt_sep` linker
keyword and `select`/`case` would need no new SIR node, and that the
`%`-prefixed constants could be constant-folded rather than needing a
dedicated node — this implementation confirms all three predictions
exactly as stated; no new `semantic-ir` core `Expr`/`SirType`/`Feature`
variant was added. The spec did not anticipate the `\`/`.\ ` uniform-
broadcast divergence from the MATLAB template, nor the string-operand
guard, nor the more complete `func_returns` three-way reading — these are
implementation-level refinements below the spec's own level of detail,
each traceable to a concrete finding (respectively:
`scilab-runtime::eval::apply_binop`'s own documented simplification, MA10
§1 finding 1's own stated concern, and
`scilab-runtime::eval::Interpreter::register_function`'s own doc comment)
rather than a change to the spec's architectural decisions.
