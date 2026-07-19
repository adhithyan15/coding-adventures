# Changelog

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
