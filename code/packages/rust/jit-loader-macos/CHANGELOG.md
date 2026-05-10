# Changelog — `jit-loader-macos`

## 0.1.0 — 2026-05-05

Initial release.  Installs runtime-generated machine code into
executable memory on Apple Silicon, completing the in-process JIT
path for the LANG-runtime stack.

### What it does

- Allocates a 16-KiB `MAP_JIT` page via `mmap` (no entitlements
  required for local development).
- Toggles the per-thread `pthread_jit_write_protect_np` flag to
  W-then-X to honor Apple's hardened-runtime W^X requirement.
- Flushes the I-cache with `sys_icache_invalidate` so the CPU sees
  the freshly-written bytes.
- Exposes `CodePage::as_function::<F>()` to transmute the entry
  pointer into a typed `extern "C"` function pointer.

### Test coverage

- 8 unit tests cover construction, hand-encoded ARM64 round-trip
  (`return 42`, `add x0, x0, x1`), repeated invocation, page
  alignment, and `Drop` releasing memory under load (1 000 churn).
- **6 integration tests** in `tests/jit_e2e.rs` drive the full
  pipeline end-to-end: hand-built IIR → `aot-core::specialise` →
  `aarch64-backend` → `jit-loader-macos` → in-process call.  One
  exercises real **Nib source** (`fn add3(a: u4, b: u4) -> u4 { ... }`)
  through `nib-iir-compiler` and JITs it.

### Out of scope (deferred)

- Linux ARM64 / x86-64 JIT loaders (different mmap dance).
- Concurrent JIT writes from multiple threads (each thread would
  need its own W^X flip).
- Profile-driven hot-path detection — that lives in `jit-compiler`
  and `jit-core`; the loader is the bottom-of-stack primitive.

### What this unlocks

Pair this with `jit-core`'s `Backend::run` and twig-vm's profiler
hooks, and the interpreter can transparently dispatch hot functions
to native code without restarting the program.  That wiring is the
next piece.
