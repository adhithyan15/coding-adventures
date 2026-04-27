# Changelog

## [0.01] - 2026-04-12

### Added

- `CodingAdventures::Intel8008Simulator` — complete behavioral simulator for the
  Intel 8008 microprocessor (April 1972), implementing the full instruction set.

- **Register operations**: `MOV` (register-to-register and via M pseudo-register),
  `MVI` (move immediate, 2-byte), `INR` (increment, preserves carry), `DCR`
  (decrement, preserves carry).

- **ALU register operations** (8 operations × register source):
  `ADD`, `ADC`, `SUB`, `SBB`, `ANA` (AND), `XRA` (XOR), `ORA` (OR), `CMP`.
  All update Z, S, P, CY. ANA/XRA/ORA always clear carry.

- **ALU immediate operations** (2-byte): `ADI`, `ACI`, `SUI`, `SBI`, `ANI`,
  `XRI`, `ORI`, `CPI`. Same semantics as register variants.

- **Rotate instructions**: `RLC` (circular left), `RRC` (circular right),
  `RAL` (left through carry), `RAR` (right through carry). Only CY affected;
  Z, S, P preserved.

- **Jump instructions** (3-byte): `JMP` (unconditional), `JFC`/`JTC` (carry),
  `JFZ`/`JTZ` (zero), `JFS`/`JTS` (sign), `JFP`/`JTP` (parity).

- **Call instructions** (3-byte): `CAL` (unconditional), conditional variants
  (`CFC`/`CTC`, `CFZ`/`CTZ`, `CFS`/`CTS`, `CFP`/`CTP`). Pushes return address
  onto the 8-level hardware push-down stack.

- **Return instructions** (1-byte): `RET` (unconditional), conditional variants
  (`RFC`/`RTC`, `RFZ`/`RTZ`, `RFS`/`RTS`, `RFP`/`RTP`). Pops from the stack.

- **Restart** (1-byte): `RST 0`–`RST 7` — fast 1-byte calls to fixed addresses
  0, 8, 16, 24, 32, 40, 48, 56.

- **I/O**: `IN 0`–`IN 7` (reads 8-bit value from input port into A), `OUT 0`–`OUT 23`
  (writes A to output port). Port values set/read via `set_input_port` and
  `get_output_port`.

- **Halt**: Both encodings — `0x76` (MOV M,M) and `0xFF`.

- **8-level push-down stack** — Entry 0 is always the current PC. CALL rotates
  entries down; RETURN rotates up. Stack depth tracked in `stack_depth`.

- **14-bit address space** — PC and HL address masked to 0x3FFF.
  `hl_address` computes `(H & 0x3F) << 8 | L`.

- **Flag semantics**: Carry=1 on overflow (ADD) or borrow (SUB). Parity=1 means
  even number of 1-bits (even parity, 8008 convention). INR/DCR preserve carry.

- `run($program, $max_steps)` — loads and executes a program, returns arrayref
  of trace hashrefs with address, raw bytes, mnemonic, before/after A and flags,
  plus memory address/value for M-touching instructions.

- `step()` — single-step execution returning one trace hashref.

- `reset()` — restores all state to power-on defaults.

- `set_input_port($port, $value)` / `get_output_port($port)` — I/O port access.

- Comprehensive Test2::V0 test suite covering every instruction group,
  flag semantics, stack operations, memory indirect access, I/O ports,
  and integration programs (arithmetic, memory store/load, countdown loop,
  call/return sequences).
