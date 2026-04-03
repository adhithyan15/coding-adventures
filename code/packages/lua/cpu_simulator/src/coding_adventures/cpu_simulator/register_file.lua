-- register_file.lua — CPU register storage
--
-- Registers are the fastest storage in a computer. A CPU typically has
-- 8-64 registers, each holding one word (e.g., 32 bits). All computation
-- happens between registers — you load data from (slower) memory into a
-- register, operate on it, then store it back.
--
-- WHY NOT JUST USE MEMORY FOR EVERYTHING?
-- Memory takes ~100 ns to access. Registers take ~0.3 ns — 300x faster.
-- Registers live inside the CPU die, right next to the ALU. Memory lives
-- on a separate chip connected by a bus.
--
-- REGISTER NAMING: Registers are numbered 0 through (num_registers - 1).
-- In ARM, register 15 is the program counter (PC). In RISC-V, register 0
-- is hardwired to zero (writes are silently discarded). Our generic CPU
-- doesn't enforce these conventions — that is the ISA decoder's job.
--
-- BIT WIDTH: The maximum value a register can hold is (2^bit_width - 1).
-- For 32-bit registers, the maximum is 4,294,967,295 (0xFFFFFFFF).
-- Writes are masked to the bit width, preventing overflow.

local RegisterFile = {}
RegisterFile.__index = RegisterFile

--- Creates a new register file.
--
-- @param num_registers  number  Number of registers (default 16)
-- @param bit_width      number  Bits per register (default 32)
-- @return RegisterFile
function RegisterFile.new(num_registers, bit_width)
    num_registers = num_registers or 16
    bit_width     = bit_width     or 32

    -- Compute the maximum value (mask) for this bit width.
    -- Lua integers are 64-bit, so bit_width up to 63 is safe.
    -- For bit_width >= 32 we just use 0xFFFFFFFF as our limit
    -- (consistent with 32-bit CPU simulation).
    local max_value
    if bit_width >= 32 then
        max_value = 0xFFFFFFFF
    else
        max_value = (1 << bit_width) - 1
    end

    -- Initialize all registers to 0
    local values = {}
    for i = 1, num_registers do
        values[i] = 0
    end

    return setmetatable({
        num_registers = num_registers,
        bit_width     = bit_width,
        max_value     = max_value,
        values        = values,  -- 1-indexed; register 0 → values[1]
    }, RegisterFile)
end

--- Reads the value of register `index` (0-based).
-- @param index  number  Register index [0, num_registers-1]
-- @return number  Current register value
function RegisterFile:read(index)
    assert(index >= 0 and index < self.num_registers,
        string.format("register index %d out of range [0, %d)", index, self.num_registers))
    return self.values[index + 1]
end

--- Writes `value` to register `index` (0-based).
-- The value is masked to the register bit width.
-- @param index  number  Register index [0, num_registers-1]
-- @param value  number  Value to write
function RegisterFile:write(index, value)
    assert(index >= 0 and index < self.num_registers,
        string.format("register index %d out of range [0, %d)", index, self.num_registers))
    self.values[index + 1] = value & self.max_value
end

--- Returns a snapshot of all register values as a table.
-- Keys are strings like "R0", "R1", etc.
-- @return table  { "R0" → value, "R1" → value, ... }
function RegisterFile:dump()
    local result = {}
    for i = 0, self.num_registers - 1 do
        result["R" .. i] = self.values[i + 1]
    end
    return result
end

--- Returns the number of registers.
function RegisterFile:num_regs()
    return self.num_registers
end

return RegisterFile
