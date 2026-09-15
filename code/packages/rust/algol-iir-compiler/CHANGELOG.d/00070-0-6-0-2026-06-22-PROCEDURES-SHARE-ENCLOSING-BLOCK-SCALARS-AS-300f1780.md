## 0.6.0 — 2026-06-22 — procedures share enclosing-block scalars as globals (LANG-FULL E6 layer 1)

A procedure body may now read and write a scalar declared in an **enclosing
block** — the canonical typed module global.  Previously a procedure could touch
only its own value parameters; an enclosing-scope reference was out of reach
(`compile_procedure` installs a fresh, isolated scope).

### Added
- **E6 capture analysis.**  At each block, before any scalar is declared, a
  pre-pass (`collect_block_captures`) scans every procedure body for the names it
  references (minus that procedure's own parameters / result name).  A block
  scalar whose name lands in this set is materialised as a module **global**
  (`VarBinding::is_global`) instead of a register — its slot doubles as the
  global's name.
- **Typed global ops at the access sites.**  A read of a captured scalar lowers
  to `global_load "name"` (via the new `read_scalar` helper); a write lowers to
  `global_store "name", v` — in **both** the procedure and the enclosing block,
  so they share one cell.  These are the same IIR ops every backend now runs
  (VM/JIT/LLVM/JVM/CLR + BEAM/WASM/native).
- Declaration order inside a block is now: non-procedure declarations →
  procedure bodies → statements, so a captured global is declared before any
  procedure that injects it.  `compile_procedure` re-injects the visible global
  bindings into the procedure's fresh scope so its body resolves them.

### Verified
- A procedure sharing an enclosing `counter` with its block (`integer procedure
  add(x); … add := counter := counter + x; counter := 40; result := add(2)`)
  lowers `counter` to `global_load`/`global_store` in both functions and **runs
  on the VM ⇒ 42**.  72 tests; the existing suite is unchanged (plain scalars
  stay registers).

