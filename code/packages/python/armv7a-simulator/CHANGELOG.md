# Changelog

## 0.1.0 (2026-05-15)

Initial release — Layer 07x in the coding-adventures simulator series.

### Added

- `ARMv7ASimulator` implementing the SIM00 protocol:
  - `reset()`, `load()`, `step()`, `execute()`, `get_state()`
  - I/O stubs: `set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()`
- `ARMv7AState` frozen dataclass with `r0`–`r12`, `sp`, `lr` properties,
  CPSR flag properties (`n`, `z`, `c`, `v`, `thumb`)
- Thumb-2 decoder: 16-bit vs 32-bit width detection on bits [15:11]
- 16-bit instruction set:
  - Shift immediate: LSL, LSR, ASR with imm5 and barrel-shifter carry
  - Add/subtract: register, imm3, imm8 forms; full N/Z/C/V flag updates
  - Move/compare immediate: MOV, CMP, ADD, SUB with imm8
  - Data processing (16): AND, EOR, LSL(r), LSR(r), ASR(r), ADC, SBC, ROR,
    TST, NEG/RSB, CMP, CMN, ORR, MUL, BIC, MVN
  - High-register operations: MOV, ADD, CMP (R0–R15)
  - Load/store: LDR/STR word (imm5×4, reg offset, SP-relative),
    LDRB/STRB byte (imm5), LDRH/STRH halfword (imm5×2),
    LDRSB / LDRSH (register offset), ADR (PC-relative)
  - Stack: PUSH (with LR), POP (with PC), ADD/SUB SP, #imm7×4
  - Multiple: LDM, STM (low 8 registers, writeback)
  - Branch: B (conditional, 14 conds), B (unconditional), BX, BLX
- 32-bit instruction set:
  - BL (branch-and-link, T1 encoding, ±16 MB range)
  - MOVW (16-bit zero-extending immediate)
  - MOVT (16-bit into top halfword)
  - AND.W, ORR.W, EOR.W, ADD.W, ADC.W, SUB.W, RSB.W (modified immediate)
  - LDR.W, STR.W, LDRH.W, STRH.W, LDRB.W, STRB.W (12-bit unsigned offset)
- Barrel shifter: `_lsl`, `_lsr`, `_asr`, `_ror`, `_rrx`, `_apply_shift_imm`
- Thumb-2 Modified Immediate Constant (`_thumb_expand_imm`)
- `_check_cond()` evaluating all 14 ARM condition codes
- `_add_flags()` and `_sub_flags()` with correct ARM carry convention
  (subtract: C=1 means no borrow)
- Halt sentinel: 16-bit halfword `0x0000`
- Reset initializes SP=0xFFF8, CPSR.T=1, all other registers and memory = 0
- Full test suite: 13 protocol tests + ~60 instruction tests (>80% coverage)
- `py.typed` marker for PEP 561 compliance
- Literate code comments throughout
