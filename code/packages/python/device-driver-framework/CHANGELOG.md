# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `DeviceType` enum with CHARACTER, BLOCK, and NETWORK variants
- `Device` base class with name, type, major/minor numbers, interrupt number
- `CharacterDevice` abstract class with read/write byte-stream interface
- `BlockDevice` abstract class with read_block/write_block random-access interface
- `NetworkDevice` abstract class with send_packet/receive_packet interface
- `DeviceRegistry` for registering, looking up, and listing devices
- `SimulatedDisk` block device backed by an in-memory byte array
- `SimulatedKeyboard` character device with a keystroke input buffer
- `SimulatedDisplay` character device with an 80x25 text-mode framebuffer
- `SimulatedNIC` network device with packet queues
- `SharedWire` for connecting multiple SimulatedNICs
- Comprehensive test suite with 95%+ coverage target
