# D12 Device Driver Framework (Go)

A unified device driver abstraction layer for the coding-adventures simulated
computer. This package provides the infrastructure that lets an operating system
kernel talk to diverse hardware through a uniform interface.

## What Are Device Drivers?

A device driver is a piece of software that knows how to talk to a specific
piece of hardware. Without drivers, every program would need to know the exact
protocol for every hardware device. Device drivers provide a **uniform
interface** over diverse hardware -- like a universal remote control that works
on any TV brand.

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
| Character | Stream of bytes   | Keyboard, display     | Read(), Write()    |
| Block     | Fixed-size chunks | Disk, SSD             | ReadBlock(), WriteBlock() |
| Network   | Packets           | Ethernet NIC          | SendPacket(), ReceivePacket() |

## Simulated Devices

- **SimulatedDisk** -- in-memory block storage (default 1 MB, 512-byte blocks)
- **SimulatedKeyboard** -- keystroke buffer (read-only character device)
- **SimulatedDisplay** -- framebuffer (80x25 text mode, write-only character device)
- **SimulatedNIC** -- packet queues connected via SharedWire

## Usage

```go
import ddf "github.com/adhithyan15/coding-adventures/code/packages/go/device-driver-framework"

// Create a registry and a disk
registry := ddf.NewDeviceRegistry()
disk := ddf.NewSimulatedDisk("disk0", 0, 512, 2048)
disk.Init()
registry.Register(disk)

// Read and write blocks
data := make([]byte, 512)
disk.WriteBlock(0, data)
block, _ := disk.ReadBlock(0)

// Network: connect two NICs via a shared wire
wire := ddf.NewSharedWire()
nicA := ddf.NewSimulatedNIC("nic0", 0, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01}, wire)
nicB := ddf.NewSimulatedNIC("nic1", 1, []byte{0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x02}, wire)
nicA.Init()
nicB.Init()

nicA.SendPacket([]byte("Hello from NIC A!"))
packet := nicB.ReceivePacket() // []byte("Hello from NIC A!")
```

## Running Tests

```bash
go test ./... -v -cover
```

## Dependencies

- No external dependencies (stdlib only)
