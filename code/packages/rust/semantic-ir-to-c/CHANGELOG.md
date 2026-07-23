# Changelog

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
