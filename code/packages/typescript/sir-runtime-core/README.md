# @coding-adventures/sir-runtime-core

Core runtime imported by **Semantic-IR-emitted TypeScript / JavaScript**.

## What it is

Semantic-IR (SIR) backends translate most constructs to **native** code — a
sequence is an `Array`, a map is a `Map`, a loop is a `for`, a class is a `class`.
A handful of SIR semantics have **no faithful native equivalent**, and those live
here so emitted code can `import` and call them instead of inlining a runtime
namespace into every file:

| Provided | Why it's not native |
|---|---|
| `truthy(v)` | SIR truthiness is **false/nil-only** — `0`, `""`, `[]`, `{}` are *truthy*, unlike JS coercion. |
| `Sym` / `intern` | Interned identity objects; JS has no symbol *value* type with names. |
| `Pair` / `cons` / `car` / `cdr` | Lisp cons cells; no native type. |
| `eq`, `toDisplay`, `print` | Symbol-aware equality and Lisp/Ruby display (`nil`, `#t`/`#f`). |
| `puts` | Ruby `puts`: per-arg line, arrays flattened element-per-line, no double trailing newline, no-arg → one newline. |
| `write` | [SIR28](../../../specs/SIR28-syscall-primitives.md) `__sys_write__`: the general console-output primitive `print`/`puts` generalize into — `write(stream, terminator, unpackArrays, ...values)`, stream `"stdout"`\|`"stderr"`, terminator `"none"`\|`"per_value"`\|`"once"`. |
| `add`/`sub`/`mul`/`div`, `lt`/`gt` | Variadic numeric folds + truncating-integer `/`. `add`/`mul` are **type-polymorphic** like Ruby (dispatched on the first operand's runtime tag): `"a"+"b"`→`"ab"`, `[1]+[2]`→`[1,2]` (fresh array), `"ab"*3`→`"ababab"`, `[0]*3`→`[0,0,0]`, `[1,2]*", "`→`"1, 2"`. `*` repeat guards against oversize allocation (`Error("argument too big")`). |
| `Closure`/`apply`/`makeClosure`, global store, builtin dispatch | Uniform closure handles + SIR `Globals`. |

It implements **SIR** semantics, not any one source language's — so a Ruby
frontend today and a JavaScript or Python frontend tomorrow all reuse it.

## How emitted code uses it

```ts
import * as _sir from "@coding-adventures/sir-runtime-core";

function add(a, b) {
  return a + b;
}

const xs = [1, 2, 3];
let i = 0;
while (_sir.truthy(i < xs.length)) {
  _sir.print(xs[i]);
  i = i + 1;
}
```

## Where it fits

Frontend (Ruby / JS / Python …) → `semantic-ir` → `semantic-ir-to-typescript` →
emitted `.ts` that imports this package. See
[`code/specs/sir-runtime.md`](../../../specs/sir-runtime.md).

## Development

```sh
npm install
npx tsc --noEmit
npx vitest run
```

The repository's `BUILD_windows` front door first materializes the exceptions
and pairs runtimes so this package also works from a clean standalone checkout.
The development dependencies include Node declarations for the runtime's
`process.stdout` writes, so the documented strict type-check is self-contained.
