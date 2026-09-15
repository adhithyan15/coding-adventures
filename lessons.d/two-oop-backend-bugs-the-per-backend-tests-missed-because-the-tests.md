# Two OOP backend bugs the per-backend tests missed because the tests sidestepped the exercised path

The `sir-conformance` matrix (real Ruby source → every backend) caught two latent
backend bugs that every per-backend unit test passed over — both because the unit
test happened to avoid the exact construct that breaks:

1. **Ruby backend: `Foo.new` never ran `initialize`.** A `def initialize` is
   registered like every method under the reserved `sir_um_` prefix as
   `sir_um_initialize` — a name Ruby's own `Class#new` never calls. The `__new__`
   emitter emitted a native `Foo.new`, so the constructor body (its `@ivar`
   initialisers) never ran; `@n` stayed nil and `@n + 1` raised. The two existing
   ivar e2e tests used an explicit `start`/`set` method (`c = Counter.new; c.start;
   c.inc`), calling the initialiser BY HAND — so they never exercised construction-
   time init. Fix: `__new__` → a `sir_new` runtime helper that `allocate`s then
   invokes `sir_um_initialize` (mirroring Go/C/Rust). semantic-ir-to-ruby 0.18.1.

2. **C backend: `raise ArgumentError, "boom"` looked up a nonexistent constant.**
   The frontend lowers it to `raise(VarRef(Const "ArgumentError"), "boom")`; the C
   `raise` emitter used only arg 0 and let the `Const` fall through to
   `_sir_const_get("ArgumentError")` — but the C runtime registers no builtin
   exception-class CONSTANTS, so it raised `NameError: uninitialized constant
   ArgumentError` (and dropped the message). Every existing C exception test raised
   a bare STRING (`raise "boom"` → RuntimeError), never a named class — so the
   class-name path was never emitted. Fix: intercept a `Const` first arg as a class
   name → `_sir_raise(_sir_error("ArgumentError", <msg>))`, on BOTH the simple and
   the compound (non-simple message) emit paths. semantic-ir-to-c 0.17.1.

**The pattern:** a unit test that reaches the same *observable outcome* by a
different *code path* (hand-calling the initialiser; raising a string not a class)
gives false confidence. When a backend has two ways to express a feature, cover the
one the FRONTEND actually emits — a real-source conformance oracle is what surfaces
the gap. (Also: both were pre-existing failures on green main, each in a DIFFERENT
subsystem; when the whole suite is red, don't assume one root cause — bisect each
failing `(program, backend)` cell independently.)

**Third occurrence (moslayout-compiler), with a new symptom.** Inserting a test before
an existing one splits that test's `#[test]` from its `fn` too, giving:

```rust
/// G3-4: Index access …
#[test]                      // <- orphaned from its fn
/// A string literal …
#[test]
fn my_new_test() { … }       // <- now carries TWO #[test] attributes
```

`cargo test` still passes — it just silently runs `my_new_test` **twice** (the count
went 82 → 86 instead of 85, which is the tell). Only `clippy -D warnings` fails, as
`duplicate_macro_attributes`. Same root cause, so the same rule applies and is worth
restating as a hard habit: **anchor an insertion on the previous item's closing brace,
never on the next item's signature** — attributes and doc comments both live above the
signature, and anchoring there always cuts through them.
