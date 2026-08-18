# Changelog — intel8051-simulator

## v0.1.0 — 2026-08-17 — initial port (fourth lane, 9-architecture expansion)

Pure-Rust port of `code/packages/python/intel8051-simulator` (spec
07p), module-split into `opcodes`/`encoding`/`decode`/`execute`/
`simulator` mirroring this codebase's other historical-arch
simulators' shape.

### Added

- `Intel8051Simulator` — Harvard-architecture behavioral simulator
  with independent 64 KiB code memory, 256 B internal RAM (+ SFRs),
  and 64 KiB external data memory.
  - `new()`, `reset()`, `load_program(code, start_addr)`, `step()
    -> String`, `run(max_steps) -> Vec<String>`,
    `run_loaded_with_limit(max_steps) -> ExecutionResult`.
  - Register/flag accessors: `pc`, `acc`, `b_register`, `sp`, `dptr`,
    `psw`, `cy`, `ac`, `ov`, `parity_flag`, `bank`, `read_register`,
    `read_iram`, `read_code`, `read_xdata`, `halted`.
- Full instruction-set port from `simulator.py`: data transfer (MOV/
  MOVC/MOVX/PUSH/POP/XCH/XCHD family), arithmetic (ADD/ADDC/SUBB/INC/
  DEC/MUL/DIV/DA), logic (ANL/ORL/XRL/CLR/CPL/RL/RLC/RR/RRC/SWAP),
  bit manipulation (CLR/SETB/CPL/ANL/ORL/MOV on C and bit addresses),
  jumps (LJMP/SJMP/AJMP/JMP @A+DPTR/Jcc/JB/JNB/JBC), compare-and-
  branch (CJNE) and decrement-and-branch (DJNZ), and subroutines
  (LCALL/ACALL/RET/RETI).
- HALT convention: opcode `0xA5` (reserved/undefined on real
  silicon), ported unchanged from `intel8051_simulator.state.
  HALT_OPCODE` (Python, spec 07p) — see `opcodes::HALT_OPCODE`'s doc
  comment for the full rationale, including why self-jump detection
  (the historically-idiomatic 8051 "the program is done" convention)
  was considered and not used here.
- `ExecutionResult { halted, steps, pc }` — bounded-run summary.

### Design notes

- `Intel8051Simulator::new()` takes no `memory_size` parameter, unlike
  `arm1_simulator::ARM1::new(memory_size)` — the 8051's three address
  spaces are architecturally fixed by the instruction encoding itself,
  so there is no meaningful size to vary. See `simulator.rs`'s module
  doc comment.
- `decode`/`execute` are split (pure opcode → operand-length decoding,
  vs. the state-mutating semantic dispatch) so `decode` is unit-
  testable without any CPU state, mirroring `arm1_simulator::decode`'s
  `DecodedInstruction` boundary.

### Tests

40+ unit tests across all five modules: opcode/PSW-mask sanity checks,
decode operand-length coverage (every family base + both AJMP/ACALL
bit patterns + code-memory wraparound), flag-arithmetic truth tables
(`add8_flags`/`sub8_flags`/`da_flags`/`parity`), and simulator-level
behavior (reset state, load+halt, ADD carry/overflow, register
round-trip, a `DJNZ`-loop summing 1..=10, HALT idempotency, and a
bounded `SJMP $` infinite-loop test proving `run_loaded_with_limit`
genuinely stops at `max_steps` rather than spinning forever).
