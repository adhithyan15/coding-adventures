# coding-adventures-sir-runtime-oop

OOP runtime imported by **Semantic-IR-emitted Python**.

## What it is

SIR backends translate most constructs to **native** Python. Ruby-style object
orientation is the exception, for one structural reason: the Ruby→SIR frontend
**hoists every method to a detached top-level function with no receiver
(`self`)**. Inside an emitted method there is therefore no `self` to hang an
instance variable on, and a class-variable write carries no enclosing-class
context — so native attribute access (`self.x`) is impossible. This package
supplies the missing object model as an explicit, in-process runtime that
emitted code imports and calls.

| Provided | Why it's not native |
|---|---|
| `define_class` / `superclass_of` | A class registry so `is_a?` can walk ancestry; SIR `ClassDef` carries only a name + optional superclass. |
| `is_a` / `class_of` | Ruby class identity (incl. `Integer`/`Float`/`String`/`Array`/`Hash`/`NilClass`/… and the `Numeric`/`Object` umbrellas). |
| `new_instance` / `SirInstance` | Tagged objects with an instance-variable bag. |
| `push_self` / `pop_self` / `ivar_get` / `ivar_set` | Instance variables addressed through a *current-self* stack, since methods have no receiver. |
| `cvar_get` / `cvar_set` | Class-variable store. |
| `call_method` / `define_method` | Reflective dispatch for `is_a?`/`kind_of?`/`instance_of?`/`class` (emitted as SIR `__method__` calls) + a singleton-method table. `call_method` also dispatches user instance methods (O1). |
| `def_method` / `def_class_method` | Register a user instance/class method into the `(class, method)` tables (emitted as `__def_method__`/`__def_class_method__`). Explicit table lookup, never reflection. |
| `call_new` / `call_super` / `call_class_method` / `current_self` | Object construction (`Foo.new` → allocate + run `initialize`), `super` (ancestry re-dispatch on the same receiver), class-method dispatch (`def self.m`), and the current `self`. |
| `include_module` / `extend_module` | Ruby mixins (emitted as `__include__`/`__extend__`). `include` appends a module to the owner's included-modules list; `extend` copies a module's instance methods in as the owner's class methods. Instance-method resolution walks the Ruby MRO (class → included modules reverse → superclass → …), diamond-deduplicated and cycle-guarded — explicit tables, never reflection. |
| `case_eq` | Ruby case-equality (`pattern === value`) — the test a `when` clause runs. Regexp→match, Range→membership, else `==`. |

## Usage

```python
import coding_adventures_sir_runtime_oop as _sir_oop

_sir_oop.define_class("Dog", "Animal")          # class Dog < Animal
d = _sir_oop.new_instance("Dog")
_sir_oop.push_self(d)
_sir_oop.ivar_set("@name", "Rex")               # @name = "Rex"
_sir_oop.ivar_get("@name")                        # "Rex"
_sir_oop.pop_self()
_sir_oop.call_method(d, "is_a?", "Animal")        # True
_sir_oop.cvar_set("@@count", 0)                   # @@count = 0
```

## Built-in method dispatch

`recv.meth(args…)` reaches the backend as `BuiltinCall("__method__", …)` and is
dispatched by `call_method`, in order: reflective built-ins
(`is_a?`/`kind_of?`/`instance_of?`/`class`) → user `define_method` table →
built-in catalog → floor. `respond_to?` reports exactly which names resolve.

**Floor (T1 — typed runtime errors).** A **genuinely unknown** method
(`obj.undefined`, `nil.foo`, `"s".scan` — `respond_to? == False`) now raises a
typed `NoMethodError` (`"undefined method 'x' for <class>"`), so a Ruby `rescue
NoMethodError` catches it. A *known* block-taking method invoked **without** a
block (e.g. `[1,2,3].map`, `5.times`) still returns `nil` — Ruby returns an
Enumerator there, the documented v0 floor. See
`code/specs/sir-typed-runtime-errors.md`.

**Typed `.fetch` faults.** `Array#fetch(i)` out of bounds (no default) raises
`IndexError`; `Hash#fetch(k)` on a missing key (no default) raises `KeyError` —
matching Ruby. The plain index operators `arr[i]` / `hash[k]` are emitted as
native Python subscripts and still return `nil` (Ruby does not raise for `[]`).

The catalog currently covers (item **M1a** of `code/specs/sir-method-dispatch.md`)
the **non-block `Array`** surface — `length`/`size`/`count`, `first`/`last`,
`include?`, `index`, `push`/`<<`/`pop`/`shift`/`unshift`, `reverse`, `sort`,
`min`/`max`/`sum`, `uniq`/`flatten`/`compact`, `empty?`, `to_a`,
`take`/`drop`/`values_at`, `rotate`/`zip` — and the
**universal `Object`** methods `nil?`, `==`, `!=`, `equal?`, `respond_to?`,
`freeze`/`frozen?`, `dup`/`clone`, `itself`, `to_a`, plus the **Kernel
flow-control** group `send`/`__send__`/`public_send`, `tap`, `then`/`yield_self`
(item **M6**); and (item **M1b**)
**block-taking `Array`/`Enumerable`** methods `each`, `each_with_index`,
`map`/`collect`, `select`/`filter`, `reject`, `reduce`/`inject`, `find`/`detect`,
`flat_map`, `any?`/`all?`/`none?` — a trailing `Closure` block is applied via
`sir-runtime-core`'s `apply` (proc-lenient), predicates routed through SIR
`truthy`; (item **M1c**) the **`Hash`** catalog
(`keys`/`values`/`has_key?`/`fetch`/`merge`/`each`/`map`/`select`/…); and (item
**M1c**) the **`String`** catalog (`length`, `upcase`/`downcase`/`capitalize`,
`reverse`, `strip`/`lstrip`/`rstrip`, `chomp`, `chars`/`bytes`, `split`,
`include?`/`start_with?`/`end_with?`/`index`, `replace`, `sub`/`gsub` *literal*,
`to_i`/`to_f`/`to_sym`, `empty?`, `*`/`+`, `each_char`); and (item **M1c**) the
**`Integer`/`Float`** catalog (`abs`, `to_i`/`to_f`, `even?`/`odd?`/`zero?`/
`positive?`/`negative?`, `succ`/`pred`, `floor`/`ceil`/`round`, `gcd`, `pow`/`**`,
`digits`, and block `times`/`upto`/`downto`/`step`), the **`Symbol`** catalog
(`to_s`/`to_sym`/`length`/`upcase`/`downcase`/`inspect`), and universal
**`to_s`/`inspect`** Ruby display forms (so `nil`/`true`/`false` need no catalog)
plus **`Array#join`**.

### `&:sym` symbol-to-proc (item **M2**)

`sym_to_proc(sym)` builds the `Closure` Ruby's `Symbol#to_proc` returns, so a
`&:sym` block argument works — `[1, 2, 3].map(&:to_s)` → `["1", "2", "3"]`. The
backend emits a `&:sym` block-pass on a dispatched call as
`_sir_oop_sym_to_proc(intern("sym"))`; applying the closure dispatches the named
method on its first argument (the rest forwarded), through `call_method`, so an
unknown method raises `NoMethodError` (T1) — as `[42].map(&:undefined)` does in
Ruby. Operator symbols (`&:+`) are native
arithmetic, not in the dispatch catalog — a documented v0 boundary.

### Kernel flow-control + boolean operators (item **M6**)

The universal-catalog completion the spec's v0 surface called for:

- **`send`/`__send__`/`public_send`** — dynamic dispatch. `x.send(:upcase)` is
  exactly `x.upcase`; the first argument names the method (a `Symbol` or string),
  the rest forward unchanged, and a trailing block survives — so
  `[1, 2].send(:each) { |x| … }` works. An empty arg list bottoms out at `nil`.
- **`tap`** — yields the receiver to the block for a side effect, returns the
  **receiver** (`[1,2,3].tap { |a| log(a) }` → `[1,2,3]`).
- **`then`/`yield_self`** — yields the receiver, returns the **block's result**
  (`5.then { |x| x * 2 }` → `10`). Block-less `tap`/`then` return the receiver
  (the v0 Enumerator-less floor).
- **Boolean `&` / `|` / `^`** on `true`/`false` — Ruby's *eager*
  (non-short-circuit) logical operators, distinct from the lazy `&&`/`||`
  keywords; the operand is coerced by Ruby truthiness, so `true & nil == false`
  and `false | 0 == true`.

## Honest v0 limitation

Because the frontend does not thread receivers into method bodies, the *current
self* is a process-global stack (not a true per-call binding) and class
variables share a single namespace keyed by bare name. This faithfully models
single-instance / single-class programs, but full multi-object Ruby semantics
await a frontend that carries receivers into methods (out of scope for the
backend). (As of T1 the dispatch surface *does* raise typed `NoMethodError` /
`IndexError` / `KeyError` for genuine faults — see the dispatch section above —
rather than the earlier blanket nil floor.) See `code/specs/sir-runtime.md`.

## Where it fits

```
Ruby → semantic-ir → semantic-ir-to-python → emitted .py
                                              └─ from coding_adventures_sir_runtime_oop import …  (aliased _sir_oop_*)
```

The package implements **SIR** semantics, not Ruby's, so a future
JavaScript → SIR → Python path reuses it unchanged.
