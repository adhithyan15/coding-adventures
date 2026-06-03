# iir-to-intel4004

> ⚠ **DEPRECATED as of v0.4.0**.  Use [`intel4004-backend`](../intel4004-backend)
> instead — it implements `jit_core::backend::Backend` over CIR.
> Migration plan:
> [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
>
> The crate still compiles and all existing callers continue to
> work (each `pub fn` is marked `#[deprecated]` with a pointer to
> the replacement).  `lang-aot --emit=intel4004` already routes
> through `intel4004-backend` as of Phase 4.

**IIR → Intel 4004 machine code backend.**

Lowers an `IIRModule` from the [interpreter-ir](../interpreter-ir/)
shared IR to a `Vec<u8>` of encoded 8-bit Intel 4004 opcodes.

| | |
|---|---|
| **Version** | 0.1.0 — skeleton (A4) |
| **Plan** | [`MULTILANG-ARCHITECTURE-BACKENDS.md`](../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md) §A4 |
| **Spec** | [`iir-to-intel4004.md`](../../../specs/iir-to-intel4004.md) |
| **Sibling backends** | [`iir-to-intel8008`](../iir-to-intel8008/), [`iir-to-riscv`](../iir-to-riscv/), [`iir-to-armv7`](../iir-to-armv7/), [`iir-to-llvm`](../iir-to-llvm/), [`iir-to-wasm`](../iir-to-wasm/) |
| **Downstream consumers** | Any in-tree 4004 simulator, `intel-4004-assembler` for round-trip, EPROM burners |

## Why an Intel 4004 backend?

The Intel 4004 (1971) was the **world's first commercial
microprocessor**.  Tiny ISA, 4-bit data, 12-bit ROM addresses,
single 4-bit accumulator, 16 4-bit registers organised as 8 register
pairs, tiny ROM (4 KiB max) and RAM (640 bits max).

In this codebase the 4004 is primarily a **Brainfuck fit** — BF's
minimal needs map cleanly to a 4004's accumulator-and-loop programming
model.

Adding the 4004 gives the LANG VM backend matrix a fourth
architecture:

| | RV32I (A1) | Intel 8008 (A2) | ARMv7 (A3) | **Intel 4004 (A4)** |
|---|---|---|---|---|
| Width | 32-bit | 8-bit | 32-bit | **4-bit** |
| ROM size | huge | 16 KiB | huge | **4 KiB** |
| Registers | 31 GP | 7 GP | 13 GP | **8 register pairs** |
| Mnemonic style | Modern RISC | Irregular CISC | RISC + cond prefixes | Tiny MCS-4 |
| Year first shipped | 2015 | 1972 | 2005 | **1971** |

The 4004 is the most constrained target in the lane by a wide margin.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_intel4004::{validate_for_intel4004, lower_iir_to_intel4004, IIRIntel4004Config};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_intel4004(&module).is_empty());

let bytes = lower_iir_to_intel4004(&module, &IIRIntel4004Config::default())
    .expect("lowering should succeed");

// v0.1.0 emits the canonical 2-byte halt sentinel.
assert_eq!(bytes, vec![0x40, 0x00]);
```

## Public API (v0.1.0)

```rust
pub struct IIRIntel4004Config { pub module_name: String }
impl IIRIntel4004Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRIntel4004Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_intel4004(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_intel4004(
    module: &IIRModule,
    cfg: &IIRIntel4004Config,
) -> Result<Vec<u8>, IIRIntel4004Error>;

pub const HALT_LOOP: [u8; 2] = [0x40, 0x00];
```

## The halt sentinel: `JUN 0x000`

The 4004 has no formal `HLT` instruction.  The canonical "halt"
idiom in 4004 ROM development is `JUN 0x000` — an unconditional
jump back to ROM address 0, which (when this instruction is itself
at address 0) loops forever, simulating halt.

Bit layout (JUN = `0100 aaaa aaaaaaaa`):

```text
byte 1: 0100 0000 = 0x40   (JUN opcode + high nibble of 12-bit addr = 0)
byte 2: 0000 0000 = 0x00   (low byte of address = 0)
```

### Why JUN-self over NOP-cycle or unimplemented-opcode?

| Candidate | Pros | Cons |
|-----------|------|------|
| `JUN 0x000` (this) | Self-documenting; portable across all 4004 implementations | None for skeleton purposes |
| `NOP NOP NOP ...` (0x00 cycle) | Even simpler bytes | Doesn't halt — keeps running into whatever follows |
| Unimplemented opcode | Forces a trap | 4004 silicon executes most "unused" bit patterns as NOPs; not portable |

## What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A4+) lowers `LDM` (load
  immediate) + `ret`/`ret_void`.
* **No `lang-aot --emit=intel4004`.**  Deferred to A4+++.
* **No external assembler / linker integration.**

## Tests

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_two_bytes`
* `lower_emits_the_canonical_jun_self_bytes` (exact `[0x40, 0x00]`)
* `halt_loop_constant_pinned_to_40_00`
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

Run with `cargo test -p iir-to-intel4004`.

## License

Same as the parent repository.
