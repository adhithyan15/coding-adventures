# Changelog

All notable changes to `@coding-adventures/sir-runtime-core` are documented here.

## [0.1.2] - 2026-06-15

### Changed

- The cons-pair value type (`Pair` / `cons` / `car` / `cdr` / `isPair`) has been
  **extracted** into the dedicated `@coding-adventures/sir-runtime-pairs`
  package. `./pairs` is now a thin re-export shim, so every existing import and
  the builtin dispatch table keep working unchanged, and a pair built via core
  is the *same* class as one built via the dedicated package.
- Core now **depends on** `@coding-adventures/sir-runtime-pairs` (a local
  `file:` dependency) and, when first evaluated, injects its richer `toDisplay`
  into the (dependency-free) pairs package's display hook (`setDisplay`) so a
  `Pair` still renders as a Lisp list (`(1 2 3)`, `(1 . 2)`). This keeps the
  package dependency graph acyclic.

## [0.1.1] - 2026-06-13

### Changed

- Widened the `Val` union to include the SIR16 collection types — `Val[]`
  (sequences) and `Map<Val, Val>` (maps) — so emitted native arrays/maps type as
  `Val`. Additive; existing values are unaffected. Both are truthy under SIR
  truthiness and display via `String(v)` in `toDisplay`.

## [0.1.0] - 2026-06-11

### Added

Initial release — the core runtime imported by Semantic-IR-emitted
TypeScript/JavaScript. Provides the SIR semantics that have no faithful native
equivalent:

- **SIR truthiness** (`truthy`) — only `false` and `nil` are falsy (`0`, `""`,
  `[]`, `{}` are truthy), the Lisp/Ruby convention.
- **Symbols** (`Sym`, `intern`) — interned identity objects.
- **Pairs** (`Pair`, `cons`, `car`, `cdr`, `isPair`) — cons cells with Lisp list
  display.
- **Equality / predicates** (`eq`, `isNull`, `isNumber`, `isSymbol`) and
  **display** (`toDisplay`, `print`).
- **Arithmetic** (`add`, `sub`, `mul`, variadic; `div` truncating-integer;
  `lt`, `gt`).
- **Closures** (`Closure`, `apply`, `makeClosure`), an in-memory **global store**
  (`globalSet`, `globalGet`, `globalGetStatic`), and **builtin dispatch**
  (`callBuiltin`, `builtinClosure`).

Migrated (behaviour-preserving) from the inline `__Sir` namespace that
`semantic-ir-to-typescript` used to paste into every artifact. See
`code/specs/sir-runtime.md`.
