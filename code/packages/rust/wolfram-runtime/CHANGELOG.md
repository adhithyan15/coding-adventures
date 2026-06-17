# Changelog

All notable changes to `wolfram-runtime` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

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
