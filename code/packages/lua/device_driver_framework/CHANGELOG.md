# Changelog — device_driver_framework (Lua)

## 0.1.0 — 2026-03-31

### Added
- `SimulatedDisk` — block device with configurable block size and total blocks
- `SimulatedSerial` — character device with TX/RX buffers and baud rate control
- `SimulatedNIC` — network device with TX/RX packet queues and MAC address
- `Registry` — kernel device catalog with register, get, get_by_major_minor, update, unregister, list
- Immutable (functional-style) API throughout
- 95%+ test coverage via busted test suite
