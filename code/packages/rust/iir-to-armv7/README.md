# iir-to-armv7

> ⚠ **DEPRECATED as of v0.5.0**.  Use [`armv7-backend`](../armv7-backend)
> instead.  Migration plan:
> [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
>
> Note: `armv7-backend` v0.1.0 is a **minimal-viable** port — only
> `const_*` + `ret_*` are covered.  The full op set
> (add/sub/cmp/branches/calls) that this crate had can be ported
> to `armv7-backend` in future increments.

**IIR → ARMv7 (A32) machine code backend.**

Lowers an `IIRModule` from the [interpreter-ir](../interpreter-ir/)
shared IR to a `Vec<u32>` of encoded 32-bit ARMv7-A instructions.

| | |
|---|---|
| **Version** | 0.1.0 — skeleton (A3) |
| **Plan** | [`MULTILANG-ARCHITECTURE-BACKENDS.md`](../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md) §A3 |
| **Spec** | [`iir-to-armv7.md`](../../../specs/iir-to-armv7.md) |
| **Sibling backends** | [`iir-to-riscv`](../iir-to-riscv/), [`iir-to-intel8008`](../iir-to-intel8008/), [`iir-to-llvm`](../iir-to-llvm/), [`iir-to-wasm`](../iir-to-wasm/) |
| **Downstream consumers** | [`arm-simulator`](../arm-simulator/), `qemu-arm`, `objcopy` + a phone-class Linux linker |

## Why an ARMv7 backend?

ARMv7 (32-bit ARM, A32 encoding) is the **phone-class target** of
the LANG VM architecture-backend lane.  It covers Cortex-A7/A8/A9-era
SoCs and many embedded boards (early Raspberry Pi, BeagleBone, Olimex
A20-OLinuXino).

Adding ARMv7 gives the LANG VM matrix a third architecture backend:

| | RV32I (A1) | Intel 8008 (A2) | **ARMv7 (A3)** |
|---|---|---|---|
| Width | 32-bit | 8-bit | 32-bit |
| Style | Clean RISC | Irregular CISC | RISC + cond prefixes + barrel shifter |
| Decoding | Fixed 32-bit words | 1/2/3-byte variable | Fixed 32-bit words |
| Special twist | Compressed `C` extension | 14-bit address bus | `cond` field on every instruction |
| Deployed silicon | Embedded controllers | Historical (Oct) | Billions of phones / IoT |

All three speak different architectural dialects of "what a backend
must care about", which is the point — exercising the IIR's
neutrality across genuinely different ISAs.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_armv7::{validate_for_armv7, lower_iir_to_armv7, IIRArmv7Config};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_armv7(&module).is_empty());

let words = lower_iir_to_armv7(&module, &IIRArmv7Config::default())
    .expect("lowering should succeed");

// v0.1.0 emits a single BKPT #0xFFFF.
assert_eq!(words, vec![0xE12F_FF7F]);
```

## Public API (v0.1.0)

```rust
pub struct IIRArmv7Config { pub module_name: String }
impl IIRArmv7Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRArmv7Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_armv7(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_armv7(
    module: &IIRModule,
    cfg: &IIRArmv7Config,
) -> Result<Vec<u32>, IIRArmv7Error>;

pub const BKPT: u32 = 0xE12FFF7F;
```

## Why `Vec<u32>` output, not textual asm?

* **Round-trips with `arm-simulator`** — its decoder consumes raw
  little-endian 32-bit words.
* **Deterministic test surface** — `assert_eq!(words[0], 0xE12FFF7F)`
  is unambiguous; ARM assembler syntax has GNU `as`, LLVM `clang`,
  and ARMASM divergence.
* **Trivial encoding shape** — every A32 instruction is exactly 4
  bytes (no 1/2/3-byte variability like the 8008).

## The `BKPT` encoding (v0.1.0)

`BKPT #0xFFFF` = `0xE12FFF7F`.  Bit layout:

```text
31..28  cond    = 0xE = 1110            (always — unconditional)
27..20          = 0001 0010 = 0x12      (BKPT opcode family)
19.. 8  imm12   = 0xFFF                 (top 12 bits of imm16)
 7.. 4          = 0111 = 0x7            (BKPT opcode family)
 3.. 0  imm4    = 0xF                   (bottom 4 bits of imm16)
```

### Why BKPT and not WFI or `b .`?

| Candidate | Pros | Cons |
|-----------|------|------|
| `BKPT #imm16` | Semantically "stop"; every ARM debugger / emulator recognises it | None for skeleton purposes |
| `WFI`         | True halt | Requires kernel/hypervisor privilege; illegal in userspace |
| `B .`         | Pure userspace, no traps | Burns CPU; harder to detect without a host timeout |

BKPT wins on simplicity + emulator round-trip.  The
`arm-simulator`'s decoder flags it as `bkpt` and stops single-stepping.

## What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A3+) lowers `const` + `bx lr`.
* **No `lang-aot --emit=armv7`.**  Deferred to A3+++.
* **No external assembler / linker integration.**

## Tests

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_word`
* `lower_emits_the_canonical_bkpt_word` (exact `0xE12FFF7F`)
* `bkpt_constant_pinned_to_e12fff7f`
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

Run with `cargo test -p iir-to-armv7`.

## License

Same as the parent repository.
