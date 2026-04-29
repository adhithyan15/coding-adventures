# intel-8008-ir-validator

Hardware-constraint validation for `IrProgram`s targeting the Intel 8008.

This crate answers the question: **"Can this IR program run on real Intel 8008
hardware?"**  It is the gate between the Oct language frontend and the
`ir-to-intel-8008-compiler` backend — programs that fail validation cannot be
lowered to correct 8008 assembly.

## Where it fits

```
Oct source
  → oct-lexer / parser / type-checker / ir-compiler → IrProgram
        │
        ├─ intel-8008-ir-validator.validate()   ← THIS PACKAGE
        │       (detect hardware constraint violations)
        │
        └─ ir-to-intel-8008-compiler            (if no errors)
                → 8008 assembly text
```

## The Intel 8008 constraints

The Intel 8008 (1972) is the world's first commercially available 8-bit
microprocessor.  It imposes hard constraints that the Oct type checker cannot
verify:

| Constraint | Reason |
|---|---|
| No LOAD_WORD / STORE_WORD | 8-bit data bus only; no 16-bit memory instruction |
| ≤ 8 191 bytes of static data | RAM region 0x2000–0x3FFE (8 192 bytes minus guard) |
| Call graph depth ≤ 7 | 8-level hardware push-down stack; level 0 = current PC |
| ≤ 6 distinct registers | v0–v5 map to B, A, C, D, E, spare; H/L reserved |
| Immediates ∈ [0, 255] | 8-bit immediate field in MVI, ADI, etc. |
| SYSCALL ∈ {3,4,11–16,20–27,40–63} | Only hardware-supported intrinsics |

## SYSCALL whitelist

| Number | 8008 intrinsic | Assembly lowering |
|---|---|---|
| 3 | `adc(a, b)` | `ADC r` |
| 4 | `sbb(a, b)` | `SBB r` |
| 11 | `rlc(a)` | `RLC` |
| 12 | `rrc(a)` | `RRC` |
| 13 | `ral(a)` | `RAL` |
| 14 | `rar(a)` | `RAR` |
| 15 | `carry()` | `MVI A,0; ACI 0` |
| 16 | `parity(a)` | `ORA A; JFP …` |
| 20–27 | `in(p)` (p = num−20) | `IN p` |
| 40–63 | `out(p,v)` (p = num−40) | `OUT p` |

## Call depth calculation

The 8008 hardware stack has 8 levels.  Level 0 is always the current PC.
That leaves 7 levels for `CAL` instructions.

```
_start → main → encrypt → xor_byte   (depth 3 — fine)
_start → a → b → c → d → e → f → g → h  (depth 8 — rejected!)
```

Recursive call graphs are **always** rejected — the 8008 push-down stack
wraps without any overflow detection, silently corrupting return addresses.

## Quick start

```rust
use compiler_ir::{IrProgram, IrInstruction, IrOp, IrOperand};
use intel_8008_ir_validator::IrValidator;

let mut prog = IrProgram::new("_start");
prog.add_instruction(IrInstruction::new(
    IrOp::LoadImm,
    vec![IrOperand::Register(0), IrOperand::Immediate(42)],
    0,
));
prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));

let errors = IrValidator.validate(&prog);
assert!(errors.is_empty());
// Proceed to ir-to-intel-8008-compiler
```

## Running tests

```
cargo test -p intel-8008-ir-validator
```
