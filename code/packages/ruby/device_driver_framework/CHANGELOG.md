# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `DeviceType` module with CHARACTER, BLOCK, NETWORK constants
- `Device` base class with name, device_type, major, minor, interrupt_number fields
- `CharacterDevice` abstract class with read/write interface
- `BlockDevice` abstract class with read_block/write_block interface
- `NetworkDevice` abstract class with send_packet/receive_packet/has_packet? interface
- `DeviceRegistry` for registering and looking up devices by name or major/minor
- `SimulatedDisk` — in-memory block storage (default 512-byte blocks, 2048 blocks)
- `SimulatedKeyboard` — character device with internal byte buffer
- `SimulatedDisplay` — character device with 80x25 text-mode framebuffer
- `SimulatedNIC` — network device with SharedWire packet exchange
- `SharedWire` — simulated network cable connecting multiple NICs
- Comprehensive test suite covering all device types and registry operations
