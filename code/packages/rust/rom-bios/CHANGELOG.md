# Changelog

## 0.1.0 — 2026-03-19

### Added
- `Rom` struct: read-only memory with `read`, `read_word`, `write` (ignored), `contains`
- `RomConfig` with default configuration
- `HardwareInfo` struct with `to_bytes` and `from_bytes` serialization
- `BiosFirmware` generator producing RISC-V machine code for boot sequence
- `BiosConfig` with default configuration
- `AnnotatedInstruction` struct for debugging and educational output
- Memory probe, IDT initialization, HardwareInfo write, bootloader jump
- Comprehensive test suite
