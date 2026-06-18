# Changelog

All notable changes to `wolfram-runtime` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

## [0.2.0] — 2026-06-17

The **W-5** deliverable (MA04 §8): more built-ins & evaluation, layered onto the
*same* symbolic substrate W-4 uses — no bespoke evaluator, and no edit to
`symbolic-vm`'s shared handler table.

### Added

- **`WolframBackend`** (`backend` module) — a decorator over the shared
  `SymbolicBackend`. It answers `handler_for` from a small W-5 built-in table and
  delegates everything else (`lookup`/`bind`/`on_unresolved`/`on_unknown_head`/
  `rules`/`hold_heads`, and every W-4 handler) to the inner backend. This keeps
  the new surface local to the Wolfram lane while reusing the entire evaluation
  engine, the `Plus`→`Add` bridge, user-defined functions, and `/.`.
- **List/functional/control/numeric built-ins** (`builtins` module):
  - `Length[{…}]` — element count (`0` for an atom; argument count for a non-list
    head).
  - `First` / `Last` — first/last element; **empty list left unevaluated** (no
    panic).
  - `Part[expr, i]` — **1-based** indexing; `i = 0` is the head; negative `i`
    counts from the end; out-of-range / non-integer index left unevaluated.
  - `Append[{…}, x]` — a new list with `x` appended (values are immutable).
  - `Range[n]` / `Range[a, b]` / `Range[a, b, d]` — integer ranges, **DoS-capped**
    at `MAX_RANGE_LENGTH` (1,000,000) elements *before* allocation, so a tiny
    `Range[10^9]` is left unevaluated rather than exhausting memory.
  - `Map[f, {…}]` and `Apply[f, {…}]` — re-evaluate the constructed `f[…]` through
    the VM, routing the head through the same canonical bridge as W-4 lowering
    (`build_canonical_application`), so `Apply[Plus, {1, 2, 3}]` folds to `6`.
  - `N[expr]` — coerce exact `Integer`/`Rational` to `Float`, mapping over a list
    element-wise; symbolic and already-float values pass through.
- `MAX_RANGE_LENGTH` is re-exported.
- **`If` and the comparison/logical heads** (`==`, `!=`, `<`, `>`, `<=`, `>=`,
  `&&`, `||`, `!`) already evaluated through the shared backend in W-4; W-5 pins
  them with end-to-end tests.

### Notes

- No grammar/lexer change: every W-5 head is a function-call form the existing
  `head[args]` grammar already parses. The operator *sugar* (`/@` Map, `@@` Apply,
  `[[ ]]` Part) is deferred to W-6 (MA04 §2/§4).
- All new built-ins run inside the existing W-4 worker-thread `catch_unwind`, so
  an unforeseen handler panic still becomes a clean `Err` and the session is
  rebuilt.

## [0.1.0] — 2026-06-17

Initial release — the **W-4** deliverable of the Wolfram-language lane (MA04 §7).

### Added

- `WolframSession` — a persistent, string-in / string-out runtime that lowers the
  parsed M-expression `GrammarASTNode` from `wolfram-parser` to `symbolic-ir` and
  evaluates it with `symbolic-vm`'s `SymbolicBackend`. Variable bindings (`x = 5`)
  and user-defined functions (`f[x_] := x^2`) persist across `feed` calls; the
  `Out[n]` counter persists too.
- `WolframSession::feed` (string echo) and `eval_to_outputs` (structured
  `Output`s), plus a one-shot `eval` helper.
- **Lowering** (`lower` module): the surface→IR desugaring. The head-name bridge
  maps both the infix operators and the explicit head-applications
  (`Plus`/`Times`/`Power`/`Subtract`/`Divide`/`Minus`/`Equal`/`And`/…) onto the
  canonical IR heads (`Add`/`Mul`/`Pow`/`Sub`/`Div`/`Neg`/`Equal`/`And`/…), so
  `1 + 2` and `Plus[1, 2]` evaluate identically. n-ary `Plus`/`Times` are
  left-folded into binary chains the VM folds. `Set`→`Assign`, `SetDelayed`→
  `Define` (with `x_` parameters reduced to the bound symbol for the VM's
  symbol-based parameter binding). Pattern blanks (`_`, `x_`, `_h`, `x_h`) and
  rules (`->`, `:>`) lower to the `cas-pattern-matching` `Blank`/`Pattern`/`Rule`/
  `RuleDelayed` node shapes.
- **ReplaceAll** (`/.`): a synthetic `ReplaceAll` head is intercepted before VM
  evaluation and dispatched through `cas-pattern-matching::rewrite`. A rule's RHS
  bare references to LHS-bound pattern names are rewritten into the
  `Pattern(name, Blank())` reference form the matcher's `substitute` understands.
  Supports a single rule or a `List` of rules.
- **Pretty-printing** (`printer` module): renders the evaluated IR back to Wolfram
  surface notation (infix operators, `f[…]` application, `{…}` lists), with
  precedence-aware parenthesisation so the output re-parses to the same tree.
- **Trust-boundary hardening**, mirroring `maxima-runtime`: `MAX_INPUT_LEN` (64
  KiB) input cap; `MAX_STATEMENT_TOKENS` per-statement token cap measured on the
  real `wolfram-lexer` token stream (bounding parse-tree depth so deep nesting
  cannot overflow the stack on build or drop); evaluation on a bounded
  large-stack worker thread inside `catch_unwind`, with full session rebuild after
  any caught panic. `MAX_REWRITE_ITERATIONS` bounds `/.` rewriting.

### Notes

- Scope is the W-1 grammar subset (MA04 §4): explicit `*` required (no
  juxtaposition multiplication), no `[[…]]`/`;;`/`@`/`&`/`#` etc. `Simplify`/
  `Expand` and the full `cas-*` surface are W-6.
- Built on `symbolic-ir` 0.2, `symbolic-vm` 0.20, `cas-pattern-matching` 0.1.
