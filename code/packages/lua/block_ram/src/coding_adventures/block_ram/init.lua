-- block_ram — SRAM Cells, Arrays, and RAM Modules
-- ================================================
--
-- This package implements the memory building blocks that every CPU, cache,
-- and FPGA uses to store data. It builds a complete memory hierarchy from
-- the lowest level (a single-bit SRAM cell) up to configurable Block RAM
-- with dual-port access and reconfigurable aspect ratios.
--
-- # What is SRAM?
--
-- SRAM (Static Random-Access Memory) is the fastest type of memory in a
-- computer. It is used for CPU caches (L1/L2/L3), register files, and
-- FPGA Block RAM. "Static" means the memory holds its value as long as
-- power is supplied — unlike DRAM, which must be periodically refreshed.
--
-- # The Memory Hierarchy Built Here
--
--   1. SRAMCell   — a single bit of storage (6-transistor model)
--   2. SRAMArray  — a 2D grid of cells with row/column addressing
--   3. SinglePortRAM — synchronous RAM with one read/write port
--   4. DualPortRAM   — synchronous RAM with two independent ports
--   5. ConfigurableBRAM — FPGA-style Block RAM with aspect ratio control
--
-- # Dependencies
--
-- This package does NOT import logic-gates at runtime. The Go version
-- similarly duplicates the `validateBit` helper to avoid importing
-- internal helpers. The conceptual dependency is on logic-gate-level
-- thinking (AND, NOT, inverter loops) but the simulation models
-- steady-state behavior directly for performance.
--
-- This is Layer 8 of the coding-adventures computing stack.

local block_ram = {}
block_ram.VERSION = "0.1.0"

-- =========================================================================
-- Read Modes — What data_out Shows During a Write
-- =========================================================================
--
-- During a write operation, what should the data output show? There are
-- three valid answers, and different hardware designs need different
-- behaviors:
--
--   READ_FIRST (read-before-write):
--     Output shows the OLD value at the address being written. The read
--     happens before the write within the same cycle. Useful when you
--     need to know what was there before overwriting it.
--
--   WRITE_FIRST (read-after-write):
--     Output shows the NEW value being written. The write happens first,
--     then the read sees the new data. Useful for pipeline forwarding
--     where downstream stages need the freshest data immediately.
--
--   NO_CHANGE:
--     Output retains its previous value during writes. The read circuitry
--     does not activate at all during writes. This saves power in FPGA
--     Block RAMs because fewer transistors switch.

block_ram.READ_FIRST  = 1
block_ram.WRITE_FIRST = 2
block_ram.NO_CHANGE   = 3


-- =========================================================================
-- Input Validation
-- =========================================================================

--- Validate that a value is a binary digit (0 or 1).
--
-- In digital electronics, a "bit" is a signal that is either LOW (0) or
-- HIGH (1). Anything else is meaningless. Real hardware enforces this
-- through voltage thresholds; we enforce it with a runtime check.
--
-- @param value number The value to check.
-- @param name string  A human-readable name for error messages.
local function validate_bit(value, name)
    if value ~= 0 and value ~= 1 then
        error(string.format("block_ram: %s must be 0 or 1, got %s",
                            name, tostring(value)), 2)
    end
end


-- =========================================================================
-- SRAMCell — Single-Bit Storage
-- =========================================================================
--
-- # The 6-Transistor SRAM Cell
--
-- In real hardware, each SRAM cell uses 6 transistors:
--   - 2 cross-coupled inverters forming a bistable latch (stores the bit)
--   - 2 access transistors controlled by the word line (gates read/write)
--
-- At the gate level:
--   - Cross-coupled inverters = two NOT gates in a feedback loop
--   - Access transistors = AND gates that pass data only when word_line=1
--
-- The cell has three modes of operation:
--
--   Hold  (word_line=0): Access transistors block external access.
--         The inverter loop maintains the stored value indefinitely.
--
--   Read  (word_line=1): Access transistors open. The stored value
--         appears on the bit lines without disturbing it.
--
--   Write (word_line=1, drive bit lines): The external driver
--         overpowers the internal inverters, forcing a new value.
--
-- We model the steady-state behavior directly rather than simulating
-- individual gate delays. This matches real behavior while keeping the
-- simulation fast enough to model arrays of thousands of cells.

local SRAMCell = {}
SRAMCell.__index = SRAMCell

--- Create a new SRAM cell initialized to 0.
--
-- The initial state of 0 represents the cell after power-on reset.
-- In real hardware, SRAM cells power up in an indeterminate state,
-- but we initialize to 0 for predictability in simulation.
--
-- @return SRAMCell A new SRAM cell.
function SRAMCell.new()
    return setmetatable({ value = 0 }, SRAMCell)
end

--- Read the stored bit if the cell is selected (word_line=1).
--
-- When word_line=0 (cell not selected), returns nil — the cell's access
-- transistors are closed, so no output appears on the bit line.
-- When word_line=1, returns the stored value.
--
-- @param word_line number 0 or 1.
-- @return number|nil The stored bit, or nil if word_line=0.
function SRAMCell:read(word_line)
    validate_bit(word_line, "word_line")
    if word_line == 0 then
        return nil
    end
    return self.value
end

--- Write a bit to the cell if selected (word_line=1).
--
-- When word_line=1, the access transistors open and the external bit_line
-- driver overpowers the internal inverter loop, forcing the cell to store
-- the new value.
--
-- When word_line=0, the access transistors are closed and the write has
-- no effect — the cell retains its previous value.
--
-- @param word_line number 0 or 1.
-- @param bit_line number  0 or 1 — the value to store.
function SRAMCell:write(word_line, bit_line)
    validate_bit(word_line, "word_line")
    validate_bit(bit_line, "bit_line")
    if word_line == 1 then
        self.value = bit_line
    end
end

--- Return the current stored value (for inspection/debugging).
--
-- @return number The stored bit (0 or 1).
function SRAMCell:get_value()
    return self.value
end

block_ram.SRAMCell = SRAMCell


-- =========================================================================
-- SRAMArray — 2D Grid of SRAM Cells
-- =========================================================================
--
-- An SRAM array organizes cells into rows and columns:
--   - Each row shares a word line (activated by the row decoder)
--   - Each column shares a bit line (carries data in/out)
--
-- To read: activate a row's word line -> all cells in that row
-- output their values onto their respective bit lines.
--
-- To write: activate a row's word line and drive the bit lines
-- with the desired data -> all cells in that row store the new values.
--
-- Memory map (4x4 array example):
--
--   Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
--   Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
--   Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
--   Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]
--
-- Note on indexing: Lua tables are 1-based, so rows are 1..rows and
-- columns are 1..cols internally. The public API uses 0-based addresses
-- to match hardware conventions, and we translate internally.

local SRAMArray = {}
SRAMArray.__index = SRAMArray

--- Create an SRAM array initialized to all zeros.
--
-- @param rows number Number of rows (words). Must be >= 1.
-- @param cols number Number of columns (bits per word). Must be >= 1.
-- @return SRAMArray A new SRAM array.
function SRAMArray.new(rows, cols)
    if rows < 1 then
        error(string.format(
            "block_ram: SRAMArray rows must be >= 1, got %d", rows), 2)
    end
    if cols < 1 then
        error(string.format(
            "block_ram: SRAMArray cols must be >= 1, got %d", cols), 2)
    end

    local cells = {}
    for r = 1, rows do
        cells[r] = {}
        for c = 1, cols do
            cells[r][c] = SRAMCell.new()
        end
    end

    return setmetatable({
        _rows = rows,
        _cols = cols,
        _cells = cells,
    }, SRAMArray)
end

--- Read all columns of a row.
--
-- Activates the word line for the given row, causing all cells in that
-- row to output their stored values.
--
-- @param row number 0-based row index (0 to rows-1).
-- @return table Array of bits (length = cols).
function SRAMArray:read(row)
    self:_validate_row(row)
    local result = {}
    -- Translate 0-based row to 1-based Lua index
    local r = row + 1
    for c = 1, self._cols do
        result[c] = self._cells[r][c]:read(1)  -- word_line=1
    end
    return result
end

--- Write data to a row.
--
-- Activates the word line for the given row and drives the bit lines
-- with the given data, storing values in all cells of the row.
--
-- @param row number 0-based row index (0 to rows-1).
-- @param data table Array of bits (length must equal cols).
function SRAMArray:write(row, data)
    self:_validate_row(row)
    if #data ~= self._cols then
        error(string.format(
            "block_ram: SRAMArray write data length %d does not match cols %d",
            #data, self._cols), 2)
    end
    for i = 1, #data do
        validate_bit(data[i], string.format("data[%d]", i))
    end

    local r = row + 1
    for c = 1, self._cols do
        self._cells[r][c]:write(1, data[c])
    end
end

--- Return the array dimensions as rows, cols.
--
-- @return number, number The (rows, cols) shape.
function SRAMArray:shape()
    return self._rows, self._cols
end

--- Validate that a row index is in range.
-- @param row number 0-based row index.
function SRAMArray:_validate_row(row)
    if row < 0 or row >= self._rows then
        error(string.format(
            "block_ram: SRAMArray row %d out of range [0, %d]",
            row, self._rows - 1), 2)
    end
end

block_ram.SRAMArray = SRAMArray


-- =========================================================================
-- SinglePortRAM — Synchronous RAM with One Port
-- =========================================================================
--
-- A single-port synchronous RAM adds the interface that digital circuits
-- actually use on top of the raw SRAM array:
--
--   1. Address decoding — an integer address selects a row
--   2. Synchronous operation — reads and writes happen on clock edges
--   3. Read modes — what the output shows during a write operation
--
-- Interface:
--
--                   +----------------------------+
--     address ------+                            |
--                   |     Single-Port RAM        |
--     data_in ------+                            +---- data_out
--                   |     (depth x width)        |
--     write_en -----+                            |
--                   |                            |
--     clock --------+                            |
--                   +----------------------------+
--
-- Operations happen on the rising edge of the clock (transition 0->1).

local SinglePortRAM = {}
SinglePortRAM.__index = SinglePortRAM

--- Create a single-port synchronous RAM.
--
-- @param depth number Number of addressable words (>= 1).
-- @param width number Bits per word (>= 1).
-- @param read_mode number One of READ_FIRST, WRITE_FIRST, NO_CHANGE.
-- @return SinglePortRAM A new single-port RAM.
function SinglePortRAM.new(depth, width, read_mode)
    if depth < 1 then
        error(string.format(
            "block_ram: SinglePortRAM depth must be >= 1, got %d", depth), 2)
    end
    if width < 1 then
        error(string.format(
            "block_ram: SinglePortRAM width must be >= 1, got %d", width), 2)
    end

    -- Default to READ_FIRST if not specified
    read_mode = read_mode or block_ram.READ_FIRST

    -- Initialize last_read to all zeros (width bits)
    local last_read = {}
    for i = 1, width do
        last_read[i] = 0
    end

    return setmetatable({
        _depth     = depth,
        _width     = width,
        _read_mode = read_mode,
        _array     = SRAMArray.new(depth, width),
        _prev_clock = 0,
        _last_read = last_read,
    }, SinglePortRAM)
end

--- Execute one half-cycle. Operations happen on the rising edge (0->1).
--
-- @param clock number        Clock signal (0 or 1).
-- @param address number      Word address (0 to depth-1).
-- @param data_in table       Data to write (array of width bits).
-- @param write_enable number 0 = read, 1 = write.
-- @return table              data_out: array of width bits.
function SinglePortRAM:tick(clock, address, data_in, write_enable)
    validate_bit(clock, "clock")
    validate_bit(write_enable, "write_enable")
    self:_validate_address(address)
    self:_validate_data(data_in)

    -- Detect rising edge: previous clock was 0, now it is 1
    local rising_edge = (self._prev_clock == 0 and clock == 1)
    self._prev_clock = clock

    if not rising_edge then
        -- No edge: return a copy of the last read value
        return self:_copy_last_read()
    end

    -- Rising edge: perform the operation
    if write_enable == 0 then
        -- Read operation: fetch from array and update last_read
        self._last_read = self._array:read(address)
        return self:_copy_last_read()
    end

    -- Write operation — behavior depends on read mode
    if self._read_mode == block_ram.READ_FIRST then
        -- Read the old value first, then write the new value
        self._last_read = self._array:read(address)
        self._array:write(address, data_in)
        return self:_copy_last_read()

    elseif self._read_mode == block_ram.WRITE_FIRST then
        -- Write the new value first, then output the new value
        self._array:write(address, data_in)
        local copy = {}
        for i = 1, self._width do
            copy[i] = data_in[i]
        end
        self._last_read = copy
        return self:_copy_last_read()

    else  -- NO_CHANGE
        -- Write the value but do NOT update data_out
        self._array:write(address, data_in)
        return self:_copy_last_read()
    end
end

--- Return the number of addressable words.
-- @return number
function SinglePortRAM:depth()
    return self._depth
end

--- Return the bits per word.
-- @return number
function SinglePortRAM:width()
    return self._width
end

--- Dump all contents for inspection.
--
-- Returns a table of tables, one per address, each containing the word's bits.
-- Useful for debugging and verification.
--
-- @return table Array of word arrays (1-indexed by address+1).
function SinglePortRAM:dump()
    local result = {}
    for i = 0, self._depth - 1 do
        result[i + 1] = self._array:read(i)
    end
    return result
end

--- Validate that an address is in range.
-- @param address number 0-based address.
function SinglePortRAM:_validate_address(address)
    if address < 0 or address >= self._depth then
        error(string.format(
            "block_ram: address %d out of range [0, %d]",
            address, self._depth - 1), 2)
    end
end

--- Validate that data_in has the correct length and all bits are valid.
-- @param data_in table Array of bits.
function SinglePortRAM:_validate_data(data_in)
    if #data_in ~= self._width then
        error(string.format(
            "block_ram: data_in length %d does not match width %d",
            #data_in, self._width), 2)
    end
    for i = 1, #data_in do
        validate_bit(data_in[i], string.format("data_in[%d]", i))
    end
end

--- Return a copy of the last read value.
-- @return table A fresh copy of _last_read.
function SinglePortRAM:_copy_last_read()
    local copy = {}
    for i = 1, self._width do
        copy[i] = self._last_read[i]
    end
    return copy
end

block_ram.SinglePortRAM = SinglePortRAM


-- =========================================================================
-- DualPortRAM — True Dual-Port Synchronous RAM
-- =========================================================================
--
-- Two completely independent ports (A and B), each with its own address,
-- data, and write enable. Both can operate simultaneously:
--
--   - Read A + Read B at different addresses -> both get their data
--   - Write A + Read B at different addresses -> both succeed
--   - Write A + Write B at the SAME address -> collision (undefined in
--     hardware, we raise an error)
--
-- Interface:
--
--   +--------------------------------------------+
--   |               Dual-Port RAM                |
--   |  Port A                      Port B        |
--   |  addr_a, din_a, we_a        addr_b, din_b  |
--   |  dout_a                      we_b, dout_b  |
--   +--------------------------------------------+
--
-- Write collision: if both ports write to the same address in the same
-- cycle, real hardware produces undefined results (the cell may store
-- either value, or a corrupted value). We detect this and raise an error
-- to prevent silent bugs.

local DualPortRAM = {}
DualPortRAM.__index = DualPortRAM

--- Create a true dual-port synchronous RAM.
--
-- @param depth number       Number of addressable words (>= 1).
-- @param width number       Bits per word (>= 1).
-- @param read_mode_a number Read mode for port A (READ_FIRST, WRITE_FIRST, NO_CHANGE).
-- @param read_mode_b number Read mode for port B.
-- @return DualPortRAM A new dual-port RAM.
function DualPortRAM.new(depth, width, read_mode_a, read_mode_b)
    if depth < 1 then
        error(string.format(
            "block_ram: DualPortRAM depth must be >= 1, got %d", depth), 2)
    end
    if width < 1 then
        error(string.format(
            "block_ram: DualPortRAM width must be >= 1, got %d", width), 2)
    end

    local last_read_a = {}
    local last_read_b = {}
    for i = 1, width do
        last_read_a[i] = 0
        last_read_b[i] = 0
    end

    return setmetatable({
        _depth      = depth,
        _width      = width,
        _read_mode_a = read_mode_a,
        _read_mode_b = read_mode_b,
        _array      = SRAMArray.new(depth, width),
        _prev_clock = 0,
        _last_read_a = last_read_a,
        _last_read_b = last_read_b,
    }, DualPortRAM)
end

--- Execute one half-cycle on both ports.
--
-- @param clock number          Clock signal (0 or 1).
-- @param address_a number      Port A word address (0 to depth-1).
-- @param data_in_a table       Port A write data (array of width bits).
-- @param write_enable_a number Port A: 0 = read, 1 = write.
-- @param address_b number      Port B word address (0 to depth-1).
-- @param data_in_b table       Port B write data (array of width bits).
-- @param write_enable_b number Port B: 0 = read, 1 = write.
-- @return table, table, string|nil data_out_a, data_out_b, error_message.
--
-- The error_message is nil on success, or a string describing a write
-- collision. Callers should check the third return value.
function DualPortRAM:tick(clock,
                          address_a, data_in_a, write_enable_a,
                          address_b, data_in_b, write_enable_b)
    validate_bit(clock, "clock")
    validate_bit(write_enable_a, "write_enable_a")
    validate_bit(write_enable_b, "write_enable_b")
    self:_validate_address(address_a, "address_a")
    self:_validate_address(address_b, "address_b")
    self:_validate_data(data_in_a, "data_in_a")
    self:_validate_data(data_in_b, "data_in_b")

    local rising_edge = (self._prev_clock == 0 and clock == 1)
    self._prev_clock = clock

    if not rising_edge then
        return self:_copy_bits(self._last_read_a),
               self:_copy_bits(self._last_read_b),
               nil
    end

    -- Check for write collision: both ports writing to the same address
    if write_enable_a == 1 and write_enable_b == 1 and address_a == address_b then
        return nil, nil,
               string.format(
                   "block_ram: write collision: both ports writing to address %d",
                   address_a)
    end

    -- Process port A
    local out_a = self:_process_port(
        address_a, data_in_a, write_enable_a,
        self._read_mode_a, self._last_read_a)
    self._last_read_a = out_a

    -- Process port B
    local out_b = self:_process_port(
        address_b, data_in_b, write_enable_b,
        self._read_mode_b, self._last_read_b)
    self._last_read_b = out_b

    return self:_copy_bits(out_a), self:_copy_bits(out_b), nil
end

--- Handle a single port's operation (read or write with read mode).
--
-- @param address number       0-based word address.
-- @param data_in table        Write data.
-- @param write_enable number  0 = read, 1 = write.
-- @param read_mode number     READ_FIRST, WRITE_FIRST, or NO_CHANGE.
-- @param last_read table      Previous read output (for NO_CHANGE mode).
-- @return table               The output data for this port.
function DualPortRAM:_process_port(address, data_in, write_enable,
                                    read_mode, last_read)
    if write_enable == 0 then
        return self._array:read(address)
    end

    if read_mode == block_ram.READ_FIRST then
        local result = self._array:read(address)
        self._array:write(address, data_in)
        return result

    elseif read_mode == block_ram.WRITE_FIRST then
        self._array:write(address, data_in)
        local copy = {}
        for i = 1, self._width do
            copy[i] = data_in[i]
        end
        return copy

    else  -- NO_CHANGE
        self._array:write(address, data_in)
        local copy = {}
        for i = 1, self._width do
            copy[i] = last_read[i]
        end
        return copy
    end
end

--- Return the number of addressable words.
-- @return number
function DualPortRAM:depth()
    return self._depth
end

--- Return the bits per word.
-- @return number
function DualPortRAM:width()
    return self._width
end

--- Validate that an address is in range.
-- @param address number 0-based address.
-- @param name string    Human-readable name for error messages.
function DualPortRAM:_validate_address(address, name)
    if address < 0 or address >= self._depth then
        error(string.format(
            "block_ram: %s %d out of range [0, %d]",
            name, address, self._depth - 1), 2)
    end
end

--- Validate that data has the correct length and all bits are valid.
-- @param data table   Array of bits.
-- @param name string  Human-readable name for error messages.
function DualPortRAM:_validate_data(data, name)
    if #data ~= self._width then
        error(string.format(
            "block_ram: %s length %d does not match width %d",
            name, #data, self._width), 2)
    end
    for i = 1, #data do
        validate_bit(data[i], string.format("%s[%d]", name, i))
    end
end

--- Copy a bit array.
-- @param bits table Array of bits.
-- @return table     A fresh copy.
function DualPortRAM:_copy_bits(bits)
    local copy = {}
    for i = 1, self._width do
        copy[i] = bits[i]
    end
    return copy
end

block_ram.DualPortRAM = DualPortRAM


-- =========================================================================
-- ConfigurableBRAM — FPGA-style Block RAM
-- =========================================================================
--
-- In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
-- from the configurable logic. Each tile has a fixed total storage
-- (typically 18 Kbit or 36 Kbit) but can be configured with different
-- width/depth ratios:
--
--   18 Kbit BRAM configurations:
--   +-----------------+-------+-------+------------+
--   | Configuration   | Depth | Width | Total bits |
--   +-----------------+-------+-------+------------+
--   | 16K x 1         | 16384 |     1 |      16384 |
--   |  8K x 2         |  8192 |     2 |      16384 |
--   |  4K x 4         |  4096 |     4 |      16384 |
--   |  2K x 8         |  2048 |     8 |      16384 |
--   |  1K x 16        |  1024 |    16 |      16384 |
--   | 512 x 32        |   512 |    32 |      16384 |
--   +-----------------+-------+-------+------------+
--
-- The total storage is fixed; you trade depth for width by changing how
-- the address decoder and column MUX are configured. The underlying SRAM
-- cells do not change — only the access pattern changes.
--
-- This implementation wraps DualPortRAM with reconfiguration support.
-- Both ports share the underlying storage. Independent port operations
-- are exposed via tick_a (port A) and tick_b (port B).

local ConfigurableBRAM = {}
ConfigurableBRAM.__index = ConfigurableBRAM

--- Create a Block RAM with the given total capacity and initial word width.
--
-- @param total_bits number Total storage in bits (>= 1).
-- @param width number      Initial bits per word (>= 1, must divide total_bits).
-- @return ConfigurableBRAM A new configurable Block RAM.
function ConfigurableBRAM.new(total_bits, width)
    if total_bits < 1 then
        error(string.format(
            "block_ram: ConfigurableBRAM total_bits must be >= 1, got %d",
            total_bits), 2)
    end
    if width < 1 then
        error(string.format(
            "block_ram: ConfigurableBRAM width must be >= 1, got %d",
            width), 2)
    end
    if total_bits % width ~= 0 then
        error(string.format(
            "block_ram: width %d does not evenly divide total_bits %d",
            width, total_bits), 2)
    end

    local depth = total_bits // width

    local last_read_a = {}
    local last_read_b = {}
    for i = 1, width do
        last_read_a[i] = 0
        last_read_b[i] = 0
    end

    return setmetatable({
        _total_bits = total_bits,
        _width      = width,
        _depth      = depth,
        _ram        = DualPortRAM.new(depth, width,
                                      block_ram.READ_FIRST,
                                      block_ram.READ_FIRST),
        _prev_clock = 0,
        _last_read_a = last_read_a,
        _last_read_b = last_read_b,
    }, ConfigurableBRAM)
end

--- Reconfigure the aspect ratio. Clears all stored data.
--
-- This models what happens in real FPGA hardware when you change the
-- BRAM configuration: the address decoder and column MUX are rewired,
-- and the memory contents are lost (or at least undefined).
--
-- @param width number New bits per word (>= 1, must divide total_bits).
function ConfigurableBRAM:reconfigure(width)
    if width < 1 then
        error(string.format(
            "block_ram: ConfigurableBRAM width must be >= 1, got %d",
            width), 2)
    end
    if self._total_bits % width ~= 0 then
        error(string.format(
            "block_ram: width %d does not evenly divide total_bits %d",
            width, self._total_bits), 2)
    end

    self._width = width
    self._depth = self._total_bits // width
    self._ram = DualPortRAM.new(self._depth, self._width,
                                 block_ram.READ_FIRST,
                                 block_ram.READ_FIRST)
    self._prev_clock = 0

    self._last_read_a = {}
    self._last_read_b = {}
    for i = 1, width do
        self._last_read_a[i] = 0
        self._last_read_b[i] = 0
    end
end

--- Perform a port A operation.
--
-- Uses the dual-port RAM with port B idle (reading address 0).
--
-- @param clock number        Clock signal (0 or 1).
-- @param address number      Word address (0 to depth-1).
-- @param data_in table       Write data (array of width bits).
-- @param write_enable number 0 = read, 1 = write.
-- @return table              data_out: array of width bits.
function ConfigurableBRAM:tick_a(clock, address, data_in, write_enable)
    validate_bit(clock, "clock")
    local zeros = {}
    for i = 1, self._width do
        zeros[i] = 0
    end
    local out_a, _, _ = self._ram:tick(
        clock,
        address, data_in, write_enable,
        0, zeros, 0)
    return out_a
end

--- Perform a port B operation.
--
-- Uses the dual-port RAM with port A idle (reading address 0).
--
-- @param clock number        Clock signal (0 or 1).
-- @param address number      Word address (0 to depth-1).
-- @param data_in table       Write data (array of width bits).
-- @param write_enable number 0 = read, 1 = write.
-- @return table              data_out: array of width bits.
function ConfigurableBRAM:tick_b(clock, address, data_in, write_enable)
    validate_bit(clock, "clock")
    local zeros = {}
    for i = 1, self._width do
        zeros[i] = 0
    end
    local _, out_b, _ = self._ram:tick(
        clock,
        0, zeros, 0,
        address, data_in, write_enable)
    return out_b
end

--- Return the number of addressable words at current configuration.
-- @return number
function ConfigurableBRAM:depth()
    return self._depth
end

--- Return the bits per word at current configuration.
-- @return number
function ConfigurableBRAM:width()
    return self._width
end

--- Return the total storage capacity in bits (fixed).
-- @return number
function ConfigurableBRAM:total_bits()
    return self._total_bits
end

block_ram.ConfigurableBRAM = ConfigurableBRAM

return block_ram
