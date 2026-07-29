# Changelog

## 0.20.0 — OOP mirror slice 6: class variables (`@@x`)

Class variables — the sixth slice of the C OOP mirror. Accepts
`Feature::ClassVars`.

- A class variable belongs to a **class** and is shared **down its hierarchy**
  (a `@@x` defined in a parent is the same storage in every subclass). Storage
  is a flat `(class, @@name) → value` table with an ancestry-resolved owner
  (`_sir_cvar_owner` walks `_sir_class_super`, bounded by `SIR_ANCESTRY_MAX`), so
  a subclass method shares its parent's `@@x`.
- A **method body**'s `@@x` read/write → `_sir_cvar_get` / `_sir_cvar_set`, which
  resolve the owning class from a new `_sir_current_class` — bound by dispatch to
  the receiver's class (`_sir_call_method`) or the dispatched class
  (`_sir_call_class_method`), and restored after (so it composes with `super`
  and nested calls).
- A **class-body** initializer (`@@x = 0` inside `class C`) runs where `self` is
  the top-level `main`, so it names its class **explicitly**:
  `_sir_cvar_set_in("C", "@@x", …)`. The `ClassDef` emit now admits a body of
  **only** such `@@x` initializers; any other class-level statement is still
  rejected cleanly (it would otherwise be silently dropped).
- `emit_var_ref` is now exhaustive over `Scope` (all eight variants have a real
  emit path), so its catch-all `unreachable!` is removed — the exhaustive match
  is the compile-time totality signal.

**Anti-RCE.** The `@@`-name and class name emit as **quoted C string literals**
used only as table keys (no injection). Modules (mixins) are the final slice
(still rejected cleanly).

## 0.19.0 — OOP mirror slice 5: class methods

Class (singleton) methods — the fifth slice of the C OOP mirror. No new
`Feature` (class methods lower to builtins).

- `def self.m` → a hoisted function + `__def_class_method__("C", "m",
  MakeClosure(fn))` → `_sir_def_class_method`, which registers the closure in a
  **separate** `(class, method) → closure` table from instance methods. A class
  method `m` and an instance method `m` on the same class therefore never
  collide (both are legal and distinct in Ruby).
- `Class.m(args…)` → `__class_method__("C", "m", args…)` →
  `_sir_call_class_method`, an explicit table lookup — never reflection — that
  **walks the ancestry** (`_sir_class_super`), so a subclass inherits its
  parent's class methods (`class A; def self.m; end; end; class B < A; end; B.m`).
- A class method has no instance receiver, so `_sir_current_self` is bound to
  **nil** for its body (and restored after) — a class method called from inside
  an instance method never sees the caller's `self`.

**Anti-RCE / totality.** Class and method names emit as **quoted C string
literals** used only as table keys (no injection). A `__class_method__` dispatch
to a name the module never registers via `__def_class_method__` is a built-in
class method (`Foo.name`, the Collections batch) and is rejected cleanly via a
`DEFINED_CLASS_METHODS` allowlist (collected in the same walk as the instance-
method allowlist); a malformed registration/dispatch, or one carrying a
control-flow argument, is likewise rejected rather than mis-emitted. `@@class`
variables and modules remain the last two slices (still rejected cleanly).

## 0.18.0 — `fmt_float`: C-printf-faithful float formatting

One builtin, mirroring the Ruby backend, for the C frontend's faithful `printf`
(SIR27 milestone 10).

- `fmt_float(value, precision, kind)` → `_sir_fmt_float_c`, which renders a
  `double` with `snprintf` for the conversion `kind` (`'f'`/`'F'`/`'e'`/`'E'`/
  `'g'`/`'G'`) and precision. The format string is chosen by a `switch` over the
  fixed `kind` character — never built from source text, so there is no
  format-string vulnerability. The output is measured first (`snprintf(NULL, 0,
  …)`) then arena-allocated to the exact size, so any precision fits.

Compiles clean on clang + gcc + MSVC; matches reference C and emitted Ruby
byte-for-byte on the faithful-`printf` corpus.

## 0.17.1 — fix: `raise ClassName, "msg"` constructs the exception

Fixes a cross-backend conformance failure (`exception_reflection` / `puts(e)`): a
`raise ArgumentError, "boom"` lowers to `BuiltinCall("raise", [VarRef(Const
"ArgumentError"), StrLit("boom")])`, but the C `raise` emitter used only the first
argument and let the `Const` fall through to `emit_var_ref` as
`_sir_const_get("ArgumentError")` — and the C runtime registers no builtin
exception-class CONSTANTS, so that raised `NameError: uninitialized constant
ArgumentError` (crashing the program) while dropping the `"boom"` message.

- The `raise` emitter now intercepts a `Const` first argument as a CLASS NAME and
  constructs the exception directly: `_sir_raise(_sir_error("ArgumentError",
  <msg or nil>))` (nil message for a bare `raise Foo`, whose `#message` then
  defaults to the class name). Mirrors the Go/Rust/JS/Python backends. Any other
  first argument (`raise "boom"`, `raise some_exc`) keeps the value path
  (`_sir_raise_value`).
- Handled on BOTH emit paths: the simple inline path and the compound path
  (`emit_compound_call`) taken when the message is a non-simple expression — so a
  computed message (`raise ArgumentError, cond && "x"`) does not regress to the
  same `uninitialized constant` failure.
- The class name stays a QUOTED C string literal (no injection); rescue matching
  and `puts(e)` display are unchanged (they already read the exception's class /
  message). Existing `raise "string"` behaviour is untouched.
- Regression tests: `raise_named_class_with_message_is_caught_and_prints_the_message`,
  `raise_bare_named_class_defaults_its_message_to_the_class_name`, and
  `raise_named_class_with_a_compound_message_still_constructs_the_exception` (the
  prior exception tests only raised bare string messages, so the class-name path
  was never exercised).

## 0.17.0 — numeric conversions: `to_f` / `to_i`

Two numeric-conversion builtins mirroring the Ruby backend, for the C frontend's
floating-point value track (SIR27 milestone 9b).

- `to_f` → `_sir_to_f` = `_sir_float(_sir_as_num(v))` (numeric → double).
- `to_i` → `_sir_to_i` = `_sir_int(_sir_as_int(v))` (double → int, **truncating
  toward zero** like C's `(int)double`; the frontend then narrows to the target
  width with a `Convert`, i.e. `_sir_iN`/`_sir_uN`).

Float arithmetic itself needs no new code: `_sir_plus/minus/times/divide_v` and
the comparison helpers already promote to `double` when any operand is a
`SIR_FLOAT` (so `_sir_divide_v` does true division), and `Feature::Floats` /
`Expr::FloatLit` were already supported.  The emitted C compiles clean on
clang + gcc + MSVC and matches the reference / emitted-Ruby legs byte-for-byte.

## 0.16.0 — OOP mirror slice 4: inheritance + `super`

Class inheritance and `super` — the fourth slice of the C OOP mirror. No new
`Feature` (a superclass is a `ClassDef` field; `super` is a builtin).

- `class Dog < Animal` (a `ClassDef` with a `superclass`) emits
  `_sir_register_super("Dog", "Animal")`, recording the `sub → super` edge in a
  mutable user-ancestry table.
- **One ancestry, two consumers.** `_sir_class_super` now consults that user
  table **first**, falling back to the baked-in exception hierarchy — so the same
  `super_of` relation drives BOTH `rescue`-by-class matching (a user class that
  subclasses `StandardError` is caught) AND OOP method resolution.
- **Inherited dispatch.** `_sir_call_method` resolves a method by walking the
  ancestry (`_sir_resolve_method`: look up on the class, else climb `super`),
  so a subclass that doesn't define a method inherits the parent's closure.
- `super` (`__super__(method, definingClass, …args)`) → `_sir_call_super`, which
  resolves `method` from the **superclass of the defining class** (so it doesn't
  re-enter the override) and applies it to the **current** receiver — `super`
  does not rebind `self`, so `@x` and nested calls still see the original object.
  No ancestor defines it ⇒ a (rescuable) `NoMethodError`.
- **DoS guard.** Every ancestry walk (`_sir_class_is_a`, `_sir_resolve_method`)
  is bounded by `SIR_ANCESTRY_MAX` steps, so a hand-built cyclic hierarchy
  (`A<B`, `B<A` — which the Ruby frontend never emits) resolves to a clean "not
  found" instead of looping.

**Anti-RCE by construction.** Class / method / defining-class names emit as
**quoted C string literals** used only as table keys and `strcmp` targets —
never as C source — so no name can inject code. Class methods, `@@class` vars,
and modules remain the next slices (still rejected cleanly).

## 0.15.0 — OOP mirror slice 3: instance variables (`@x`) + `self`

Instance state — the third slice of the C OOP mirror. Accepts
`Feature::InstanceVars`, so a method body can now read and write the receiver's
instance variables and refer to the receiver directly.

- `@v = x` (a `Scope::Instance` `Assign`) → `_sir_ivar_set("@v", x)`, and `@v`
  (a `Scope::Instance` `VarRef`) → `_sir_ivar_get("@v")`. Each instance carries a
  lazily-allocated `@name → value` map (`struct SirInstance` gains an `ivars`
  slot, `NULL` until the first write); an unset `@v` reads **nil**, matching Ruby.
- A bare `self` (`__self__`) → `_sir_self()`, the current receiver — so a method
  can return `self` for chaining (`w.me.size`).
- **How a hoisted method body finds its receiver.** A method lowers to a
  top-level function with no lexical `self`, so dispatch carries the receiver in a
  process-global `_sir_current_self`: `_sir_call_method` saves the caller's
  `self`, binds it to the receiver for the call, and restores it after (nested
  calls stack correctly through these C-local saves). `@x`/`self` read that
  global; the top-level `main` object gets its own ivar bag (`_sir_toplevel_ivars`).
- **Exceptions interaction.** A method that `raise`s inside a `begin` `longjmp`s
  past `_sir_call_method`'s own restore, so an enclosing `TryCatch` snapshots
  `_sir_current_self` at the `begin` and restores it on the rescue/ensure/escape
  paths — so `@x` in a rescue body reads the *catcher's* ivars, not the raiser's.

**Anti-RCE by construction.** The `@`-name (including the leading `@`) emits as a
**quoted C string literal** and is used only as an interned map key — never as C
source — so no `@`-name can inject code, exactly as with class/method/rescue
names. `@@class` variables (`Feature::ClassVars`), inheritance/`super`, class
methods, and modules remain the next slices (still rejected cleanly).

## 0.14.0 — OOP mirror slice 2: instance methods

Instance-method definition and dispatch — the second slice of the C OOP mirror.

- `__def_method__("Class", "m", MakeClosure(fn))` registers a method: it inserts
  the closure into an explicit `(class, method) → closure` table
  (`_sir_def_method`, keyed on the interned class + method).
- `__method__(recv, "m", args…)` dispatches: `_sir_call_method` resolves
  `(recv's class, "m")` in the table and applies the closure to the args; a
  non-instance receiver or an unresolved method is a (rescuable) `NoMethodError`.

**Anti-RCE by construction.** Dispatch is an **explicit data lookup** on the
`(class, method)` key — never reflection on a source-derived string (the SIR24
§Security invariant). A user method literally named `system`/`eval` is only ever
a table KEY; an unknown method is a controlled `NoMethodError`, never a jump.
(Class/method names emit as quoted C string literals, so there is no injection
surface either.)

**Totality / clean rejection.** A `__method__` dispatch to a name the module
never registers via `__def_method__` is a **built-in method call** (`.length`,
`.upcase`, … — the separate Collections batch) and is rejected cleanly, not
compiled to a runtime `NoMethodError`: a first pass collects the registered
method names (a thread-local allowlist), and the scan validates each dispatch. A
malformed `__def_method__` (not `[StrLit, StrLit, MakeClosure]`) or `__method__`,
and a `__def_method__`/`__method__` with a control-flow argument (which the
compound emit path cannot render), are also rejected. `self`/`@ivars` are the
next slice, so method bodies here don't yet reference the instance.

## 0.13.0 — OOP mirror slice 1: instance runtime + empty class + constants

Accepts `Feature::Classes` + `Feature::Constants` — the first slice of the C
backend's OOP mirror (the Ruby backend just finished the full 7-slice arc). This
slice is the **instance-runtime foundation**:

- A new `SIR_INSTANCE` value tag + `struct SirInstance { const char *sir_class; }`
  stored **inline in the `SirValue` union** — unlike the Go/Rust backends (which
  hold an integer id into a side-table because their value type is `Copy`), the
  C pointer IS the handle, so pointer-identity is object identity (no id table).
- `class Foo; end` → `Stmt::ClassDef` → a comment: a class is just a NAME in the
  C runtime (an instance carries its class string; there is no class object).
- `Foo.new` → `_sir_new_instance("Foo")`, printing `#<Foo>` (deterministic — no
  address, so tests can assert on it). `_sir_value_eq` gains a `SIR_INSTANCE` arm
  (pointer identity, Ruby's default `==` on an object).
- **Constants** ride in (entangled: the frontend records `Constants` for any
  `Foo.new`, since the receiver is a constant). `PI = 3` / `PI` →
  `_sir_const_set` / `_sir_const_get` over a tiny runtime name→value table; an
  undefined constant raises a rescuable `NameError`. Class/constant names are
  emitted as **quoted C string literals** (no injection, as with rescue types).

**Deferred, rejected cleanly** (each a later slice): `__new__` with constructor
arguments (needs `initialize`), a `class << self` singleton, the OOP method
builtins (`__def_method__` / `__method__` / …), and — via their still-unaccepted
features — `@ivars`, `@@class vars`, method-resolving inheritance, and modules.

## 0.12.0 — exceptions (SIR17)

Accepts `Feature::Exceptions`. C has no stack unwinding, so `begin … rescue …
ensure … end` (`Stmt::TryCatch`) and `raise` lower to a **`setjmp`/`longjmp`
handler stack** — the C analogue of Go `panic`/`recover`, per the SIR24
exception-model design.

- **Runtime**: a new `SIR_ERROR` value (`struct SirError { const char *sir_class;
  SirValue msg; }`); a static stack of `jmp_buf` (`_sir_push_handler`/`_sir_pop_
  handler`); `_sir_current_error` (the exception being handled); `_sir_raise`
  (records the error and `longjmp`s to the top handler, or prints `class:
  message` to stderr and exits non-zero when uncaught); `_sir_raise_value`
  (re-raises an exception object, or wraps any other value — a message string —
  in a `RuntimeError`); and a **baked-in exception-class ancestry table**
  (`RuntimeError`/`ZeroDivisionError`/… → `StandardError` → `Exception`,
  `KeyError` → `IndexError`, `NoMethodError` → `NameError`) with
  `_sir_class_is_a` / `_sir_rescue_matches` so `rescue StandardError` catches a
  raised `RuntimeError`. A single `#include <setjmp.h>` is added to the preamble.
- **`TryCatch` codegen**: a TWO-handler structure — an OUTER "ensure" handler
  wraps the whole thing so `ensure` runs even when a rescue body itself raises
  (Ruby semantics), and an INNER "body" handler catches an exception from the
  guarded body. The inner handler is popped BEFORE the rescue dispatch, so a
  raise in a rescue clause (or an unmatched exception) unwinds to the outer
  handler; the outer handler is popped before `ensure` runs, and an unmatched
  exception is re-raised (propagated) after `ensure`.
- **`raise`**: bare (`raise`) re-raises `_sir_current_error`; `raise "msg"`
  raises a `RuntimeError`; `raise <exception>` re-raises it.

**Injection safety**: a `rescue` clause's exception-type names are emitted as
**quoted string literals** (`quote_c_string`) passed to `_sir_rescue_matches` —
never as bare identifiers — so no rescue type can inject source, and the SIR24
"dispatch is an explicit name-switch" anti-RCE invariant holds. The
unsupported-builtin pre-check descends into a `TryCatch`'s guarded/rescue/ensure
bodies (co-total with the emitter).

Deferred to a follow-up (each a clean rejection): `raise SomeClass` (a specific
class) lowers to a `Const` reference → observes `Feature::Constants`
(unaccepted) → rejected; `retry` is not yet lowered (rejected by the builtin
gate — it needs loop machinery in the `setjmp` model).

Documented v0 limitation (correctness, not memory-safety): a *bare* `raise`
(re-raise) inside a rescue body reads the global current-error, so if a nested
`begin/rescue` completes between the clause's entry and the bare `raise`, it
re-raises that inner (already-handled) exception rather than the clause's own —
faithful `$!` save/restore around nested handling is deferred. (An `ensure` body
that handles a nested exception does NOT mis-propagate — the escaping exception
is snapshotted before `ensure` runs; regression-tested.)

First of the exceptions parity arc's C half: with the Ruby backend (0.10.0),
`Exceptions` is now accepted on all six backends. Verified with hand-built
modules compiled and run through a real `cc`: a bare rescue catching a message,
`rescue StandardError` matching a `RuntimeError` via the ancestry, the rescue
binding, `ensure` on both the normal and the exception path, an unmatched
exception propagating through an outer handler after the inner `ensure` runs,
and an uncaught exception exiting non-zero. Bumps semantic-ir-to-c 0.11.0 →
0.12.0.

## 0.11.0 — keyword parameters (SIR19)

Accepts `Feature::KeywordParams`, building directly on the `_sir_missing`
default-parameter machinery (0.10.0). C has no native keyword calls, so — like
the Go backend's KW6 — a keyword argument is resolved to its callee's parameter
**slot by name at emit time**, producing a plain positional C call:

- A **keyword parameter** needs NO special signature — it is a positional
  `SirValue` C parameter like any other. Only the call site resolves by name.
- A `DirectCall` carrying any `KeywordArg` routes to a dedicated resolver
  (`emit_keyword_call`) instead of the generic left-to-right hoist. For each
  callee slot, in declared order, the filler is: the leading positional argument
  at that index; else the `KeywordArg` naming that parameter; else
  `_sir_missing()` (an omitted optional — the validator guarantees a required
  keyword is never left out, and the same default prologue as `DefaultParams`
  substitutes the default).
- The thread-local signature map — previously just a per-callee arity for
  default padding — now stores each callee's **parameter names** in order, so
  the resolver can place a keyword argument at its slot. (Renamed `ARITY` →
  `SIGNATURES`; `callee_arity` derives the length, `callee_param_names` the
  names. Still read only by key, so emission stays deterministic.)
- Each filler is hoisted into a temp first (matching the statement-oriented
  emitter), so a compound keyword value (`f(b: g(), a: 10)`) is evaluated
  exactly once; the temps are computed in slot order, matching Go's
  declared-order evaluation. The unsupported-builtin pre-check scans a keyword
  argument's value.

Because a `KeywordArg` argument is non-`is_simple`, a keyword-bearing call is
always compound → routed through `emit_keyword_call`, so a `KeywordArg` node
never reaches the generic arg emit or `emit_expr` (where it has no arm). A
`KeywordArg` outside a call is rejected by the validator before emit.

First of the KeywordParams parity arc's C half: with the Ruby backend (0.9.0),
`KeywordParams` is now accepted on five of six backends (the Rust backend is a
separate gap). Verified with hand-built modules compiled and run through a real
`cc`: a keyword argument binding by name, order-independent resolution (`f(b: 2,
a: 10)` → `8` for `f(a:, b:) = a - b`), an optional keyword using its default
when omitted (`f()` → `7` for `f(x: 7)`) and overridden when supplied, a mixed
positional + keyword call, and a compound keyword value hoisted once. Bumps
semantic-ir-to-c 0.10.0 → 0.11.0.

## 0.10.0 — default parameters (SIR19)

Accepts `Feature::DefaultParams`. C has no native default parameters, so — like
the Go backend — this uses a `_sir_missing` sentinel with call-site padding and
a per-function prologue:

- **Runtime**: a new `SIR_MISSING` tag with `_sir_missing()` / `_sir_is_missing`.
  It is an INTERNAL "argument omitted" sentinel — a `SIR_MISSING` value is
  replaced by its default before the body runs, so user code never observes it.
- **Call site**: a `DirectCall` that leaves trailing defaulted arguments off
  pads the call with `_sir_missing()` up to the callee's declared arity. The
  arity is looked up in a thread-local map (`ARITY`) snapshotted at the top of
  `emit_module` — the same mechanism as the `TEMP_ID` counter, so the deep
  `emit_expr`/`emit_assign` call tree reads it without threading a context.
  (The map is only read by key, so emission stays deterministic.)
- **Prologue**: each function opens with `if (_sir_is_missing(p)) { p =
  <default>; }` for every defaulted parameter, in declaration order — so a later
  default may reference an earlier parameter (whose own default is already
  filled), matching the validator and the Go/Ruby backends. A C parameter is a
  mutable lvalue, so it is reassigned in place; a compound default hoists
  through `emit_assign`.

Only the positional case is `DefaultParams`; a keyword default is the separate
(still-unaccepted) `KeywordParams` feature. An `IndirectCall` (a closure with no
statically-known signature) is not padded — the closure's own arity handling
applies; the DirectCall path is the default-parameter path.

Also extends `first_unsupported_builtin` to scan each parameter default, not
just the body — a default is evaluated (in the prologue) at call time, so a
deferred builtin hidden in one must be rejected cleanly rather than reach the
emitter's `unreachable!`. (The C `scan_expr_for_builtin` already scanned an
`IndirectCall`'s target, so — unlike the Ruby backend — no target-scan fix was
needed here.)

This closes the DefaultParams parity arc: with the Ruby backend's default
parameters (0.8.0), `Feature::DefaultParams` is now accepted on all six
backends. Verified with hand-built modules compiled and run through a real `cc`:
a single default used when omitted (`f(1)` → `6` for `f(a, b = 5) = a + b`) and
overridden when supplied (`f(1, 2)` → `3`), two trailing defaults each filling
independently, a default referencing an earlier parameter, and the prologue /
call-site sentinel shape. Bumps semantic-ir-to-c 0.9.0 → 0.10.0.

## 0.9.0 — short-circuit (SIR16)

Accepts `Feature::ShortCircuit`. `Expr::LogicalAnd` / `Expr::LogicalOr`
(`&&` / `||`) reuse the SAME lowering the emitter already applies to the eager
`and`/`or` builtins — no new machinery:

- assign the LEFT operand into the destination, then conditionally OVERWRITE it
  with the right (`dst = lhs; if (_sir_truthy(dst)) { dst = rhs; }` for `&&`,
  `if (!_sir_truthy(dst))` for `||`).

Because the right operand is emitted only inside the `if` body, it is not
evaluated when the left already decides (true short-circuit), and `dst` holds
the DECIDING OPERAND — not a coerced bool. This is the value-returning semantics
Go models with an IIFE and Ruby gets from native `&&`/`||`: `1 && 2` is `2`,
`false && 2` is `false`, `nil || 7` is `7`. It is deliberately NOT lowered to a
bare C `&&`/`||`, which would collapse to an `int` 0/1 and lose the operand.

The nodes are not `is_simple`, so they route through `emit_assign` — and, in
return position, through the existing "compute a compound value into a temp,
then return it" tail fallback — so no other emit arm is needed and the emitter
stays total. The `scan_expr_for_builtin` pre-check recurses into both operands,
so a deferred builtin nested in a `&&`/`||` is still reported cleanly.

This closes the ShortCircuit parity arc: with the Ruby backend's short-circuit
(0.7.0), `Feature::ShortCircuit` is now accepted on all six backends. Verified
with hand-built modules (the frontend constant-folds a literal `&&`) compiled
and run through a real `cc`: operand-return for both operators, a short-circuit
proof where the dead operand is `1 / 0` (which traps if evaluated — a correct
lowering skips it and the program exits 0), and a `LogicalAnd` in tail position.
Bumps semantic-ir-to-c 0.8.0 → 0.9.0.

## 0.8.0 — floats (SIR16)

Accepts `Feature::Floats`. Unlike the sequences and maps batches, this needed
**no new runtime**: `SirValue` has carried a `SIR_FLOAT` tag since v0, and the
runtime already handled floats throughout — `_sir_float` constructor,
`_sir_is_num`/`_sir_as_num`, int→float promotion in `_sir_plus_v`/`_sir_minus_v`/
`_sir_times_v`, an IEEE float path in `_sir_divide_v`, and `_sir_fmt_float`. The
one missing piece was the emitter: a `FloatLit` had no arm and hit
`unreachable!`. `Feature::Floats` gates ONLY `FloatLit`, so this batch is a
single emit arm plus accepting the feature — the emitter stays total.

- `Expr::FloatLit` → `_sir_float(<literal>)` via a new `emit_float_literal`:
  - a **finite** value is spelled with Rust's `{:?}` (Debug) form, whose
    shortest round-tripping text always carries a decimal point or exponent
    (`7.0`, `-0.0`, `1e300`) — a valid C `double` literal that `strtod` parses
    back to the identical bit pattern;
  - a **non-finite** value (which a literal can only carry when hand-built —
    normal arithmetic produces `inf`/`nan` at runtime) uses the C99 `<math.h>`
    macros `INFINITY` / `-INFINITY` / `NAN`, mirroring the Ruby backend's
    `Float::INFINITY` / `Float::NAN`. A single `#include <math.h>` is added to
    the emitted preamble for these (standard, available on every C99 compiler
    including MSVC).

Float arithmetic reuses the existing `+`/`-`/`*`/`/` variadic helpers: an
integral result of a float operation stays a Float (`1.5 + 2.5 == 4.0`, not
`4`), and the division frontier is preserved — a Float operand promotes to true
division (`7.0 / 2 == 3.5`) while two Integers floor (`7 / 2 == 3`); Float
division by zero yields IEEE `Infinity`/`NaN` (no trap — that is Integer-only).
`_sir_fmt_float` renders integral floats with a trailing `.0`, `-0.0` with its
sign, and non-finite values as `Infinity`/`-Infinity`/`NaN`.

This closes the Floats parity arc: with the Ruby backend's floats (0.6.0),
`Feature::Floats` is now accepted on all six backends. Verified with hand-built
modules (the frontend masks `FloatLit`) compiled and run through a real `cc`:
literal display incl. `-0.0`, native arithmetic staying Float, the division
frontier, non-finite results AND non-finite literals, and value-based equality
(`7.0 == 7`). Bumps semantic-ir-to-c 0.7.0 → 0.8.0.

## 0.7.0 — maps (SIR16)

Accepts `Feature::Maps`. `SirValue` gains a `SIR_MAP` tag — a heap-boxed,
insertion-ordered **assoc-array** (`struct SirMap { struct SirMapEntry
*entries; int64_t len; int64_t cap; }`, arena allocated), a shared mutable
handle exactly like `SIR_SEQ`. It is a linear-scan assoc-array, NOT a hash
table — the same representation as the Go (`[]MapEntry`) and Rust
(`Vec<(Value, Value)>`) reference backends: lookups are O(n), but structural
keys and insertion-ordered iteration/printing come for free, with no `Hash`/`Eq`
requirement on the value type. Every construct the feature can surface is
lowered:

- `MapLit` (`{k => v, …}`) → `_sir_map_lit(n, k0, v0, …)`, boxing `n` key/value
  pairs. A later duplicate key overwrites the earlier entry (`{1 => 1, 1 => 2}`
  is `{1 => 2}`), matching Ruby's Hash literal and the Go/Rust `_sir_map_lit`.
- `MapGet` (`h[k]`) → `_sir_map_get`: a missing key yields nil (it does NOT
  raise — matching Ruby's default-less `Hash#[]` and the reference); keys are
  compared by STRUCTURAL equality, so a composite key like `[1, 2]` matches by
  value.
- `MapSet` (`h[k] = v`) → `_sir_map_set`: insert-or-update, mutating the shared
  box so a write through one binding is visible through every alias. A map has
  no bounds, so — unlike `SeqSet` — there is nothing to trap on; a new key
  APPENDS (growing the backing array, capacity doubling from 4), preserving
  insertion order.

`_sir_value_eq` gains a `SIR_MAP` arm: STRUCTURAL and POSITIONAL — equal length,
then entry-wise in insertion order (`entries[i]` key AND value equal) — exactly
mirroring the Go (`[]MapEntry` zip) and Rust (`iter().zip()`) backends, with an
identical-handle fast path. `_sir_fmt` renders a map as `{k: v, k2: v2}` (brace,
colon-space, insertion order), also matching Go/Rust.

**Documented family-wide divergence from real Ruby (unchanged by this batch):**
Ruby's own `Hash#==` is order-INsensitive and its `Hash#inspect` uses ` => ` for
non-symbol keys (and `key:` only for symbol keys). All three source-emitting
backends (Go, Rust, and now C) are instead positional and print a uniform `: ` —
so the three **agree with each other**, which is the property the cross-backend
conformance corpus checks (no corpus program prints or reorder-compares a whole
map, so the real-Ruby form is unexercised). Aligning all three to Ruby's exact
`Hash` semantics is a separate, family-wide change.

Because `MapSet` mutates in place, a self-referential map (`m[k] = m`) is now
constructible; both the `value_eq` and `fmt` `SIR_MAP` arms reuse the
recursion-depth caps introduced for `SeqSet` in 0.6.0, so a cyclic map
terminates rather than overflowing the C stack (verified adversarially).

`ForEach` over a map is deliberately NOT special-cased: iterating a map is
reference-undefined (Go's `_sir_seq_iter` panics on a non-sequence), and C's
lenient `_sir_seq_iter` else-branch already treats a non-seq/non-cons iterable
as an empty iteration — so the loop body runs zero times and the emitter stays
total (no new `unreachable!`), consistent with its pre-existing handling of any
other non-iterable.

Every node verified by hand-built modules (bypassing the frontend, which does
not yet produce these) compiled and run through a real `cc` — covering present/
missing-key reads, insert/update/alias writes, structural composite keys,
duplicate-key overwrite, positional structural equality, brace-list display, the
zero-iteration `ForEach`-over-map, and the cyclic-map stack-safety guard.

## 0.6.0 — sequences (SIR16)

Accepts `Feature::Sequences`. `SirValue` gains a `SIR_SEQ` tag — a heap-boxed
dynamic array (`struct SirSeq { SirValue *items; int64_t len; }`, arena
allocated like every other heap value) — so a sequence is a shared, mutable
handle: a `SeqSet` through one binding is visible through every alias, matching
the Go/Rust `*Seq`. Every construct the feature can surface is lowered:

- `SeqLit` (`[1, 2, 3]`) → `_sir_seq_lit(n, …)`.
- `SeqIndex` (`a[i]`) → `_sir_seq_index`: a negative index counts from the end,
  an out-of-range index yields nil (it does NOT raise — matching the reference
  and every other backend).
- `SeqLen` (`a.length`) → `_sir_seq_len`.
- `SeqSet` (`a[i] = v`) → `_sir_seq_set`, which TRAPS (`stderr` + `exit(1)`) on
  a negative or out-of-range index, matching the Go/Rust `panic`.
- `ForEach` (`for x in a`) → a `for` loop over `_sir_seq_iter(a)`, which
  snapshots the iterable (a real sequence is copied so a mutating body does not
  disturb iteration; a cons-list is flattened). `x` is declared inside the loop
  body block, so it is block-scoped — matching the validator's rewind and Go's
  `:=` counter. This is why `ForEach` is no longer rejected by the `first_foreach`
  pre-pass added in 0.5.0 (that pre-pass and its clean-rejection are removed).

`_sir_value_eq` gains a structural `SIR_SEQ` arm — equal length, element-wise
equal, with an identical-handle fast path (which also short-circuits the common
self-referential `a == a`). `_sir_fmt` renders a sequence as `[1, 2, 3]`
(bracket, comma-space), matching the Go/Rust backends. With this, the
cross-backend composite-equality conformance (`[1,2] == [1,2]`) now asserts on
**all six** backends — C was the last that skipped it.

Because `SeqSet` is the first MUTABLE heap aggregate (cons pairs are immutable
and so cannot form a cycle), a self-referential sequence (`a[0] = a`) is now
constructible; both `_sir_value_eq` and `_sir_fmt` carry a recursion-depth cap
so a cyclic structure terminates rather than overflowing the C stack — a guard
the immutable pair path never needed. (Found by security review, which also
caught that the earlier "matches the pair arm" claim was wrong.)

Every node is verified by hand-built modules (producer-agnostic), compiled with
a real `cc` under `-Werror=unused-variable` and run: display, structural
equality (positive/negative/nested), index (in-range/negative/OOB), length,
in-bounds set, and block-scoped ForEach.

## 0.5.0 — `ForRange` (numeric for-loop) + a scan hole (SIR16)

Fixes a **pre-existing panic**: `Stmt::ForRange` (`for i in 0...3`) is gated by
`Feature::Loops` alone (accepted since 0.4.0), so a producer emitting a numeric
for-loop reached the emitter — which sent it to `unreachable!`. It now lowers to
a native `int64_t` counter loop mirroring the Go/Rust backends byte-for-byte:

- `start`/`stop`/`step` are evaluated ONCE (they may have side effects) into
  `SirValue` temporaries, then reduced to `int64_t` via the new `_sir_as_int`
  runtime helper (a truncating integer view — a float bound truncates toward
  zero).
- the stop is EXCLUSIVE and the direction follows the step's sign
  (`step >= 0 ? i < stop : i > stop`), so a descending loop with a negative step
  works — matching Go's `_sir_range_cont`.
- the loop `var` is declared INSIDE the loop body block, so it (and any
  body-local) is block-scoped — matching the validator (which rewinds the loop
  body) and Go's `:=` counter, never clobbering an enclosing same-named local.
  The outer `{…}` scopes the counter temporaries (nesting-safe via `fresh_id`).

Also closes a **pre-existing scan hole** (same class): the unsupported-builtin
pre-check (`scan_block_for_builtin`) did not recurse into `While` or `ForRange`
bodies, so an unknown builtin hidden in a loop body escaped the clean rejection
and hit the emitter's `unreachable!`. It now scans both; such input rejects
cleanly with a `BackendError` instead of panicking.

Makes the emitter TOTAL for its accepted feature set. `ForEach` also observes
only `Feature::Loops` (not gated out), so it was likewise a latent
`unreachable!` — `compile` now rejects it CLEANLY via a `first_foreach`
pre-pass (a clear `UnsupportedFeature` error) until the sequences batch gives it
an iterator, rather than panicking. The sequence nodes stay rejected at the
feature gate — a follow-up adds `Feature::Sequences` (a real `SIR_SEQ`
runtime).

## 0.4.0 — control flow, mutation & the rest of the comparisons (SIR16)

Accepts `Feature::Loops` and `Feature::MutableBindings`, and:

- Renders `Stmt::While` as a portable `for (;;) { SirValue c; c = <cond>; if
  (!_sir_truthy(c)) break; <body> }` — the condition is re-evaluated each
  iteration, so it may be compound.
- Renders `Stmt::Assign` (re-binding an already-declared `SirValue`).
- Adds the missing comparison builtins `<=`, `>=`, `==`, `!=` (runtime helpers
  `_sir_le`/`_sir_ge`/`_sir_ne`; previously only `<`/`>`/`=` were lowered, so a
  `<=` reached `_sir_unknown_builtin` and failed).
- **Portability fix:** user functions named `min`/`max` are now escaped (trailing
  `_`).  `<stdlib.h>` on MSVC/UCRT defines `min`/`max` as function-like macros,
  so `SirValue min(SirValue a, SirValue b)` expanded to garbage under clang-cl /
  MSVC — now they compile on all three compilers.

## 0.3.0 — lower unary minus (`neg` builtin) — negative literals no longer skip

Ruby lowers unary minus (`-x`) to `BuiltinCall("neg", [x])`, but the v0 C
emitter had no lowering for `neg`, so `first_unsupported_builtin` rejected it and
the whole program was reported `UnsupportedFeature` (i.e. **skipped**) — meaning
ANY negative literal, not just division, was unrunnable on the C backend.

Unary minus IS single-argument subtraction, and the runtime's `_sir_minus_v`
already negates a single argument tag-preservingly (a `SIR_FLOAT` stays float,
otherwise int). So `neg` now lowers to `_sir_minus(1, x)` via `variadic_helper`
— no new runtime code — matching the Go/Rust/Python runtimes that gained `neg`
in SIR21 §E3. New `unary_minus` exec-proof in `tests/compile_and_run.rs`
(`puts(-7)` → `-7`, `puts(-7 / 2)` → `-4` floored, `puts(-(3 * 2))` → `-6`),
compiled and run through a real C compiler.

This closes the **C arm** of the division frontier: with the runtime already
flooring (`_sir_ifloordiv`), C now reproduces Ruby's floor `/` on negative
dividends too, so `sir-conformance`'s `division_matches_ruby_floor_on_every_backend`
asserts (rather than skips) C's negative cases.

## 0.2.0 — render SIR26 integer conversions

Accepts `Feature::Conversions` (plus the SIR21 type-implied `SizedIntegers`,
`Unsigned`, `WrappingArithmetic`) and renders `Expr::Convert`, so C→SIR→C
round-trips a source language's integer width/wrapping/truncating semantics.

- A conversion emits the portable runtime helper `_sir_convert(v, bits, signed)`
  (with `_sir_mask_to` doing a two's-complement reduction over `int64`/`uint64`
  — mask then sign-fold — no reliance on native fixed-width casts, so it behaves
  identically on MSVC/GCC/Clang).  A target width of `Arbitrary` is the identity
  and emits no wrapper.  `bits >= 64` is the `int64` storage floor (u64 above
  2^63 is the documented bignum frontier, shared with the Go/Rust backends).
- Verified on **clang, gcc, and MSVC**: `(uint8_t)300==44`, `(int8_t)200==-56`,
  `(uint16_t)70000==4464`, `(uint32_t)-1==4294967295`,
  `(int32_t)4e9==-294967296`, arbitrary-width identity.

## 0.1.0 — v0 core (SIR24)

First release of the sixth SIR backend: lowers a `semantic_ir::Module` to a
**self-contained ISO C99 source file** compilable on MSVC (`/std:c11`), GCC, and
Clang.  Gives **Ruby → C** (and Python/JS/Twig → C) through the shared
narrow-waist IR.

### Added

- `compile(&Module) -> Result<Artifact, BackendError>` and `CBackend`
  implementing `semantic_ir::Backend` with `target_tag() == "c"`.
- **Capability set (v0):** `Closures`, `Pairs`, `Symbols`, `Strings`,
  `DynamicTyping`, `OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.
  Rejects `TailCalls`, `Intrinsics`, and every later feature (including
  `Bignum`) cleanly rather than mis-compiling.
- **Inlined C runtime** (`runtime.rs`) — a tagged-union `SirValue`
  (nil/bool/int64/float/str/sym/pair/closure), arena/leak-on-exit memory, symbol
  interning, SIR truthiness (false/nil-only), polymorphic `+ - * / < > =` (string
  concat on `+`, int-floor vs float-true division), structural equality,
  `cons`/`car`/`cdr` and type predicates, closures (`make_closure`/`apply`), a
  string-keyed global store, and Ruby/Lisp-aware `print`/`puts` display.  Runtime
  functions use external linkage so the fully-inlined runtime never trips
  `-Wunused-function` on a small program.
- **Emitter** (`emit.rs`) — statement-oriented lowering (`emit_tail` /
  `emit_assign`) so an `if`/block produces a value without any
  statement-expression; variadic builtins via C variadic functions; closure
  thunks; identifier sanitisation (`sanitize_ident`) and C string/comment
  escaping; deterministic (byte-stable) output.
- **Portability:** `#define _CRT_SECURE_NO_WARNINGS`, `snprintf` (no `sprintf`/
  `strcat`), no compiler-specific extensions — verified building and running on
  MSVC, GCC, and Clang.
- **Injection hardening:** string/symbol literals escape `?` as `\?` so a
  source `??/` cannot expand (via C trigraphs under `-std=c99`) into a `\` that
  breaks out of the emitted C literal; `_sir_builtin_dispatch` reads arguments
  through a bounds-checked `_sir_arg` so an under-applied builtin-as-value reads
  `nil` rather than indexing out of bounds.
- **Tests:** `tests/emit.rs` (emit-shape, determinism, sanitisation, capability
  rejection — no compiler needed) and `tests/compile_and_run.rs` (compiles and
  runs each corpus program through a discovered `cc`/`clang`/`gcc`, skipping when
  none is present).  Corpus covers arithmetic, method calls, tail-`if`,
  sequential assignment, string concat, and Twig closures.
- `examples/dump_c.rs` — dump the emitted C for a Ruby/Twig snippet.
- README documenting the design, portability contract, and roadmap to parity.
