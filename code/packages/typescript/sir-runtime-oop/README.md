# @coding-adventures/sir-runtime-oop

OOP runtime imported by **Semantic-IR-emitted TypeScript / JavaScript**.

## What it is

SIR backends translate most constructs to **native** code. Ruby-style object
orientation is the exception, for one structural reason: the Ruby→SIR frontend
**hoists every method to a detached top-level function with no receiver
(`self`)**. Inside an emitted method there is therefore no `this`/`self` to hang
an instance variable on, and a class-variable write carries no enclosing-class
context — so native member access (`this.x`) is impossible. This package
supplies the missing object model as an explicit, in-process runtime that
emitted code imports and calls.

| Provided | Why it's not native |
|---|---|
| `defineClass` / `superclassOf` | A class registry so `is_a?` can walk ancestry; SIR `ClassDef` carries only a name + optional superclass. |
| `isA` / `classOf` | Ruby class identity (incl. `Integer`/`Float`/`String`/`Array`/`Hash`/`NilClass`/… and the `Numeric`/`Object` umbrellas). |
| `newInstance` / `SirInstance` | Tagged objects with an instance-variable bag. |
| `pushSelf` / `popSelf` / `ivarGet` / `ivarSet` | Instance variables addressed through a *current-self* stack, since methods have no receiver. |
| `cvarGet` / `cvarSet` | Class-variable store. |
| `callMethod` / `defineMethod` | Reflective dispatch for `is_a?`/`kind_of?`/`instance_of?`/`class` (emitted as SIR `__method__` calls) + a singleton-method table. |

## Usage

```ts
import * as __SirOop from "@coding-adventures/sir-runtime-oop";

__SirOop.defineClass("Dog", "Animal");        // class Dog < Animal
const d = __SirOop.newInstance("Dog");
__SirOop.pushSelf(d);
__SirOop.ivarSet("@name", "Rex");             // @name = "Rex"
__SirOop.ivarGet("@name");                     // "Rex"
__SirOop.popSelf();
__SirOop.callMethod(d, "is_a?", "Animal");     // true
__SirOop.cvarSet("@@count", 0);                // @@count = 0
```

## Built-in method dispatch

`recv.meth(args…)` reaches the backend as `BuiltinCall("__method__", …)` and is
dispatched by `callMethod`, in order: reflective built-ins
(`is_a?`/`kind_of?`/`instance_of?`/`class`) → user `defineMethod` table →
built-in catalog → `null` floor. `respond_to?` reports exactly which names
resolve, so an out-of-catalog method is both `null` *and* `respond_to? == false`.

The catalog currently covers (item **M1a** of `code/specs/sir-method-dispatch.md`)
the **non-block `Array`** surface — `length`/`size`/`count`, `first`/`last`,
`include?`, `index`, `push`/`<<`/`pop`/`shift`/`unshift`, `reverse`, `sort`,
`min`/`max`/`sum`, `uniq`/`flatten`/`compact`, `empty?`, `to_a` — and the
**universal `Object`** methods `nil?`, `==`, `!=`, `equal?`, `respond_to?`,
`freeze`/`frozen?`, `dup`/`clone`, `itself`, `to_a` (`include?`/`index`/`==` use
deep value equality); and (item **M1b**) **block-taking `Array`/`Enumerable`**
methods `each`, `each_with_index`, `map`/`collect`, `select`/`filter`, `reject`,
`reduce`/`inject`, `find`/`detect`, `flat_map`, `any?`/`all?`/`none?` — a trailing
`Closure` block is applied via `@coding-adventures/sir-runtime-core`'s `apply`
(proc-lenient), predicates routed through SIR `truthy`; and (item **M1c**) the
**`Hash`** catalog (`keys`/`values`/`has_key?`/`fetch`/`merge`/`each`/`map`/
`select`/…). The String/Numeric/Symbol catalogs land in follow-up releases.

## Honest v0 limitation

Because the frontend does not thread receivers into method bodies, the *current
self* is a process-global stack (not a true per-call binding) and class
variables share a single namespace keyed by bare name. This faithfully models
single-instance / single-class programs and never crashes on the OO surface, but
full multi-object Ruby semantics await a frontend that carries receivers into
methods (out of scope for the backend). See `code/specs/sir-runtime.md`.

## Where it fits

```
Ruby → semantic-ir → semantic-ir-to-typescript → emitted .ts
                                                    └─ import * as __SirOop from "@coding-adventures/sir-runtime-oop"
```

The package implements **SIR** semantics, not Ruby's, so a future
JavaScript → SIR → TypeScript path reuses it unchanged.
