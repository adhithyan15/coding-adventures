-- coding_adventures/intel4004_simulator — Intel 4004 CPU Simulator
--
-- The Intel 4004 was released in November 1971, designed by Federico Faggin,
-- Ted Hoff, and Stanley Mazor for the Busicom 141-PF calculator. It contained
-- just 2,300 transistors and ran at 740 kHz — roughly one million times slower
-- than a modern CPU core. Yet it proved a general-purpose processor could be
-- built on a single chip, launching the microprocessor revolution.
--
-- ## Architecture
--
--   Data width:      4 bits (values 0-15)
--   Instructions:    8 bits (some are 2 bytes)
--   Registers:       16 × 4-bit (R0-R15), organized as 8 pairs (P0-P7)
--   Accumulator:     4-bit (A) — most arithmetic goes through here
--   Carry flag:      1 bit — set on overflow/borrow
--   Program counter: 12 bits (addresses 4096 bytes of ROM)
--   Stack:           3-level hardware stack (12-bit return addresses)
--   RAM:             4 banks × 4 registers × (16 main + 4 status) nibbles
--   Clock:           740 kHz (original hardware)
--
-- ## Why is the 4004 interesting?
--
--   1. Historical: the chip that started the microprocessor revolution
--   2. Tiny: 4-bit data, 46 instructions, 2,300 transistors
--   3. Real hardware constraints: forces thinking about 4-bit arithmetic
--   4. BCD arithmetic: built for decimal calculators, not binary computing
--   5. Contrast: shows how far we've come from 1971 to today
--
-- ## Register pairs
--
-- The 16 registers are organized as 8 pairs for certain instructions:
--   P0: R0 (high nibble), R1 (low nibble)
--   P1: R2 (high nibble), R3 (low nibble)
--   ...
--   P7: R14 (high nibble), R15 (low nibble)
--
-- Pair value = (R_high << 4) | R_low  (8-bit value)
--
-- ## 3-Level Hardware Stack
--
-- The 4004 has a 3-deep hardware stack for subroutine calls. It is NOT in
-- RAM — it uses dedicated registers inside the chip. Stack wraps silently
-- on overflow (4th push overwrites the oldest entry).
--
-- ## Complete Instruction Set (46 + HLT)
--
--   0x00       NOP          No operation
--   0x01       HLT          Halt (simulator-only)
--   0x1_       JCN c,a *    Conditional jump
--   0x2_ even  FIM Pp,d *   Fetch immediate to pair
--   0x2_ odd   SRC Pp       Send register control
--   0x3_ even  FIN Pp       Fetch indirect from ROM via P0
--   0x3_ odd   JIN Pp       Jump indirect via pair
--   0x4_       JUN a   *    Unconditional jump
--   0x5_       JMS a   *    Jump to subroutine
--   0x6_       INC Rn       Increment register
--   0x7_       ISZ Rn,a *   Increment and skip if zero
--   0x8_       ADD Rn       Add register to accumulator with carry
--   0x9_       SUB Rn       Subtract register with borrow
--   0xA_       LD Rn        Load register into accumulator
--   0xB_       XCH Rn       Exchange accumulator and register
--   0xC_       BBL n        Branch back and load
--   0xD_       LDM n        Load immediate
--   0xE0-0xEF  I/O ops      RAM/ROM read/write
--   0xF0-0xFD  Accum ops    Accumulator manipulation
--
--   (* = 2-byte instruction)

local Intel4004 = {}
Intel4004.__index = Intel4004

-- ---------------------------------------------------------------------------
-- Constructor
-- ---------------------------------------------------------------------------

-- Create a new Intel 4004 simulator with all state zeroed.
function Intel4004.new()
    local self = setmetatable({}, Intel4004)
    self:_init_state()
    return self
end

function Intel4004:_init_state()
    -- 4-bit accumulator (0-15)
    self.accumulator = 0
    -- 16 × 4-bit registers (R0-R15)
    self.registers = {}
    for i = 1, 16 do self.registers[i] = 0 end
    -- 1-bit carry flag
    self.carry = false
    -- 12-bit program counter
    self.pc = 0
    -- Halted flag
    self.halted = false
    -- 3-level hardware stack (12-bit addresses, 1-indexed)
    self.hw_stack = {0, 0, 0}
    self.stack_pointer = 1   -- points to next-write slot (1-3)
    -- RAM: [bank][reg][char] = nibble (0-15)
    -- Organized as 4 banks × 4 registers × 16 main characters
    self.ram = {}
    self.ram_status = {}
    self.ram_output = {0, 0, 0, 0}  -- 4 banks × output port
    for b = 1, 4 do
        self.ram[b] = {}
        self.ram_status[b] = {}
        for r = 1, 4 do
            self.ram[b][r] = {}
            self.ram_status[b][r] = {}
            for c = 1, 16 do self.ram[b][r][c] = 0 end
            for s = 1, 4 do self.ram_status[b][r][s] = 0 end
        end
    end
    -- Current RAM addressing (set by SRC instruction)
    self.ram_bank      = 1    -- 1-indexed (maps to 4004 bank 0)
    self.ram_register  = 1    -- 1-indexed
    self.ram_character = 1    -- 1-indexed (maps to 4004 char 0)
    -- ROM (4096 bytes)
    self.rom = {}
    for i = 1, 4096 do self.rom[i] = 0 end
    -- ROM I/O port (WRR/RDR)
    self.rom_port = 0
end

-- ---------------------------------------------------------------------------
-- Public API
-- ---------------------------------------------------------------------------

-- Load a program (array of bytes) into ROM starting at address 0.
-- program: array of integers (0-255), or a Lua string
function Intel4004:load_program(program)
    self.pc = 0
    if type(program) == "string" then
        for i = 1, #program do
            self.rom[i] = program:byte(i)
        end
    else
        for i, byte in ipairs(program) do
            self.rom[i] = byte & 0xFF
        end
    end
end

-- Run a program and return {final_cpu, traces}.
-- program: byte array or string
-- max_steps: maximum instructions to execute (default 10000)
function Intel4004:run(program, max_steps)
    max_steps = max_steps or 10000
    self:load_program(program)
    local traces = {}
    local steps = 0
    while not self.halted and self.pc < 4096 and steps < max_steps do
        local trace = self:step()
        table.insert(traces, trace)
        steps = steps + 1
    end
    return traces
end

-- Execute one instruction.
-- Returns a trace table:
--   {address, raw, raw2, mnemonic,
--    accumulator_before, accumulator_after,
--    carry_before, carry_after}
function Intel4004:step()
    if self.halted then
        error("CPU is halted — cannot step further")
    end

    local address = self.pc
    local raw = self.rom[self.pc + 1] or 0   -- 1-indexed ROM
    self.pc = self.pc + 1

    local raw2 = nil
    if self:_is_two_byte(raw) then
        raw2 = self.rom[self.pc + 1] or 0
        self.pc = self.pc + 1
    end

    local acc_before   = self.accumulator
    local carry_before = self.carry

    local mnemonic = self:_execute(raw, raw2, address)

    return {
        address            = address,
        raw                = raw,
        raw2               = raw2,
        mnemonic           = mnemonic,
        accumulator_before = acc_before,
        accumulator_after  = self.accumulator,
        carry_before       = carry_before,
        carry_after        = self.carry,
    }
end

-- Reset CPU to initial state.
function Intel4004:reset()
    self:_init_state()
end

-- ---------------------------------------------------------------------------
-- Private: 2-byte instruction detection
-- ---------------------------------------------------------------------------

-- Returns true if this opcode is the first byte of a 2-byte instruction.
--
-- 2-byte instructions:
--   0x1_  JCN — conditional jump (8-bit page-relative address)
--   0x2_ even FIM — fetch immediate (8-bit data to register pair)
--   0x4_  JUN — unconditional jump (12-bit address)
--   0x5_  JMS — jump to subroutine (12-bit address)
--   0x7_  ISZ — increment and skip if zero (8-bit page-relative address)
function Intel4004:_is_two_byte(raw)
    local upper = (raw >> 4) & 0xF
    if upper == 0x1 or upper == 0x4 or upper == 0x5 or upper == 0x7 then
        return true
    end
    -- 0x2_ even: FIM (even register pairs only)
    if upper == 0x2 and (raw & 0x1) == 0 then
        return true
    end
    return false
end

-- ---------------------------------------------------------------------------
-- Private: instruction dispatcher
-- ---------------------------------------------------------------------------

function Intel4004:_execute(raw, raw2, addr)
    if raw == 0x00 then return "NOP" end
    if raw == 0x01 then
        self.halted = true
        return "HLT"
    end

    local upper = (raw >> 4) & 0xF
    local lower = raw & 0xF

    if upper == 0x1 then
        return self:_exec_jcn(lower, raw2, addr)
    elseif upper == 0x2 and (raw & 1) == 0 then
        return self:_exec_fim(lower >> 1, raw2)
    elseif upper == 0x2 then
        return self:_exec_src(lower >> 1)
    elseif upper == 0x3 and (raw & 1) == 0 then
        return self:_exec_fin(lower >> 1, addr)
    elseif upper == 0x3 then
        return self:_exec_jin(lower >> 1, addr)
    elseif upper == 0x4 then
        return self:_exec_jun(lower, raw2)
    elseif upper == 0x5 then
        return self:_exec_jms(lower, raw2, addr)
    elseif upper == 0x6 then
        return self:_exec_inc(lower)
    elseif upper == 0x7 then
        return self:_exec_isz(lower, raw2, addr)
    elseif upper == 0x8 then
        return self:_exec_add(lower)
    elseif upper == 0x9 then
        return self:_exec_sub(lower)
    elseif upper == 0xA then
        return self:_exec_ld(lower)
    elseif upper == 0xB then
        return self:_exec_xch(lower)
    elseif upper == 0xC then
        return self:_exec_bbl(lower)
    elseif upper == 0xD then
        return self:_exec_ldm(lower)
    elseif upper == 0xE then
        return self:_exec_io(raw)
    elseif upper == 0xF then
        return self:_exec_accum(raw)
    else
        return string.format("UNKNOWN(0x%02X)", raw)
    end
end

-- ---------------------------------------------------------------------------
-- Instruction implementations
-- ---------------------------------------------------------------------------

-- LDM: Load immediate nibble into accumulator.
-- A = N (lower nibble of opcode byte)
function Intel4004:_exec_ldm(n)
    self.accumulator = n & 0xF
    return "LDM " .. n
end

-- LD: Load register into accumulator.
-- A = Rn (non-destructive read)
function Intel4004:_exec_ld(reg)
    self.accumulator = self.registers[reg + 1] & 0xF
    return "LD R" .. reg
end

-- XCH: Exchange accumulator and register.
-- tmp = A; A = Rn; Rn = tmp
function Intel4004:_exec_xch(reg)
    local old_a = self.accumulator
    self.accumulator = self.registers[reg + 1] & 0xF
    self.registers[reg + 1] = old_a & 0xF
    return "XCH R" .. reg
end

-- INC: Increment register (modulo 16, no carry effect).
-- Rn = (Rn + 1) & 0xF
function Intel4004:_exec_inc(reg)
    self.registers[reg + 1] = (self.registers[reg + 1] + 1) & 0xF
    return "INC R" .. reg
end

-- ADD: Add register to accumulator with carry.
-- A = A + Rn + carry_in. Sets carry if result > 15.
-- The carry participates — this enables multi-digit BCD arithmetic.
function Intel4004:_exec_add(reg)
    local carry_in = self.carry and 1 or 0
    local result = self.accumulator + self.registers[reg + 1] + carry_in
    self.accumulator = result & 0xF
    self.carry = result > 0xF
    return "ADD R" .. reg
end

-- SUB: Subtract register from accumulator (complement-add method).
-- A = A + (~Rn) + borrow_in   where borrow_in = carry ? 0 : 1
-- carry=true means NO borrow (MCS-4 carry semantics).
function Intel4004:_exec_sub(reg)
    local reg_val = self.registers[reg + 1]
    local complement = (~reg_val) & 0xF
    local borrow_in = self.carry and 0 or 1
    local result = self.accumulator + complement + borrow_in
    self.accumulator = result & 0xF
    self.carry = result > 0xF
    return "SUB R" .. reg
end

-- JUN: Unconditional jump to 12-bit address.
-- Target = (lower_nibble << 8) | second_byte
function Intel4004:_exec_jun(lower, raw2)
    local target = (lower << 8) | raw2
    self.pc = target
    return string.format("JUN 0x%03X", target)
end

-- JCN: Conditional jump (page-relative).
-- Condition nibble bits:
--   bit 3 (0x8): invert the test result
--   bit 2 (0x4): test accumulator == 0
--   bit 1 (0x2): test carry == 1
--   bit 0 (0x1): test input pin (always 0 in simulator)
-- Multiple test bits are OR'd. Jump target is same-page (upper 4 bits of PC).
function Intel4004:_exec_jcn(cond, raw2, addr)
    local test_zero  = (cond & 0x4) ~= 0 and self.accumulator == 0
    local test_carry = (cond & 0x2) ~= 0 and self.carry
    local test_pin   = (cond & 0x1) ~= 0 and false  -- pin always 0
    local result     = test_zero or test_carry or test_pin
    if (cond & 0x8) ~= 0 then result = not result end

    -- Target is page-relative: (addr+2) & 0xF00 | raw2
    local page   = (addr + 2) & 0xF00
    local target = page | raw2

    if result then self.pc = target end
    return string.format("JCN %d,0x%02X", cond, raw2)
end

-- JMS: Jump to subroutine (push return address, jump to 12-bit address).
-- Push (addr+2) onto 3-level stack, then set PC.
function Intel4004:_exec_jms(lower, raw2, addr)
    local target      = (lower << 8) | raw2
    local return_addr = addr + 2
    self:_stack_push(return_addr)
    self.pc = target
    return string.format("JMS 0x%03X", target)
end

-- BBL: Branch back and load.
-- Pop return address from stack. Set A = immediate nibble.
function Intel4004:_exec_bbl(n)
    local return_addr = self:_stack_pop()
    self.accumulator  = n & 0xF
    self.pc           = return_addr
    return "BBL " .. n
end

-- ISZ: Increment register and skip next instruction if zero.
-- Rn = (Rn + 1) & 0xF.  If Rn != 0: jump to page-relative target.
-- Used as a loop counter: load with -N (e.g., 12 for -4 in 4-bit), loop
-- until ISZ finds zero.
function Intel4004:_exec_isz(reg, raw2, addr)
    local val = (self.registers[reg + 1] + 1) & 0xF
    self.registers[reg + 1] = val

    if val ~= 0 then
        local page   = (addr + 2) & 0xF00
        local target = page | raw2
        self.pc = target
    end
    return string.format("ISZ R%d,0x%02X", reg, raw2)
end

-- FIM: Fetch immediate byte into register pair.
-- R_high = (data >> 4) & 0xF,  R_low = data & 0xF
function Intel4004:_exec_fim(pair, data)
    local high_reg = pair * 2 + 1   -- 1-indexed
    local low_reg  = high_reg + 1
    self.registers[high_reg] = (data >> 4) & 0xF
    self.registers[low_reg]  = data & 0xF
    return string.format("FIM P%d,0x%02X", pair, data)
end

-- SRC: Send register pair as address for RAM/ROM operations.
-- High nibble of pair selects RAM register (0-3).
-- Low nibble selects character within that register (0-15).
function Intel4004:_exec_src(pair)
    local pair_val = self:_read_pair(pair)
    -- Convert from 0-indexed 4004 values to 1-indexed Lua arrays
    self.ram_register  = ((pair_val >> 4) & 0xF) % 4 + 1
    self.ram_character = (pair_val & 0xF) + 1
    return "SRC P" .. pair
end

-- FIN: Fetch indirect from ROM via pair P0.
-- ROM address = (current_page) | value_of_P0
-- Loads the byte at that ROM address into register pair Pp.
function Intel4004:_exec_fin(pair, addr)
    local p0_val     = self:_read_pair(0)
    local page       = addr & 0xF00
    local rom_addr   = (page | p0_val) + 1   -- 1-indexed
    local rom_byte   = self.rom[rom_addr] or 0
    self:_write_pair(pair, rom_byte)
    return "FIN P" .. pair
end

-- JIN: Jump indirect via register pair (page-relative).
-- PC = (current_page) | (pair_high << 4) | pair_low
function Intel4004:_exec_jin(pair, addr)
    local pair_val = self:_read_pair(pair)
    local page     = addr & 0xF00
    self.pc        = page | pair_val
    return "JIN P" .. pair
end

-- ---------------------------------------------------------------------------
-- I/O instructions (0xE0-0xEF)
-- ---------------------------------------------------------------------------

function Intel4004:_exec_io(raw)
    if raw == 0xE0 then
        -- WRM: Write accumulator to RAM main character
        self.ram[self.ram_bank][self.ram_register][self.ram_character] =
            self.accumulator & 0xF
        return "WRM"

    elseif raw == 0xE1 then
        -- WMP: Write accumulator to RAM output port
        self.ram_output[self.ram_bank] = self.accumulator & 0xF
        return "WMP"

    elseif raw == 0xE2 then
        -- WRR: Write accumulator to ROM I/O port
        self.rom_port = self.accumulator & 0xF
        return "WRR"

    elseif raw == 0xE3 then
        -- WPM: Write program RAM (not simulated)
        return "WPM"

    elseif raw >= 0xE4 and raw <= 0xE7 then
        -- WR0-WR3: Write accumulator to RAM status character 0-3
        local idx = raw - 0xE4 + 1
        self.ram_status[self.ram_bank][self.ram_register][idx] =
            self.accumulator & 0xF
        return "WR" .. (raw - 0xE4)

    elseif raw == 0xE8 then
        -- SBM: Subtract RAM main character from accumulator (complement-add)
        local ram_val    = self.ram[self.ram_bank][self.ram_register][self.ram_character]
        local complement = (~ram_val) & 0xF
        local borrow_in  = self.carry and 0 or 1
        local result     = self.accumulator + complement + borrow_in
        self.accumulator = result & 0xF
        self.carry       = result > 0xF
        return "SBM"

    elseif raw == 0xE9 then
        -- RDM: Read RAM main character into accumulator
        self.accumulator =
            self.ram[self.ram_bank][self.ram_register][self.ram_character]
        return "RDM"

    elseif raw == 0xEA then
        -- RDR: Read ROM I/O port into accumulator
        self.accumulator = self.rom_port & 0xF
        return "RDR"

    elseif raw == 0xEB then
        -- ADM: Add RAM main character to accumulator with carry
        local ram_val = self.ram[self.ram_bank][self.ram_register][self.ram_character]
        local carry_in = self.carry and 1 or 0
        local result   = self.accumulator + ram_val + carry_in
        self.accumulator = result & 0xF
        self.carry       = result > 0xF
        return "ADM"

    elseif raw >= 0xEC and raw <= 0xEF then
        -- RD0-RD3: Read RAM status character 0-3 into accumulator
        local idx = raw - 0xEC + 1
        self.accumulator =
            self.ram_status[self.ram_bank][self.ram_register][idx]
        return "RD" .. (raw - 0xEC)

    else
        return string.format("UNKNOWN(0x%02X)", raw)
    end
end

-- ---------------------------------------------------------------------------
-- Accumulator instructions (0xF0-0xFD)
-- ---------------------------------------------------------------------------

function Intel4004:_exec_accum(raw)
    if raw == 0xF0 then
        -- CLB: Clear both A and carry
        self.accumulator = 0
        self.carry       = false
        return "CLB"

    elseif raw == 0xF1 then
        -- CLC: Clear carry flag
        self.carry = false
        return "CLC"

    elseif raw == 0xF2 then
        -- IAC: Increment accumulator
        local result = self.accumulator + 1
        self.accumulator = result & 0xF
        self.carry       = result > 0xF
        return "IAC"

    elseif raw == 0xF3 then
        -- CMC: Complement (invert) carry flag
        self.carry = not self.carry
        return "CMC"

    elseif raw == 0xF4 then
        -- CMA: Complement (bitwise NOT) accumulator in 4 bits
        self.accumulator = (~self.accumulator) & 0xF
        return "CMA"

    elseif raw == 0xF5 then
        -- RAL: Rotate accumulator left through carry
        -- [carry | A3 A2 A1 A0] → [A3 | A2 A1 A0 old_carry]
        local old_carry  = self.carry and 1 or 0
        local new_carry  = (self.accumulator & 0x8) ~= 0
        self.accumulator = ((self.accumulator << 1) | old_carry) & 0xF
        self.carry       = new_carry
        return "RAL"

    elseif raw == 0xF6 then
        -- RAR: Rotate accumulator right through carry
        -- [carry | A3 A2 A1 A0] → [A0 | old_carry A3 A2 A1]
        local old_carry  = self.carry and 1 or 0
        local new_carry  = (self.accumulator & 0x1) ~= 0
        self.accumulator = ((self.accumulator >> 1) | (old_carry << 3)) & 0xF
        self.carry       = new_carry
        return "RAR"

    elseif raw == 0xF7 then
        -- TCC: Transfer carry to accumulator, clear carry
        self.accumulator = self.carry and 1 or 0
        self.carry       = false
        return "TCC"

    elseif raw == 0xF8 then
        -- DAC: Decrement accumulator
        -- carry = true if no borrow (A > 0), false if borrow (A was 0)
        local new_carry  = self.accumulator > 0
        self.accumulator = (self.accumulator - 1) & 0xF
        self.carry       = new_carry
        return "DAC"

    elseif raw == 0xF9 then
        -- TCS: Transfer carry subtract
        -- A = 10 if carry, 9 if not carry. Carry is always cleared.
        -- Used in BCD subtraction to provide the complement correction factor.
        self.accumulator = self.carry and 10 or 9
        self.carry       = false
        return "TCS"

    elseif raw == 0xFA then
        -- STC: Set carry flag
        self.carry = true
        return "STC"

    elseif raw == 0xFB then
        -- DAA: Decimal adjust accumulator (BCD correction after addition)
        -- If A > 9 or carry is set, add 6 to A.
        -- If that addition overflows past 0xF, set carry.
        if self.accumulator > 9 or self.carry then
            local result = self.accumulator + 6
            local new_carry = result > 0xF and true or self.carry
            self.accumulator = result & 0xF
            self.carry       = new_carry
        end
        return "DAA"

    elseif raw == 0xFC then
        -- KBP: Keyboard process — converts 1-hot encoding to binary
        -- 0→0, 1→1, 2→2, 4→3, 8→4, anything else→15 (error)
        local kbp = {[0]=0, [1]=1, [2]=2, [4]=3, [8]=4}
        local val = kbp[self.accumulator]
        self.accumulator = val ~= nil and val or 15
        return "KBP"

    elseif raw == 0xFD then
        -- DCL: Designate command line — select RAM bank based on A bits 0-2.
        -- A values 0-3 select banks 1-4 (1-indexed in our implementation).
        local bank_bits = self.accumulator & 0x7
        if bank_bits > 3 then bank_bits = bank_bits & 0x3 end
        self.ram_bank = bank_bits + 1
        return "DCL"

    else
        return string.format("UNKNOWN(0x%02X)", raw)
    end
end

-- ---------------------------------------------------------------------------
-- Private helpers: register pairs
-- ---------------------------------------------------------------------------

-- Read a register pair value (8-bit: high_nibble<<4 | low_nibble).
-- pair: 0-7
function Intel4004:_read_pair(pair)
    local high_reg = pair * 2 + 1   -- 1-indexed
    local low_reg  = high_reg + 1
    local high = self.registers[high_reg] or 0
    local low  = self.registers[low_reg]  or 0
    return (high << 4) | low
end

-- Write an 8-bit value into a register pair.
function Intel4004:_write_pair(pair, value)
    local high_reg = pair * 2 + 1
    local low_reg  = high_reg + 1
    self.registers[high_reg] = (value >> 4) & 0xF
    self.registers[low_reg]  = value & 0xF
end

-- ---------------------------------------------------------------------------
-- Private helpers: 3-level hardware stack
-- ---------------------------------------------------------------------------

-- Push an address onto the hardware stack (wraps on overflow).
function Intel4004:_stack_push(addr)
    self.hw_stack[self.stack_pointer] = addr & 0xFFF
    self.stack_pointer = (self.stack_pointer % 3) + 1
end

-- Pop an address from the hardware stack.
function Intel4004:_stack_pop()
    self.stack_pointer = ((self.stack_pointer - 2) % 3) + 1
    return self.hw_stack[self.stack_pointer] or 0
end

return Intel4004
