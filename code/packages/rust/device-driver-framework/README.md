# Device Driver Framework (Rust)

## What Are Device Drivers?

A device driver is a piece of software that knows how to talk to a specific piece of hardware. Without drivers, every program that wanted to read from a disk would need to know the exact protocol for that specific disk model. Device drivers solve this by providing a **uniform interface** over diverse hardware.

Think of a universal remote control: you press "Volume Up" and it works on your Samsung TV, your Sony soundbar, and your LG projector. Each device speaks a different protocol, but the remote translates your single button press into the right signal. Device drivers are the universal remote for your operating system.

## Where This Fits

```
User Programs
    |
    v
OS Kernel (S04)
    |
    v
Device Driver Framework (D12) <-- YOU ARE HERE
    |
    v
Simulated Hardware (display, keyboard, disk, NIC)
```

## Three Device Families

| Type | Data Model | Examples | Trait |
|------|-----------|----------|-------|
| Character | Stream of bytes | Keyboard, display | `CharacterDevice` |
| Block | Fixed-size chunks | Disk, SSD | `BlockDevice` |
| Network | Variable packets | Ethernet NIC | `NetworkDevice` |

## Usage

```rust
use device_driver_framework::*;

// Create a disk
let mut disk = SimulatedDisk::new("disk0", 0, 2048, 512);
disk.init();

// Write and read blocks
let data = vec![0x42u8; 512];
disk.write_block(0, &data).unwrap();
let read_back = disk.read_block(0).unwrap();

// Registry
let mut registry = DeviceRegistry::new();
registry.register(Box::new(disk)).unwrap();
```

## Running Tests

```bash
cargo test -p device-driver-framework
```

## Dependencies

- **S03 Interrupt Handler** — devices raise interrupts (33=keyboard, 34=disk, 35=NIC)
- **S05 Display** — SimulatedDisplay wraps the display framebuffer
