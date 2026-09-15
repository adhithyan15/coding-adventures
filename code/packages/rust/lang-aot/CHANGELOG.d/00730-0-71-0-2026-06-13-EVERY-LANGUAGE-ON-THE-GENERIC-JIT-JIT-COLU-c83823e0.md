## 0.71.0 — 2026-06-13 — Every language on the generic JIT — JIT COLUMN COMPLETE → MATRIX COMPLETE (LANG-MATRIX Phase I)

Completes the **JIT column** — the last open column of the platform matrix. All six
matrix languages now run on the **generic JIT**, so every language runs on every backend
except BEAM, **verified by running**.

`tests/lang_matrix.rs` gains a `Backend::Jit` runner: source →
`compile_source_to_iir` (the *same* shared pipeline every other column uses) →
`jit_core::JITCore` driving the language-agnostic `GenericCirJit` over the shared IIR.
`execute_with_jit` eagerly compiles each fully-typed function to JIT bytecode (installing
a native handler) and interprets the rest on `VMCore`, so each program runs *through the
JIT pipeline*. The I/O builtins (`print_i64`/`putchar`/`getchar`) are registered as
callbacks on **both** the VM (interpreter-fallback tier) and the `GenericCirJit` backend
(compiled tier) — **no per-language code**, exactly the way a future Ruby/JS frontend
would plug in. Verified by RUNNING in-process: Twig→42, Nib→42, Oct→0, ALGOL `17 mod 5`→2,
BASIC `10 PRINT 42`→stdout `42`, Brainfuck `++++++++[…]>+.`→`A`. The matrix's floor test
asserts every JIT-tagged cell actually runs.

The Nib cell (`double(21)`) surfaced — and this slice fixes, in `jit-core` 0.4.0 — a
genuine generic-JIT gap: `GenericCirJit::run` ignored its `args`, so a JIT-compiled
function read its parameters as zero. See that crate's changelog; the upshot is any
frontend whose functions take arguments now JITs correctly.

### Also — fix the pre-existing `cil_emit.rs` test

`tests/cil_emit.rs` (CLR-emit, McCarthy W6a) no longer compiled: the `clr-simulator`
evaluation stack became `Vec<Option<Value>>` in W6b (#5296), but this scalar test — which
predates that change and only ever ran via its own (dev-dependency) edge — still expected a
bare `i32`. It now matches `Value::Int(n)`. Unrelated to the JIT work, but `cil_emit` is a
`lang-aot` test, so this slice (which touches `lang-aot`) restores it to green.

