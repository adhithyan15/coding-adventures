# Changelog — coding-adventures-intel8086-gatelevel

## [0.1.0] — 2026-06-15

### Added

- Initial gate-level simulator for the Intel 8086 (1978).

#### `bits.rs`
- `int_to_bits8` / `int_to_bits16` — LSB-first bit-vector conversion
- `bits_to_u8` / `bits_to_u16` — bit-vector to integer
- `add_8bit` / `add_16bit` — 8- and 16-stage ripple-carry adders
- `add_8bit_full` / `add_16bit_full` — adders returning full carry chains for overflow detection
- `compute_parity` — 7-gate XOR tree + NOT; PF = 1 for even popcount
- `compute_zero` — NOR tree; ZF = 1 when all bits zero
- `invert_8bit` / `invert_16bit` — 8 or 16 NOT gates in parallel
- `nibble_borrow` — dedicated 4-bit two's-complement subtractor for the AF flag in SUB/SBB/CMP/DEC/NEG

#### `alu.rs`
- `AluResult8086` — result struct carrying all six ALU flags
- 16-bit arithmetic: `add16`, `sub16`, `and16`, `or16`, `xor16`, `inc16`, `dec16`, `neg16`, `not16`
- 8-bit arithmetic: `add8`, `sub8`, `and8`, `or8`, `xor8`, `inc8`, `dec8`, `neg8`, `not8`
- Shifts: `shl`, `shr`, `sar` (logical and arithmetic)
- Rotates: `rol`, `ror`, `rcl`, `rcr` (the latter two rotate through carry)
- BCD: `daa`, `das`, `aaa`, `aas`, `aam`, `aad`
- MUL/DIV (host arithmetic — gate-level multiplier documented exception):
  `mul8`, `mul16`, `imul8`, `imul16`, `div8`, `div16`, `idiv8`, `idiv16`
- INC/DEC: correct CF-preservation behaviour (caller preserves old CF)
- SUB: CF = NOT(carry_out), AF = nibble_borrow()
- Overflow detection: XOR(carries[N-2], carries[N-1]) single-gate at MSB

#### `registers.rs`
- `RegisterFile8086` — all 14 registers (AX BX CX DX SI DI SP BP CS DS SS ES IP) + 9 flag flip-flops
- `read16` / `write16` — ModRM-encoded 16-bit register access (0=AX, 1=CX, 2=DX, 3=BX, 4=SP, 5=BP, 6=SI, 7=DI)
- `read8` / `write8` — ModRM-encoded 8-bit register access (0=AL, 1=CL, …, 4=AH, 5=CH, …)
- `read8_low` / `write8_low` / `read8_high` / `write8_high` — byte-half access
- `read_seg` / `write_seg` — segment register access (0=ES, 1=CS, 2=SS, 3=DS)
- Named helpers: `al()`, `set_al()`, `ah()`, `set_ah()`, etc. for AX/BX/CX/DX
- `pack_flags` — assemble FLAGS word (bit 1 always 1; each flag gated through OR)
- `unpack_flags` — restore flags from FLAGS word (each bit latched through AND)
- `physical_address` — (seg × 16 + offset) & 0xFFFFF via `add_20bit()`
- `add_20bit` — 20-stage ripple-carry adder for physical address computation

#### `cpu.rs`
- `Cpu8086` struct with 1 MB flat memory (`Box<[u8; 1_048_576]>`), 256-byte I/O port arrays
- `new()`, `reset()`, `load()`, `step()`, `execute()`, `get_state()` public API
- Prefix handling: ES:/CS:/SS:/DS: segment overrides, REP/REPNZ/REPNE, LOCK (ignored)
- Full ModRM decode: mod/reg/rm extraction, 8 effective-address modes, disp8/disp16
- Segment selection: BP-based → SS, otherwise → DS, overridable
- 120 opcodes implemented:
  - MOV: r/m↔reg, imm8/imm16, byte/word, segment register variants, moffset
  - XCHG, NOP
  - PUSH/POP: registers, segment registers, r/m, PUSHF/POPF
  - LEA, LDS, LES
  - LAHF, SAHF, CBW, CWD, XLAT
  - 80-group ALU (imm), accumulator-imm ALU, standard ALU r/m↔reg
  - TEST (r/m,reg and r/m,imm)
  - INC/DEC (40–47/48–4F register encoding, FE/FF group)
  - FF group: INC/DEC/CALL/JMP/PUSH r/m16, CALL FAR, JMP FAR
  - F6/F7 group: TEST/NOT/NEG/MUL/IMUL/DIV/IDIV
  - BCD: DAA, DAS, AAA, AAS, AAM, AAD
  - Shifts/rotates: D0/D1 (count=1), D2/D3 (count=CL), all 8 ext codes
  - JMP short/near/far, CALL near/far, RET/RETF (±n variants)
  - Conditional jumps Jcc (70–7F, 16 conditions)
  - LOOP, LOOPE/LOOPZ, LOOPNE/LOOPNZ, JCXZ
  - INT 3, INT n, INTO, IRET
  - String ops with REP/REPE/REPNE: MOVSB/W, LODSB/W, STOSB/W, CMPSB/W, SCASB/W
  - Flag ops: CLC, STC, CMC, CLD, STD, CLI, STI
  - I/O: IN AL/AX,imm8 / IN AL/AX,DX / OUT imm8,AL/AX / OUT DX,AL/AX
  - HLT, WAIT (NOP in this model)
- 56 unit tests, 14 doctests — all passing
