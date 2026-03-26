# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `DeviceType` enum with Character, Block, Network variants
- `DeviceInfo` struct with name, device_type, major, minor, interrupt_number fields
- `CharacterDevice` trait with read/write interface
- `BlockDevice` trait with read_block/write_block interface
- `NetworkDevice` trait with send_packet/receive_packet/has_packet interface
- `DeviceRegistry` for registering and looking up devices by name or major/minor
- `SimulatedDisk` — in-memory block storage (configurable block size and count)
- `SimulatedKeyboard` — character device with internal byte buffer
- `SimulatedDisplay` — character device with 80x25 text-mode framebuffer
- `SimulatedNIC` — network device using SharedWire for packet exchange
- `SharedWire` — simulated network cable connecting multiple NICs
- Comprehensive test suite covering all device types and registry operations
