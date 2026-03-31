# Changelog — device-driver-framework (Perl)

## 0.01 — 2026-03-31

### Added
- `SimulatedDisk` — In-memory block device with configurable block size and sector count
- `SimulatedSerial` — Character device with TX/RX string buffers and baud rate ioctl
- `SimulatedNIC` — Network device with TX/RX packet queues and MAC address ioctl
- `Registry` — Device registry indexed by name and major:minor number
- Driver lifecycle: initialize → open → read/write/ioctl → close
- 95%+ test coverage via Test2::V0
