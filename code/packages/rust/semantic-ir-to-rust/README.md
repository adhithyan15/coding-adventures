# semantic-ir-to-rust

Second backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Rust
source code — every produced `.rs` file embeds the runtime helpers
inline as a `mod __sir { ... }` block, so the output compiles with
`rustc <file>.rs` and has no external crate dependencies.

Implements [SIR13](../../../specs/SIR13-semantic-ir-to-rust.md).

## Pipeline

```text
semantic_ir::Module
   │
   ▼  semantic_ir_to_rust::compile
Artifact { filename, source, metadata }
```

## Public API

```rust
use semantic_ir_to_rust::{compile, RustBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;
// or:
let backend = RustBackend::new();
let artifact = backend.compile(&sir_module)?;
```

## Capability declaration

Accepts (v0): `Closures`, `Pairs`, `Symbols`, `Strings`,
`DynamicTyping`, `OptionalTypeAnnotations`, `MutualRecursion`,
`Globals`.

Accepts (SIR16 / v1 — **all six** features, full v1 parity): `Floats`,
`ShortCircuit`, `MutableBindings`, `Loops`, `Sequences`, `Maps`.

- `Floats` adds a `Value::Float(f64)` arm to the runtime value model with
  numeric promotion (`int op float ⇒ float`).
- `ShortCircuit` `&&`/`||` emit a truthy-guarded block so the rhs is
  evaluated only when the lhs decides.
- `MutableBindings` lets a `let`-binding be re-targeted by a later
  assignment: a per-function pre-pass declares every reassigned name
  `let mut`, and the assignment emits a bare `<name> = <value>;`
  (Local/Param/Capture) or a runtime `global_set` (Global).
- `Loops` covers `while`, `for-range`, and `for-each`.  `while` and
  `for-range` route through SIR truthiness / cached `i64` bounds;
  `for-each` iterates via the runtime `seq_iter` helper, which now
  snapshots a real `Value::Seq` **and** still walks the legacy cons-list
  (`Pair`-chain) — so `for x in [1, 2, 3]` works end to end.
- `Sequences` adds a shared, mutable `Value::Seq(Rc<RefCell<Vec<Value>>>)`.
  `SeqLit`/`SeqIndex`/`SeqLen` lower to `seq_lit`/`seq_index`/`seq_len`;
  the `SeqSet` statement mutates the backing vector via `seq_set`.
- `Maps` adds a shared, mutable, insertion-ordered
  `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)`.  `MapLit`/`MapGet`
  lower to `map_lit`/`map_get`; `MapSet` mutates via `map_set`.  Keys
  compare with the runtime's `value_eq` (so any value type is a key), and
  a missing-key `MapGet` returns `Nil`.

With all six SIR16 features accepted, every SIR16 IR node has a real emit
arm — no reachable `panic!` remains for v1.  The remaining emit panics
cover SIR17/18 nodes (modules, string interpolation, instance/class vars,
non-exception constant refs) whose features stay unaccepted; `try/catch`
and exception-subclass `class` declarations are now accepted (see
**Exceptions (E4)** below).

Accepts (P2e): `DefaultParams` — a `Param` may carry a `default`
expression that runs when the caller omits that trailing argument.  Rust
functions are fixed-arity over `__sir::Value` with no native defaults, so
the backend uses a **runtime-mimic** strategy that preserves the
language's **call-time, param-scope** semantics:

- A `Value::Missing` sentinel marks an *omitted* positional argument,
  with `__sir::missing()` (constructor) and `__sir::is_missing(&Value)`
  (predicate) helpers.  `format` renders a stray `Missing` as `<missing>`
  and `value_eq` treats it as equal only to another `Missing`.
- Each defaulted param gets a **body-top prologue**
  `let p = if __sir::is_missing(&p) { <default> } else { p };`.  Emitting
  the default *inside the body* — in declaration order — is what gives the
  call-time + param-scope rule: an earlier parameter is already bound, so
  a default like `b = a + 1` resolves `a`.
- A `DirectCall` that omits trailing defaulted arguments **pads** the
  omitted positions with `__sir::missing()` so the emitted Rust call is
  full-arity.  The callee's parameter count comes from the same
  `FN_ARITY` table the closure emitter uses.

Non-default behaviour is byte-for-byte unchanged.

Accepts (KW5): `KeywordParams` — name-matched keyword parameters
(`def f(a:)` / `def f(a: 1)`) and keyword arguments (`f(a: 1)`).  Rust has
**no native keyword-argument syntax**, so the backend resolves keywords to
a plain positional call **statically, at emit time** (no runtime library):

- **Def side** — a `Keyword` param emits as an *ordinary positional*
  Rust parameter in its declared order (the by-name affordance is dropped;
  the name becomes the Rust parameter name).  An *optional* keyword (one
  with a `default`) reuses the same `DefaultParams` body-top prologue —
  it is a defaulted parameter like any other.
- **Call side** — for a `DirectCall` whose callee signature is known
  (looked up in a `FN_PARAMS` signature table, populated alongside
  `FN_ARITY`), the emitter builds the full positional argument list in the
  callee's *declared* order: positionals fill positional params in order;
  each `KeywordArg { name, value }` fills the param whose name matches
  (a name→position reorder); an omitted optional keyword is padded with
  `__sir::missing()` so the callee's prologue substitutes its default
  (deferring default evaluation to callee scope — correct even when a
  default references an earlier param).  The result is a plain positional
  Rust call `f(a, b_val, c_default)`.

Worked example — `def greet(greeting, name: "world")`:

```text
  greet("hi")               → greet("hi", missing())   // name omitted → default "world"
  greet("hi", name: "ada")  → greet("hi", "ada")        // name supplied by keyword
```

Out of scope (v0): an `IndirectCall`/closure call carrying keywords has no
statically-known signature, so it cannot be resolved; the frontends do not
emit it.

Executes (C6): **collection-method dispatch**.  A source-level
`recv.meth(args…)` / `recv.meth { |x| … }` reaches every backend as the
narrow-waist envelope
`BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`.  This
needs **no new feature gate** — `__method__` observes no dedicated feature,
and a pure collection module's observed features (`Sequences`/`Closures`/
`Strings`) are already accepted.  The backend now *executes* the dispatch
rather than panicking:

- **Emit** — the `"__method__"` arm lowers to
  `__sir::call_method(<recv>, "meth", vec![<args>])`.  The method name is
  lifted out of the `StrLit` to a Rust `&str` **literal**, so dispatch is a
  closed, compile-time-known set.  A trailing block (a `MakeClosure`, or an
  `&:sym` block-pass lowered via the `"block_pass"` arm to
  `__sir::sym_to_proc`) is the last `Value::Closure` in the arg `Vec`.
- **Runtime catalog** — `call_method` in the inline `__sir` module matches
  **explicitly** on `(receiver type, method name)`, ported from the
  Python/TS `sir-runtime-oop` reference for parity:
  - **Array** (`Seq`): `length`/`size`, `first`, `last`, `push`/`append`,
    `pop`, `include?`, `reverse`, `sort`, `join`, `map`, `select`/`filter`,
    `reject`, `find`, `reduce`/`inject`, `each`, `any?`/`all?`/`none?`.
  - **Hash** (`Map`): `keys`, `values`, `size`, `has_key?`, `each`, `map`,
    `select`.
  - **String** (`Str`): `length`, `upcase`, `downcase`, `reverse`, `strip`,
    `include?`, `split`, char-set `tr`/`count`/`delete`/`squeeze` (literal
    sets), and justify `ljust`/`rjust`/`center`/`swapcase` (rune-aware; `center`
    puts any odd extra pad char on the RIGHT, Ruby's rule).
  - **Numeric** (`Int`/`Float`): `abs`, `to_i`, `to_f`, `even?`, `odd?`,
    `zero?`, `times`; plus a universal `to_s`.
  - Block-taking methods apply the trailing closure via `apply_closure`;
    `sym_to_proc` implements `Symbol#to_proc` (`&:sym`).
  - **Strict vs. lenient reads** — `Array#[]` / `Hash#[]` are lenient (out of
    bounds / missing key ⇒ `nil`, Ruby parity), while `Array#fetch` /
    `Hash#fetch` are strict (raise `IndexError` / `KeyError`, or return a
    supplied default).
- **Security** — the catalog *is* the allowlist.  Dispatch is the explicit
  match only; a genuinely unknown method name raises a controlled, typed
  `NoMethodError` (`no_method_error`, carrying only the data name string),
  never a reflective lookup on the raw name.  (A KNOWN method used block-less
  still floors to Ruby `nil` via `unknown_method`.)  No new `unsafe`.

Executes (E4): **exception handling**.  `Feature::Exceptions` (Ruby
`begin/rescue/ensure`, `raise`) is accepted.  Rust has **no native
exceptions**, so v0 maps the SIR exception model onto Rust's **unwinding
panic** machinery — a *localized* transform touching only the
`raise`/`TryCatch` arms:

- **`raise`** → `__sir::raise("Class", <msg>)`, which
  `std::panic::panic_any(SirError { class, msg })`.  A `Const` class name
  (`raise Foo`/`raise Foo, "m"`) is **lifted to a string literal** (never a
  runtime constant read); a non-const first arg → `raise("RuntimeError",
  <arg>)`; bare `raise` → `__sir::reraise()`.
- **`TryCatch`** → a `std::panic::catch_unwind(AssertUnwindSafe(|| { …
  }))` region.  Its `match` runs `ensure` on the `Ok` (no-exception) arm;
  on the `Err` arm it downcasts the payload with `exc_from_payload`
  (**re-`resume_unwind`ing a non-`SirError` panic** — a real Rust bug is
  never swallowed as a rescue), then dispatches the rescue clauses in
  source order via `rescue_matches`, binding `=> e` with `exc_value`.  A
  matched clause runs its body then `ensure`; an unmatched exception runs
  `ensure` then `resume_unwind`s (re-raise).  **`ensure` runs on every
  path.**  `main` wraps the user body in a top-level `catch_unwind` so an
  uncaught exception exits cleanly non-zero (`Class: message`).
- **Ancestry matching** — `rescue_matches(&SirError, &[&str])` walks an
  **explicit** built-in ancestry table (a verbatim parity port of the TS
  `sir-runtime-exceptions` `ANCESTRY`: `ArgumentError`/`TypeError`/… →
  `StandardError` → `Exception`, etc.) merged with **user edges** collected
  from the module's `ClassDef { name, superclass }` pairs and registered
  once at init via `register_ancestry`.  So `class MyErr < StandardError`
  makes a raised `MyErr` catchable by `rescue StandardError`.  A `seen`-set
  **cycle guard** bounds the walk.
- **Classes/Constants — narrow acceptance.**  `Feature::Classes` is
  accepted only for an **empty-body** exception-subclass declaration
  (`class MyErr < StandardError; end`; methods hoist to top-level
  `Function`s); a non-empty body is rejected cleanly by
  `reject_stateful_class`.  `Feature::Constants` is accepted only because
  `raise MyErr` names its class via a `Const` VarRef (lifted to a string);
  any other `Const` reference is rejected cleanly by `reject_const_ref`.
- **Security** — rescue matching is the explicit table lookup only, never
  reflection; the cycle guard is mandatory; a non-`SirError` panic passes
  through untouched.  `AssertUnwindSafe` is used for generated code (the
  `Err` path re-derives what it needs and never reads partially-mutated
  captured state).  No new `unsafe`.

Executes (O5): **user-defined-class OOP**.  `Feature::Classes` (now for
REAL classes, not just exception subclasses) plus `Modules`,
`InstanceVars`, and `ClassVars` are accepted.  The Ruby→SIR frontend (O2)
lowers OOP to a small family of builtins the backend routes to the inline
`__sir` OOP runtime — mirroring the `__method__`→`call_method` routing:

- **Emit arms** — `Foo.new(args)` → `__new__` → `__sir::call_new`;
  `super(args)` → `__super__` → `call_super`; `def m` in `class C` →
  `__def_method__` → `def_method`; `def self.m` → `__def_class_method__`
  → `def_class_method`; `self` → `__self__` → `current_self`.  A class/method
  NAME argument (a `StrLit`, or a `Const` VarRef like `Dog.new`) is **lifted
  to a `&str` literal** (never a runtime constant read).  `@ivar`/`@@cvar`
  reads route to `ivar_get`/`cvar_get`, writes to `ivar_set`/`cvar_set`.
  Method `def`s still **hoist** to top-level `Function`s referenced by the
  `__def_*` builtins' `MakeClosure`, so an accepted class body stays empty.
- **Runtime** — an object is a `Value::Instance(u64)` handle into a
  `thread_local` `INSTANCES` side-table of `SirInstance { class, ivars }`
  (see **Value model** for why a narrow variant + side-table).  Instance and
  class ("static") method tables are `HashMap<(String, String), Value>`
  (the `Value` is the method-body `Closure`).  `call_new` allocates then runs
  the inherited `initialize` (ancestry-resolved) with `self` bound;
  `call_method` gains a **user-instance branch taken FIRST** (every other
  receiver keeps the unchanged collection path); `call_super` resolves from
  the superclass and reuses the live `self`.  The dynamic `self` stack pops
  via an **RAII drop guard**, so a panic mid-method still balances it.  The
  user `subclass → superclass` ancestry (shared with the exception matcher)
  is registered at init whenever the module declares `Classes` or
  `Exceptions`.
- **Security** — every method/class lookup is an EXPLICIT `HashMap::get` on
  a `(class, method)` key — never reflection or `dyn Any`-by-name.  A
  class/method literally named `constructor`/`new`/`drop` is inert data; an
  unresolved instance method surfaces a controlled, typed `NoMethodError`
  (never a host callable).  The ancestry walk
  carries a `seen`-set **cycle guard** (a cyclic `A < B < A` hierarchy
  terminates).  Widening `Classes`/`Constants` acceptance stays sound:
  `reject_stateful_class` still rejects a class/module with an executable
  body, and `reject_const_ref` skips only the lifted-to-string OOP name
  slots.  No new `unsafe`.

Executes (PO5): **polymorphic `+` / `*`** on strings and arrays
(sir-polymorphic-operators).  Ruby overloads `+`/`*` by receiver type;
before this change the runtime `plus`/`times` were **numeric-only**, so
`"a" + "b"` called `as_i64("a")` and produced integer garbage.  Both
helpers now **dispatch on the tag of the first operand** via an explicit
`match` (never reflection — see the dynamic-dispatch-RCE lesson), adding
the string/array arms *ahead of* the unchanged numeric fold:

| expression      | result      | arm                       |
|-----------------|-------------|---------------------------|
| `"a" + "b"`     | `"ab"`      | `Str` → concat all args   |
| `[1] + [2]`     | `[1, 2]`    | `Seq` → new concatenated  |
| `"ab" * 3`      | `"ababab"`  | `Str * Int` → repeat      |
| `[0] * 3`       | `[0, 0, 0]` | `Seq * Int` → repeat      |
| `[1, 2] * ", "` | `"1, 2"`    | `Seq * Str` → join        |
| `1 + 2`         | `3`         | numeric fold (unchanged)  |
| `2 * 3`         | `6`         | numeric fold (unchanged)  |

- **`plus`** — a `Str` first operand concatenates every operand's string
  contents into a new `Str`; a `Seq` first operand concatenates the element
  vectors into a **fresh** `Seq` (no aliasing/shared mutation of inputs);
  anything else takes the unchanged int/float promotion fold.
- **`times`** — `Str * Int` repeats the string (count ≤ 0 → `""`); `Seq *
  Int` repeats the element vector into a fresh `Seq`; `Seq * String` joins
  the elements with the separator (via the same `format` display `join`
  uses).  `*` is binary in Ruby but the SIR builtin is variadic, so the
  string/array arms fold **left-associatively** pairwise (`times_binary`),
  preserving the variadic contract; the numeric path is unchanged.
- **No core-IR or frontend change** — this is purely the backend runtime.
  `+`/`*` already lower to `__sir::plus`/`__sir::times`; only those two
  helpers gained the polymorphic arms.

Rejects: `TailCalls` (Rust does not guarantee TCO), `Intrinsics`
(empty whitelist in v0), `StringInterpolation`, and a stateful (non-empty
body) `Class`/`Module` or a non-name-slot `Const` reference (rejected
cleanly by the soundness gates).

## Value model

```rust
#[derive(Clone)]
enum Value {
    Int(i64), Float(f64), Bool(bool), Nil,
    Missing,                                 // P2e DefaultParams sentinel
    Sym(Rc<str>), Str(Rc<str>),
    Pair(Rc<Pair>),
    Closure(Rc<Closure>),
    Seq(Rc<RefCell<Vec<Value>>>),            // SIR16 Sequences
    Map(Rc<RefCell<Vec<(Value, Value)>>>),   // SIR16 Maps (insertion-ordered)
    Instance(u64),                           // SIR17 O5 user-object handle
}
```

- Single-threaded (`Rc`, not `Arc`).
- `Instance(u64)` is a **narrow, dedicated** variant carrying only an opaque
  id; the object state (`SirInstance { class, ivars }`) lives in a
  `thread_local` `INSTANCES` side-table keyed by that id.  This hybrid keeps
  `Value: Clone` a trivial `u64` copy and gives instances a *distinct*
  discriminator (no `pair?`/`car`/`cdr` leak, correct `format`/`value_eq`),
  while confining mutable object state to one `thread_local`.  A disguised
  handle (reusing `Pair`/`Sym`) would leak into the built-in catalogs;
  inlining `SirInstance` would burden every `Value` clone — the id-handle
  avoids both.  The arm touches only THIS backend's emitted-runtime `Value`,
  never the core semantic-IR.
- Closures wrap a `Box<dyn Fn(Vec<Value>) -> Value>` inside an `Rc`.
- Symbols and strings are interned `Rc<str>` for cheap clones.
- Globals live in a `thread_local!` `HashMap<String, Value>`.
- Sequences and maps are `Rc<RefCell<…>>` so `SeqSet`/`MapSet` mutate the
  shared value in place; maps key by `value_eq` (linear lookup) and keep
  insertion order.

## `main` collision

SIR's synthesised `main` function is renamed to `__sir_user_main`
in the generated Rust because `main` is Rust's process entry
point.  The emitter generates its own `main()` that calls `_init()`
(if present) then `__sir_user_main()`.  For an exception-using module,
`main()` additionally installs the quiet panic hook, registers the
module's user ancestry, and wraps the body in a top-level `catch_unwind`
so an uncaught SIR exception exits cleanly non-zero.

## Tests

`cargo test -p semantic-ir-to-rust`

Covers per-node lowering, identifier sanitisation (including
raw-identifier syntax for Rust keywords), deterministic output,
and end-to-end pipelines from Twig source.

Exception execution-proof (`tests/compile_and_run_exceptions.rs`) compiles
emitted Rust with `rustc` and runs it, checking five cases against the
Python/TS reference behaviour: typed rescue via built-in ancestry, bare
rescue, unmatched re-raise (non-zero exit), `ensure` on caught + uncaught
paths, and user ancestry (`MyErr < StandardError`).  It skips (never fails)
when no linker is available; point it at one via `SIR_TEST_RUSTC_LINKER`
(e.g. the toolchain's bundled `rust-lld`).

OOP execution-proof (`tests/compile_and_run_oop.rs`) does the same for
user-defined classes: `Dog#initialize`/`speak` (ivar-through-method
dispatch), inheritance + `super` (`Cat.new(4).describe` → `104`), a security
test (a `constructor`-named class works as data while an unregistered method
floors to `nil`), and cyclic-ancestry termination.  Same linker gating.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR itself
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/) —
  sister backend
