# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `DeviceType` enum with CHARACTER, BLOCK, and NETWORK values
- `CharacterDevice`, `BlockDevice`, and `NetworkDevice` interfaces
- `DeviceRegistry` for registering and looking up devices by name or major/minor
- `SimulatedDisk` block device with configurable size and 512-byte blocks
- `SimulatedKeyboard` character device with FIFO buffer and interrupt 33
- `SimulatedDisplay` character device with 80x25 framebuffer and cursor tracking
- `SimulatedNIC` network device with packet queues and SharedWire broadcast
- `SharedWire` for connecting multiple NICs on a simulated network
- Type guard helpers: `isCharacterDevice`, `isBlockDevice`, `isNetworkDevice`
- Comprehensive test suite covering all device types, registry, and integration
