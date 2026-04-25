# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase A skeleton.

- `MacsymaBackend` — `SymbolicBackend` subclass with MACSYMA-specific
  heads (`Display`, `Suppress`, `Kill`, `Ev`) and option flags.
- `History` — input/output table, resolves `%`, `%i1`, `%o1`, ...
  via a backend lookup hook.
- `Display` / `Suppress` heads (`;` vs `$` statement terminators).
  Identity handlers; the REPL inspects the wrapper before eval to
  decide whether to print.
- `Kill(symbol)` and `Kill(all)` handlers.
- `Ev(expr, ...flags)` — minimal first cut: only the `numer` flag
  is honored.
- `MACSYMA_NAME_TABLE` — extends `macsyma-compiler`'s standard-name
  map so identifiers like `expand`, `factor`, `subst`, `solve`,
  `taylor`, `limit` route to canonical heads (the substrate handlers
  may not yet exist; the user gets `Expand(...)` unevaluated until
  they do).
- Type-checked, ruff- and mypy-clean.
