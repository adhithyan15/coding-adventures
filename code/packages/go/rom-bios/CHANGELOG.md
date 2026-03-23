# Changelog

## 0.1.0 — 2026-03-19

### Added
- `ROM` type: read-only memory region with `Read`, `ReadWord`, `Write` (ignored), `Contains`
- `ROMConfig` and `DefaultROMConfig()` for configuring ROM base address and size
- `HardwareInfo` struct with `ToBytes()` and `HardwareInfoFromBytes()` serialization
- `BIOSFirmware` generator producing RISC-V machine code for the boot sequence
- `BIOSConfig` and `DefaultBIOSConfig()` for controlling firmware generation
- `AnnotatedInstruction` type for debugging and educational output
- `GenerateWithComments()` method returning annotated assembly listing
- Memory probe step: writes/reads 0xDEADBEEF pattern at 1 MB intervals
- IDT initialization: 256 entries with default fault handler, timer, keyboard, syscall stubs
- HardwareInfo write: populates struct at 0x00001000 with hardware configuration
- Jump to bootloader: JALR to configured entry point (default 0x00010000)
- Comprehensive test suite covering ROM, HardwareInfo, firmware generation, and annotations
