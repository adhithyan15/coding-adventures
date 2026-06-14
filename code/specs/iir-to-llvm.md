# iir-to-llvm — IIR → textual LLVM IR backend

**Status:** v0.1.0 — skeleton (LLVM01)
**Plan:** [`MULTILANG-BACKEND-PLAN.md`](MULTILANG-BACKEND-PLAN.md) §LLVM
**Related:** [`iir-to-wasm`][wasm], [`iir-to-jvm-class-file`][jvm], [`iir-to-cil-bytecode`][clr]

[wasm]: ../packages/rust/iir-to-wasm/
[jvm]: ../packages/rust/iir-to-jvm-class-file/
[clr]: ../packages/rust/iir-to-cil-bytecode/

## Why a new crate?

The four existing IIR backends (wasm / JVM / CLR / BEAM) all target *managed*
runtimes that own register allocation, memory layout, GC, and exception
handling.  LLVM is a different beast: an **AOT-native** target whose output
runs on the bare metal of whatever the user's CPU is, with the user's choice
of LLVM-quality optimizer in front of it.  Adding LLVM as a backend gives us:

1. **A second AOT path** alongside our hand-rolled aarch64/x86_64 emitters.
   The hand-rolled emitters are the right call when we want full control of
   the encoding (e.g. for the debugger story); LLVM is the right call when
   we want world-class O2 optimization for free.
2. **A direct comparison axis.**  Same IIR, two AOT-native code generators —
   what does each do well, what does each do poorly?
3. **A bridge to every CPU LLVM ships a backend for** without writing per-CPU
   encoders (Apple Silicon, x86_64, RISC-V, MIPS, PowerPC, …).

## Why *textual* LLVM IR, not `llvm-sys`?

`llvm-sys` requires a native LLVM install on the build machine and pins us to
a specific LLVM major version per the bindgen-style API.  For a hobby /
educational codebase the textual `.ll` output is strictly better:

- **Zero build-time dep.**  We emit a string; that's it.  CI doesn't need
  LLVM installed; doctests still work; `cargo install` ships a tiny crate.
- **Debuggability.**  The output IS the human-readable form.  No FFI ABI
  drift, no opaque builder API — just strings we can `assert!` on in tests.
- **Forward-compat with llvm-sys.**  If we ever want JIT execution or want
  to skip the textual round-trip, we can add a second emitter alongside the
  textual one without breaking existing callers.

Trade-off: textual `.ll` is ~10× slower to ingest than bitcode for the
optimizer.  For a hobby codebase compiling small modules that's irrelevant.

## Pipeline

```text
IIRModule
  → validate_for_llvm()     pre-flight, returns Vec<String>
  → lower_iir_to_llvm()     two-pass, returns String (the .ll source)
  → (optional) llc / opt    user runs these — out of scope for this crate
  → object file → linker → native executable
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (LLVM01) | crate skeleton: empty module header with `target triple` + `; ModuleID = …` comment. No instruction lowering yet. | **merged** |
| v0.2.0 (LLVM02) | function signatures + `ret`, `ret_void`, `const`, `mov` | **merged** |
| v0.3.0 (LLVM03) | typed arithmetic + cmp + branches | **merged** |
| **v0.4.0 (LLVM04 — this PR)** | `call` + `call_builtin` print_i64 → `declare/call @__print_i64(i64)` + `lang-aot --emit=llvm-ir` | this PR |
| (later) | memory ops, GC, debug info via `!dbg` metadata | future |

## Public surface (v0.1.0)

```rust
pub struct IIRLlvmConfig {
    pub module_name: String,
    /// LLVM target triple — defaults to the host triple at build time, but
    /// callers can override (e.g. `"riscv32-unknown-elf"`).
    pub target_triple: String,
}

pub enum IIRLlvmError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_llvm(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_llvm(
    module: &IIRModule,
    cfg: &IIRLlvmConfig,
) -> Result<String, IIRLlvmError>;
```

## Output shape (v0.1.0)

```llvm
; ModuleID = '<module_name>'
target triple = "<target_triple>"

; (function bodies emitted in later versions)
```

Every emitted module must satisfy the test
`output_starts_with_comment_or_target` — the first non-blank line begins
with either `;` (comment) or `target` (LLVM directive).  This is the LLVM01
acceptance criterion from `MULTILANG-BACKEND-PLAN.md` §LLVM01.

## Why a default `target_triple`?

We default to a fixed host triple string (`"x86_64-unknown-linux-gnu"`) so
that emitting and `assert!`-ing on the output is deterministic across
machines.  Tests and CI use the default; callers that care about actually
running the `.ll` can override via `IIRLlvmConfig::with_target(...)`.

Picking a host-derived triple at *build* time would make tests host-dependent
and is a footgun for cross-compilation — better to make the override explicit.

## Non-goals (v0.1.0)

- No instruction lowering (deferred to LLVM02+).
- No `lang-aot --backend=llvm` wiring (deferred to LLVM04).
- No `opt` / `llc` invocation; this crate is a pure emitter.
- No SSA construction beyond what `IIRInstr` already provides; we trust the
  upstream renaming pass.
- No GC / exception unwinding / debug metadata.

## Tests (v0.1.0)

- `validate_returns_empty_for_empty_module` — stub validator behaves.
- `output_contains_module_id_comment` — header line present.
- `output_contains_target_triple` — `target triple = "…"` line present.
- `output_starts_with_comment_or_target` — LLVM01 acceptance criterion.
- `default_config_has_nonempty_triple` — config invariant.

Test count grows with each version.
