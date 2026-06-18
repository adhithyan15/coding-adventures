# Changelog

All notable changes to `wolfram-runtime` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

## [0.4.0] — 2026-06-17

The **W-7** deliverable (MA04 §10): iteration constructs — the first Wolfram-lane
forms that introduce a *scoped local index*. `Table`, `Do`, `Sum`, and `Product`
bind a fresh variable `i` to each value of a range and evaluate a body once per
value, lowered onto the *same* `symbolic-vm` substrate (no bespoke loop opcode,
no new evaluator).

### Added (iteration heads)

- **`Table[expr, {i, imax}]`** / **`{i, imin, imax}`** / **`{i, imin, imax, di}`**
  → the list of `expr` evaluated with `i` bound over the range. So
  `Table[i^2, {i, 3}]` is `{1, 4, 9}` and `Table[i, {i, 2, 4}]` is `{2, 3, 4}`.
- **`Do[expr, {i, n}]`** → evaluate `expr` `n` times for side effects (e.g. a
  `Set` in the body), returning `Null`.
- **`Sum[expr, {i, imin, imax}]`** → fold `+` over the range
  (`Sum[i, {i, 1, 10}]` is `55`); an empty range sums to `0`.
- **`Product[expr, {i, imin, imax}]`** → fold `×`
  (`Product[i, {i, 1, 4}]` is `24`); an empty range is `1`.

### How the index binds

- The four heads are **held** — `WolframBackend::hold_heads` now returns the
  union of the inner `SymbolicBackend` held set (`If`, `Assign`, `Define`, …) and
  `{Table, Do, Sum, Product}`, so the body and iterator spec arrive unevaluated.
- Each iteration binds `i → value` with the **same `vm.rs::substitute`** that
  binds user-function parameters, then re-evaluates the body through the VM. The
  index stays *local* (it never leaks into the session), and nested `Table`s each
  bind their own index cleanly.
- The iterator-spec *bounds* are evaluated by the handler (the head is held, so
  `{i, 1+1}` and `{i, n}`-with-`n`-bound resolve correctly), while the body
  stays held until substitution.

### DoS surface

- The per-iteration count is **capped at `MAX_RANGE_LENGTH`** (the same bound
  `Range` uses), computed in `i128` *before* any allocation or looping — an
  oversize or extreme-span iterator (e.g. `Table[0, {i, 2000000}]`) is left
  unevaluated rather than hanging or exhausting memory. `Do` is capped
  identically (the cap bounds wall-clock work, not just memory), and the cap
  composes for nested `Table`. A malformed spec (`{i}` with no bound, a zero
  step, a non-integer/non-symbol binder, or a non-list spec) stays unevaluated —
  never a panic. See MA04 §10.3.

### Notes

- No grammar/lexer change: `Table[…]`/`Do[…]`/`Sum[…]`/`Product[…]` are ordinary
  `Head[args]` applications over list-literal specs the W-1 grammar already
  parses. W-7 touches only `wolfram-runtime` (`builtins.rs` + `backend.rs`).
- `Sum`/`Product` fold onto the canonical `Add`/`Mul` IR heads, so symbolic terms
  combine through the same engine as `1 + 2` (a symbolic body like
  `Sum[x, {i, 1, 3}]` yields `x + x + x`, the engine doing no further `3x`
  normalisation — consistent with W-4 behaviour).

## [0.3.0] — 2026-06-17

The **W-6** deliverable (MA04 §9): operator sugar for the W-5 Tier-1 heads. No
new evaluation logic and no new handler — each sugar form desugars in lowering
to the exact same head the W-5 built-in table already answers, so the sugar and
its head form produce byte-identical IR.

### Added (operator sugar)

- **`f /@ x` ≡ `Map[f, x]`** — lowered by the new `lower_mapapply` over the
  parser's `mapapply` rule.
- **`f @@ x` ≡ `Apply[f, x]`** — same path; `/@` and `@@` share one
  left-associative precedence level (`g @@ f /@ x` ⇒ `Map[Apply[g, f], x]` —
  parenthesise when mixing).
- **`x[[i]]` ≡ `Part[x, i]`** — `lower_postfix` gains an `LDBRACKET` arm that
  emits `Part`; a multi-index `x[[i, j]]` folds into nested parts
  `Part[Part[x, i], j]`, and `[[ ]]` chains/interleaves with `f[…]` application
  (`x[[1]][[2]]`, `f[x][[1]]`, `Range[3][[2]]`).

So `Plus @@ {1, 2, 3}` is `6`, `f /@ {1, 2}` is `{f[1], f[2]}`,
`{a, b, c}[[2]]` is `b`, and `{{1,2},{3,4}}[[1]][[2]]` is `2`, each identical to
its long head form. Negative/out-of-range `Part` and the `Map`/`Apply`
re-evaluation behaviour carry over from W-5 unchanged.

### Notes

- `Map`/`Apply`/`Part` are **not** run through the `Plus`→`Add`-style
  `canonical_head` bridge (they are not arithmetic heads), so they reach the
  `WolframBackend` decorator handler table verbatim.
- No new DoS surface: `/@`/`@@` inherit `Map`/`Apply`'s bounds (the
  already-materialised list); `[[ ]]` only reads one element; deep `[[…]]`
  chains are parsed iteratively (bounded by the W-4 per-statement token cap), not
  by grammar recursion. See MA04 §9.4.

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
