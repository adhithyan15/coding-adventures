# Changelog — intel8080-gatelevel

## 0.1.0 — 2026-05-04

### Added

- Initial release: Intel 8080A gate-level simulator
- `bits.py`: bit conversion helpers (int_to_bits, bits_to_int, compute_parity,
  compute_zero, add_8bit, add_16bit)
- `alu.py`: `ALU8080` — full 8-bit ALU routing every operation through gate
  primitives (half_adder, full_adder, ripple_carry_adder, AND/OR/XOR/NOT gates)
  - Operations: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP, INR, DCR
  - Rotates: RLC, RRC, RAL, RAR
  - Special: CMA (complement), DAA (decimal adjust)
  - 8080-specific ANA AC quirk: AC = OR(bit3(a), bit3(b))
- `decoder.py`: `Decoder8080` — combinational gate tree mapping opcode bits
  to control signals (group, dst, src, alu_op, extra_bytes, is_halt, etc.)
- `register_file.py`: `Register8`, `Register16`, `RegisterFile` — 7 × 8-bit
  working registers + 16-bit SP, each modeled as D flip-flop arrays
- `control.py`: `ControlUnit` FSM — FETCH/DECODE/EXECUTE/WRITEBACK states,
  orchestrates all 244 8080 instructions through the gate-level components
- `simulator.py`: `Intel8080GateLevelSimulator` — SIM00-conforming
  `Simulator[Intel8080State]` with load/reset/step/execute/get_state
- Tests: 220+ tests, ≥95% coverage
  - `test_bits.py`: bit conversion round-trips
  - `test_alu.py`: all ALU operations through gate paths
  - `test_decoder.py`: gate-level opcode decode for all instruction groups
  - `test_register_file.py`: Register8/16 and RegisterFile ops
  - `test_adder16.py`: 16-bit PC incrementer and SP ±2
  - `test_equivalence.py`: gate-level == behavioral for all instruction groups
  - `test_programs.py`: end-to-end programs matching behavioral output
  - `test_simulator.py`: SIM00 protocol compliance, MOV/INR/DCR with M
    pseudo-register, all 8 conditional branches (including PO/PE/P/M), LXI D,
    RST instruction, invalid condition code validation
