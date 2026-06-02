# Changelog — iir-to-riscv

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-01 (A1 — crate skeleton)

### Added — `ret`-only emission

First release.  Implements item A1 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single RV32I `ret` instruction
(`jalr x0, x1, 0`, encoded as `0x0000_8067`).

#### Public surface

```rust
pub struct IIRRiscvConfig { pub module_name: String }
impl IIRRiscvConfig {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRRiscvError {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_riscv(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_riscv(
    module: &IIRModule,
    cfg: &IIRRiscvConfig,
) -> Result<Vec<u32>, IIRRiscvError>;
```

#### Why an architecture backend?

The wasm / JVM / CLR / BEAM / LLVM backends all target *software*
runtimes that own register allocation and instruction selection.
RISC-V is the first **architecture** backend: output is real hardware
ISA, decodeable by the in-tree `riscv-simulator` (RV32I + M-mode
traps), QEMU, or a physical SiFive / Espressif RISC-V chip.

Strategic priority: RISC-V is the most open of the architecture
candidates (royalty-free spec, broad simulator availability, growing
hardware footprint).  A2-A5 (Intel 8008, ARMv7, Intel 4004, GE-225)
follow the same shape once A1's lessons are baked in.

#### Why `Vec<u32>` output, not textual assembly?

* **Round-trips with `riscv-simulator`** — it consumes raw 32-bit words.
* **Deterministic test surface** — `assert!(words[0] == 0x0000_8067)`
  is unambiguous; assembly syntax has GNU vs LLVM divergence.
* **No textual-format coupling.**

A textual `.s` emitter can be added as a sibling later without
breaking callers.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A1+) lowers function
  entry/exit prologue/epilogue + arithmetic + cmp + control flow.
* **No `lang-aot --target=riscv32`.**  Deferred to v0.4.0 (A1+++).
* **No external linker integration.**  Output is raw words; downstream
  linkers / loaders are the caller's responsibility.

#### Tests added (6 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_ret_word` (exact `0x0000_8067`)
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
