# @coding-adventures/sir-runtime-exceptions

Exception runtime for **Semantic-IR-emitted TypeScript/JavaScript**.

The SIR (Semantic IR) backends translate most Ruby-surface constructs to
*native* TypeScript: a sequence becomes an `Array`, a loop becomes a `for`,
a `begin/rescue/ensure` becomes a native `try { … } catch (e) { … } finally
{ … }`. Two pieces of exception handling have **no faithful native
equivalent**, and this package supplies them:

1. **A SIR exception object** — `SirError`, a real `Error` (so stack traces
   work) tagged with the Ruby/SIR class name in `sirClass`. JavaScript's
   `throw`/`Error` carries no class tag of its own.
2. **Rescue-clause type matching** — a native `catch` binds one variable and
   catches *everything*. Ruby's `rescue TypeError, ArgumentError => e` matches a
   *set* of classes (and their subclasses) and falls through otherwise.
   `rescueMatches` answers "does this caught value match this clause?" so the
   emitted `catch` body can dispatch to the right clause or re-`throw`.

It is **keyed to SIR, not Ruby**: a future JavaScript→SIR→TypeScript path reuses
it unchanged. See [`code/specs/sir-runtime.md`](../../../specs/sir-runtime.md).

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-typescript ─▶ .ts
                                                                                  │ imports
                                                                                  ▼
                                                              @coding-adventures/sir-runtime-exceptions
```

The TypeScript backend emits an `import * as __SirExc from
"@coding-adventures/sir-runtime-exceptions"` **only** when a module uses the
`Exceptions` feature (a `try/catch` or a `raise`); pure modules never gain the
dependency.

## API

| Export | Purpose |
|---|---|
| `class SirError extends Error` | The thrown object; `sirClass` holds the Ruby class name, `message` the raise message (defaults to the class name). |
| `raiseError(className?, message?): never` | Throw a `SirError`. Bare `raiseError()` re-raises as a generic `RuntimeError`. |
| `classOfThrown(err): string` | The SIR class name of a caught value (native errors and non-errors → `StandardError`). |
| `rescueMatches(err, classNames): boolean` | Does a caught value match a `rescue` clause naming `classNames`? Empty list = bare `rescue` (catch-all). |
| `registerAncestry(mapping): void` | Merge user `{childClassName: superclassName}` edges (from `class Child < Parent`) into the ancestry the matcher walks, so `rescue StandardError` catches a raised user `MyErr extends StandardError`. Additive over the built-in table; explicit string map (no reflection). |

### Emitted shape

```ts
import * as __SirExc from "@coding-adventures/sir-runtime-exceptions";

try {
  __SirExc.raiseError("ArgumentError", "bad");
} catch (__exc) {
  if (__SirExc.rescueMatches(__exc, ["StandardError"])) {
    const e = __exc;
    // … rescue StandardError => e body …
  } else {
    throw __exc; // no clause matched → propagate
  }
} finally {
  // … ensure body …
}
```

## Usage

```ts
import { SirError, raiseError, rescueMatches } from "@coding-adventures/sir-runtime-exceptions";

try {
  raiseError("KeyError", "missing");
} catch (e) {
  rescueMatches(e, ["IndexError"]); // true — KeyError < IndexError
  (e as SirError).sirClass;         // "KeyError"
}
```

## Built-in exception hierarchy

SIR has no exception-class symbol table, so this package bakes in a curated
slice of Ruby's built-in tree so `rescue StandardError` catches the everyday
subclasses:

```
Exception
└─ StandardError
   ├─ RuntimeError  ├─ ArgumentError      ├─ TypeError
   ├─ NameError ─ NoMethodError           ├─ RangeError
   ├─ IndexError ─ KeyError               ├─ ZeroDivisionError
   ├─ IOError     ├─ StopIteration        └─ NotImplementedError
```

`Exception` (and a bare `rescue`) matches anything; **user-defined** exception
classes match by exact name only.

## v0 limitation (honest)

Because SIR threads no exception-class definitions, the ancestry of
*user-defined* classes is unknown here — `rescue StandardError` will **not**
catch a user `class MyError < StandardError` (it matches `MyError` by exact
name only). A bare `raise` with no in-flight exception re-raises as a generic
`RuntimeError` rather than the original. Both await a frontend that threads the
exception class model and in-flight exception into SIR.

## Development

```bash
npm ci
npx tsc --noEmit      # strict typecheck
npx vitest run --coverage
```

## License

MIT
