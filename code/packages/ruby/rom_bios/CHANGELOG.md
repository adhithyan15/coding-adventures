# Changelog

## 0.1.0 — 2026-03-19

### Added
- `ROM` class: read-only memory with `read`, `read_word`, `write` (ignored), `contains?`
- `ROMConfig` data class and default configuration
- `HardwareInfo` data class with `to_bytes` and `from_bytes` serialization
- `BIOSFirmware` generator producing RISC-V machine code for boot sequence
- `BIOSConfig` data class for firmware configuration
- `AnnotatedInstruction` data class for debugging and educational output
- Memory probe, IDT initialization, HardwareInfo write, bootloader jump
- Comprehensive test suite
