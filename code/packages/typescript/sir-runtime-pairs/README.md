# @coding-adventures/sir-runtime-pairs

Cons-pair runtime for **Semantic-IR-emitted TypeScript/JavaScript**.

The SIR (Semantic IR) backends translate most Ruby-surface constructs to
*native* TypeScript: a sequence becomes an `Array`, a map becomes a `Map`. The
Lisp **cons cell** has no native JavaScript equivalent, so the SIR `Pair` value
type and its `cons` / `car` / `cdr` operators live here.

A *pair* is an immutable two-field record holding a `car` (first) and `cdr`
(rest). Linked pairs build lists. A proper list `(1 2 3)` is
`cons(1, cons(2, cons(3, null)))`; an improper (dotted) pair `(1 . 2)` is
`cons(1, 2)`.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-typescript ─▶ .ts
                                                                                  │ imports
                                                                                  ▼
                                                              @coding-adventures/sir-runtime-pairs
```

The TypeScript backend emits an import of this package only when a module uses
pairs; pure modules never gain the dependency.

## The extraction + injection design (no cycle with core)

The general SIR value display lives in
[`@coding-adventures/sir-runtime-core`](../sir-runtime-core) as `toDisplay`. A
pair wants to render its elements with that richer display (so a boolean inside
a list prints as `#t`/`#f` rather than `true`/`false`). But core *also* needs to
display pairs — a pair nested inside some other value — so the two importing
each other would form a load-time cycle.

We break the cycle by **inverting the dependency**: this package depends on
**nothing** and exposes a module-level display *hook*, defaulting to `String`.
When core is present it calls `setDisplay(toDisplay)` once at import time, and
from then on pairs render as proper Lisp lists. Used standalone, a pair still
prints sensibly — just with `String` for each element. **Pairs never import
core.**

```text
pairs ◀───── setDisplay(toDisplay) ───── core   (core knows pairs;
  │                                              pairs never imports
  └─ depends on nothing ──────────────────────── core)
```

## API

| Export | Purpose |
|---|---|
| `class Pair` | Immutable cons cell with readonly `car` / `cdr`; `toString` is the Lisp list display via the injected hook. |
| `cons(a, b): Pair` | Construct the pair `(a . b)`. |
| `car(p): Val` | First field; throws `TypeError("car on non-pair")` on a non-pair. |
| `cdr(p): Val` | Rest field; throws `TypeError("cdr on non-pair")` on a non-pair. |
| `isPair(v): boolean` | True iff `v` is a `Pair`. |
| `setDisplay(fn): void` | Inject the element renderer (core does this with `toDisplay`). |
| `Val` | The universal SIR value type alias (`any`) at this boundary. |

## Usage

```ts
import { cons, car, cdr, isPair } from "@coding-adventures/sir-runtime-pairs";

const p = cons(1, cons(2, cons(3, null))); // the proper list (1 2 3)
car(p);          // 1
String(cdr(p));  // "(2 3)"
isPair(p);       // true
String(p);       // "(1 2 3)"
String(cons(1, 2)); // "(1 . 2)"  (dotted pair)
```

Injecting a richer display (what core does):

```ts
import { cons, setDisplay } from "@coding-adventures/sir-runtime-pairs";

setDisplay((v) => (v === null ? "nil" : String(v)));
String(cons(1, null)); // "(1 . nil)"
```

## Development

```bash
npm ci
npx tsc --noEmit      # strict typecheck
npx vitest run --coverage
```

## License

MIT
