# Changelog

## 0.1.0 — 2026-03-19

### Added
- `ROM` class: read-only memory with `read`, `read_word`, `write` (ignored), `contains`
- `ROMConfig` dataclass and `DefaultROMConfig()` for configuration
- `HardwareInfo` dataclass with `to_bytes()` and `from_bytes()` serialization
- `BIOSFirmware` generator producing RISC-V machine code for boot sequence
- `BIOSConfig` dataclass and `DefaultBIOSConfig()` for firmware configuration
- `AnnotatedInstruction` dataclass for debugging and educational output
- `generate_with_comments()` method returning annotated assembly listing
- Memory probe, IDT initialization, HardwareInfo write, bootloader jump
- Comprehensive test suite with full coverage
