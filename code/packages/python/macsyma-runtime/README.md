# macsyma-runtime

The thin shell on top of `symbolic-vm` that holds genuinely-MACSYMA
conventions. Everything reusable across CAS frontends lives in
substrate packages (`cas-substitution`, `cas-simplify`,
`cas-factor`, ...); this package holds only what is specifically
MACSYMA / Maxima.

## What's in here

- `MacsymaBackend` — subclass of `SymbolicBackend` that adds the
  MACSYMA-specific heads (`Display`, `Suppress`, `Kill`, `Ev`) and
  the option flags (`numer`, `simp`, ...).
- `History` — the `%i1`/`%o1`/`%i2`/`%o2`/... input/output table the
  REPL writes and the VM reads via a binding-resolution hook.
- Built-in name-table extensions — maps MACSYMA identifiers
  (`expand`, `factor`, `subst`, `solve`, `taylor`, `limit`, ...) to
  their canonical IR heads, so the compiler can route them to the
  substrate handlers regardless of whether those handlers are yet
  implemented.

## Why this is the only deliberately-non-reusable package

A future `mathematica-runtime` would sit at the same layer position —
above `symbolic-vm`, alongside the language-specific
`lexer/parser/compiler` — but it would write its own runtime entirely.
Mathematica's `Hold`/`HoldComplete` semantics, `OwnValues`/`DownValues`
rules, `Print[]`, and `Message[]` are very different from MACSYMA's
`%i`/`%o`/`$`/`;`/`kill`/`ev`. Same for a `matlab-runtime` (with
`ans`, `disp`, `clear`, `format`).

Everything underneath the runtime is shared across all of them.

## Phase A scope

- `Display` / `Suppress` heads — wrappers the compiler emits around
  each top-level statement so the REPL can distinguish `;` (display)
  from `$` (suppress).
- `History` and the `%`/`%i1`/`%o1` binding-resolution shim.
- `Kill(name)` and `Kill(all)` for clearing bindings.
- Basic `Ev(expr, numer)` — re-evaluate with the `numer` flag set.
- Name-table additions for substrate heads (`Subst`, `Simplify`,
  `Expand`, `Factor`, `Solve`, `Taylor`, `Limit`, `Length`, etc.).

Later phases add `Block`, full `Ev`, `Assume`/`Forget`/`Is`.

## Usage

```python
from macsyma_runtime import MacsymaBackend, History
from symbolic_vm import VM

history = History()
backend = MacsymaBackend(history=history)
vm = VM(backend)
```

The REPL program reads `History` to format `(%i1) ` / `(%o1) ` prompts
and to resolve `%`, `%i1`, `%o1` references typed by the user.

## Dependencies

- `coding-adventures-symbolic-ir`
- `coding-adventures-symbolic-vm`
