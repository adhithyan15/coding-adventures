# Changelog

## 0.1.1

- **CRITICAL — self-referential reassignment DoS, shared fix (no bypass
  here).** A security audit found and directly reproduced (against the
  `derive-repl` binary) an unbounded value-growth denial-of-service in
  `symbolic-vm`'s shared `handlers::assign_handler`: `a: a * a` / `a: a +
  a` (Macsyma's `:` assignment), repeated even a handful of times,
  doubles the bound value's node count and/or nesting depth every step,
  reaching millions of nodes from a few hundred bytes of source. Fixed at
  the shared choke point (`symbolic_vm::handlers::MAX_BOUND_VALUE_NODES`/
  `MAX_BOUND_VALUE_DEPTH`, see that crate's own changelog for the full
  mechanism). Verified `coding-adventures-macsyma-runtime`'s own `:`
  assignment lowers straight to `symbolic_ir::ASSIGN` and evaluates
  through the shared `vm.eval` (via `MacsymaBackend`'s handler table,
  built from `symbolic_vm::handlers::build_handler_table` with no
  override for `Assign`) — no crate-specific bypass, so no code change was
  needed in `macsyma-runtime` itself. `macsyma-runtime`'s own `eval_source`
  has no `catch_unwind` boundary of its own — this crate is where that
  boundary actually lives for Macsyma's real deployed surface
  (`catch_parser_panics`, wrapping `eval_source` inside `eval_json`) — so
  the regression tests reproducing the exact audited scenario end-to-end
  (both the node-count-tripping `a: a * a` shape and the depth-tripping
  `a: a + a` shape — the shared `Add` handler's flatten-then-left-
  associate canonicalization makes that shape independently dangerous,
  see `symbolic-vm`'s changelog) live here, plus a non-false-positive
  check that a handful of self-multiplications under the caps still
  evaluates correctly.

## 0.1.0

- Add JSON-oriented WASM facade for the Rust MACSYMA runtime.
