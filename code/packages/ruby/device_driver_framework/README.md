# Device Driver Framework (Ruby)

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

The kernel routes I/O requests through the DeviceRegistry instead of directly calling hardware-specific code. This means adding a new device type requires only implementing the protocol and registering it — no kernel changes needed.

## Three Device Families

| Type | Data Model | Examples | Interface |
|------|-----------|----------|-----------|
| Character | Stream of bytes | Keyboard, display | `read(count)`, `write(data)` |
| Block | Fixed-size chunks | Disk, SSD | `read_block(n)`, `write_block(n, data)` |
| Network | Variable packets | Ethernet NIC | `send_packet(data)`, `receive_packet` |

## Usage

```ruby
require "coding_adventures/device_driver_framework"

include CodingAdventures::DeviceDriverFramework

# Create and register devices
registry = DeviceRegistry.new

disk = SimulatedDisk.new(total_blocks: 2048)
disk.init
registry.register(disk)

keyboard = SimulatedKeyboard.new
keyboard.init
registry.register(keyboard)

# Look up devices
dev = registry.lookup_by_name("disk0")
dev.write_block(0, [0x42] * 512)
data = dev.read_block(0)

# Network communication
wire = SharedWire.new
nic_a = SimulatedNIC.new(name: "nic0", wire: wire, mac_address: [0xAA] * 6)
nic_b = SimulatedNIC.new(name: "nic1", minor: 1, wire: wire, mac_address: [0xBB] * 6)
nic_a.init
nic_b.init
nic_a.send_packet([1, 2, 3])
nic_b.receive_packet  # => [1, 2, 3]
```

## Running Tests

```bash
bundle install
bundle exec rake test
```

## Dependencies

- **S03 Interrupt Handler** — devices raise interrupts (33=keyboard, 34=disk, 35=NIC)
- **S05 Display** — SimulatedDisplay wraps the display framebuffer
