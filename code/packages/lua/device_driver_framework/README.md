# device_driver_framework (Lua)

Device driver abstraction framework for the coding-adventures simulated OS.

## What It Does

Implements a uniform device driver interface for three hardware families:

- **SimulatedDisk** — block device (in-memory disk with 512-byte sectors)
- **SimulatedSerial** — character device (serial port with TX/RX buffers)
- **SimulatedNIC** — network device (Ethernet card with TX/RX packet queues)
- **Registry** — kernel device catalog indexed by name and (major, minor) number

## Usage

```lua
local DDF = require("coding_adventures.device_driver_framework")

-- Register and use a disk
local disk = DDF.SimulatedDisk.new({ block_size = 512 })
local _, disk2 = disk:initialize()
local _, disk3 = disk2:open()
local block_data = string.rep("\0", 512)  -- one zero-filled block
local _, disk4 = disk3:write_block(0, block_data)

-- Registry
local reg = DDF.Registry.new()
local _, reg2 = reg:register(disk4)
local _, found = reg2:get("disk0")
```

## Device Lifecycle

```
register() → initialize() → open() → read()/write()/ioctl() → close()
```

## Major/Minor Numbers

| Device    | Major | Minor |
|-----------|-------|-------|
| Disk      | 3     | 0+    |
| Serial    | 4     | 0+    |
| NIC       | 5     | 0+    |
