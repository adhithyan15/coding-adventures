-- =============================================================================
-- RegisterFile — fast CPU register storage
-- =============================================================================
--
-- A register file models the bank of general-purpose registers found in every
-- CPU.  Key properties:
--
--   * Zero-indexed: register 0 is "R0", register N-1 is "R(N-1)"
--   * Bit-width masking: writing a 64-bit value to a 32-bit register file
--     silently truncates the value to 32 bits (value & max_value)
--   * All registers initialised to 0
--
-- RISC architecture (RISC-V) hardwires R0 to zero — writes are silently
-- dropped and reads always return 0.  ARM does NOT hardwire R0; it is a
-- general-purpose register.  This implementation follows the ARM convention
-- (all registers are writable) unless the caller chooses to enforce the
-- zero-register invariant at a higher level.

local RegisterFile = {}
RegisterFile.__index = RegisterFile

-- RegisterFile.new(num_registers, bit_width)
--   num_registers — how many GPRs (default 16)
--   bit_width     — bits per register: 8, 16, 32 or 64 (default 32)
function RegisterFile.new(num_registers, bit_width)
    num_registers = num_registers or 16
    bit_width     = bit_width     or 32

    local self = setmetatable({}, RegisterFile)
    self.num_registers = num_registers
    self.bit_width     = bit_width

    -- Compute the mask used to truncate writes.
    -- For 32-bit: max_value = 0xFFFFFFFF
    -- For 8-bit:  max_value = 0xFF
    -- Lua 5.4 integers are 64-bit, so (1 << 64) - 1 would overflow.
    -- We cap at 64 bits = 0xFFFFFFFFFFFFFFFF.
    if bit_width >= 64 then
        self.max_value = 0xFFFFFFFFFFFFFFFF
    else
        self.max_value = (1 << bit_width) - 1
    end

    self.values = {}
    for i = 0, num_registers - 1 do
        self.values[i] = 0
    end

    return self
end

-- read(index) → integer
-- index is 0-based.  Reads beyond num_registers-1 raise an error.
function RegisterFile:read(index)
    assert(index >= 0 and index < self.num_registers,
        string.format("RegisterFile.read: index %d out of range [0, %d)", index, self.num_registers))
    return self.values[index]
end

-- write(index, value) — writes value & max_value
-- index is 0-based.
function RegisterFile:write(index, value)
    assert(index >= 0 and index < self.num_registers,
        string.format("RegisterFile.write: index %d out of range [0, %d)", index, self.num_registers))
    self.values[index] = value & self.max_value
end

-- dump() → table {R0=v, R1=v, …}
-- Returns a human-readable snapshot of all register values.
-- Useful for tests and debug output.
function RegisterFile:dump()
    local result = {}
    for i = 0, self.num_registers - 1 do
        result[string.format("R%d", i)] = self.values[i]
    end
    return result
end

return RegisterFile
