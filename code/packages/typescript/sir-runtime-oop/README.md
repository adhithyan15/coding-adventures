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
| `callMethod` / `defineMethod` | Reflective dispatch for `is_a?`/`kind_of?`/`instance_of?`/`class` (emitted as SIR `__method__` calls) + a singleton-method table. `callMethod` also dispatches user instance methods (O1). |
| `defMethod` / `defClassMethod` | Register a user instance/class method into the `(class, method)` tables (emitted as `__def_method__`/`__def_class_method__`). Explicit `Map` lookup, never reflection. |
| `callNew` / `callSuper` / `callClassMethod` / `currentSelfVal` | Object construction (`Foo.new` → allocate + run `initialize`), `super` (ancestry re-dispatch on the same receiver), class-method dispatch (`def self.m`), and the current `self`. |
| `caseEq` | Ruby case-equality (`pattern === value`) — the test a `when` clause runs. RegExp→match, Range→membership, else `==`. |

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
built-in catalog → floor. `respond_to?` reports exactly which names resolve.

**Floor behaviour (T2).** A method the receiver genuinely does not have (an
out-of-catalog name, `respond_to? == false`) raises a typed `NoMethodError`
(`undefined method 'x' for <Class>`), matching Ruby — replacing the earlier
silent `nil`. A *known* method invoked in a shape v0 does not model (most
notably a block-taking method called *without* a block — `[1,2,3].map`,
`5.times`, which Ruby answers with an Enumerator) still bottoms out at `nil` and
is **not** mis-raised. Likewise `Array#fetch`/`Hash#fetch` raise
`IndexError`/`KeyError` on an out-of-bounds index / missing key with no default
(the plain `arr[i]`/`hash[k]` index operators still return nil). All raises go
through `@coding-adventures/sir-runtime-exceptions`' explicit-string `raiseError`
— no reflection.

The catalog currently covers (item **M1a** of `code/specs/sir-method-dispatch.md`)
the **non-block `Array`** surface — `length`/`size`/`count`, `first`/`last`,
`include?`, `index`, `push`/`<<`/`pop`/`shift`/`unshift`, `reverse`, `sort`,
`min`/`max`/`sum`, `uniq`/`flatten`/`compact`, `empty?`, `to_a`,
`take`/`drop`/`values_at`, `rotate`/`zip`, `each_slice`/`each_cons`, `tally` — and the
**universal `Object`** methods `nil?`, `==`, `!=`, `equal?`, `respond_to?`,
`freeze`/`frozen?`, `dup`/`clone`, `itself`, `to_a` (`include?`/`index`/`==` use
deep value equality), plus the **Kernel flow-control** group
`send`/`__send__`/`public_send`, `tap`, `then`/`yield_self` (item **M6**); and
(item **M1b**) **block-taking `Array`/`Enumerable`**
methods `each`, `each_with_index`, `map`/`collect`, `select`/`filter`, `reject`,
`reduce`/`inject`, `find`/`detect`, `flat_map`, `any?`/`all?`/`none?`, `chunk_while` — a trailing
`Closure` block is applied via `@coding-adventures/sir-runtime-core`'s `apply`
(proc-lenient), predicates routed through SIR `truthy`; (item **M1c**) the
**`Hash`** catalog (`keys`/`values`/`has_key?`/`fetch`/`merge`/`each`/`map`/
`select`/`transform_values`/`transform_keys`/ Enumerable aggregates
`find`/`any?`/`all?`/`none?`/`count`/`sort_by`/`min_by`/`max_by` and Enumerable
breadth `group_by`/`partition`/`flat_map`/`collect_concat`/`reduce`/`inject`/`sum`
(all yielding the `[k, v]` pair; `reduce`/`inject` use Ruby's `(memo, pair)`
convention), and `to_h` (block + no-block)/`each_with_index`/`each_with_object`
(the last two yield the `[k, v]` pair as a single argument alongside the
index/memo)/…); and (item **M1c**) the
**`String`** catalog (`length`,
`upcase`/`downcase`/`capitalize`, `reverse`, `strip`/`lstrip`/`rstrip`, `chomp`,
`chars`/`bytes`, `split`, `include?`/`start_with?`/`end_with?`/`index`, `replace`,
`sub`/`gsub` *literal*, `to_i`/`to_f`/`to_sym`, `empty?`, `*`/`+`,
`ljust`/`rjust`/`center`, `swapcase`, `tr`/`count`/`delete`/`squeeze` *literal
char sets*, `each_char`);
and (item **M1c**) the **`Integer`/`Float`** catalog (`abs`, `to_i`/`to_f`,
`even?`/`odd?`/`zero?`/`positive?`/`negative?`, `succ`/`pred`,
`floor`/`ceil`/`round` (with optional `ndigits`), `divmod`, `fdiv`, `clamp`,
`between?`, `gcd`, `pow`/`**`, `digits`, and block
`times`/`upto`/`downto`/`step`), the **`Symbol`** catalog
(`to_s`/`to_sym`/`length`/`upcase`/`downcase`/`inspect`), and universal
**`to_s`/`inspect`** Ruby display forms (so `null`/`true`/`false` need no catalog)
plus **`Array#join`**.

### `&:sym` symbol-to-proc (item **M2**)

`symToProc(sym)` builds the `Closure` Ruby's `Symbol#to_proc` returns, so a
`&:sym` block argument works — `[1, 2, 3].map(&:to_s)` → `["1", "2", "3"]`. The
backend emits a `&:sym` block-pass on a dispatched call as
`__SirOop.symToProc(intern("sym"))`; applying the closure dispatches the named
method on its first argument (the rest forwarded), through `callMethod`, so an
unknown method still bottoms out at `null`. Operator symbols (`&:+`) are native
arithmetic, not in the dispatch catalog — a documented v0 boundary.

### Kernel flow-control + boolean operators (item **M6**)

The universal-catalog completion the spec's v0 surface called for:

- **`send`/`__send__`/`public_send`** — dynamic dispatch. `x.send("upcase")` is
  exactly `x.upcase`; the first argument names the method (a `Sym` or string),
  the rest forward unchanged, and a trailing block survives — so
  `[1, 2].send("each", blk)` works. An empty arg list bottoms out at `null`.
- **`tap`** — yields the receiver to the block for a side effect, returns the
  **receiver** (`[1,2,3].tap { |a| log(a) }` → `[1,2,3]`).
- **`then`/`yield_self`** — yields the receiver, returns the **block's result**
  (`5.then { |x| x * 2 }` → `10`). Block-less `tap`/`then` return the receiver
  (the v0 Enumerator-less floor).
- **Boolean `&` / `|` / `^`** on `true`/`false` — Ruby's *eager*
  (non-short-circuit) logical operators, distinct from the lazy `&&`/`||`
  keywords; the operand is coerced by Ruby truthiness, so `true & null == false`
  and `false | 0 == true`.

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
