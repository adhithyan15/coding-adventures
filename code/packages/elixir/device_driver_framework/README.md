# Device Driver Framework (Elixir)

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
| Character | Byte stream | Keyboard, display | `read/2`, `write/2` |
| Block | Fixed-size chunks | Disk, SSD | `read_block/2`, `write_block/3` |
| Network | Packets | Ethernet NIC | `send_packet/3`, `receive_packet/1` |

## Usage

```elixir
alias CodingAdventures.DeviceDriverFramework.{
  SimulatedDisk, SimulatedKeyboard, SimulatedDisplay,
  SimulatedNIC, DeviceRegistry
}

# Create and initialize devices
disk = SimulatedDisk.new(total_blocks: 2048) |> SimulatedDisk.init()
kb = SimulatedKeyboard.new() |> SimulatedKeyboard.init()

# Register in the device registry
reg = DeviceRegistry.new()
{:ok, reg} = DeviceRegistry.register(reg, disk)
{:ok, reg} = DeviceRegistry.register(reg, kb)

# Look up by name
device = DeviceRegistry.lookup_by_name(reg, "disk0")

# Look up by major/minor
device = DeviceRegistry.lookup_by_major_minor(reg, 3, 0)

# List all block devices
disks = DeviceRegistry.list_by_type(reg, :block)
```

## Functional Style

Unlike the TypeScript implementation, Elixir uses immutable data. Device operations return new structs rather than mutating in place:

```elixir
# Disk operations return updated disk
{:ok, disk} = SimulatedDisk.write_block(disk, 0, data)
{:ok, block} = SimulatedDisk.read_block(disk, 0)

# Keyboard reads return {data, updated_keyboard}
{data, kb} = SimulatedKeyboard.read(kb, 10)
```

## Simulated Devices

- **SimulatedDisk** -- In-memory block device (default 1 MB, 512-byte blocks)
- **SimulatedKeyboard** -- FIFO byte buffer, interrupt 33
- **SimulatedDisplay** -- 80x25 framebuffer with cursor tracking
- **SimulatedNIC** -- Packet queues with broadcast via functional style

## Running Tests

```bash
mix deps.get
mix test
```
