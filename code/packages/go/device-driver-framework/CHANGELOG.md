# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `DeviceType` constants: `DeviceCharacter`, `DeviceBlock`, `DeviceNetwork`
- `DeviceBase` struct with name, type, major/minor numbers, interrupt number
- `CharacterDevice` interface with `Read` and `Write` methods
- `BlockDevice` interface with `ReadBlock` and `WriteBlock` methods
- `NetworkDevice` interface with `SendPacket`, `ReceivePacket`, `HasPacket`
- `DeviceRegistry` for registering, looking up, and listing devices
- `SimulatedDisk` block device backed by an in-memory byte slice
- `SimulatedKeyboard` character device with a keystroke input buffer
- `SimulatedDisplay` character device with an 80x25 text-mode framebuffer
- `SimulatedNIC` network device with packet queues
- `SharedWire` for connecting multiple SimulatedNICs
- Comprehensive test suite with 90%+ coverage target
