# Changelog

All notable changes to `@coding-adventures/sir-runtime-core` are documented here.

## [0.1.5] - 2026-06-21

### Added (Q10f — call-position `**h` merge helper)

- New `doubleSplatMerge(...maps)` helper. JavaScript has no keyword-argument
  call form, so the TypeScript backend collapses a contiguous run of `**`
  markers at a call site into one trailing argument built by this helper
  (`__Sir.doubleSplatMerge(h1, h2)`) — the conventional JS "options object",
  except the bag is a SIR `Map<Val, Val>`. It returns a **fresh** `Map`
  (defensive copy, matching Ruby's `**`), merges left-to-right so later keys
  win, preserves any `Val` key (symbols, numbers, …), and throws a clear
  backend-coverage-gap error on a non-map operand. Exported from the package
  root. v0 cut-line: mixing inline `key: value` pairs with `**h` at one call
  site is not modelled (see `code/specs/sir-runtime.md`).

## [0.1.4] - 2026-06-19

### Added (Q10a — no-block-given `LocalJumpError`)

- New `LocalJumpError` class. `apply(null, args)` (and `undefined`) now
  throws it ("no block given (yield)") instead of a generic `TypeError`.
  This is the SIR analogue of Ruby's `LocalJumpError`: under the explicit
  block-param ABI a method that `yield`s through a block parameter the
  caller never supplied reaches `apply` with a `null` target, and that
  failure is now distinct and recognisable (a genuine non-closure still
  throws `TypeError`). Exported from the package root. Ruby's exact class
  identity is not modelled — the analogue is keyed to the error's shape.

## [0.1.3] - 2026-06-15

### Changed

- `callBuiltin` now throws a descriptive error for an unregistered SIR builtin
  — naming the builtin, listing the known ones, and explaining it indicates a
  backend coverage gap — rather than a bare `unknown builtin: <name>`.

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
