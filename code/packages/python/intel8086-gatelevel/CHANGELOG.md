# Changelog — intel8086-gatelevel

All notable changes to this package are documented here.

## [1.0.0] — 2026-05-15

### Added

- `src/intel8086_gatelevel/bits.py` — Gate-level arithmetic primitives
  - `int_to_bits(value, width)` — integer to LSB-first bit list
  - `bits_to_int(bits)` — LSB-first bit list to integer
  - `add_8bit(a, b, carry_in)` — 8-bit add via `ripple_carry_adder`; auxiliary carry captured with a separate 4-bit add
  - `add_16bit(a, b, carry_in)` — 16-bit add via `ripple_carry_adder`
  - `add_20bit(a, b)` — 20-bit add for physical address computation
  - `invert_8bit(value)` — 8 `not_gate` calls
  - `invert_16bit(value)` — 16 `not_gate` calls
  - `compute_parity(bits)` — XOR tree over low 8 bits (PF = 1 if even parity)
  - `compute_zero(bits)` — NOR tree (ZF = 1 if all bits zero)

- `src/intel8086_gatelevel/alu.py` — Gate-level ALU
  - `ALUResult8086` dataclass: `result`, `flag_cf`, `flag_of`, `flag_sf`, `flag_zf`, `flag_af`, `flag_pf`
  - 16-bit operations: `add16`, `sub16`, `and16`, `or16`, `xor16`, `inc16`, `dec16`, `neg16`, `not16`
  - 8-bit operations: `add8`, `sub8`, `and8`, `or8`, `xor8`, `inc8`, `dec8`, `neg8`, `not8`
  - Shift and rotate: `shl`, `shr`, `sar`, `rol`, `ror`, `rcl`, `rcr`
  - Multiply/divide: `mul8`, `mul16`, `imul8`, `imul16`, `div8`, `div16`, `idiv8`, `idiv16`
  - BCD adjust: `daa`, `das`, `aaa`, `aas`, `aam`, `aad`
  - Two's complement subtraction: `sub16(a, b)` = `add16(a, NOT(b), 1)`; CF = NOT(adder carry) per 8086 convention (CF=1 means borrow)
  - Overflow detection: `XOR(carry_into_msb, carry_out_of_msb)`

- `src/intel8086_gatelevel/register_file.py` — Bit-array register file
  - `RegisterFile8086`: 13 registers stored as 16-element `list[int]` (LSB-first)
  - 9 flag bits stored as individual integers
  - `read16` / `write16` — 16-bit access
  - `read8_low` / `write8_low` — AL/BL/CL/DL byte halves
  - `read8_high` / `write8_high` — AH/BH/CH/DH byte halves
  - `pack_flags()` — assembles 16-bit FLAGS word (bit 1 always set)
  - `unpack_flags(flags)` — disperses FLAGS to individual flag bits
  - `physical_address(seg, offset)` — 20-bit EA via `add_20bit`

- `src/intel8086_gatelevel/decoder.py` — Instruction decoder
  - `DecodedInstr` dataclass: mnemonic, length, word, mod, reg, rm, disp, imm, seg_override, rep_prefix, opcode, has_modrm, extra
  - `decode_instruction(memory, cs, ip)` — decodes one instruction from a byte array
  - Handles all 8086 prefix bytes: segment overrides (0x26/0x2E/0x36/0x3E), REP/REPNE (0xF3/0xF2), LOCK (0xF0)
  - Covers all instruction classes: MOV, ALU (ADD/SUB/AND/OR/XOR/CMP), INC/DEC, PUSH/POP, JCC (all 16 conditions), LOOP/JCXZ, CALL/RET, string ops, shifts/rotates, BCD, IN/OUT, INT, miscellaneous
  - Unknown opcodes produce `DB(0xNN)` mnemonic

- `src/intel8086_gatelevel/simulator.py` — Top-level simulator
  - `Intel8086GateLevelSimulator` implementing `Simulator[X86State]` from `simulator-protocol`
  - `reset()` — zero all registers and memory; set DS/ES/SS = 0, CS = 0xFFFF, SP = 0xFFFE
  - `load(program, origin)` — copy bytes into flat memory array
  - `step()` — decode and execute one instruction; return `StepTrace`
  - `execute(program, max_steps)` — run until HLT or step limit; return `ExecutionResult[X86State]`
  - `get_state()` — snapshot as `X86State`
  - `set_input_port(port, value)` / `get_output_port(port)` — I/O port map
  - `interrupt(vector)` / `nmi()` — software and hardware interrupt injection via `_trigger_interrupt`
  - All IP increments use `add_16bit` for gate-level fidelity
  - All effective-address computations use `add_16bit` for gate-level fidelity
  - `_push16` / `_pop16` use `add_16bit` for SP arithmetic
  - Segment computation (physical address) uses `add_20bit`

- `tests/test_bits.py` — 8 test classes, ~40 test cases
- `tests/test_alu.py` — 17 test classes, ~90 test cases
- `tests/test_register_file.py` — 4 test classes, ~30 test cases
- `tests/test_decoder.py` — 12 test classes, ~60 test cases
- `tests/test_programs.py` — 5 test classes, ~25 test cases
- `tests/test_equivalence.py` — 40+ cross-validation programs
- `tests/test_simulator_coverage.py` — 10+ test classes covering all JCC conditions, addressing modes, LOOP variants, string ops, BCD, INT/IRET

- `BUILD` — shell-format build script using `uv venv` + `uv pip install` + `pytest`
- `pyproject.toml` — hatchling build, ruff lint config, pytest coverage settings
- `README.md` — usage guide, module table, gate-level commitment explanation
- `CHANGELOG.md` — this file

### Implementation notes

- MUL/DIV operations use Python host arithmetic internally. A full gate-level
  multiplier/divider is outside the scope of this layer.
- BCD adjustment operations (DAA, DAS, AAA, AAS, AAM, AAD) mirror the
  behavioral simulator and use Python arithmetic for correction values.
- INT instructions (0xCC, 0xCE, 0xCD) trigger the interrupt mechanism and
  then halt the simulator, matching the behavioral simulator's behavior in
  the test harness.
