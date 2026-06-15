# Changelog

## 0.1.0 — initial release

First release of the SIR cons-pair runtime for TypeScript/JavaScript, per
`code/specs/sir-runtime.md`. Extracts the `Pair` value type (and its
`cons`/`car`/`cdr` operators) into a dedicated per-concern runtime so the
display can be injected rather than imported, avoiding a load-time cycle with
`@coding-adventures/sir-runtime-core`.

### Added

- `class Pair` — an immutable cons cell with readonly `car` / `cdr` fields;
  `toString` renders the Lisp list display (proper list `(1 2 3)`, dotted pair
  `(1 . 2)`) via the injectable display hook.
- `cons(a, b): Pair`, `car(p)`, `cdr(p)` — construct/access; `car`/`cdr` throw a
  `TypeError` on a non-pair.
- `isPair(v): boolean` — pair predicate.
- `setDisplay(fn): void` — inject the element renderer; defaults to `String`.
  Core injects its richer `toDisplay` here so pairs render with full SIR display
  while this package keeps **zero dependencies** and never imports core.
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `package.json`, `tsconfig.json`, `vitest.config.ts`,
  `BUILD`, `BUILD_windows`, `required_capabilities.json` (no capabilities),
  README. vitest suite at 100% coverage; `tsc --strict` clean.

### Design note

The display is a module-level **hook** rather than an import of core's
`toDisplay`. This inverts the pairs↔core dependency so neither imports the other
at module-load time; core injects its display via `setDisplay` at import.
