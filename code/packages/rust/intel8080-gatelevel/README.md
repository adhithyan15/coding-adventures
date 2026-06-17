# intel8080-gatelevel

Gate-level simulator for the **Intel 8080A** (1974) microprocessor.

Every arithmetic and logic operation routes through real gate primitives from the
`logic-gates` and `arithmetic` crates — no host integer arithmetic in the execution
path. Registers are modelled as D flip-flop arrays.

## Architecture

```
bits.rs      — integer ↔ LSB-first bit-vector helpers (8-bit + 16-bit)
alu.rs       — GateAlu8080: ADD/SUB/AND/OR/XOR/rotate through gate chains
decoder.rs   — combinational AND/NOT/OR gate tree → control signals
registers.rs — 7×8-bit flip-flop arrays + 16-bit PC and SP
cpu.rs       — fetch-decode-execute loop; 244 Intel 8080A instructions
```

## Quick start

```rust
use coding_adventures_intel8080_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();
// MVI A,10 ; MVI B,5 ; ADD B ; HLT
let (traces, state) = cpu.run(&[0x3E, 0x0A, 0x06, 0x05, 0x80, 0x76], 100);
assert_eq!(state.a, 15);
assert!(!state.flag_cy);
```

## Gate count estimate

| Component               | Gates |
|-------------------------|-------|
| 8-bit ALU (add/sub/log) | ~104  |
| Register file (7×8-bit) | 336   |
| PC + SP (2×16-bit)      | 192   |
| 16-bit adder (DAD/INX)  | 80    |
| Instruction decoder     | ~80   |
| Control + wiring        | ~300  |
| **Total**               | **~1,092** |

Real 8080A: ~6,000 transistors (NMOS, ~1,500 gate equivalents).

## Key differences from 8008

| Feature      | Intel 8008       | Intel 8080           |
|--------------|------------------|----------------------|
| PC width     | 14-bit           | 16-bit               |
| Stack        | 8-level push-down | Explicit SP + memory |
| Memory       | 16 KB            | 64 KB                |
| Flags        | S, Z, P, CY      | S, Z, **AC**, P, CY  |
| I/O ports    | 8 input / 24 out | **256** in / 256 out |
| Instruction  | ~50 instructions | 244 instructions     |

## Instruction coverage

- **Group 0 (misc)**: NOP, MVI, LXI, LDA, STA, LHLD, SHLD, LDAX, STAX,
  INR, DCR, INX, DCX, DAD, XCHG, XTHL, SPHL, PCHL, RLC, RRC, RAL, RAR,
  CMA, CMC, STC, DAA
- **Group 1 (MOV)**: 63 register-to-register moves + HLT
- **Group 2 (ALU reg)**: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP × 8 src
- **Group 3 (branch/stack)**: JMP, CALL, RET (+ conditional variants),
  PUSH, POP, IN, OUT, EI, DI, RST n, ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI

## Package layout

Part of the `coding-adventures` gate-level simulator series:

| Layer | Package                    | Processor      | Year |
|-------|----------------------------|----------------|------|
| 07d2  | `intel4004-gatelevel`      | Intel 4004     | 1971 |
| 07f2  | `intel8008-gatelevel`      | Intel 8008     | 1972 |
| **07i2** | **`intel8080-gatelevel`** | **Intel 8080** | **1974** |
