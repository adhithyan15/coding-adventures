# Changelog

## 0.1.0 - 2026-04-12

### Added

- Initial implementation of the Intel 8008 behavioral simulator.
- `Intel8008Simulator` class with complete fetch-decode-execute loop.
- All instruction groups implemented:
  - MOV (register-to-register), MVI (move immediate, 2 bytes)
  - INR (increment), DCR (decrement) — Z, S, P update; CY preserved
  - ALU register: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP
  - ALU immediate: ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI
  - Rotates: RLC, RRC, RAL, RAR
  - Jumps: JMP, JFC/JTC, JFZ/JTZ, JFS/JTS, JFP/JTP (3 bytes)
  - Calls: CAL, CFC/CTC, CFZ/CTZ, CFS/CTS, CFP/CTP (3 bytes)
  - Returns: RET, RFC/RTC, RFZ/RTZ, RFS/RTS, RFP/RTP (1 byte)
  - RST 0–7 (restart to fixed addresses 0, 8, 16, ..., 56)
  - IN 0–7 (read from input port), OUT (write to output port)
  - HLT (two encodings: 0x76 and 0xFF)
- 8-level push-down stack (entry 0 = current PC).
- 4 condition flags: Carry, Zero, Sign, Parity (even parity = P=1).
- 14-bit PC, 16 KiB unified memory.
- `Trace` interface capturing before/after state for each instruction.
- `run()`, `step()`, `reset()`, `loadProgram()` API.
- 8 input ports (`setInputPort`) and 24 output ports (`getOutputPort`).
