# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Device type atoms: `:character`, `:block`, `:network` with integer conversion
- `SimulatedDisk` block device with configurable size and 512-byte blocks
- `SimulatedKeyboard` character device with FIFO buffer and interrupt 33
- `SimulatedDisplay` character device with 80x25 framebuffer and cursor tracking
- `SimulatedNIC` network device with packet queues and functional broadcast
- `SharedWire` for connecting NIC names on a simulated network
- `DeviceRegistry` for registering and looking up devices by name or major/minor
- Comprehensive ExUnit test suite covering all device types, registry, and integration
