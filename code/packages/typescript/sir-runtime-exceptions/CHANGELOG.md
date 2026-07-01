# Changelog

## 0.2.0 — user-defined exception-class ancestry (E2)

Adds `registerAncestry(mapping)`: the SIR backend threads user
`class Child < Parent` edges here at program init (an explicit
`{childClassName: superclassName}` string map — no `eval`/reflection), so
`rescueMatches` walks a user class up through its registered superclass and on
into the built-in table. A `rescue StandardError` now catches a raised user
`MyErr extends StandardError`.

- User edges are **additive**: they layer over the frozen built-in ancestry
  (`BUILTIN_ANCESTRY`, spread into a mutable live copy) without replacing it, so
  built-in matching is unchanged. A user class with no registered edge still
  matches only by exact name (or via `Exception` / a bare `rescue`).
- The cycle guard in `isAncestorOrSelf` already tolerates a malformed
  self-referential edge.

## 0.1.0 — initial release

First release of the SIR exception runtime for TypeScript/JavaScript, per
`code/specs/sir-runtime.md`. Supplies the two pieces of Ruby-surface exception
handling that have no faithful native equivalent, so the rest can translate to
a native `try/catch/finally`.

### Added

- `class SirError extends Error` — the thrown object, a real `Error` carrying
  the Ruby/SIR class name in `sirClass`; `message` defaults to the class name
  and a prototype fix-up keeps `instanceof` working under down-level targets.
- `raiseError(className?, message?): never` — throws a `SirError`; bare
  `raiseError()` re-raises as a generic `RuntimeError`.
- `classOfThrown(err)` — the SIR class name of a caught value (native errors and
  thrown non-errors bucket as `StandardError`).
- `rescueMatches(err, classNames)` — rescue-clause class matching against a
  curated built-in Ruby ancestry table; empty list = bare `rescue` (catch-all),
  `Exception` = universal root, user classes match by exact name.
- Full standard layout: `package.json`, `tsconfig.json`, `vitest.config.ts`,
  `BUILD`, `BUILD_windows`, `required_capabilities.json` (no capabilities),
  README. 13 vitest cases; `tsc --strict` clean.

### v0 limitation (documented)

User-defined exception-class ancestry is unknown (no SIR exception-class symbol
table), so `rescue StandardError` matches only the built-in subclasses, and user
classes match by exact name. A bare re-raise becomes a generic `RuntimeError`.
Both await a frontend that threads the exception model into SIR.
