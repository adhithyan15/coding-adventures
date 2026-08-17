# Changelog

All notable changes to `@coding-adventures/sir-runtime-core` are documented here.

## [0.6.0] - 2026-08-17

### Added — SIR21 T3b-2 Slice 2: `truncDiv`/`trueDiv`

Adds the two genuinely-new division functions from the SIR21 T3b-2
four-way `/` split (`code/specs/SIR21-type-system-and-integer-semantics.md`
§E3). `div` (already present) is `div_floor`'s dispatch target — see the
note below on why that stays unchanged.

- `truncDiv(a, b)` — signed/unsigned truncating division (rounds toward
  zero, matching C's integer `/`). This is exactly what `div` already
  computes; exported under its own correctly-scoped name (`div_trunc`/
  `udiv_trunc` in the SIR builtin table) purely so this package exposes
  the same four division-op names every sibling runtime does. Fully
  correct — truncation toward zero needs no int/float distinction, so it
  is well-defined in this package's untagged `Val` numeric model.
- `trueDiv(a, b)` — always true-divides, even when both operands are
  meant as Ruby Integers. Models Python's `/`. Also fully correct for the
  same reason: unconditional float coercion needs no tagging either.

### Known limitation — `div_floor` still truncates, not floors

`div`'s existing truncating behavior (documented since its original
implementation, item 2 of the module doc comment) is NOT Ruby-floor-
faithful, and `div_floor` (the new SIR21 T3b-2 name) dispatches to it
unchanged. Every sibling backend's `div_floor` is either a bare rename of
already-floor-faithful logic, or (`semantic-ir-to-javascript`, the
closest sibling) uses a boxed-float runtime tag (`SirFloat`/`isFloat`) to
dispatch floor-vs-true-divide correctly. This package's `Val` has no such
tag — every number, whether it came from a Ruby `Integer` or `Float`
literal, is an indistinguishable plain JS `number` by the time it reaches
`div` — so `div_floor` cannot be made correct without first adding
value-level float tagging throughout this package (touching `add`/`sub`/
`mul`/comparisons/display, not division alone). That is out of scope for
the additive SIR21 T3b-2 rollout; see `arithmetic.ts`'s `div` doc comment
for the full writeup. Logged as follow-up work, not silently left
undocumented.

## [0.5.0] - 2026-08-13

### Added — APL display convention + NDArray display support

Found via `apl-to-semantic-ir/tests/e2e_typescript.rs` (a new Rust test
that runs this backend's array codegen through a real `tsc`/`tsx`
toolchain against this package for real, for the first time): `+/1 2 3`
printed `[object Object]`, not `6`. This package has (deliberately) no
dependency on `@coding-adventures/sir-runtime-array` — a non-array-sourced
program should pull in zero array code — so `toDisplay` had no way to
recognise the `{shape, data}` `NDArray` shape that package constructs, and
fell through to the generic `String(v)` fallback.

- `setDisplayConvention` accepts a new `"apl"` value (alongside the
  existing `"ruby"`/`"lisp"`), selected once at program startup by an
  APL-sourced module's emitted `__Sir.setDisplayConvention("apl")` call
  (see `semantic-ir-to-typescript` 0.13.0).
- `toDisplay` now renders a negative number with APL's own high-minus
  glyph (`¯`, not ASCII `-`) under the `"apl"` convention, and duck-types
  (`{shape: number[], data: Float64Array}`, exported as `NDArrayLike`) an
  NDArray value to render it the way a real APL session echoes a bare
  auto-printed result — a 1:1 port of `apl_runtime::value::display`/
  `fmt_num`, already ported once before into `semantic-ir-to-javascript`'s
  self-contained `ArrayRt` (this is the third port of the same logic, now
  into this package's own layout, mirroring `sir-runtime-array`'s own
  `iota.ts` doc comment about the same "ported a third time" pattern).
  Recognition is gated on the `"apl"` convention specifically, so an
  object that merely happens to have `shape`/`data` fields under any other
  convention is never mistaken for an NDArray.
- `NDArrayLike` joined the public `Val` union (the same way SIR16's
  `Val[]`/`Map<Val,Val>` already did) so a TypeScript-emitted
  `__Sir.write(...)` call passing an array/matrix result type-checks under
  `tsc --strict`.

## [0.4.0] - 2026-08-11

### Removed — SIR28 §7: dead `print`/`puts`/`putsOne`

Every SIR frontend now emits `__sys_write__` (`write` below) instead of
bare `print`/`puts` (SIR28 Slices 4-6, all merged), so `print`, `putsOne`,
and `puts` were fully dead. `writeOne` — `write`'s `per_value` terminator
dependency — is a fully independent duplicate (confirmed via grep that
nothing else called `putsOne`), so this is a straight deletion, not a
refactor; the array-flatten + cycle-guard documentation that lived on
`putsOne` moved onto `writeOne`.

Also removed: `"print"`/`"puts"` from the `callBuiltin` dispatch table,
and the `print`/`puts` public exports from `index.ts`.

This is a breaking change for anything importing `print`/`puts` directly
— nothing in this monorepo does, as of SIR28 Slice 6. Requires
`semantic-ir-to-typescript` >= 0.12.0 (which stops importing the removed
names).

Test suite: the dedicated `print`/`puts` unit tests are removed outright
(equivalent coverage — array-flattening, cycle-termination, newline
policy — already exists in the `write` test section, since every
scenario they exercised has a `terminator`-parameterized equivalent
there).

## [0.3.0] - 2026-08-10

### Added — `write` (SIR28 §2.1: `__sys_write__`, the console-output primitive)

`print`/`puts` above are each ONE fixed newline/stream policy. Real
Ruby's `print` never newline-terminates, `puts` newline-terminates
per-value (unpacking arrays), and Python's `print()`/JS's
`console.log()` space-join everything with one trailing newline — three
policies that used to lower to the *same* `BuiltinCall("print"/"puts",
...)` name, so a backend built to match one source language's semantics
silently disagreed with another's. `write(stream, terminator,
unpackArrays, ...values)` is the same underlying operation, generalized:
the newline/stream policy is carried as explicit data instead of being
implied by which frontend emitted the call.

- `stream`: `"stdout"` or `"stderr"`.
- `terminator`: `"none"` (old `print`'s behavior — write every value
  back-to-back, no newline) | `"per_value"` (old `puts`'s behavior — one
  newline per value, honoring `unpackArrays`) | `"once"` (space-join
  every value, one trailing newline — Python `print()`/JS
  `console.log()`).
- `unpackArrays`: when true and a value is an array, `per_value`
  recursively flattens it (one line per leaf) instead of printing the
  array as one value; a self-referential array prints `[...]` rather
  than looping forever.

Deliberately does NOT replicate `puts`'s trailing-newline-suppression
nuance (real Ruby: `puts "x\n"` prints `x\n`, not `x\n\n`) — that's a
pre-existing, orthogonal divergence between backends' own `puts`
implementations that SIR28 does not fix or replicate, to keep
`write` behaviorally consistent and spec-faithful across every backend.
See [SIR28](../../../specs/SIR28-syscall-primitives.md).

`semantic-ir-to-typescript` now routes `__sys_write__` to this function
(`"__sys_write__" => "__Sir.write"` in its emitter), but no frontend
emits `__sys_write__` yet — that's later SIR28 slices, so this is not
yet reachable from a real compiled program.

## Unreleased

### Fixed

- The Windows standalone build now installs `sir-runtime-exceptions` and
  `sir-runtime-pairs` before the package install, matching the generic build's
  complete local prerequisite closure.
- Clean strict type-checks now install the Node declarations used by
  `process.stdout`; builtin dispatch callbacks carry explicit `Val` parameter
  types without changing runtime behavior.

## [0.2.0] - 2026-08-03

### Fixed — `builtins` dispatch table was not null-prototype

Security review (of the `shiftLeft` addition below) caught that this
package's `builtins` table (`runtime.ts`) — indexed by a SIR-name
string via `callBuiltin`/`builtinClosure` — was a plain object literal,
so a lookup for `"constructor"`/`"toString"`/`"hasOwnProperty"`/
`"__defineGetter__"`/etc. resolved an INHERITED `Object.prototype`
member instead of the intended `undefined`, slipping past the
`fn === undefined` guard and getting INVOKED — the same
`[[dynamic-dispatch-rce]]` hazard the sibling JS backend's own
`builtins` table already guards against via `Object.create(null)`. Not
currently reachable from any call site in this monorepo (every caller
passes a compile-time string literal), but both `callBuiltin` and
`builtinClosure` are public API of a published package. Fixed by
building the table the same way the JS backend does:
`Object.assign(Object.create(null), {...})`. New regression test
asserts `callBuiltin` throws (rather than silently invoking an
inherited method) for each of those four names.

### Added — `shiftLeft` (Ruby's `<<` operator)

Part of "TypeScript backend: implement shift-operator runtime dispatch".
`ruby-to-semantic-ir` lowers `<<` to a top-level `BuiltinCall("<<", [lhs,
rhs])`, distinct from the `__method__("<<", recv, arg)` Collections
dispatch. `<<` had no entry in `runtime.ts`'s `builtins` table at all, so
it fell to `callBuiltin`'s floor and threw a `NameError`-shaped error —
every Ruby program using `<<` as an operator failed at runtime.

`shiftLeft(...args)`, polymorphic like the existing `add`, but
dispatched explicitly on the runtime tag:

- `array` — pushes each RHS operand IN PLACE (never flattened, unlike
  `add`'s array arm), returns the mutated receiver.
- `string` — concatenates via the display helper (the same tolerant
  convention `add`'s string arm already uses — never throws for a
  non-string operand).
- `number` — bitwise shift, implemented via multiplication/division by
  a power of two rather than native `<<`/`>>`: JS's native bitwise
  operators coerce both operands to a 32-bit integer and mask the shift
  count to 5 bits, so `1 << 40` would silently give the wrong answer.
  This runtime's numeric model is a plain `number` everywhere (no
  boxed/tagged Integer-vs-Float split), so precision degrades past
  `Number.MAX_SAFE_INTEGER` like `+`/`*` already do, rather than
  saturating like the fixed-width C/Go/Rust backends. A negative amount
  reverses direction (a right shift); `Math.floor` on the division
  correctly replicates arithmetic (sign-extending) right shift for a
  negative receiver too.

Exported from the package root and registered in the `callBuiltin`
dispatch table.

## [0.1.10] - 2026-07-07

### Added — source-language display convention (SIR display-convention spec)

- `setDisplayConvention(name)` selects the value-display convention: `"ruby"`
  renders booleans as `true`/`false`; any other name (the default) keeps the
  Lisp `#t`/`#f`. `toDisplay`'s boolean arm now branches on the active
  convention. Module-level state — an emitted Ruby program calls the setter
  once at startup (each program is its own process). The default is unchanged,
  so all existing behaviour and callers are byte-for-byte identical until the
  setter is invoked. Mirrors the Rust/Go/JS/Python backends' boolean
  display-convention increment. `nil`, symbol, and pair forms are unaffected
  (convention-independent for now; follow-ups per the spec rollout).

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
