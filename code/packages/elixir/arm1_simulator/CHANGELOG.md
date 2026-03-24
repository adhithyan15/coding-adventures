# Changelog

## 0.1.0 — 2026-03-23

### Added
- Full ARM1 behavioral simulator ported from Go
- Complete ARMv1 instruction set: 16 ALU operations, load/store, block transfer, branch, SWI
- Conditional execution on every instruction (16 condition codes)
- Barrel shifter with all 4 shift types (LSL, LSR, ASR, ROR) plus RRX
- 4 processor modes with banked registers (USR, FIQ, IRQ, SVC)
- Instruction decoder and disassembler
- Encoding helpers for test program construction
- Immutable functional design: step(cpu) returns {new_cpu, trace}
- 102 unit tests covering all subsystems
