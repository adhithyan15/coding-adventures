# sir-classes-oop — user-defined class & object semantics (method dispatch, new, self, super)

## Status

New. Design/spec PR (specs-first). The **largest and highest-leverage** cascade
toward the north star (any Ruby script → semantically-correct Python/JS output):
real Ruby is object-oriented, and today user-defined classes only *parse* — they
don't *execute*.

## Current state (2026-07-01 OOP survey — what works vs breaks)

**Works (syntactic + variables):** `class Foo < Bar; end` → `Stmt::ClassDef {
name, superclass: Option<String>, body: Vec<Stmt> }`; `@ivar`/`@@cvar`/`CONST`
lower to `Scope::Instance`/`ClassVar`/`Const` and read/write via runtime
`ivar_get/set`/`cvar_get/set`; method `def`s inside a class **hoist to top-level
`Function`s**. The `sir-runtime-oop` (Python + TS) already provides the PRIMITIVES:
`define_class`, `new_instance`, `ivar_get/set`, `push_self/pop_self` (a
single-threaded current-self stack), `call_method` (built-in + reflective
dispatch, plus a manual `define_method` table), `is_a`, `sym_to_proc`, `case_eq`.

**Breaks (no semantic model) — a real OO program (`Dog.new("Rex").speak`,
inheritance + `super`, `attr_accessor`) does NOT execute** because:
1. Hoisted methods are **disconnected from their class** — nothing records that
   `speak` belongs to `Dog`, and they carry no receiver.
2. `Foo.new(args)` is an ordinary call with **no wiring to `initialize`** and no
   instance allocation at the call.
3. `super` lowers to `BuiltinCall("super", args)` with **no method/class context**
   and no runtime dispatcher.
4. `self` lowers to a plain local; there is no receiver value.
5. `attr_accessor :x` is **dropped** (no getter/setter synthesized).
6. `def self.x` class methods hoist with **no class-method marker**.
7. Backends: Python/TS accept `Classes`/`InstanceVars`/`ClassVars`; **JS/TS accept
   `Classes` only for empty exception-subclass bodies** (JS/TS reject
   `InstanceVars`/`ClassVars`); Go/Rust accept `Classes` only for exception
   subclasses. So even a correct IR wouldn't run on JS/Go/Rust yet.

## Design — runtime-method-table approach, NO core-IR change

The whole cascade is expressible with the **existing** `BuiltinCall` node + the
existing runtime primitives + a handful of new runtime helpers. **No new `Expr`
variant, no `Scope::Self_`, no `Function.class_name` field** — so no cross-PR
enum/field hazard. Method↔class association lives in a **runtime method table**,
populated by emitted registrations, not in the IR.

Receiver/`self` uses the runtime's existing **single-threaded self-stack**:
`call_method`/`call_new`/`call_super` push the receiver before invoking a user
method and pop after, so `@ivar` access inside a method reads the current self
with no explicit `self` parameter. (Documented model: correct for the
single-threaded transpiled scripts we target; true per-object-per-thread binding
is out of scope for v0, consistent with the existing runtime note.)

### Frontend production (`ruby-to-semantic-ir`)
- **Method registration:** for each instance method `def m` in `class C`, after
  the `ClassDef`, emit `BuiltinCall("__def_method__", [StrLit("C"), StrLit("m"),
  MakeClosure(m_fn)])`; for `def self.m`, `BuiltinCall("__def_class_method__",
  [StrLit("C"), StrLit("m"), MakeClosure(m_fn)])`. The method `def`s still hoist
  to top-level `Function`s (referenced by `MakeClosure`).
- **`Foo.new(args)`** → `BuiltinCall("__new__", [StrLit("Foo"), ...args])`.
- **Instance method call** `recv.m(args)` on a user object already lowers to
  `__method__` dispatch — `call_method` will consult the user method table
  (below). No frontend change needed for the call site beyond what exists.
- **`super`/`zsuper`** → `BuiltinCall("__super__", [StrLit(method_name),
  StrLit(class_name), ...args])` — the lowerer already tracks the enclosing method
  and class; thread them in.
- **`self`** → `BuiltinCall("__self__", [])`.
- **`attr_accessor :x` / `attr_reader` / `attr_writer`** → synthesize getter
  `def x; @x; end` and/or setter `def x=(v); @x = v; end` method `def`s + their
  registrations (pure frontend macro expansion).
- Declares `Feature::Classes` (+ `InstanceVars`/`ClassVars`/`Constants` as used),
  already observed today.

### Runtime (`sir-runtime-oop`, per backend)
Add to the existing primitive set:
- A **method table** `methods[(class, name)] = fn` and `class_methods[(class,
  name)] = fn`, populated by `__def_method__`/`__def_class_method__`.
- **`call_new(class, args)`**: `new_instance(class)` → `push_self(obj)` → if an
  `initialize` is registered for `class` (walking ancestry), call it with `args`
  → `pop_self` → return `obj`.
- Extend **`call_method(recv, name, args)`**: when `recv` is a user
  `SirInstance`, look up `methods[(class_of(recv)…walk ancestry…, name)]`,
  `push_self(recv)`, invoke, `pop_self`; fall back to the existing built-in
  catalog otherwise. Unknown → the runtime's existing floor.
- **`call_super(method, class, args)`**: walk from `superclass_of(class)`, find
  the first ancestor with `methods[(anc, method)]`, dispatch with the *current*
  self still bound.
- **`self()`**: return the current self-stack top.
- All dispatch is **explicit table lookup — never reflection** on the
  source-derived class/method name (per the C3 RCE lesson).

### Backends
- **Emit arms** for the new builtins (`__new__`→`call_new`, `__super__`→
  `call_super`, `__def_method__`/`__def_class_method__`→ table registration,
  `__self__`→`self()`), mirroring the existing `__method__`→`call_method` routing.
- **Feature acceptance:** JS/TS/Go/Rust extend `ACCEPTED_FEATURES` to include
  `InstanceVars`/`ClassVars` (Python already does), gated so a real OO module is
  ACCEPTED and routed through the runtime — with the existing soundness gates
  keeping genuinely-unsupported constructs cleanly rejected.

## Milestones (one PR per crate) — phased like collection-methods/exceptions

**Ordering keeps main green: runtime + backend support the new builtins FIRST
(additive/inert — nothing emits them yet), THEN the frontend emits them.**

| # | Crate | Content | Phase |
|---|-------|---------|-------|
| O0 | `code/specs/` | this spec | design |
| O1 | `sir-runtime-oop` (Python + TS) + `semantic-ir-to-python` + `-typescript` | method table, `call_new`/`call_super`/`self`, `call_method` user-object path; backend emit arms for the new builtins (additive) | 1 |
| O2 | `ruby-to-semantic-ir` | frontend production: `__new__`, `__def_method__`/`__def_class_method__`, `__super__`, `__self__`, `attr_accessor` expansion, super context threading | 1 — makes P1/P2/P3 EXECUTE (Ruby→Python & Ruby→TS) |
| O3 | `semantic-ir-to-javascript` (+ JS runtime) | JS OOP runtime (method table, new/super/self) + accept `InstanceVars`/`ClassVars` + emit arms | 2 |
| O4 | new Go `sir-runtime-oop` OOP + `semantic-ir-to-go` | Go instance model + method table + new/super/self | 3 (big) |
| O5 | new Rust `sir-runtime-oop` OOP + `semantic-ir-to-rust` | Rust instance model + method table + new/super/self | 3 (big) |

Phase 1 (O1+O2) delivers **real executable OOP for Ruby→Python and Ruby→TS** —
the biggest single jump toward the north star. Phase 2 (JS) and Phase 3 (Go/Rust)
follow. Each PR: tests via linker override, clippy clean, execution-proof through
the native toolchain vs reference, security-review gate (explicit tables, never
reflection), `gh pr create`, babysit.

## Verification

The three survey programs are the golden suite, run end-to-end and diffed vs the
reference backend:
- **P1** `Dog.new("Rex").speak` → `Rex says woof`
- **P2** `Cat.new("Tom").describe` (inheritance + `super`) → `Tom with 4 legs`
- **P3** `Counter` with `attr_accessor`, `def self.zero`, `inc` returning `self`
  (chaining) → verify getter/setter, class method, and `self`-return chaining.
Plus unit tests for each runtime helper (`call_new` runs `initialize`;
`call_super` walks ancestry; `attr_accessor` expansion; method-table dispatch).

## Out of scope (documented, v0)

- True per-object/per-thread `self` (v0 uses the single-threaded self-stack).
- Metaprogramming (`method_missing`, `define_method` at runtime from user code,
  `send`, `respond_to?` beyond the reflective built-ins), `include`/`extend`
  mixins + full MRO — later cascades.
- Singleton methods on arbitrary objects; `class << obj` beyond `class << self`.
- Reopening classes / monkey-patching built-ins.
