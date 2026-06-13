# Changelog

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
