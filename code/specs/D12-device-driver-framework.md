# D12 — Device Driver Framework

## Overview

A device driver is a piece of software that knows how to talk to a specific
piece of hardware. Without drivers, every program that wanted to read from a
disk would need to know the exact protocol for that specific disk model — the
register addresses, the timing requirements, the error codes. If you replaced
the disk with a different model, every program would break.

Device drivers solve this by providing a **uniform interface** over diverse
hardware. A program says "read 512 bytes from block 7" and the driver
translates that into whatever specific commands the hardware needs. The program
never knows (or cares) whether the disk is a spinning platter, a solid-state
drive, or a simulated in-memory disk image.

**Analogy:** Think of a universal remote control. You press "Volume Up" and it
works on your Samsung TV, your Sony soundbar, and your LG projector. Each
device speaks a different infrared protocol, but the remote translates your
single button press into the right signal for each device. Device drivers are
the universal remote for your operating system.

**Why build this now?** The S04 kernel currently talks directly to specific
simulated hardware (the display framebuffer, the keyboard buffer). This works
for two devices, but it does not scale. As we add disks, network interfaces,
and other peripherals, we need a structured way to register, discover, and
communicate with devices. The device driver framework provides that structure.

## Where It Fits

```
User Programs
│
│  sys_write(fd, buf, n)
│  sys_read(fd, buf, n)
▼
OS Kernel (S04)
│
│  "Which device does fd=1 refer to?"
│  "Oh, it's the display — a CharacterDevice."
│  "Call display_driver.write(buf, n)"
▼
Device Driver Framework (D12) ← YOU ARE HERE
│
│  Uniform interface:
│  ┌─────────────────────────────────────────────────┐
│  │  CharacterDevice    BlockDevice    NetworkDevice │
│  │  .read()            .read_block()  .send_packet()│
│  │  .write()           .write_block() .recv_packet()│
│  └─────────────────────────────────────────────────┘
│
│  Translates to hardware-specific operations:
▼
Simulated Hardware
├── Display framebuffer (S05)
├── Keyboard buffer (S03 ISR deposits keystrokes)
├── Disk image (in-memory byte array)
└── NIC (in-memory packet queues)
```

**Depends on:** S03 Interrupt Handler (devices raise interrupts on I/O
completion), S05 Display (SimulatedDisplay wraps the display driver)

**Used by:** S04 Kernel (syscall handlers look up devices in the registry),
future filesystem and networking layers

## Key Concepts

### The Three Families of Devices

Not all hardware behaves the same way. A keyboard produces one byte at a time,
whenever the user presses a key. A disk reads and writes fixed-size chunks. A
network card sends and receives variable-length packets. Trying to force all
three into a single interface would be awkward — so operating systems classify
devices into families, each with an interface that matches how the hardware
naturally operates.

```
Device Type       Data Model            Examples                How It Feels
──────────────────────────────────────────────────────────────────────────────
Character         Stream of bytes       Keyboard, serial port,  Like a pipe:
                  (one at a time)       mouse, display          bytes flow
                                        terminal                through

Block             Fixed-size chunks     Hard disk, SSD,         Like a filing
                  (random access)       USB drive, CD-ROM       cabinet: grab
                                                                any drawer

Network           Variable-length       Ethernet NIC,           Like a mailbox:
                  packets               WiFi adapter            send/receive
                                                                envelopes
```

**Why three types?** Each type has fundamentally different access patterns:

- **Character devices** are sequential. You read bytes in order. You cannot
  "seek" to byte 47 of a keyboard — that concept makes no sense. Data arrives
  when it arrives.

- **Block devices** are random-access. You can read block 0, then block 9999,
  then block 42 — in any order. Every block is the same size. This is essential
  for filesystems, which store files scattered across the disk.

- **Network devices** deal in packets — discrete messages with headers,
  addresses, and payloads. You do not read "byte 5 of the network" — you send
  and receive complete packets.

### Major and Minor Numbers

In Unix, every device is identified by two numbers:

```
Major number: identifies the DRIVER (which software module handles this device)
Minor number: identifies the INSTANCE (which specific device of that type)

Example:
  Major 8 = SCSI disk driver
  Minor 0 = first SCSI disk (/dev/sda)
  Minor 1 = second SCSI disk (/dev/sdb)

  Major 4 = serial port driver
  Minor 0 = first serial port (/dev/ttyS0)
  Minor 1 = second serial port (/dev/ttyS1)
```

This lets the kernel route I/O requests to the correct driver without knowing
anything about the hardware itself. The kernel just looks up the major number
to find the driver, and passes the minor number to the driver so it knows
which physical device to talk to.

For our simulation, we will use:

```
Major   Device Type         Minor   Instance
─────────────────────────────────────────────
1       Display (char)      0       Primary display
2       Keyboard (char)     0       Primary keyboard
3       Disk (block)        0       Primary disk
4       NIC (network)       0       Primary NIC
```

### Interrupt-Driven I/O

Devices do not instantly complete I/O operations. A disk read takes time (even
a simulated one could model latency). Rather than making the CPU spin in a loop
checking "is it done yet?" (polling), devices raise interrupts when they finish.

```
Without interrupts (polling):          With interrupts:
┌──────────────────────────┐           ┌──────────────────────────┐
│ CPU: "Read block 5"      │           │ CPU: "Read block 5"      │
│ CPU: "Done yet?" No.     │           │ CPU: (does other work)   │
│ CPU: "Done yet?" No.     │           │ CPU: (does other work)   │
│ CPU: "Done yet?" No.     │           │ CPU: (does other work)   │
│ CPU: "Done yet?" No.     │           │ *** INTERRUPT 34 ***     │
│ CPU: "Done yet?" Yes!    │           │ CPU: "Disk is done!"     │
│ CPU: "Now process data." │           │ CPU: "Process data."     │
└──────────────────────────┘           └──────────────────────────┘
     Wastes CPU time!                    CPU is productive!
```

Our interrupt assignments for devices:

```
Interrupt   Source             Description
──────────────────────────────────────────────
32          Timer              Timer tick (already assigned in S03)
33          Keyboard           Key pressed (already assigned in S03)
34          Disk               Block I/O operation completed
35          NIC                Packet received
128         Software           System call (already assigned in S03)
```

## Data Structures

### DeviceType Enum

```
DeviceType:
  Character = 0    # byte-stream devices
  Block     = 1    # fixed-size block devices
  Network   = 2    # packet-oriented devices
```

### DeviceBase (common fields for all devices)

Every device, regardless of type, has these core attributes:

```
DeviceBase:
┌──────────────────────────────────────────────────────────────────┐
│ name: string              # Human-readable name, e.g. "disk0"   │
│                           # Used for lookup and logging.        │
│                                                                  │
│ device_type: DeviceType   # Character, Block, or Network.       │
│                           # Determines which protocol the       │
│                           # device implements.                  │
│                                                                  │
│ major: int                # Driver identifier. All devices      │
│                           # handled by the same driver share    │
│                           # a major number.                     │
│                                                                  │
│ minor: int                # Instance identifier within the      │
│                           # driver. First disk = 0, second = 1. │
│                                                                  │
│ interrupt_number: int     # Which interrupt this device raises  │
│                           # when it needs attention. -1 if the  │
│                           # device does not use interrupts.     │
│                                                                  │
│ initialized: bool         # Has init() been called? Prevents    │
│                           # double-initialization and ensures   │
│                           # the device is ready before use.     │
└──────────────────────────────────────────────────────────────────┘
```

### CharacterDevice Protocol

Character devices extend DeviceBase and implement:

```
CharacterDevice:
  inherits DeviceBase (device_type = Character)

  read(buffer: byte_array, count: int) → int
    # Read up to `count` bytes into `buffer`.
    # Returns the number of bytes actually read.
    # Returns 0 if no data is available (non-blocking).
    #
    # Why return a count? Because the device might have fewer
    # bytes available than you asked for. A keyboard might have
    # only 3 keystrokes buffered when you asked for 10.

  write(buffer: byte_array, count: int) → int
    # Write up to `count` bytes from `buffer` to the device.
    # Returns the number of bytes actually written.
    # Returns -1 on error.
    #
    # For a display, this renders characters to the screen.
    # For a serial port, this sends bytes over the wire.

  init() → None
    # Initialize the device. Called once at boot.
    # For a keyboard: clear the input buffer.
    # For a display: clear the screen, set cursor to (0,0).
```

### BlockDevice Protocol

Block devices extend DeviceBase and implement:

```
BlockDevice:
  inherits DeviceBase (device_type = Block)

  block_size: int = 512     # Bytes per block. The standard sector
                            # size since the IBM PC/AT in 1984.
                            # Modern disks use 4096, but 512 is
                            # simpler and traditional.

  total_blocks: int         # How many blocks this device has.
                            # A 1 MB disk with 512-byte blocks
                            # has 2048 blocks.

  read_block(block_number: int) → bytes
    # Read exactly `block_size` bytes from block `block_number`.
    # Raises error if block_number >= total_blocks.
    #
    # Why whole blocks? Disks physically read whole sectors at a
    # time. Even if you only want 1 byte, the disk reads 512.
    # The OS caches the extra bytes for later. This is why
    # filesystems exist — to manage partial-block reads/writes
    # efficiently.

  write_block(block_number: int, data: bytes) → None
    # Write exactly `block_size` bytes to block `block_number`.
    # `data` must be exactly `block_size` bytes.
    # Raises error if block_number >= total_blocks.

  init() → None
    # Initialize the device.
    # For a simulated disk: allocate the backing byte array.
```

### NetworkDevice Protocol

Network devices extend DeviceBase and implement:

```
NetworkDevice:
  inherits DeviceBase (device_type = Network)

  mac_address: bytes[6]    # Media Access Control address.
                           # A 6-byte unique identifier, like a
                           # mailing address for the network card.
                           # Example: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]
                           #
                           # In real hardware, this is burned into
                           # the NIC at the factory. In simulation,
                           # we assign it at creation time.

  send_packet(data: bytes) → int
    # Send a packet over the network.
    # Returns the number of bytes sent, or -1 on error.
    #
    # In simulation, this pushes the packet onto a shared
    # "wire" (an in-memory queue) where other SimulatedNICs
    # can receive it.

  receive_packet() → bytes | None
    # Receive the next packet from the network.
    # Returns the packet data, or None if no packet is available.
    #
    # This is non-blocking: if nothing has arrived, it returns
    # immediately with None rather than waiting.

  init() → None
    # Initialize the device.
    # For a simulated NIC: clear the send/receive queues,
    # connect to the shared wire.
```

### DeviceRegistry

The registry is the kernel's phonebook for devices. When a driver initializes
a device, it registers it here. When the kernel needs to perform I/O, it looks
up the device here.

```
DeviceRegistry:
┌──────────────────────────────────────────────────────────────────┐
│ devices_by_name: map[string → Device]                            │
│   # Fast lookup by name: "disk0" → SimulatedDisk instance       │
│                                                                  │
│ devices_by_major_minor: map[(int,int) → Device]                  │
│   # Fast lookup by major/minor: (3,0) → SimulatedDisk instance  │
│                                                                  │
│ all_devices: list[Device]                                        │
│   # Ordered list of all registered devices. Useful for           │
│   # enumeration (e.g., "list all block devices").                │
└──────────────────────────────────────────────────────────────────┘

Methods:

  register(device: Device) → None
    # Add a device to the registry.
    # Raises error if a device with the same name already exists.
    # Raises error if a device with the same (major, minor) exists.
    # The device must be initialized before registration.

  lookup_by_name(name: string) → Device | None
    # Find a device by its human-readable name.
    # Returns None if not found.

  lookup_by_major_minor(major: int, minor: int) → Device | None
    # Find a device by its major/minor number pair.
    # Returns None if not found.

  list_all() → list[Device]
    # Return all registered devices.

  list_by_type(device_type: DeviceType) → list[Device]
    # Return all devices of a specific type.
    # E.g., list_by_type(Block) returns all disk-like devices.
```

## Concrete Implementations

### SimulatedDisk

Wraps an in-memory byte array to simulate a block storage device. This is the
"hard drive" for our simulated computer.

```
SimulatedDisk:
  inherits BlockDevice

  Fields:
    name = "disk0"
    device_type = Block
    major = 3, minor = 0
    interrupt_number = 34
    block_size = 512
    total_blocks = 2048          # 1 MB disk (2048 × 512 bytes)
    storage: bytes[1_048_576]    # The backing store — a 1 MB byte array

  read_block(n):
    offset = n * block_size
    return storage[offset : offset + block_size]

  write_block(n, data):
    assert len(data) == block_size
    offset = n * block_size
    storage[offset : offset + block_size] = data
```

**How it integrates with the DiskImage concept:** If a ROM/BIOS package (S01)
already provides a DiskImage abstraction, SimulatedDisk wraps it. The DiskImage
provides the raw storage; SimulatedDisk provides the driver interface.

### SimulatedKeyboard

Wraps the keyboard input buffer (populated by the keyboard ISR in S03) to
present it as a CharacterDevice.

```
SimulatedKeyboard:
  inherits CharacterDevice

  Fields:
    name = "keyboard0"
    device_type = Character
    major = 2, minor = 0
    interrupt_number = 33
    buffer: queue[byte]          # Filled by keyboard ISR (interrupt 33)

  read(buf, count):
    bytes_read = 0
    while bytes_read < count and not buffer.empty():
      buf[bytes_read] = buffer.dequeue()
      bytes_read += 1
    return bytes_read            # May be 0 if no keys were pressed

  write(buf, count):
    return -1                    # Cannot write to a keyboard!
```

### SimulatedDisplay

Wraps the S05 display driver to present it as a CharacterDevice.

```
SimulatedDisplay:
  inherits CharacterDevice

  Fields:
    name = "display0"
    device_type = Character
    major = 1, minor = 0
    interrupt_number = -1        # Display does not generate interrupts
    display_driver: reference to S05 Display

  read(buf, count):
    return -1                    # Cannot read from a display!

  write(buf, count):
    for i in 0..count:
      display_driver.put_char(buf[i])
    return count

  init():
    display_driver.clear_screen()
```

### SimulatedNIC

A network interface card backed by in-memory packet queues. Two SimulatedNICs
connected to the same "wire" (shared queue) can exchange packets.

```
SimulatedNIC:
  inherits NetworkDevice

  Fields:
    name = "nic0"
    device_type = Network
    major = 4, minor = 0
    interrupt_number = 35
    mac_address = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01]
    rx_queue: queue[bytes]       # Packets waiting to be received
    wire: reference to SharedWire

  send_packet(data):
    wire.broadcast(data, sender=self)
    return len(data)

  receive_packet():
    if rx_queue.empty():
      return None
    return rx_queue.dequeue()

SharedWire:
  # A simulated network cable connecting multiple NICs.
  # When one NIC sends, all other NICs on the wire receive.
  connected_nics: list[SimulatedNIC]

  broadcast(data, sender):
    for nic in connected_nics:
      if nic != sender:
        nic.rx_queue.enqueue(data)
        # Raise interrupt 35 on the receiving NIC's CPU
        # so the kernel knows a packet arrived.
```

## Algorithms

### Device Registration Flow

When the system boots, the kernel (or BIOS) creates and registers devices:

```
Boot sequence:
  1. Create SimulatedDisplay(display_driver)
  2. display.init()
  3. registry.register(display)          # Now accessible as "display0"
  4. Create SimulatedKeyboard()
  5. keyboard.init()
  6. registry.register(keyboard)         # Now accessible as "keyboard0"
  7. Create SimulatedDisk(total_blocks=2048)
  8. disk.init()
  9. registry.register(disk)             # Now accessible as "disk0"
  10. Create SimulatedNIC(wire)
  11. nic.init()
  12. registry.register(nic)             # Now accessible as "nic0"
```

### I/O Request Dispatch

When a user program makes a syscall like `sys_write(fd, buf, count)`:

```
sys_write(fd=1, buf=0x1000, count=5):
  1. Kernel looks up fd=1 in process's file descriptor table
     → fd 1 = "display0"
  2. Kernel calls registry.lookup_by_name("display0")
     → returns SimulatedDisplay instance
  3. Kernel verifies device_type == Character
     → yes, so .write() is available
  4. Kernel calls display.write(buf, count)
     → SimulatedDisplay.write() calls display_driver.put_char() 5 times
  5. Kernel returns 5 to user program (5 bytes written)
```

### Interrupt-Driven Block I/O

For block devices, the flow involves interrupts:

```
User: sys_read(disk_fd, buf, 512)
  │
  ▼
Kernel: disk = registry.lookup("disk0")
        data = disk.read_block(block_number)
  │
  ▼
SimulatedDisk: reads from storage[] array
               raises interrupt 34 (I/O complete)
  │
  ▼
Interrupt Handler: delivers interrupt 34
  │
  ▼
Kernel ISR: marks I/O as complete,
            copies data to user buffer,
            wakes up the waiting process
```

Note: In our simulation, disk reads are instantaneous (they just index into an
array). The interrupt mechanism is included for educational completeness — real
disks have measurable latency, and the interrupt tells the CPU "the data is
ready now."

## Syscalls

This package introduces **no new syscalls**. It is pure infrastructure — a set
of interfaces and implementations that the existing syscalls (`sys_read`,
`sys_write`) use internally. The kernel's syscall handlers are updated to route
through the device registry instead of directly calling display/keyboard code.

Before D12:
```
sys_write → kernel directly calls display_driver.put_char()
```

After D12:
```
sys_write → kernel calls registry.lookup("display0").write()
            → SimulatedDisplay.write() calls display_driver.put_char()
```

The extra indirection costs almost nothing but gains extensibility: adding a new
device type requires implementing the protocol and registering it — no kernel
code changes needed.

## Dependencies

```
D12 Device Driver Framework
│
├── depends on: S03 Interrupt Handler
│   # Devices raise interrupts (34=disk, 35=NIC).
│   # The framework registers ISRs for device interrupts.
│
├── depends on: S05 Display
│   # SimulatedDisplay wraps the S05 display driver.
│
└── used by: S04 OS Kernel
    # Kernel routes I/O through the DeviceRegistry.
    # Syscall handlers use Device protocols instead of
    # direct hardware access.
```

## Testing Strategy

### Unit Tests

1. **DeviceType enum:** Verify all three types exist and have distinct values.

2. **DeviceBase fields:** Create a device with all fields, verify they are
   stored and retrievable correctly.

3. **SimulatedDisk:**
   - `read_block(0)` on a fresh disk returns all zeros.
   - `write_block(5, data)` followed by `read_block(5)` returns `data`.
   - `write_block(total_blocks)` raises an out-of-bounds error.
   - `write_block(3, short_data)` raises an error (data not block_size).
   - Verify `block_size` is 512 and `total_blocks` matches configuration.

4. **SimulatedKeyboard:**
   - `read()` with empty buffer returns 0 bytes.
   - Enqueue bytes, then `read()` returns them in FIFO order.
   - `read()` with count > buffered bytes returns only available bytes.
   - `write()` returns -1 (keyboards are read-only).

5. **SimulatedDisplay:**
   - `write([0x48, 0x69], 2)` calls `put_char('H')` then `put_char('i')`.
   - `read()` returns -1 (displays are write-only).
   - `init()` calls `clear_screen()`.

6. **SimulatedNIC:**
   - `receive_packet()` with empty queue returns None.
   - `send_packet(data)` on NIC A appears in NIC B's rx_queue (via wire).
   - Packets are not echoed back to the sender.
   - `mac_address` is exactly 6 bytes.

7. **DeviceRegistry:**
   - `register()` + `lookup_by_name()` round-trips.
   - `register()` + `lookup_by_major_minor()` round-trips.
   - Duplicate name registration raises error.
   - Duplicate (major, minor) registration raises error.
   - `list_all()` returns all registered devices.
   - `list_by_type(Block)` returns only block devices.
   - `lookup_by_name("nonexistent")` returns None.

### Integration Tests

8. **Full I/O path:** Register a SimulatedDisplay, simulate a `sys_write`
   through the registry, verify characters appear in the display framebuffer.

9. **Interrupt path:** Register a SimulatedDisk with interrupt 34. Perform a
   read_block. Verify interrupt 34 is raised and handled by the interrupt
   handler.

10. **Network roundtrip:** Create two NICs on the same wire. Send a packet from
    NIC A. Verify NIC B receives it. Verify NIC A does not receive its own
    packet.

### Coverage Target

Target: 95%+ line coverage across all device types and the registry. The
protocols are simple enough that exhaustive testing is feasible.
