# Changelog

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
