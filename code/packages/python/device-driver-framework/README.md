# D12 Device Driver Framework (Python)

A unified device driver abstraction layer for the coding-adventures simulated
computer. This package provides the infrastructure that lets an operating system
kernel talk to diverse hardware through a uniform interface.

## What Are Device Drivers?

A device driver is a piece of software that knows how to talk to a specific
piece of hardware. Without drivers, every program would need to know the exact
protocol for every hardware device it interacts with. Device drivers solve this
by providing a **uniform interface** over diverse hardware.

Think of a universal remote control: you press "Volume Up" and it works on any
TV brand. Each device speaks a different protocol, but the remote translates
your single button press into the right signal. Device drivers are the universal
remote for your operating system.

## Where It Fits in the Stack

```
User Programs
    |  sys_write(fd, buf, n)
    v
OS Kernel (S04)
    |  "Which device does fd=1 refer to?"
    v
Device Driver Framework (D12)  <-- YOU ARE HERE
    |  CharacterDevice / BlockDevice / NetworkDevice
    v
Simulated Hardware
```

## The Three Device Families

| Type      | Data Model        | Examples              | Interface          |
|-----------|-------------------|-----------------------|--------------------|
| Character | Stream of bytes   | Keyboard, display     | read(), write()    |
| Block     | Fixed-size chunks | Disk, SSD             | read_block(), write_block() |
| Network   | Packets           | Ethernet NIC          | send_packet(), receive_packet() |

## Simulated Devices

- **SimulatedDisk** -- in-memory block storage (default 1 MB, 512-byte blocks)
- **SimulatedKeyboard** -- keystroke buffer (read-only character device)
- **SimulatedDisplay** -- framebuffer (80x25 text mode, write-only character device)
- **SimulatedNIC** -- packet queues connected via SharedWire

## Usage

```python
from device_driver_framework import (
    DeviceRegistry, DeviceType,
    SimulatedDisk, SimulatedKeyboard, SimulatedDisplay,
    SimulatedNIC, SharedWire,
)

# Create a registry and some devices
registry = DeviceRegistry()

disk = SimulatedDisk(total_blocks=2048)
disk.init()
registry.register(disk)

# Read and write blocks
disk.write_block(0, b"\x00" * 512)
data = disk.read_block(0)

# Network: connect two NICs via a shared wire
wire = SharedWire()
nic_a = SimulatedNIC(name="nic0", minor=0, mac_address=b"\xDE\xAD\xBE\xEF\x00\x01", wire=wire)
nic_b = SimulatedNIC(name="nic1", minor=1, mac_address=b"\xDE\xAD\xBE\xEF\x00\x02", wire=wire)
nic_a.init()
nic_b.init()

nic_a.send_packet(b"Hello from NIC A!")
packet = nic_b.receive_packet()  # b"Hello from NIC A!"
```

## Running Tests

```bash
uv venv && uv pip install -e ".[dev]"
python -m pytest tests/ -v
```

## Dependencies

- No external runtime dependencies
- Dev: pytest, pytest-cov, ruff, mypy
