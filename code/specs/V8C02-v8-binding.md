# V8C02 — `v8-binding`: LangBinding impl + JS value representation

## What this is

`v8-binding` is the JavaScript implementation of
[`lang-runtime-core::LangBinding`](../packages/rust/lang-runtime-core).
It does for JS what `lispy-runtime` does for Lisp/Scheme/Twig:
defines the value representation, hooks up runtime semantics
(truthiness, equality, property access, method dispatch),
participates in inline caches and GC, and registers built-in
functions.

Per [V8C01](V8C01-overview.md) §"Composition", we're not
re-inventing the runtime substrate. `lang-runtime-core` already
provides the `LangBinding` trait + inline-cache machinery + GC
visitor protocol; `v8-binding` just plugs JS-specific semantics
into the corresponding slots.

```text
JS source (foo.js)
   │
   ▼  javascript-tokens / javascript-parser  (existing)
javascript-ast::Program
   │
   ▼  v8-ir-compiler  (V8C03, future)
interpreter-ir (IIR)
   │
   ▼  vm-core (existing) ← consults v8-binding for JS semantics
LANG VM execution
   │
   ▼  v8-host  (V8C06, future)
console.log output, files written, exit code
```

`v8-binding` lives on the seam between the LANG VM's
language-agnostic interpreter and JavaScript's specific
behavior. Everywhere the interpreter says *"how does this
language want me to test for truthiness?"*, it asks `v8-binding`.

## 1. Value representation

JS has 7 primitive types + Object. Our v1 `Value` carries
8 variants:

| JS type | v1 Value variant | Notes |
|---|---|---|
| `Number` | `Number(f64)` | All JS numbers are IEEE 754 doubles. No int/float split. |
| `String` | `String(Rc<str>)` | Heap-allocated, shared. JS strings are immutable. |
| `Boolean` | `Bool(bool)` | `true` / `false` |
| `Null` | `Null` | The `null` literal |
| `Undefined` | `Undefined` | `undefined` — *not* a literal; see §1.1 |
| `Object` | `Object(Rc<RefCell<JsObject>>)` | Plain object, `{}`-style |
| `Array` | `Array(Rc<RefCell<JsArray>>)` | `[]`-style — separate from `Object` for v1 (see §1.2) |
| `Function` | `Function(Rc<JsFunction>)` | Callable: compiled IIR routine + captured env |

```rust
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(Rc<str>),
    Bool(bool),
    Null,
    Undefined,
    Object(Rc<RefCell<JsObject>>),
    Array(Rc<RefCell<JsArray>>),
    Function(Rc<JsFunction>),
}
```

### 1.1 Why both `null` and `undefined`?

This is one of JS's famous mistakes-that-can't-be-removed. They
mean different things in practice:

- `undefined` — a binding has been declared but not assigned. A
  missing object property. The return value of a function with
  no explicit return.
- `null` — the programmer explicitly chose "no value here."

`undefined == null` is `true` (loose equality says they're both
absent); `undefined === null` is `false` (strict equality keeps
them distinct).

Our `Value::Undefined` is not a literal node (there's no
`UndefinedLiteral` in `javascript-ast` Phase 1). It's produced
by:

- Missing property access: `obj.foo` when `foo` isn't there
- Functions without `return`
- `let x;` (declared but uninitialized)
- The `void` operator (Phase 1.x — `void expr` always evaluates
  to `undefined`)

### 1.2 Why separate `Array` from `Object`?

In real V8, arrays *are* objects (an `Array` is an `Object` with
a `length` property and integer-keyed slots). Our v1 keeps them
split because:

- The implementation is much simpler. `JsArray` is just
  `Vec<Value>` + a `length`. `JsObject` is `HashMap<String,
  Value>`.
- The runtime cost of looking up `arr.length` is one field
  access, not a HashMap lookup.
- The two have different IR opcodes anyway (`load_index` vs
  `load_property`), so we don't lose anything by giving them
  distinct types.

Phase 1.x **may** unify them when we implement `Array.prototype`
and need integer-keyed properties to coexist with string-keyed
ones. That's a refactor we'll cross when we get there.

### 1.3 `JsObject` shape

```rust
pub struct JsObject {
    /// Insertion-ordered property map (ES2015+ guarantees order).
    pub properties: IndexMap<Rc<str>, Value>,
    /// Optional prototype. v1 leaves `None` everywhere; Phase 1.x
    /// hooks this up to `Object.prototype` etc.
    pub prototype: Option<Rc<RefCell<JsObject>>>,
    /// `Object.preventExtensions(obj)` flips this to true. v1
    /// implements the bit but doesn't ship the built-in yet.
    pub extensible: bool,
}
```

Property keys are `Rc<str>` (interned strings shared with the
heap; same allocation as `Value::String` for property names).

### 1.4 `JsArray` shape

```rust
pub struct JsArray {
    /// Dense backing storage. v1 doesn't model sparse arrays.
    pub elements: Vec<Value>,
}
```

`length` is `elements.len() as f64` — computed, not stored.
Setting `arr.length = N` (a JS idiom for truncation) truncates
or pads with `Undefined`. v1 implements that idiom in
`store_property` for the special key `"length"`.

### 1.5 `JsFunction` shape

```rust
pub struct JsFunction {
    /// Compiled IR routine produced by v8-ir-compiler.
    pub body: IIRFunction,
    /// Variables captured from the enclosing lexical scope.
    /// Empty for top-level functions; populated for closures.
    pub captured_env: Vec<(Rc<str>, Value)>,
    /// `function name(...) {}` — the static name. Phase 1.x
    /// might add `Function.prototype.name` lookup.
    pub name: Option<Rc<str>>,
    /// Arity. JS doesn't strict-check arity at call sites
    /// (extra args go to `arguments`, missing args become
    /// `Undefined`), but we record it for the IR.
    pub formal_param_count: u32,
}
```

## 2. Boxing strategy

Two options for representing a JS `Value` in 8 bytes (what the
LANG VM expects via `LangBinding::Value: Copy`):

### Option A — Tagged Rust enum (chosen for v1)

```rust
#[derive(Debug, Clone, Copy)]
pub struct JsValue(u64);
```

…where the low 3 bits encode the tag and the high 61 bits
encode the payload (a pointer for heap things, a sentinel for
immediates).

**Pros**: Familiar pattern (matches `lispy-runtime`'s
tagged-i64 approach). Single machine word everywhere. No `Rc`
cloning on copies.

**Cons**: Pointer-tagging requires `unsafe`. Reference counts
have to live elsewhere (we'd need a side `Rc` pool or roll
mark-sweep GC up-front).

### Option B — `Copy` wrapper around `Rc` (v1 actual)

Actually neither — Rust's `Rc` isn't `Copy`. Looking at the
existing `Value` enum I sketched in §1, the only way to make it
`Copy` is if every variant fits in 8 bytes, and the heap-cell
variants don't (they're behind `Rc`).

**Resolution**: keep `Value` as the *user-facing* enum in
`v8-binding` (not `Copy`), but the **`LangBinding::Value`
associated type is a separate tagged `u64`** that the binding
materializes into a `Value` on demand via `materialize_value()`
and serializes back via `box_value()`. Same pattern Lispy uses.

Tag layout (low 3 bits):

```
000   immediate Number (low 56 bits = float-truncated payload — for v1, just zero; full encoding lands when number-intensive code becomes a perf priority)
001   Bool — bit 3 = false/true
010   Null
011   Undefined
100   heap pointer to JsObject
101   heap pointer to JsArray
110   heap pointer to JsString (Rc<str>) — for inline-cache keys; user-facing strings are interned
111   heap pointer to JsFunction
```

Numbers are the awkward case: doubles don't fit in 61 bits. v1
*spills numbers to the heap* (allocates a `JsNumberCell` per
number, points at it with tag `100`) for simplicity. Phase 1.x
gets NaN-tagging like real V8 (every payload that isn't a NaN
*is* the double; the NaN bit-pattern space encodes
non-numbers).

**This is a v1 perf cost we explicitly accept.** A real engine
must not allocate per number. We pay the cost to keep the v1
GC story simple (everything heap-backed → one trace function).

### Why not tagged-i64 in v1?

Same reason `constant-fold` v1 doesn't have a NaN-tagging
backend: ship the obvious encoding first, optimize when perf
matters. The `LangBinding::box_value` / `materialize_value`
seam is the chokepoint; we can swap encodings without changing
anything outside `v8-binding`.

## 3. The 18 `LangBinding` methods walked through

### Associated types

- **`type Value = JsValueU64`** — the tagged `u64`. Copy, 8
  bytes, matches the LANG VM ABI.
- **`type ClassRef = JsClassId`** — per-object class identity.
  In v1 every object's class is `0` (we don't have hidden
  classes yet). Phase 1.x: each unique property-shape gets a
  fresh `JsClassId` for IC keying.
- **`type ICEntry = JsICEntry`** — a single cache slot. Holds:
  expected class id, expected property offset (or
  `prototype-walk-count`), expected value type for shape checks
  on stores. Phase 1.x feature; v1 caches are all
  `ICState::Uninitialized` (slow path every time).

### Const

- **`LANGUAGE_NAME = "javascript"`** — used in profile artefact
  files, debug dumps, IIR module's `language` field.

### Type & identity

- **`type_tag(v) -> u32`** — returns 0–7 corresponding to the
  tag-bits encoding above. Used by IR opcodes like `cmp_eq`
  fast path.
- **`class_of(v) -> Option<ClassRef>`** — for tag 100/101/111
  (heap-backed), returns the object's class id. Tag 110
  (string), 010/011/001/000 (immediate) → `None`.
- **`is_truthy(v) -> bool`** — JS truthiness rules: `false`,
  `null`, `undefined`, `0`, `NaN`, `""` are falsy; everything
  else is truthy. *Used by every `if` / `while` /
  `LogicalExpression` short-circuit at run time.* Must match
  the `literal_truthy()` helper in `closure-pass-constant-fold`
  exactly (we share semantics, but the constant-fold helper is
  compile-time and `v8-binding`'s is run-time).
- **`equal(a, b) -> bool`** — `===` strict equality. Type and
  value must match. Per IIR convention this is the `cmp_eq`
  opcode's backing — JS gets a separate `cmp_loose_eq` IR
  helper for `==`.
- **`identical(a, b) -> bool`** — `Object.is(a, b)` semantics.
  Distinct from `equal` in two cases: `Object.is(NaN, NaN)` is
  `true` (vs `NaN === NaN` is `false`); `Object.is(+0, -0)` is
  `false` (vs `+0 === -0` is `true`). Yes, JS has both.
- **`hash(v) -> u64`** — for IC keying and hash-map builtins.
  Implementations:
  - `Number(n)` — hash the IEEE 754 bits (normalize NaN to a
    canonical bit-pattern first).
  - `String(s)` — fnv-1a of the bytes.
  - `Bool(b)` — `0` or `1`.
  - `Null` / `Undefined` — distinct constant hashes.
  - `Object`/`Array`/`Function` — pointer address.

### Heap interaction

- **`unsafe trace_object(header, visitor)`** — called by the GC.
  For `JsObject`, visit every value in `properties`. For
  `JsArray`, visit every element. For `JsFunction`, visit
  `captured_env` values + the function itself isn't traced
  (it's the root).
- **`trace_value(v, visitor)`** — for immediates, no-op. For
  heap-backed values, deref and call `trace_object`.
- **`unsafe finalize(header)`** — JS has `FinalizationRegistry`,
  but v1 doesn't ship it. Default no-op.

### Dispatch (the seam)

- **`apply_callable(callable, args, cx)`** — when an IR
  `call_indirect` runs. The `callable` is a `Value::Function`;
  we look up its `JsFunction`, build a new IIR frame with the
  args bound to formal params (extras dropped, missing →
  `Undefined`, plus `arguments` object in Phase 1.x), and run
  the body. Returns the result.
- **`send_message(receiver, selector, args, ic, cx)`** —
  method invocation. `obj.foo(x)` desugars to `send(obj, "foo",
  [x])`. v1: look up `foo` on `obj` via `load_property`, then
  call it via `apply_callable`. Phase 1.x: use the inline cache
  for the property lookup.
- **`load_property(obj, key, ic) -> Value`** — `obj.foo`
  semantics. For `JsObject`, look up the key in `properties`;
  if missing, walk the `prototype` chain; if still missing,
  return `Undefined` (*not* an error — this is the JS gotcha
  that breaks half the world's typos).
- **`store_property(obj, key, val, ic) -> ()`** — `obj.foo =
  val`. For `JsObject`, insert/update the `properties` map.
  Special-case `JsArray.length` per §1.4. Future Phase 1.x:
  honor `extensible: false` by returning a non-strict-silently-
  drops vs strict-throws error.

### Builtins

- **`resolve_builtin(name) -> Option<BuiltinFn>`** — the link
  between user-callable names and the `v8-stdlib` registry.
  Looks up `"console.log"` → the host-impl function in
  `v8-host`. v1 dispatches: every IR call with callee resolving
  to `BuiltinId(n)` (assigned at link time) dispatches through
  `resolve_builtin` lookup.

### IC invalidation

- **`invalidate_ics(invalidator)`** — JS prototype mutation
  (`obj.__proto__ = …`, `Object.setPrototypeOf(…)`) invalidates
  ICs that cached property offsets via the prototype chain. v1
  no-ops because we don't have ICs to invalidate; Phase 1.x
  registers per-object-class IC lists and invalidates them on
  prototype reassignment.

### Deopt support

- **`materialize_value(repr, location_value) -> Value`** —
  reverse of `box_value`. The JIT (Phase 2+) may unbox a number
  into a raw machine register; on deopt, we need to box it
  back. v1: pure interpreter, no deopt; implementation is
  trivial (just `materialize` the tagged-u64 directly).
- **`box_value(v) -> (BoxedReprToken, u64)`** — pair the value
  to a "this is how I'd lay you out in a native register"
  descriptor + the raw bits. Same v1 simplicity caveat.
- **`materialize_frame(...)`** — reconstruct a deopt'd frame.
  v1 no-op.

## 4. GC story

v1 uses `Rc<RefCell<JsObject>>`. *Not a GC.* It's reference
counting with manual interior mutability. Drawbacks:

- Cycles leak (objects pointing at each other never drop).
- Every property write through `RefCell` has runtime borrow
  checks.

Why we ship it anyway:

- Lispy did the same: `lispy-runtime` v0.5 uses `Rc` for cons
  cells and bumped to a generational GC later (LANG47+) when
  alloc pressure justified it.
- Phase 1 JS programs don't allocate enough to OOM. Fizzbuzz
  doesn't make cycles.
- `LangBinding::trace_object` + `trace_value` are *already*
  written (the methods are required); when the real GC lands
  (cppgc-style mark-sweep — see V8C04+), it consumes those
  trace fns and stops calling `Rc::clone`. The replacement is
  contained to `v8-binding`.

The 2-line summary in code:

```rust
// v1: leak cycles, pay borrow checks, no per-Value alloc tracking.
pub type ObjectCell = Rc<RefCell<JsObject>>;

// Phase 1.x+: replace with `GcRef<JsObject>` from gc-core, kill the RefCell.
```

## 5. Inline caches — design now, implement later

LangBinding's `ICEntry` associated type forces us to declare
the IC shape now even though v1 caches are all
`Uninitialized`. The shape:

```rust
#[derive(Clone, Copy)]
pub struct JsICEntry {
    /// Expected class of the receiver. Mismatch = miss.
    pub class: JsClassId,
    /// Where the property lives.
    pub slot: JsICSlot,
    /// State: uninit, monomorphic, polymorphic (up to 4),
    /// megamorphic. Matches V8's actual mono→poly→mega
    /// progression.
    pub state: ICState,
}

#[derive(Clone, Copy)]
pub enum JsICSlot {
    /// Direct property at this byte offset in JsObject.
    Direct(u32),
    /// On the prototype chain, N hops up.
    Prototype { hops: u8, offset: u32 },
    /// On `Object.prototype` itself (terminal cache).
    ObjectPrototype { offset: u32 },
    /// Slow path — every access goes through hash lookup.
    Slow,
}
```

v1 emits `ICState::Uninitialized` for every IC. Phase 1.x
fills in the lookup fast path; the IR opcodes (`load_property`
/ `store_property`) already pass `&mut InlineCache<ICEntry>`,
so the JIT can read the cached slot in one instruction.

## 6. Test plan

Tests live alongside the implementation in
`code/packages/rust/v8-binding/src/lib.rs#[cfg(test)] mod tests`.
Categories:

1. **Round-trips** — every `Value` variant → `box_value` →
   `materialize_value` → equal to original.
2. **Truthiness** — every documented falsy value (`false`,
   `null`, `undefined`, `0`, `-0`, `NaN`, `""`) returns `false`;
   sentinel non-falsy values return `true`.
3. **Equality**:
   - `equal(Number(1.0), Number(1.0))` → true (===)
   - `equal(Number(1.0), String("1"))` → false (no coercion in
     strict eq)
   - A separate `loose_equal` helper (not on LangBinding;
     called from IR opcode wrappers) tested for coercion cases.
4. **`identical` vs `equal`**:
   - `identical(NaN, NaN)` → true
   - `equal(NaN, NaN)` → false (per spec — yes really)
   - `identical(+0.0, -0.0)` → false
   - `equal(+0.0, -0.0)` → true
5. **Property access**:
   - `load_property(obj, "missing")` → `Undefined` (not an
     error)
   - `store_property(obj, "foo", v)` then `load_property(obj,
     "foo")` → `v`
   - Prototype chain (Phase 1.x; v1 just verifies the chain is
     `None` everywhere)
6. **Hash collisions** — distinct values must (with high
   probability) hash to distinct `u64`s. We verify
   `Number(NaN)` and `Null` and `Undefined` have *distinct*
   canonical hashes.
7. **GC tracing** — synthesize an object graph, run the visitor,
   verify every reachable value was visited exactly once. Real
   GC tests land alongside the real GC; v1 verifies the visitor
   protocol via a mock.

## 7. What this PR locks down

1. The `Value` enum (8 variants) and the supporting
   `JsObject` / `JsArray` / `JsFunction` shapes.
2. The decision to keep `null` and `undefined` as distinct
   variants from day one (no "we'll add undefined later"
   shortcut — too much breakage if we do).
3. The decision to keep `Array` separate from `Object` in v1,
   with a documented Phase 1.x unification path.
4. The boxing strategy: tagged `u64` `JsValueU64` as the
   `LangBinding::Value` associated type; numbers spill to the
   heap in v1 (`Phase 1.x: NaN-tag like real V8`).
5. The 18 `LangBinding` method semantics for JS, including the
   tricky bits (`equal` vs `identical` on `NaN` / `±0`;
   `load_property` returns `Undefined` not an error).
6. The IC shape (`JsICEntry`, `JsICSlot`, `ICState`) — declared
   now so the trait associated type is committed, even though
   v1 caches stay `Uninitialized`.
7. The GC plan: `Rc<RefCell>` v1, mark-sweep Phase 1.x+, with
   `trace_object`/`trace_value` already plumbed.
8. The test plan as above.

## 8. What comes next

After this spec merges:

- **Scaffold PR**: `code/packages/rust/v8-binding/` crate with
  the `Value` enum, `JsObject` / `JsArray` / `JsFunction`
  structs, but an *empty* `LangBinding` impl (`unimplemented!()`
  in every method). Sets the public API surface, locks the
  Cargo.toml dep graph.
- **Real-body PRs**: implement methods in priority order —
  `is_truthy` → `equal`/`identical` → `load_property` /
  `store_property` → `apply_callable` → `send_message` → GC
  tracing → IC stubs.

Per the V8C series schedule from V8C01, the next spec is V8C03
— `v8-ir-compiler` (AST → IIR lowering).
