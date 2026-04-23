# Changelog

All notable changes to the intel8008-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-12

### Added
- Initial implementation of the Intel 8008 behavioral simulator
- Complete 48-instruction set implementation including:
  - MOV D,S (register-to-register transfer, including M pseudo-register)
  - MVI D,d (move immediate, 2-byte)
  - INR/DCR (increment/decrement, Z/S/P flags only, CY preserved)
  - ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP with register and M source
  - ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI (immediate ALU, 2-byte)
  - RLC/RRC/RAL/RAR (rotate accumulator)
  - JMP/JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP (jump, 3-byte)
  - CAL/CFC/CTC/CFZ/CTZ/CFS/CTS/CFP/CTP (call, 3-byte)
  - RFC/RTC/RFZ/RTZ/RFS/RTS/RFP/RTP/RET (return, conditional and unconditional)
  - RST 0–7 (1-byte call to fixed addresses)
  - IN 0–7 / OUT 0–23 (I/O ports)
  - HLT (both 0x76 and 0xFF encodings)
- 14-bit address space (16,384 bytes)
- 8-level push-down stack where entry 0 is always the current PC
- 4 condition flags: Carry (CY), Zero (Z), Sign (S), Parity (P)
- M pseudo-register (indirect memory via H:L pair)
- 8 input ports, 24 output ports
- `Intel8008Trace` dataclass for step-by-step execution tracing
- `Intel8008Flags` dataclass for named flag access
- `step()` returns a single trace; `run()` accumulates all traces until HLT
- `reset()` clears all state back to power-on defaults
