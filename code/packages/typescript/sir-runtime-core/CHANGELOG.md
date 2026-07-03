# Changelog

All notable changes to `@coding-adventures/sir-runtime-core` are documented here.

## [0.1.9] - 2026-07-01

### Added — typed `ZeroDivisionError` on division by zero (T2)

Part of the `sir-typed-runtime-errors` cascade (spec
`code/specs/sir-typed-runtime-errors.md`). Ruby's `/` raises `ZeroDivisionError`
("divided by 0") for both integer and float division by zero, but native
JavaScript `/` silently yields `Infinity`/`NaN`, so a Ruby `rescue
ZeroDivisionError` never caught it.

- **`div`** (`arithmetic.ts`) now performs an explicit `=== 0` divisor check
  *before* each division step and raises a typed `SirError` of class
  `ZeroDivisionError` (message `"divided by 0"`) via the exceptions runtime's
  `raiseError` entry point. The guard sits inside the variadic fold, so a zero
  divisor anywhere (`div(10, 2, 0)`) raises; a zero *dividend* (`div(0, 5)`) is
  unaffected. Truncating-integer semantics are otherwise unchanged.
- The typed raise is explicit-string (`raiseError("ZeroDivisionError", …)`) — no
  reflection, no source-derived dispatch.

### Dependencies

- Adds a `file:` dependency on `@coding-adventures/sir-runtime-exceptions` (for
  the shared `raiseError`/`SirError` typed-raise entry point). BUILD deps updated
  to list the full transitive `file:` set.

## [0.1.8] - 2026-07-01

### Added — polymorphic `+`/`*` on strings and arrays (PO2)

Ruby overloads `+`/`*` by the runtime type of the first operand, and every case
lowers to the same SIR `+`/`*` builtins (emitted as `__Sir.add`/`__Sir.mul`), so
`add`/`mul` in `arithmetic.ts` now dispatch on the concrete JS runtime tag
(`typeof x === "string"`, `Array.isArray(x)` — never reflection/`eval`):

- **`add`**: first operand a **string** → concat all operands as strings (each
  non-string rendered via `toDisplay`); first operand an **array** → return a
  **fresh** array concatenating each operand's elements (never relies on JS
  `[] + []`, which wrongly yields `""`; inputs are never aliased or mutated);
  otherwise the numeric fold is unchanged.
- **`mul`** (binary, per Ruby): **string × int** → repeat; **array × int** →
  fresh repeated-element array; **array × string** → join elements with the
  separator via `toDisplay`; otherwise the variadic numeric fold is unchanged.

### Security — guard both `*` repeat arms against oversize allocation (CWE-1284/CWE-770)

- A `*` repeat with a huge count (`[0] * 1e18`, `"x" * 1e18`) would try to
  allocate an astronomically large result and hang/OOM the process. `repeatCount`
  now rejects any repeat whose resulting length would exceed
  `Number.MAX_SAFE_INTEGER` by throwing a Ruby-shaped `Error("argument too big")`
  (Ruby's `ArgumentError`). Non-positive / non-integer / non-finite counts
  collapse to an empty result (matching Ruby: `"ab" * 0 == ""`). An **empty
  receiver** short-circuits before any loop or `String.prototype.repeat` call, so
  a huge count over `""`/`[]` does no work and cannot throw a spurious JS
  `RangeError`.
- New vitest cases cover every arm plus the overflow guard and empty-receiver
  short-circuit; bumps `package.json` to `0.1.8`.

## [0.1.7] - 2026-07-01

### Security — cycle-guard the `puts` array flatten (CWE-674)

- `putsOne` flattened arrays by recursing per element with **no bound**. An
  array is a shared, mutable reference, so a translated program can build a
  self-referential array (`a = []; a << a; puts a`) or a pathologically deep
  one; the unguarded recursion threw `RangeError: Maximum call stack size
  exceeded` — a denial of service (uncontrolled recursion). `putsOne` / `puts`
  now thread a `Set` of the array references on the active flatten path: an
  array re-encountered within its own subtree is a cycle and is written as
  Ruby's `[...]` placeholder + newline instead of recursing, so `puts a` on a
  self-referential array now **terminates** exactly as real Ruby does.
  Non-cyclic output is unchanged (`puts([1, [2, 3]])` → `1\n2\n3\n`); new
  regression tests cover the self-referential and mutually-recursive cases.
- Bumps `package.json` to `0.1.7`.

## [0.1.6] - 2026-07-01

### Added (`puts` — Ruby's most common output method)

- New `puts(...args)` implementing Ruby `puts` semantics, and a `"puts"` entry
  in the builtin dispatch table (so backends that route builtins by name reach
  it). Exported from the package root.
- Semantics, matching Ruby exactly:
  - `puts()` (no args) → a single newline.
  - `puts(x)` → `x` + newline, **unless** the rendered text already ends in
    `"\n"` (then no second newline: `puts("x\n")` writes `x\n`, not `x\n\n`).
  - `puts(a, b)` → each argument on its own line, in order.
  - `puts([])` → a single newline (an argument that flattens to nothing still
    writes a blank line).
  - `puts([1, [2, 3]])` → each **element** on its own line, arrays flattened
    recursively (`1\n2\n3\n`).
  - `puts(null)` → a blank line (not the display form `"nil"`).
- Writes via `process.stdout.write` (not `console.log`) so the
  trailing-newline suppression rule can be honoured. Reuses `toDisplay` for
  element rendering.

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
