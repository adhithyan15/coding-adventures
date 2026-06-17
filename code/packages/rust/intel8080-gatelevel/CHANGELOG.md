# Changelog — coding-adventures-intel8080-gatelevel

## [0.1.0] — 2026-05-16

### Added

- `bits.rs`: `int_to_bits8`, `int_to_bits16`, `bits_to_u8`, `bits_to_u16`,
  `compute_parity` (7-gate XOR chain + NOT), `compute_zero` (NOR fold),
  `add_8bit` (full 8-stage ripple-carry adder, returns AC flag),
  `sub_8bit` (two's complement via NOT + adder), `add_16bit` (16-stage adder)
- `alu.rs`: `GateAlu8080` with all operations routing through gate primitives:
  `add`, `adc`, `sub`, `sbb`, `inr`, `dcr`, `ana` (with 8080 AC quirk),
  `xra`, `ora`, `cmp`, `rlc`, `rrc`, `ral`, `rar`, `cma`, `stc`, `cmc`, `daa`
- `decoder.rs`: combinational `decode()` using AND/NOT/OR gate tree;
  group decode, dst/src field extraction, HLT detection, memory-operand
  detection, instruction-length decoder (0/1/2 extra bytes)
- `registers.rs`: `RegisterFile` (7×8-bit D flip-flop arrays, reg codes 0–7),
  `Register16` (16 flip-flops, increment/decrement via adder chain),
  register-pair read/write (BC, DE, HL), HL-address helper
- `cpu.rs`: `GateLevelCpu` implementing all 244 Intel 8080A instructions:
  - Group 0: NOP, MVI r/M, LXI, LDA, STA, LHLD, SHLD, LDAX, STAX,
    INR r/M, DCR r/M, INX, DCX, DAD, XCHG, XTHL, SPHL, PCHL,
    RLC, RRC, RAL, RAR, CMA, CMC, STC, DAA
  - Group 1: 63 MOV r,r + HLT
  - Group 2: ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP (register operands)
  - Group 3: JMP, CALL, RET, conditional J/C/R, PUSH, POP, IN, OUT,
    EI, DI, RST n, ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI (immediate)
- `CpuState` snapshot struct for test assertions
- `StepTrace` for per-instruction trace output
- 76 unit tests across all modules
