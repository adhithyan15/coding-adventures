-- ============================================================================
-- device_driver_framework — Device Driver Abstraction Framework
-- ============================================================================
--
-- A device driver is software that knows how to talk to a specific piece of
-- hardware.  Without drivers, every program that wanted to read from a disk
-- would need to know that disk's exact register addresses, timing requirements,
-- and error codes.  Different disk brands would require completely different
-- code.
--
-- Device drivers provide a **uniform interface** over diverse hardware.  A
-- program says "read 512 bytes from block 7" and the driver translates that
-- into whatever specific commands the hardware needs.  The program never knows
-- whether the disk is a spinning platter, an SSD, or an in-memory simulation.
--
-- ## Analogy: The Universal Remote
--
-- A universal remote has one "Volume Up" button that works on your Samsung TV,
-- Sony soundbar, and LG projector.  Device drivers are the universal remote
-- for your operating system: one API ("read", "write", "ioctl") works for
-- keyboards, disks, and network cards.
--
-- ## Three Device Families
--
-- Not all hardware behaves the same way:
--
--   +------------------+-------------------+----------------------------------+
--   | Family           | Model             | Examples                         |
--   +------------------+-------------------+----------------------------------+
--   | Character device | Byte stream       | Keyboard, serial port, display   |
--   | Block device     | Fixed-size chunks | Hard disk, SSD, flash drive      |
--   | Network device   | Packets           | Ethernet NIC, Wi-Fi card         |
--   +------------------+-------------------+----------------------------------+
--
-- ## Major and Minor Numbers
--
-- Unix identifies devices by two numbers:
--
--   major — which *type* of device (disk=3, character=4, network=5, etc.)
--   minor — which *instance* (disk0, disk1, disk2…)
--
-- /dev/sda has major=8, minor=0.  /dev/sdb has major=8, minor=16.
--
-- ## Driver Lifecycle
--
--   1. Register:   add driver to DeviceRegistry with major+minor
--   2. Initialize: driver sets up hardware, allocates buffers
--   3. Open:       a process opens the device (/dev/sda)
--   4. Read/Write: the process transfers data
--   5. Ioctl:      optional control commands (e.g., set baud rate)
--   6. Close:      the process closes the device
--
-- ## Module Structure
--
--   device_driver_framework
--   ├── DeviceTypes     — :character, :block, :network
--   ├── SimulatedDisk   — in-memory block device
--   ├── SimulatedSerial — in-memory character device (serial port)
--   ├── SimulatedNIC    — in-memory network device
--   └── Registry        — register, lookup, open/close/read/write/ioctl
--
-- ============================================================================

local M = {}

-- ============================================================================
-- Device Types
-- ============================================================================

M.TYPE_CHARACTER = 0   -- byte stream (keyboard, serial, display)
M.TYPE_BLOCK     = 1   -- fixed-size blocks (disk)
M.TYPE_NETWORK   = 2   -- packets (NIC)

-- Default device parameters
M.DEFAULT_BLOCK_SIZE   = 512    -- bytes per disk sector (IBM PC/AT standard since 1984)
M.DEFAULT_TOTAL_BLOCKS = 2048   -- 2048 × 512 = 1 MB disk
M.DEFAULT_BUFFER_SIZE  = 4096   -- serial port ring buffer

-- ============================================================================
-- SimulatedDisk — Block Device
-- ============================================================================
--
-- Wraps an in-memory table of bytes to simulate a block storage device.
--
-- ### Block Layout
--
--   ┌────────────┬────────────┬────────────┬─────┐
--   │  Block 0   │  Block 1   │  Block 2   │ ... │   total_blocks blocks
--   │ 512 bytes  │ 512 bytes  │ 512 bytes  │     │   each block_size bytes
--   └────────────┴────────────┴────────────┴─────┘
--   ▲                                             ▲
--   address 0                        address total_blocks × block_size - 1
--
-- read_block(n)    → bytes at offset n × block_size
-- write_block(n, data) → replaces bytes at that offset
--
-- Default configuration:
--   block_size   = 512 bytes  (standard sector size)
--   total_blocks = 2048       (giving a 1 MB disk)
--   major        = 3          (Linux disk major number convention)
--   minor        = 0          (first disk)

M.SimulatedDisk = {}
M.SimulatedDisk.__index = M.SimulatedDisk

--- Create a new simulated disk.
-- @param opts  Table with optional fields:
--   name         (string, default "disk0")
--   major        (number, default 3)
--   minor        (number, default 0)
--   block_size   (number, default 512)
--   total_blocks (number, default 2048)
function M.SimulatedDisk.new(opts)
  opts = opts or {}
  local bs    = opts.block_size   or M.DEFAULT_BLOCK_SIZE
  local total = opts.total_blocks or M.DEFAULT_TOTAL_BLOCKS
  local storage = {}
  for i = 1, bs * total do storage[i] = 0 end
  return setmetatable({
    name             = opts.name  or "disk0",
    device_type      = M.TYPE_BLOCK,
    major            = opts.major or 3,
    minor            = opts.minor or 0,
    interrupt_number = opts.interrupt_number or 34,
    initialized      = false,
    block_size        = bs,
    total_blocks      = total,
    storage           = storage,
    open_count        = 0,
  }, M.SimulatedDisk)
end

local function copy_disk(d)
  local s = {}
  for i, v in ipairs(d.storage) do s[i] = v end
  return setmetatable({
    name             = d.name,
    device_type      = d.device_type,
    major            = d.major,
    minor            = d.minor,
    interrupt_number = d.interrupt_number,
    initialized      = d.initialized,
    block_size        = d.block_size,
    total_blocks      = d.total_blocks,
    storage           = s,
    open_count        = d.open_count,
  }, M.SimulatedDisk)
end

--- Initialize the device (must be called before open/read/write).
function M.SimulatedDisk:initialize()
  local d = copy_disk(self)
  d.initialized = true
  return "ok", d
end

--- Open the device (increment open count).
function M.SimulatedDisk:open()
  if not self.initialized then return "not_initialized", self end
  local d = copy_disk(self)
  d.open_count = d.open_count + 1
  return "ok", d
end

--- Close the device (decrement open count).
function M.SimulatedDisk:close()
  if self.open_count <= 0 then return "not_open", self end
  local d = copy_disk(self)
  d.open_count = math.max(0, d.open_count - 1)
  return "ok", d
end

--- Read a block of data.
-- @param block_num  Block number (0-based)
-- @return "ok", updated_disk, data_string  OR  "out_of_bounds", disk, nil
function M.SimulatedDisk:read_block(block_num)
  if block_num < 0 or block_num >= self.total_blocks then
    return "out_of_bounds", self, nil
  end
  local offset = block_num * self.block_size
  local result = {}
  for i = 1, self.block_size do
    result[i] = string.char(self.storage[offset + i])
  end
  return "ok", self, table.concat(result)
end

--- Write a block of data.
-- @param block_num  Block number (0-based)
-- @param data       String of bytes (must be block_size bytes)
-- @return "ok", updated_disk  OR  "out_of_bounds"/"wrong_size", disk
function M.SimulatedDisk:write_block(block_num, data)
  if block_num < 0 or block_num >= self.total_blocks then
    return "out_of_bounds", self
  end
  if #data ~= self.block_size then
    return "wrong_size", self
  end
  local d = copy_disk(self)
  local offset = block_num * d.block_size
  for i = 1, d.block_size do
    d.storage[offset + i] = string.byte(data, i)
  end
  return "ok", d
end

--- Ioctl — control commands.
-- Supported commands: "get_block_size", "get_total_blocks"
function M.SimulatedDisk:ioctl(cmd, _arg)
  if cmd == "get_block_size"   then return "ok", self.block_size   end
  if cmd == "get_total_blocks" then return "ok", self.total_blocks end
  return "unsupported", nil
end

-- ============================================================================
-- SimulatedSerial — Character Device
-- ============================================================================
--
-- A character device produces or consumes a stream of bytes.  This simulates
-- a serial port (UART): bytes written to the TX (transmit) buffer appear in
-- the TX log; bytes placed in the RX (receive) buffer can be read by a process.
--
-- ### Ring Buffer
--
--   TX buffer (bytes written by process → sent to hardware/log)
--   RX buffer (bytes produced by hardware → read by process)
--
-- Baud rate is the speed of the serial link in bits per second.
-- 9600 baud = 9600 bits/s ≈ 960 bytes/s.

M.SimulatedSerial = {}
M.SimulatedSerial.__index = M.SimulatedSerial

--- Create a new simulated serial port.
function M.SimulatedSerial.new(opts)
  opts = opts or {}
  return setmetatable({
    name             = opts.name  or "serial0",
    device_type      = M.TYPE_CHARACTER,
    major            = opts.major or 4,
    minor            = opts.minor or 0,
    interrupt_number = opts.interrupt_number or 33,
    initialized      = false,
    baud_rate        = opts.baud_rate or 9600,
    tx_buffer        = {},   -- bytes written by process
    rx_buffer        = {},   -- bytes to be read by process
    open_count       = 0,
  }, M.SimulatedSerial)
end

local function copy_serial(s)
  local tx = {}
  for _, v in ipairs(s.tx_buffer) do table.insert(tx, v) end
  local rx = {}
  for _, v in ipairs(s.rx_buffer) do table.insert(rx, v) end
  return setmetatable({
    name             = s.name,
    device_type      = s.device_type,
    major            = s.major,
    minor            = s.minor,
    interrupt_number = s.interrupt_number,
    initialized      = s.initialized,
    baud_rate        = s.baud_rate,
    tx_buffer        = tx,
    rx_buffer        = rx,
    open_count       = s.open_count,
  }, M.SimulatedSerial)
end

function M.SimulatedSerial:initialize()
  local s = copy_serial(self)
  s.initialized = true
  return "ok", s
end

function M.SimulatedSerial:open()
  if not self.initialized then return "not_initialized", self end
  local s = copy_serial(self)
  s.open_count = s.open_count + 1
  return "ok", s
end

function M.SimulatedSerial:close()
  if self.open_count <= 0 then return "not_open", self end
  local s = copy_serial(self)
  s.open_count = math.max(0, s.open_count - 1)
  return "ok", s
end

--- Write bytes to the TX buffer.
function M.SimulatedSerial:write(data)
  local s = copy_serial(self)
  for i = 1, #data do
    table.insert(s.tx_buffer, string.byte(data, i))
  end
  return "ok", s, #data
end

--- Read up to max_bytes bytes from the RX buffer.
function M.SimulatedSerial:read(max_bytes)
  if #self.rx_buffer == 0 then
    return "empty", self, ""
  end
  local s = copy_serial(self)
  local to_read = math.min(max_bytes, #s.rx_buffer)
  local result = {}
  for i = 1, to_read do
    result[i] = string.char(table.remove(s.rx_buffer, 1))
  end
  return "ok", s, table.concat(result)
end

--- Inject bytes into the RX buffer (simulates hardware sending data).
function M.SimulatedSerial:inject_rx(data)
  local s = copy_serial(self)
  for i = 1, #data do
    table.insert(s.rx_buffer, string.byte(data, i))
  end
  return s
end

--- Get TX buffer as a string (what the device "sent").
function M.SimulatedSerial:tx_contents()
  local result = {}
  for i, b in ipairs(self.tx_buffer) do result[i] = string.char(b) end
  return table.concat(result)
end

--- Ioctl: get/set baud rate.
function M.SimulatedSerial:ioctl(cmd, arg)
  if cmd == "get_baud_rate" then return "ok", self.baud_rate end
  if cmd == "set_baud_rate" then
    local s = copy_serial(self)
    s.baud_rate = arg
    return "ok", s
  end
  return "unsupported", nil
end

-- ============================================================================
-- SimulatedNIC — Network Device
-- ============================================================================
--
-- A network device sends and receives packets.  This simulation holds:
--   - tx_queue: packets enqueued for transmission
--   - rx_queue: packets received from the (simulated) network
--
-- In real hardware, a DMA engine would copy packets to/from memory directly.
-- Here, we simply append to and pop from lists.

M.SimulatedNIC = {}
M.SimulatedNIC.__index = M.SimulatedNIC

function M.SimulatedNIC.new(opts)
  opts = opts or {}
  return setmetatable({
    name             = opts.name  or "eth0",
    device_type      = M.TYPE_NETWORK,
    major            = opts.major or 5,
    minor            = opts.minor or 0,
    interrupt_number = opts.interrupt_number or 35,
    initialized      = false,
    mac_address      = opts.mac_address or { 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF },
    tx_queue         = {},
    rx_queue         = {},
    open_count       = 0,
  }, M.SimulatedNIC)
end

local function copy_nic(n)
  local tx = {}
  for _, v in ipairs(n.tx_queue) do table.insert(tx, v) end
  local rx = {}
  for _, v in ipairs(n.rx_queue) do table.insert(rx, v) end
  local mac = {}
  for _, v in ipairs(n.mac_address) do table.insert(mac, v) end
  return setmetatable({
    name             = n.name,
    device_type      = n.device_type,
    major            = n.major,
    minor            = n.minor,
    interrupt_number = n.interrupt_number,
    initialized      = n.initialized,
    mac_address      = mac,
    tx_queue         = tx,
    rx_queue         = rx,
    open_count       = n.open_count,
  }, M.SimulatedNIC)
end

function M.SimulatedNIC:initialize()
  local n = copy_nic(self)
  n.initialized = true
  return "ok", n
end

function M.SimulatedNIC:open()
  if not self.initialized then return "not_initialized", self end
  local n = copy_nic(self)
  n.open_count = n.open_count + 1
  return "ok", n
end

function M.SimulatedNIC:close()
  if self.open_count <= 0 then return "not_open", self end
  local n = copy_nic(self)
  n.open_count = math.max(0, n.open_count - 1)
  return "ok", n
end

--- Send (enqueue for TX) a packet (list of bytes).
function M.SimulatedNIC:send(packet)
  local n = copy_nic(self)
  table.insert(n.tx_queue, packet)
  return "ok", n
end

--- Receive a packet from the RX queue.
-- Returns "ok", updated_nic, packet  OR  "empty", self, nil
function M.SimulatedNIC:receive()
  if #self.rx_queue == 0 then return "empty", self, nil end
  local n = copy_nic(self)
  local pkt = table.remove(n.rx_queue, 1)
  return "ok", n, pkt
end

--- Inject a packet into the RX queue (simulates incoming network traffic).
function M.SimulatedNIC:inject_rx(packet)
  local n = copy_nic(self)
  table.insert(n.rx_queue, packet)
  return n
end

--- Ioctl: get MAC address.
function M.SimulatedNIC:ioctl(cmd, _arg)
  if cmd == "get_mac" then return "ok", self.mac_address end
  return "unsupported", nil
end

-- ============================================================================
-- Registry — Device Registry
-- ============================================================================
--
-- The Registry is the kernel's catalog of installed devices.  Devices are
-- registered by their (major, minor) pair.  Higher-level code looks up a
-- device by name or by (major, minor) and then uses the standard interface.
--
-- Fields:
--   devices        Map of name → device object
--   by_major_minor Map of "major:minor" → name

M.Registry = {}
M.Registry.__index = M.Registry

function M.Registry.new()
  return setmetatable({
    devices        = {},
    by_major_minor = {},
  }, M.Registry)
end

local function copy_registry(r)
  local d = {}
  for k, v in pairs(r.devices) do d[k] = v end
  local m = {}
  for k, v in pairs(r.by_major_minor) do m[k] = v end
  return setmetatable({ devices = d, by_major_minor = m }, M.Registry)
end

--- Register a device.
-- @param device  A SimulatedDisk, SimulatedSerial, or SimulatedNIC
-- @return "ok", updated_registry  OR  "already_registered", self
function M.Registry:register(device)
  local key = device.major .. ":" .. device.minor
  if self.devices[device.name] or self.by_major_minor[key] then
    return "already_registered", self
  end
  local r = copy_registry(self)
  r.devices[device.name] = device
  r.by_major_minor[key]  = device.name
  return "ok", r
end

--- Look up a device by name.
function M.Registry:get(name)
  local d = self.devices[name]
  if d then return "ok", d end
  return "not_found", nil
end

--- Look up a device by (major, minor).
function M.Registry:get_by_major_minor(major, minor)
  local key  = major .. ":" .. minor
  local name = self.by_major_minor[key]
  if not name then return "not_found", nil end
  return "ok", self.devices[name]
end

--- Update a device in the registry (after state changes from read/write).
function M.Registry:update(name, device)
  if not self.devices[name] then return "not_found", self end
  local r = copy_registry(self)
  r.devices[name] = device
  return "ok", r
end

--- Unregister a device.
function M.Registry:unregister(name)
  if not self.devices[name] then return "not_found", self end
  local device = self.devices[name]
  local key = device.major .. ":" .. device.minor
  local r = copy_registry(self)
  r.devices[name] = nil
  r.by_major_minor[key] = nil
  return "ok", r
end

--- List all registered device names.
function M.Registry:list()
  local names = {}
  for name, _ in pairs(self.devices) do table.insert(names, name) end
  table.sort(names)
  return names
end

return M
