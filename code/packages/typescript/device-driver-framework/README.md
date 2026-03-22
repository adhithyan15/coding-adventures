# Device Driver Framework (TypeScript)

A device driver is a piece of software that knows how to talk to a specific piece of hardware. Without drivers, every program that wanted to read from a disk would need to know the exact protocol for that specific disk model. Device drivers provide a **uniform interface** over diverse hardware.

## Where It Fits

```
User Programs
    |  sys_write(fd, buf, n)
    v
OS Kernel (S04)
    |  "Which device does fd=1 refer to?"
    v
Device Driver Framework (D12) <-- THIS PACKAGE
    |  CharacterDevice / BlockDevice / NetworkDevice
    v
Simulated Hardware (display, keyboard, disk, NIC)
```

## Three Device Families

| Type | Data Model | Examples | Interface |
|------|-----------|----------|-----------|
| Character | Byte stream | Keyboard, display | `read(count)`, `write(data)` |
| Block | Fixed-size chunks | Disk, SSD | `readBlock(n)`, `writeBlock(n, data)` |
| Network | Packets | Ethernet NIC | `sendPacket(data)`, `receivePacket()` |

## Usage

```typescript
import {
  DeviceRegistry, SimulatedDisk, SimulatedKeyboard,
  SimulatedDisplay, SimulatedNIC, SharedWire, DeviceType
} from "@coding-adventures/device-driver-framework";

// Create and initialize devices
const registry = new DeviceRegistry();

const display = new SimulatedDisplay();
display.init();
registry.register(display);

const disk = new SimulatedDisk({ totalBlocks: 2048 });
disk.init();
registry.register(disk);

// Look up a device by name
const dev = registry.lookupByName("display0");
dev.write(new Uint8Array([0x48, 0x69])); // "Hi"

// Look up by major/minor number
const d = registry.lookupByMajorMinor(3, 0); // disk0

// List all block devices
const disks = registry.listByType(DeviceType.BLOCK);
```

## Simulated Devices

- **SimulatedDisk** — In-memory block device (default 1 MB, 512-byte blocks)
- **SimulatedKeyboard** — FIFO byte buffer, interrupt 33
- **SimulatedDisplay** — 80x25 framebuffer with cursor tracking
- **SimulatedNIC** — Packet queues connected via SharedWire, interrupt 35

## Running Tests

```bash
npm install
npx vitest run --coverage
```
