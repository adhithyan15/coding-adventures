# Changelog

## 0.10.0 — `puts` builtin (Ruby semantics)

### Added

- The Rust backend now emits and executes Ruby's `puts`, the most common
  output method. `puts` maps to a new **variadic** runtime helper
  `__sir::puts(vec![…])` (routed both by the emit helper table and
  `call_builtin_by_name`), reusing `__sir::format` for element rendering.
- Ruby semantics implemented exactly: no-arg → one newline; `puts x` →
  `x.to_s` + newline (no double newline when the text already ends in `"\n"`);
  `puts a, b` → one line per arg; `puts []` → a single newline; a
  `Value::Seq` is flattened recursively, one **element** per line; `puts nil`
  → a blank line.
- Execution proof `compile_and_run_puts.rs` compiles `puts "hello"; puts;
  puts [1,2,3]` with `rustc`, runs it, and asserts stdout is exactly
  `hello\n\n1\n2\n3\n` (the Ruby reference output).

### Security — cycle-guard the `puts` array flatten (CWE-674)

- `__sir::puts_one` flattened arrays by recursing per element with **no
  bound**. A `Value::Seq` is a shared, mutable `Rc<RefCell<..>>` handle, so a
  translated program can build a self-referential array
  (`a = []; a << a; puts a`) or a pathologically deep one; the unguarded
  recursion overflowed the native stack and aborted — a denial of service
  (uncontrolled recursion). The flatten now threads a `visited` set of the `Rc`
  handle addresses on the active path (the same `seq_handle_id` key
  `__sir::format` uses): a handle re-encountered within its own subtree is a
  cycle and renders as Ruby's `[...]` placeholder + newline instead of
  recursing, so `puts a` on a self-referential array now **terminates** exactly
  as real Ruby does. Non-cyclic output is byte-for-byte unchanged
  (`puts [1,[2,3]]` → `1\n2\n3\n`); a new regression test
  (`puts_cyclic_array_terminates`) proves the self-referential case exits
  cleanly with `[...]\n`.

## 0.9.0 — user-defined class OOP runtime + emit (O5)

Makes the Rust backend **accept and execute** real user-defined-class OOP
(`Foo.new`, `initialize`, instance/class methods, `super`, `self`, `@ivar`,
`@@cvar`, inheritance) — the Rust analogue of the O1/O3/O4 backends. Before
this change `Feature::Classes` was accepted ONLY for empty-body
exception-subclass declarations, and `@ivar`/`@@cvar`/`self`/`new`/`super`
had no runtime; a real OO program was rejected or hit a `panic!` guard.

### Value-model decision (variant vs. side-table)

A **narrow, dedicated `Value::Instance(u64)` variant backed by a
`thread_local` side-table**. The `u64` is an opaque instance id; the object
state (`SirInstance { class, ivars }`) lives in the `INSTANCES` side-table
keyed by that id. This is a *hybrid* of the two options the milestone
weighed:

- A side-table alone (reusing a magic `Pair`/`Sym` as a disguised handle)
  would **leak**: `pair?`/`car`/`cdr` would operate on an "instance" and
  `format`/`value_eq` would mis-render it.
- Storing `SirInstance` **inline** (`Instance(Rc<SirInstance>)`) would put a
  `RefCell<HashMap>` on the hot, frequently-cloned `Value`.

The id-handle-plus-side-table keeps `Value: Clone` a trivial `u64` copy,
gives instances a *distinct* discriminator (no built-in-type leak, correct
`format`/`value_eq`), and confines mutable object state to one
`thread_local`. Adding the arm touches ONLY this backend's emitted-runtime
`Value` — never the core semantic-IR — and only two existing exhaustive
sites (`format_d`; an identity arm in `value_eq_d`); every other `match`
already has a `_`/`matches!` fallback.

### Added

- **Runtime (`runtime.rs`).** A user-defined-class OOP model in the inline
  `__sir` module, reusing the exception runtime's `seen`-guarded ancestry
  walk (`super_of`/`is_ancestor_or_self`):
  - `Value::Instance(u64)` + `SirInstance { class, ivars: RefCell<HashMap> }`
    in the `INSTANCES` side-table; `new_instance(cls)` allocates a fresh
    handle.
  - `METHOD_TABLE` / `CLASS_METHOD_TABLE` — `HashMap<(String, String),
    Value>` keyed by the `(class, method)` pair (the `Value` is the
    method-body `Closure`). `def_method`/`def_class_method` populate them.
  - `call_new(cls, args)` — allocate → run the inherited `initialize`
    (ancestry-resolved, `seen`-guarded) with `self` bound → return the
    instance (Ruby discards `initialize`'s result). `call_super(method,
    cls, args)` — resolve from the superclass of `cls`, reuse the live
    `self`. `call_method` gains a **user-instance branch, taken FIRST**,
    that resolves the user table walking ancestry; **every other receiver
    keeps the unchanged collection/built-in path.**
  - `current_self()` (`__self__`); `ivar_get`/`ivar_set` and
    `cvar_get`/`cvar_set` acting on the current self (per-class cvar bags).
  - **RAII self-stack:** a `SelfGuard` whose `Drop` pops the self-stack, so
    a panic mid-method (a SIR `raise` unwinds as a panic) still balances the
    stack — the Rust analogue of the JS runtime's `try { … } finally {
    popSelf(); }`.
  - **SECURITY:** every lookup is an EXPLICIT `HashMap::get` on a `(class,
    method)` key — never reflection/`dyn Any`-by-name. A class/method named
    `constructor`/`new`/`drop` is inert data; a miss floors to the honest
    `Nil`/NoMethodError boundary the collection catalog uses. The ancestry
    walk carries a `seen`-set cycle guard so a cyclic hierarchy terminates.
- **Emit (`emit.rs`).** Emit arms mirroring `__method__`→`call_method`:
  `__new__`→`call_new`, `__super__`→`call_super`,
  `__def_method__`→`def_method`, `__def_class_method__`→`def_class_method`,
  `__self__`→`current_self`. Class/method NAME args (a `StrLit` or a `Const`
  VarRef like `Dog.new`) are LIFTED to `&str` string literals via
  `emit_oop_name_arg` (never a runtime constant read). `@ivar`/`@@cvar`
  reads route to `ivar_get`/`cvar_get`, writes to `ivar_set`/`cvar_set`
  (both statement and inline contexts). The user `subclass → superclass`
  ancestry registration now fires for `Feature::Classes` too (not only
  `Exceptions`), so the OOP resolver's shared ancestry table is populated.
- **Feature acceptance (`lib.rs`).** `ACCEPTED_FEATURES` now includes
  `Modules`, `InstanceVars`, `ClassVars`, and widens the `Classes`/`Constants`
  rationale to real OOP. `reject_const_ref` skips the class-name slots of
  `__new__`/`__super__` (lifted to strings), keeping `Constants` acceptance
  sound. `reject_stateful_class` still rejects a class/module with an
  executable body (methods hoist to top-level functions, so an accepted
  class body is empty) — the soundness gate is unchanged.

### Tests

- **Emit-shape unit tests** (`emit.rs`) for `__new__`/`__super__`/
  `__def_method__`/`__def_class_method__`/`__self__` and `@ivar`/`@@cvar`
  read+write routing. **Runtime-shape unit tests** (`runtime.rs`) pinning
  the `Instance` variant, the tables, the explicit-lookup + cycle-guard, and
  the RAII self-guard.
- **Execution proof through `rustc`** (`tests/compile_and_run_oop.rs`, gated
  on `SIR_TEST_RUSTC_LINKER`): P1 `Dog#initialize`/`speak` (ivar-through-
  method dispatch → `42`); P2 inheritance + `super` (`Cat.new(4).describe` →
  `104`); a security test (`constructor` class + unregistered `drop` → clean
  data / `nil` floor); cyclic ancestry terminates (`A<B<A` → `nil`); and a
  self-stack-balanced check.

### Notes

- No new `unsafe`. No core semantic-IR change (the `Instance` arm is the
  backend's emitted-runtime `Value` only). No new clippy warnings on touched
  files.

## 0.8.0 — exception handling via catch_unwind + ancestry (E4)

Makes the Rust backend **accept and execute** structured exceptions
(`Feature::Exceptions`). Before this change `Stmt::TryCatch` and the
`raise` builtin hit `panic!` guards in `emit.rs` (the feature was not in
`ACCEPTED_FEATURES`), so any `begin/rescue/ensure` module was rejected.
Rust has no native exceptions, so v0 maps Ruby's exception model onto
Rust's **unwinding panic** machinery — a *localized* transform touching
only the `raise`/`TryCatch` arms; every other emit path is unchanged.

### Added

- **Runtime (`runtime.rs`).** An exception model in the inline `__sir`
  module:
  - `SirError { class: String, msg: String }` — the panic payload. `msg`
    is a `String` (not `Value`) because `std::panic::panic_any<M>` requires
    `M: Send`, and our `Rc`-based `Value` is not `Send`; the message is
    rendered at raise time, matching Ruby's string `exception.message`.
  - `raise(class, msg: Value) -> !` → `std::panic::panic_any(SirError{…})`;
    `reraise() -> !` for a bare `raise`.
  - `exc_from_payload(Box<dyn Any + Send>) -> SirError` — downcasts the
    caught payload to a `SirError`, or **`resume_unwind`s** a non-`SirError`
    payload (a genuine Rust panic is never swallowed as a rescuable
    exception).
  - `rescue_matches(&SirError, &[&str]) -> bool` over an **explicit**
    built-in ancestry table (a verbatim parity port of the TS
    `sir-runtime-exceptions` `ANCESTRY`) merged with user edges, with a
    `seen`-set **cycle guard**. `exc_value(&SirError) -> Value` re-wraps the
    message for a `rescue … => e` binding.
  - `register_ancestry(&[(&str, &str)])` — the ONLY channel for user
    ancestry edges (no reflection). `install_panic_hook()` quiets Rust's
    default panic banner for `SirError` payloads; `report_uncaught` renders
    an unrescued exception (`Class: message`) and exits non-zero.
- **Emit (`emit.rs`).**
  - `raise` arm: a `Const` class name (`raise Foo`/`raise Foo, "m"`) is
    **lifted to a string literal** (never emitted as a runtime `Const`
    read); a non-const first arg → `raise("RuntimeError", <arg>)`; bare
    `raise` → `reraise()`.
  - `Stmt::TryCatch` → a `std::panic::catch_unwind(AssertUnwindSafe(||
    {…}))` region whose `match` dispatches rescue clauses in order via
    `rescue_matches`, binds `=> e` with `exc_value`, and **runs `ensure` on
    every path** (Ok, matched, and unmatched-before-`resume_unwind`).
  - `main` wraps the user body in a top-level `catch_unwind` so an uncaught
    SIR exception exits cleanly non-zero; the module's `ClassDef` ancestry
    edges are registered once at init (`register_ancestry`), and the quiet
    panic hook is installed.
  - `Stmt::ClassDef` (empty-body exception subclass) emits no runtime code —
    it is pure ancestry metadata.

### Accepted features

- `Feature::Exceptions`, plus `Feature::Classes` and `Feature::Constants`
  **for the narrow exception use case only**:
  - `Classes` — an empty-body exception-subclass declaration `class MyErr <
    StandardError; end` (methods hoist to top-level `Function`s). A
    **non-empty** class body is rejected cleanly by `reject_stateful_class`.
  - `Constants` — a `raise MyErr` names its class via a `Scope::Const`
    VarRef (lifted to a string). Any **other** `Const` reference is rejected
    cleanly by `reject_const_ref`, keeping the acceptance sound (no
    `emit_var_ref` `Const` panic on validated input).

### Tests

- Emitted-shape unit tests (`emit.rs`): `raise` variants, `TryCatch` →
  `catch_unwind`/`match`, ensure-on-all-paths, empty ClassDef.
- Capability-gate unit tests (`lib.rs`): accept exceptions/subclass, reject
  stateful class, reject non-raise const ref, allow raise-class-name const.
- Runtime-shape tests (`runtime.rs`): exception helpers present, explicit
  table + cycle guard, non-`SirError` passthrough.
- Execution-proof through `rustc` (`tests/compile_and_run_exceptions.rs`,
  gated on `SIR_TEST_RUSTC_LINKER`): (a) typed rescue via built-in ancestry,
  (b) bare rescue, (c) unmatched re-raise exits non-zero, (d) ensure runs on
  caught + uncaught, (e) user ancestry `MyErr < StandardError` caught by
  `rescue StandardError`.

### Security

- Rescue matching is an **explicit ancestry-table lookup** — never
  reflection / type-name introspection. A `seen`-set cycle guard bounds the
  ancestry walk. A non-`SirError` panic (a real Rust bug) is `resume_unwind`
  ed, never mis-dispatched to a rescue. `AssertUnwindSafe` is used for
  generated code only (documented rationale: the `Err` path re-derives what
  it needs and never reads partially-mutated captured state). No new
  `unsafe`.

## 0.7.0 — collection-method dispatch + runtime catalog (C6)

Makes the Rust backend **execute** collection-method dispatch. A
source-level `recv.meth(args…)` / `recv.meth { |x| … }` reaches every
backend as the narrow-waist envelope
`BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`. Before
this change the Rust backend had no `__method__` arm, so the call fell into
the `call_builtin_by_name` catch-all and hit its runtime floor
`panic!("unknown builtin: __method__")` — a collection program compiled but
crashed at run time. (No capability gate rejected it: `__method__` observes
no dedicated feature, and a pure collection module's observed features —
`Sequences`/`Closures`/`Strings` — were already accepted.)

### Added

- **Emit (`emit.rs`).** A `"__method__"` case in `emit_builtin_call`
  (`emit_method_dispatch`) lowers the envelope to
  `__sir::call_method(<recv>, "meth", vec![<arg0>, …])`. The receiver is
  passed by value; the method name is lifted out of the `StrLit` at
  `args[1]` to a Rust `&str` **literal** (keeping dispatch a closed,
  compile-time-known set); the remaining args — including any trailing
  `MakeClosure` block, which emits a `Value::Closure` — fill the arg `Vec`.
  A `"block_pass"` case lowers `&:sym` / `&blk` to `__sir::sym_to_proc(…)`.
- **Runtime catalog (`runtime.rs`).** A `call_method(recv: Value, name:
  &str, args: Vec<Value>) -> Value` in the inline `__sir` module,
  implementing the collection catalog by an **explicit** match on the
  receiver's runtime type then the method name, ported from the Python/TS
  `sir-runtime-oop` reference for parity:
  - **Array** (`Value::Seq`): `length`/`size`, `first`, `last`, `push`/
    `append`, `pop`, `include?`, `reverse`, `sort`, `join`, `map`/
    `collect`, `select`/`filter`, `reject`, `find`/`detect`, `reduce`/
    `inject`, `each`, `any?`, `all?`, `none?`.
  - **Hash** (`Value::Map`): `keys`, `values`, `size`/`length`,
    `has_key?`/`key?`/`include?`/`member?`, `each`/`each_pair`, `map`,
    `select`/`filter`.
  - **String** (`Value::Str`): `length`/`size`, `upcase`, `downcase`,
    `reverse`, `strip`, `include?`, `split`.
  - **Numeric** (`Value::Int`/`Value::Float`): `abs`, `to_i`, `to_f`,
    `even?`, `odd?`, `zero?`, `times`.
  - Universal `to_s` on every receiver (via the runtime `format`), so
    `&:to_s` works across types.
  - `sym_to_proc` implements Ruby `Symbol#to_proc` (`&:sym`): the returned
    `Closure` dispatches `recv.sym(rest…)` through `call_method`. An
    already-callable `&blk` passes through unchanged.
- **Execution-proof test** (`tests/compile_and_run_collection_methods.rs`):
  hand-builds SIR modules for `map { x*2 }` → `[2, 4, 6]`,
  `select { even? }` → `[2, 4]`, `length` → `3`, `reduce(0)`/`inject` sum
  → `6`, `map(&:to_s).join(",")` → `"1,2,3"`, `sort` → `[1, 2, 3]`, and
  `bogus_xyz` → `nil`; emits Rust, compiles with `rustc`, runs it, and
  diffs stdout against the Python/TS reference. Skips gracefully if
  `rustc`/linker is absent (`SIR_TEST_RUSTC_LINKER`).
- Emitted-shape unit tests for the `__method__` and `block_pass` arms, and
  runtime-content tests asserting the catalog + the absence of a reflective
  fallback.

### Security

- Dispatch is an **explicit allowlist**: `call_method` matches only the
  hand-written `(type, name)` catalog. An unknown method name falls through
  to `unknown_method`, which returns a controlled Ruby `nil` — never a
  reflective lookup on the raw name and never an out-of-catalog effect.
  This mirrors the C3 RCE lesson (the catalog *is* the security boundary).
  No new `unsafe`.

### Notes

- No `Feature` variant added: `Feature::MethodDispatch` (deferred C1) is
  not required here — the catalog is the gate. A pure collection-method
  module was already capability-accepted; this change only makes it
  *execute* instead of panicking. Genuinely-unsupported features stay
  rejected cleanly.

## Unreleased — reject keyword params mixed with rest/kwrest (hardening)

### Fixed

- **Reachable emit panic on validator-accepted input (DoS).** The core
  validator's M3 ordering rule accepts a signature that mixes a keyword
  parameter with a variadic slot (`Required* Rest? Keyword* KwRest?`),
  e.g. Ruby `def f(a, *rest, x: 1)`. Because this backend accepts
  `Feature::KeywordParams`, such a module reached the emitter's static
  keyword→positional resolution path and hit the
  `ParamKind::Rest | ParamKind::KwRest` `panic!` — a reachable panic on
  validated input (and frontend-reachable once the Ruby frontend emits
  keyword+splat methods). Static keyword resolution genuinely cannot
  handle a variadic slot: a `*rest`/`**kwrest` param absorbs a *variable*
  number of arguments, so the name→position map that keyword resolution
  depends on is no longer a function of the signature alone (variable
  arity breaks fixed slot indices). The backend now REJECTS such modules
  cleanly at capability-check time (`reject_keyword_with_variadic`,
  `BackendErrorKind::UnsupportedFeature`, message
  `rust backend cannot emit a function mixing keyword parameters with
  *rest/**kwrest (static keyword resolution requires fixed arity)`)
  instead of panicking. With the check in place, the emit-side variadic
  arm is now a true internal-bug guard, never reachable through the normal
  `compile` path. The happy path (keyword params WITHOUT rest/kwrest) is
  unchanged and still emits.

### Added

- Unit tests: keyword+`*rest` and keyword+`**kwrest` callees with a
  keyword call are rejected via `compile()` (return `Err`, do NOT panic);
  a keyword-only module (no variadic) still compiles.

## 0.6.0 — keyword-parameter & argument emission (KW5)

Adds `Feature::KeywordParams` support: name-matched keyword parameters
(`def f(a:)` / `def f(a: 1)`) and keyword arguments (`f(a: 1)`). Rust has
**no native keyword-argument syntax**, so — per `sir-keyword-params.md`
§4 — the backend performs **static keyword→positional resolution at emit
time** (no runtime library). This replaces the KW1 compile-compat stub
(a `KeywordArg` panic arm; `ParamKind::Keyword` folded into a positional
arm) with real emission.

### Added

- **Def-side positional-ization** — a `Keyword` param emits as an
  ORDINARY positional Rust parameter in its declared order (the by-name
  affordance is dropped; the name becomes the Rust parameter name). An
  OPTIONAL keyword (one carrying a `default`) reuses the existing
  `DefaultParams` body-top prologue unchanged — it is a defaulted
  parameter like any other — so no new def-side machinery is required.
- **Call-side static resolution** — for a `DirectCall` whose callee
  signature is known, the emitter builds the FULL positional argument
  list in the callee's DECLARED order: positionals fill positional params
  in order; each `KeywordArg { name, value }` fills the callee param whose
  name matches `name` (a name→position reorder); an omitted OPTIONAL
  keyword is padded with the `__sir::missing()` sentinel so the callee's
  prologue substitutes its default (deferring default evaluation to callee
  scope — correct even when a default references an earlier param). The
  result is a plain positional Rust call `f(a, b_val, c_default)`.
- **`FN_PARAMS` thread-local signature table** — SIR function name → its
  full parameter list (kinds + defaults). Where `FN_ARITY` records only
  the param count, keyword resolution needs the params' ORDER, NAMES, and
  DEFAULTS. Populated alongside `FN_ARITY` in
  `emit_module_with_arity_table` and consulted by the `DirectCall`
  emitter.
- **`Feature::KeywordParams`** added to the backend's `ACCEPTED_FEATURES`
  (mirroring `DefaultParams`).
- **Unit tests** — def-side positional-ization + default prologue;
  call-side supplied-keyword → positional; call-side omitted-optional →
  sentinel; call-side name→declared-position reorder.
- **Execution proof** (`tests/compile_and_run_keyword_params.rs`) — a
  `def greet(greeting, name: "world") -> name` module, compiled with
  `rustc` and run: `greet("hi")` prints `world` (default) and
  `greet("hi", name: "ada")` prints `ada` (supplied), matching the
  Python/TS reference for `name`. Skips gracefully if `rustc`/linker are
  absent (`SIR_TEST_RUSTC_LINKER`).

### Out of scope (documented)

- **Indirect/closure keyword calls** — an `IndirectCall`/closure carrying
  keywords has no statically-known signature, so keyword→position
  resolution cannot run. The frontends do not emit this (spec
  §"Out of scope"); the `emit_expr` `KeywordArg` arm keeps a positioned
  panic documenting that narrow, internal-bug-only reachability.

## 0.5.0 — default-parameter emission (P2e)

Adds `Feature::DefaultParams` support: a `Param` may now carry a
`default` expression that runs when the caller omits that trailing
argument.  Rust functions are fixed-arity over `__sir::Value` with no
native default parameters, so the backend uses a **runtime-mimic**
strategy built around a `Missing` sentinel — preserving the language's
**call-time, param-scope** default semantics (a default expression is
evaluated on each call that omits the argument, in body scope where
*earlier* parameters are already bound, so `b = a + 1` resolves `a`).

### Added

- **`__sir::Value::Missing`** — a new sentinel variant marking an
  *omitted* positional argument, plus **`__sir::missing()`** (constructor)
  and **`__sir::is_missing(&Value)`** (predicate).  `Missing` is internal:
  it is created only at call sites that drop a trailing argument and is
  consumed by the callee's prologue before any value flows on.
- **Defensive runtime arms** — `format` renders a stray `Missing` as
  `<missing>` and `value_eq` treats `Missing` as equal only to another
  `Missing` (never to `Nil` or a real value).  These should be
  unreachable in well-formed programs but degrade gracefully instead of
  panicking.
- **Function-body default-param prologue** — for each defaulted param, in
  declaration order, the emitter now writes
  `let <name> = if __sir::is_missing(&<name>) { <default> } else { <name> };`
  at the top of the function body.  Emitting the default *inside the body*
  is what gives call-time + param-scope semantics.
- **`DirectCall` caller padding** — a call that omits trailing defaulted
  arguments now pads the omitted positions with `__sir::missing()` so the
  emitted Rust call is full-arity.  The callee's full parameter count is
  read from the existing `FN_ARITY` thread-local arity table (the same
  table `MakeClosure` consults), keyed by the callee's SIR name.
- **`Feature::DefaultParams`** added to `ACCEPTED_FEATURES`.

### Tests

- Unit tests for the emitted shape: the sentinel-guarded prologue (with a
  default that references an earlier param), the padded full-arity call,
  and the no-padding case for a fully-supplied call.
- A `rustc` compile-and-run integration test
  (`tests/compile_and_run_default_params.rs`): hand-builds
  `f(a, b = a + 1) -> b`, prints `f(5)` then `f(5, 10)`, compiles the
  emitted Rust with `rustc`, runs it, and asserts stdout `6` then `10`.

Non-default behaviour is byte-for-byte unchanged — every existing test
(floats, loops, seq/maps, cyclic) passes untouched.

## 0.4.1 — harden emitted runtime against cyclic Seq/Map

`Value::Seq`/`Value::Map` are shared, *mutable* handles, so an emitted
program can build a cyclic structure (`xs = []; xs[0] = xs`).  Before this
release the emitted runtime walked such values structurally with no cycle
protection, so a cyclic value could:

- **`format`** — recurse forever and overflow the stack while printing.
- **`value_eq`** — recurse forever when comparing two *distinct* cyclic
  structures (a self-cycle was already short-circuited by the `Rc::ptr_eq`
  fast path, but distinct cyclic operands were not).
- **`map_get`/`map_set`/`map_lit`** — hit a `RefCell` "already mutably
  borrowed" panic when a self-referential key was compared while the map's
  entries were `borrow_mut`'d.

This is a robustness fix only — the public runtime API and the printed
form of every *non-cyclic* value are byte-identical (all existing tests
pass unchanged).

### Fixed

- **`format` / `format_seq` / `format_map`** now thread a visited-pointer
  set (`HashSet<usize>` of each Seq/Map `Rc` handle address).  A handle is
  inserted on entry and removed on exit, so it is only "seen" along the
  *current* path: a true cycle re-entering a handle within its own subtree
  prints a placeholder (`[...]` for a seq, `{...}` for a map) and returns
  instead of recursing, while a value reached twice by sibling
  (non-cyclic) paths still prints in full.  `format_pair` threads the set
  too (a pair can hold a cyclic seq/map).
- **`value_eq`** keeps the `Rc::ptr_eq` identity fast path and adds a
  co-inductive `pending` set of handle-pairs currently being compared:
  re-encountering a pair already in flight (a cycle matched in lock-step)
  is treated as equal, bounding the deep comparison of two distinct cyclic
  operands so it always terminates.
- **`map_get` / `map_set` / `map_lit`** no longer call `value_eq` while
  holding a borrow on the same map's entries: each snapshots/collects the
  comparison inputs and resolves to an *index* before taking the borrow it
  needs for the read/write, so a self-referential key can no longer trigger
  an "already borrowed" panic.

### Tests

- New `tests/compile_and_run_cyclic.rs` integration test: hand-builds a
  module that constructs a cyclic seq (`xs = [0]; xs[0] = xs; print(xs)`),
  emits Rust, compiles it with `rustc`, runs it, and asserts the program
  *terminates* and prints the `[...]` placeholder.  It also checks that
  `value_eq` terminates on both a self-cyclic operand (via `ptr_eq`) and
  two *distinct* cyclic structures (via the co-inductive guard).

## 0.4.0 — SIR16 Sequences + Maps (completes SIR16 / v1 parity)

The final two SIR-v1 (SIR16) features land in the Rust backend.  With
`Sequences` and `Maps` now accepted, the Rust backend supports **all six**
SIR16 / SIR-v1 features — `Floats`, `ShortCircuit`, `MutableBindings`,
`Loops`, `Sequences`, `Maps` — reaching full v1 parity with the
TypeScript backend.  Every SIR16 IR node now has a real emit arm; the
only remaining `panic!`s cover SIR17/18 nodes (classes, modules,
singleton classes, try/catch, string interpolation, instance/class/const
vars, intrinsics) whose features stay unaccepted, so they are unreachable
for any validated module.

### Added

- `Feature::Sequences` and `Feature::Maps` in the accepted-feature set
  (`lib.rs`).
- **Runtime value model** — two new shared, mutable `Value` arms:
  - `Value::Seq(Rc<RefCell<Vec<Value>>>)` — a growable vector.  The
    `Rc<RefCell<…>>` is essential: `SeqSet` (`xs[i] = v`) must mutate the
    sequence the caller holds, and aliasing bindings must observe each
    other's writes — the reference semantics of a Python list / JS array.
  - `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)` — an insertion-ordered
    association list.  Keys compare with the runtime's own `value_eq`
    (linear scan) rather than a `HashMap`, because `Value` is neither
    `Hash` nor `Eq` (floats, closures, nested seqs/maps).  This gives
    correct lookup semantics for *any* key type and preserves insertion
    order for deterministic iteration and printing.
- **Sequences** — `SeqLit`/`SeqIndex`/`SeqLen` expressions lower to the
  `seq_lit`/`seq_index`/`seq_len` helpers; the `SeqSet` statement mutates
  the backing vector through `seq_set`.  Out-of-range index reads/writes
  panic (strict, like `car`/`cdr` on a non-pair).
- **Maps** — `MapLit`/`MapGet` expressions lower to `map_lit`/`map_get`;
  the `MapSet` statement mutates via `map_set`.  A missing-key `MapGet`
  returns `Nil` (mirroring the TypeScript backend's `?? null`).  Literal
  and `map_set` writes are last-write-wins on an existing key while
  preserving first-seen insertion order.
- **`format`** renders sequences as `[1, 2, 3]` and maps as `{a: 1, b: 2}`
  (insertion order); **`value_eq`** compares seqs/maps structurally
  (element-wise, with an `Rc::ptr_eq` fast path).

### Changed (ForEach reconciliation)

- A2 introduced `Stmt::ForEach` with a `seq_iter` helper that walked a
  cons-list (there was no `Seq` value yet).  Now that a real
  `Value::Seq` exists, `seq_iter` was reconciled to **snapshot a
  `Value::Seq`** as well as walk the legacy cons-list, so a
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while the
  cons-list path keeps working unchanged.
- Fixed a latent `ForEach` emit bug surfaced by the new end-to-end test:
  the loop emitted `for <var>: __sir::Value in …`, but a type annotation
  on a `for` pattern is not valid Rust.  Dropped the annotation
  (`for <var> in …`); the element type is already `Value` from
  `seq_iter`'s return.  This path had no prior compile-and-run coverage
  (the loops test exercised only `while`/`for-range`).
- The mutable-binding pre-pass (`collect_assigned_locals`) now recurses
  into Seq/Map statements and expressions, so an `Assign` nested inside a
  `SeqSet`/`MapSet` value (or a `SeqLit`/`MapLit` sub-expression) is
  still discovered and its binding declared `let mut`.

### Tests

- `tests/compile_and_run_seq_maps.rs`: an end-to-end proof that emits a
  module using a sequence (literal, index, len, set), a map (literal,
  get with a present *and* a missing key, set), and a `for v in <SeqLit>`
  `ForEach` accumulation, compiles it with `rustc`, runs the binary, and
  asserts its stdout (`20`, `3`, `99`, `2`, `nil`, `7`, `10`).
- Unit tests for every new emit arm (seq/map literal, index, len, get,
  and the seq/map set statements in both the block and inline paths),
  for `ForEach`-over-`SeqLit` composition, and for the mutable-name
  pre-pass recursing into a `SeqSet` value; runtime tests for the new
  value arms and helpers.

## 0.3.0 — SIR16 MutableBindings + Loops

The next two SIR-v1 (SIR16) features land in the Rust backend, matching
the TypeScript backend's existing support.  Until now `MutableBindings`
and `Loops` were undeclared and their IR nodes hit the `panic!` reject
group; this PR replaces those arms with real emission.

### Added

- `Feature::MutableBindings` and `Feature::Loops` in the accepted-feature
  set (`lib.rs`).
- **MutableBindings**: a per-function pre-pass (`collect_assigned_locals`)
  finds every name that is later the target of a `Stmt::Assign`.  A
  `LetBinding` for such a name is emitted as `let mut` (immutable bindings
  stay plain `let`), and `Stmt::Assign` then emits a bare
  `<name> = <value>;` for Local/Param/Capture scopes.  A `Global`-scoped
  assign writes through the runtime store
  (`__sir::global_set(&__sir::intern("name"), value)`).  Mirrors the
  TypeScript backend's `const`/`let` mutable-name tracking.
- **Loops** — all three loop statements emit real Rust:
  - `While { cond, body }` → `while __sir::truthy(&(<cond>)) { <body> }`,
    routing the test through SIR truthiness (only `false`/`nil` are
    falsy), never Rust's native `bool`.
  - `ForRange { var, start, stop, step, body }` → a numeric loop that
    caches `stop`/`step` into block-scoped `i64` temporaries (evaluated
    once, like Python's `range`), with a direction-aware condition so a
    negative `step` counts down.  The loop variable is rebound each
    iteration as a fresh `__sir::Value::Int`.  Fresh per-loop temp ids
    keep nested loops collision-free; the counter resets per module for
    deterministic output.
  - `ForEach { var, iter, body }` → `for <var> in __sir::seq_iter(&(<iter>))`.
    This backend has no dedicated `Seq` value yet (Sequences land in a
    later PR), so a "sequence" is the existing cons-list (`Pair`-chain
    terminated by `Nil`); `seq_iter` flattens it into a `Vec<Value>`.
    No `Feature::Sequences` runtime is required — the validator observes
    only `Feature::Loops` for `ForEach`, so accepting `Loops` covers all
    three loop forms with **no reachable `panic!`**.
- Runtime helpers `as_int` (public face of `as_i64`, for the `ForRange`
  bound temporaries) and `seq_iter` (cons-list → `Vec<Value>` for
  `ForEach`).

### Tests

- `tests/compile_and_run_loops.rs`: an end-to-end proof that emits a
  module using a `while` loop, two `for-range` accumulators, and mutable
  reassignment, compiles it with `rustc`, runs the binary, and asserts
  its stdout (`sum 0..5 = 10`, countdown ends at `0`, product `= 6`).
- Unit tests for each new emit arm: bare/global assign, `let mut`
  selection, while/truthy, for-range bound caching + int var binding +
  direction-aware condition + nested fresh ids, and for-each via
  `seq_iter`.

### Notes

- The remaining two SIR16 features (Sequences, Maps) are still
  undeclared; their `SeqSet`/`MapSet` and Seq/Map expression emit arms
  keep the `panic!` (unreachable via the capability check) until a later
  PR extends them.

## 0.2.0 — SIR16 Floats + ShortCircuit

The first two SIR-v1 (SIR16) features land in the Rust backend.  Until
now `Floats` and `ShortCircuit` were undeclared and their IR nodes hit
the `panic!` reject group; the TypeScript and Python backends already
supported them, so this closes part of the cross-backend parity gap.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` in the accepted-feature
  set (`lib.rs`).
- Runtime value model gains `Value::Float(f64)`.  The arithmetic helpers
  (`plus`/`minus`/`times`/`divide`) stay on the exact i64 path while
  every operand is an integer and promote the whole fold to f64 as soon
  as any operand is a float (Python/Ruby/JS "int op float ⇒ float").
  `number?` now covers floats, `=` is cross-representation (`1 == 1.0`
  is true; `NaN == NaN` is false), and `<`/`>` compare numerically.
- `Expr::FloatLit` emits a `Value::Float` literal — `{:?}` keeps the
  trailing `.0` on integral values so the literal is never mistaken for
  an `i64`; non-finite values use `f64::NAN`/`INFINITY`/`NEG_INFINITY`.
- `Expr::LogicalAnd`/`Expr::LogicalOr` emit a truthy-guarded block
  (`{ let __l = lhs; if truthy(&__l) { ... } else { ... } }`) that
  evaluates the rhs only when the lhs decides — same semantics as the
  TypeScript backend's truthy-guarded arrow IIFE.

### Tests

- `tests/compile_and_run_floats.rs`: an end-to-end proof that emits a
  float + short-circuit module, compiles it with `rustc`, runs the
  binary, and asserts its stdout.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still undeclared; their emit arms keep the `panic!`
  (unreachable via the capability check) until later PRs extend them.

## 0.1.2 — SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 — SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 — initial release (SIR13 v0)

Second backend for the narrow-waist Semantic IR.  Emits self-contained
Rust source from a `semantic_ir::Module`.

### Added

- `RustBackend` implementing `semantic_ir::Backend` with:
  - `target_tag() = "rust"`
  - `accepts_features()` covering the full v0 surface minus
    `TailCalls` and `Intrinsics`.
- `compile(module)` convenience function returning an
  `Artifact { filename, source, metadata }`.
- Per-node lowering rules per SIR13:
  - Literals → typed `__sir::Value::*` constructors.
  - Symbols → `__sir::intern("...")`.
  - VarRef Local/Param/Capture → `<name>.clone()`.
  - VarRef Global → `__sir::global_get_static("...")`.
  - VarRef Builtin → `__sir::builtin_closure("...")`.
  - If → Rust `if/else` with `__sir::truthy(&cond)`.
  - Block → Rust block expression `{ stmts...; value }`.
  - LetBinding / LetStarBinding → `let name: __sir::Value = ...;`.
  - DirectCall → `<fn>(<args>)`; SIR `main` is renamed to
    `__sir_user_main` to avoid collision with Rust's process entry.
  - IndirectCall → `__sir::apply_closure(&target, vec![args])`.
  - BuiltinCall → typed helper or `call_builtin_by_name` fallback.
  - MakeClosure → `__sir::Value::Closure(Rc::new(__sir::Closure {
    fun: Box::new(move |args| <fn>(<captures>, <pos-args>)) }))`.
- Inlined `__sir` runtime (~280 lines) covering:
  - `Value` enum, `Pair` struct, `Closure` wrapping a `Box<dyn Fn>`.
  - `intern` / `apply_closure` / `truthy` / `format`.
  - All v0 builtins (`plus`, `minus`, `times`, `divide`, `eq`,
    `lt`, `gt`, `cons`, `car`, `cdr`, `is_null`, `is_pair`,
    `is_number`, `is_symbol`, `print`).
  - `thread_local!` storage for globals + symbol interning.
  - `call_builtin_by_name` dispatch for VarRef Builtin and
    forward-compat new builtins.
- Identifier sanitisation:
  - Valid Rust identifiers pass through.
  - Rust keywords (`fn`, `type`, `match`, etc.) get the `r#`
    raw-identifier prefix so the original spelling stays visible.
  - Other invalid characters (`?`, `!`, `-`, `+`, `*`) are encoded
    as `_<hex>` underscore-escaped forms.
  - Empty input becomes `"_$empty"`.
  - SIR's `main` is specially renamed to `__sir_user_main`.
- Function arity table threaded via TLS so `MakeClosure` knows
  how many positional arguments to drain from the runtime args
  iterator when calling the synthesised lambda function.
- `sanitize_comment` strips line terminators (`\n`, `\r`, U+0085,
  U+2028, U+2029) from any external string written into `//`
  comments, mirroring the TypeScript backend's defense.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via the `Backend::check_module` default impl.

### Deferred

- Static type narrowing.  Optional SIR types widen to `Value`.
- `no_std` / `alloc`-only target.
- Source-map generation (function-level comments only).
- Raw-Rust intrinsic embedding.
- Async / `await` support (no SIR async surface yet).
