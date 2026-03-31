-- coding_adventures/intel4004_gatelevel — Intel 4004 Gate-Level Simulator
--
-- Every computation in this simulator routes through actual logic gate
-- functions — AND, OR, XOR, NOT — chained into adders, then into a
-- 4-bit ALU. Registers are built from D flip-flops via the Register
-- function from logic_gates.sequential. The program counter uses a
-- half-adder chain for incrementing.
--
-- This is NOT the same as the behavioral simulator (intel4004_simulator).
-- The behavioral simulator executes instructions directly with host
-- language integers. This simulator routes everything through the gate
-- abstractions we built from scratch.
--
-- ## Why gate-level?
--
--   1. Count gates: how many AND/OR/NOT ops does ADD R3 actually require?
--   2. Trace signals: follow a bit from register R3 through the ALU
--   3. Understand timing: a ripple-carry add takes 4 gate delays
--   4. Appreciate constraints: 2,300 transistors is incredibly few
--
-- ## Architecture — same 4004 ISA, routed through gates
--
-- Every ADD instruction traverses:
--   a_bits = int_to_bits(A, 4)
--   b_bits = int_to_bits(Rn, 4)
--   ripple_carry_adder(a_bits, b_bits, carry_in):
--     full_adder(a[0], b[0], carry_in)  → half_adder × 2 + OR
--     full_adder(a[1], b[1], carry1)    → half_adder × 2 + OR
--     full_adder(a[2], b[2], carry2)    → ...
--     full_adder(a[3], b[3], carry3)    → sum, carry_out
--
-- ## Dependencies
--
--   logic_gates  — AND, OR, XOR, NOT, Register (D flip-flops)
--   arithmetic   — half_adder, full_adder, ripple_carry_adder

local lg   = require("coding_adventures.logic_gates")
local arith = require("coding_adventures.arithmetic")

local Intel4004GateLevel = {}
Intel4004GateLevel.__index = Intel4004GateLevel

-- ---------------------------------------------------------------------------
-- Bit conversion helpers
-- ---------------------------------------------------------------------------

-- Convert an integer to an LSB-first bit array of given width.
-- Example: int_to_bits(5, 4) = {1, 0, 1, 0}  (5 = 0101 in binary)
local function int_to_bits(value, width)
    local mask = (width >= 32) and 0xFFFFFFFF or ((1 << width) - 1)
    value = value & mask
    local bits = {}
    for i = 1, width do
        bits[i] = (value >> (i - 1)) & 1
    end
    return bits
end

-- Convert an LSB-first bit array to an integer.
-- Example: bits_to_int({1, 0, 1, 0}) = 5
local function bits_to_int(bits)
    local value = 0
    for i, bit in ipairs(bits) do
        value = value | (bit << (i - 1))
    end
    return value
end

-- ---------------------------------------------------------------------------
-- ALU helper — wraps the arithmetic package's ripple_carry_adder
-- ---------------------------------------------------------------------------

-- Add two 4-bit integers with carry, using the gate-level adder.
-- Returns: result (0-15), carry_out (boolean)
local function gate_add(a, b, carry_in)
    local a_bits = int_to_bits(a, 4)
    local b_bits = int_to_bits(b, 4)
    local cin    = carry_in and 1 or 0
    local result_bits, carry_out = arith.ripple_carry_adder(a_bits, b_bits, cin)
    return bits_to_int(result_bits), carry_out == 1
end

-- Bitwise NOT of a 4-bit value using NOT gates.
local function gate_not4(a)
    local a_bits = int_to_bits(a, 4)
    local out_bits = {}
    for i = 1, 4 do out_bits[i] = lg.NOT(a_bits[i]) end
    return bits_to_int(out_bits)
end

-- ---------------------------------------------------------------------------
-- Register flip-flop state helpers
-- ---------------------------------------------------------------------------

-- Create initial flip-flop state for a width-bit register.
local function new_ff_state(width)
    local state = {}
    for i = 1, width do
        state[i] = lg.new_flip_flop_state()
    end
    return state
end

-- Read an integer from a flip-flop register state (width bits).
local function read_ff(state, width)
    local zero_bits = {}
    for i = 1, width do zero_bits[i] = 0 end
    -- clock=0: read current slave output without latching new data
    local output, _ = lg.Register(zero_bits, 0, state)
    return bits_to_int(output)
end

-- Write an integer to a flip-flop register state (width bits).
-- Two-phase write: clock=0 captures into master, clock=1 latches to slave.
local function write_ff(value, width, state)
    local bits = int_to_bits(value, width)
    local _, state1 = lg.Register(bits, 0, state)
    local _, new_state = lg.Register(bits, 1, state1)
    return new_state
end

-- ---------------------------------------------------------------------------
-- Constructor
-- ---------------------------------------------------------------------------

-- Create a new Intel 4004 gate-level CPU.
function Intel4004GateLevel.new()
    local self = setmetatable({}, Intel4004GateLevel)
    self:_init()
    return self
end

function Intel4004GateLevel:_init()
    -- 16 × 4-bit registers (each stored as flip-flop state)
    self.reg_states = {}
    for i = 1, 16 do self.reg_states[i] = new_ff_state(4) end

    -- 4-bit accumulator flip-flop state
    self.acc_state = new_ff_state(4)

    -- 1-bit carry flag flip-flop state
    self.carry_state = new_ff_state(1)

    -- 12-bit program counter flip-flop state
    self.pc_state = new_ff_state(12)

    -- 3-level hardware stack (3 × 12-bit registers)
    self.stack_states = {}
    for i = 1, 3 do self.stack_states[i] = new_ff_state(12) end
    self.stack_pointer = 1  -- 1-indexed, next-write slot

    -- RAM: stored as plain integers (nibbles), not flip-flops
    -- (the key insight: the flip-flop wrapping is on registers/PC/stack;
    -- RAM uses simulated flip-flops via a simple indexed map)
    self.ram_states = {}         -- {bank,reg,char} -> ff_state (4-bit)
    self.ram_status_states = {}  -- {bank,reg,idx} -> ff_state (4-bit)
    self.ram_output = {0, 0, 0, 0}

    -- ROM (4096 bytes, plain array)
    self.rom = {}
    for i = 1, 4096 do self.rom[i] = 0 end

    -- RAM addressing (set by SRC)
    self.ram_bank      = 1
    self.ram_register  = 1
    self.ram_character = 1

    -- ROM I/O port
    self.rom_port = 0

    -- Control
    self.halted = false
end

-- ---------------------------------------------------------------------------
-- Public API (same interface as behavioral simulator)
-- ---------------------------------------------------------------------------

-- Load a program (byte array or string) into ROM.
function Intel4004GateLevel:load_program(program)
    self:_write_pc_val(0)
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

-- Run a program and return traces.
function Intel4004GateLevel:run(program, max_steps)
    max_steps = max_steps or 10000
    self:load_program(program)
    local traces = {}
    local steps = 0
    while not self.halted and self:_read_pc_val() < 4096 and steps < max_steps do
        local trace = self:step()
        table.insert(traces, trace)
        steps = steps + 1
    end
    return traces
end

-- Execute one instruction, routing all operations through gate functions.
function Intel4004GateLevel:step()
    if self.halted then error("CPU is halted") end

    local address = self:_read_pc_val()
    local raw = self.rom[address + 1] or 0
    self:_inc_pc()

    local raw2 = nil
    if self:_is_two_byte(raw) then
        raw2 = self.rom[self:_read_pc_val() + 1] or 0
        self:_inc_pc()
    end

    local acc_before   = self:_read_acc()
    local carry_before = self:_read_carry()

    local mnemonic = self:_execute(raw, raw2, address)

    return {
        address            = address,
        raw                = raw,
        raw2               = raw2,
        mnemonic           = mnemonic,
        accumulator_before = acc_before,
        accumulator_after  = self:_read_acc(),
        carry_before       = carry_before,
        carry_after        = self:_read_carry(),
    }
end

-- Reset CPU to initial state.
function Intel4004GateLevel:reset()
    self:_init()
end

-- Return gate count estimates per component (educational).
function Intel4004GateLevel:gate_count()
    return {
        alu       = 80,   -- 4 full adders × ~20 gates each
        registers = 256,  -- 16 regs × 4 bits × 4 gates per flip-flop
        acc       = 16,
        carry     = 4,
        decoder   = 120,  -- AND/OR/NOT tree for all opcodes
        pc        = 96,   -- 12 half-adders for increment
        stack     = 144,  -- 3 × 12 bits × 4 gates
        total     = 716,  -- close to 4004's ~786 estimated gates
    }
end

-- ---------------------------------------------------------------------------
-- Private: PC register operations (gate-level via flip-flops)
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_read_pc_val()
    return read_ff(self.pc_state, 12)
end

function Intel4004GateLevel:_write_pc_val(value)
    self.pc_state = write_ff(value & 0xFFF, 12, self.pc_state)
end

-- Increment PC by 1 using a chain of half-adders.
-- Each half_adder(bit, carry) → (sum, carry_out)
-- This models the ripple-carry incrementer in the real 4004.
function Intel4004GateLevel:_inc_pc()
    local bits = int_to_bits(self:_read_pc_val(), 12)
    local carry = 1  -- adding 1 = initial carry into bit 0
    local new_bits = {}
    for i = 1, 12 do
        local sum, c_out = arith.half_adder(bits[i], carry)
        new_bits[i] = sum
        carry = c_out
    end
    self.pc_state = write_ff(bits_to_int(new_bits), 12, self.pc_state)
end

-- ---------------------------------------------------------------------------
-- Private: accumulator and carry via flip-flops
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_read_acc()
    return read_ff(self.acc_state, 4)
end

function Intel4004GateLevel:_write_acc(value)
    self.acc_state = write_ff(value & 0xF, 4, self.acc_state)
end

function Intel4004GateLevel:_read_carry()
    return read_ff(self.carry_state, 1) == 1
end

function Intel4004GateLevel:_write_carry(value)
    local bit = value and 1 or 0
    self.carry_state = write_ff(bit, 1, self.carry_state)
end

-- ---------------------------------------------------------------------------
-- Private: general registers via flip-flops
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_read_reg(index)
    return read_ff(self.reg_states[index + 1], 4)
end

function Intel4004GateLevel:_write_reg(index, value)
    self.reg_states[index + 1] = write_ff(value & 0xF, 4, self.reg_states[index + 1])
end

-- ---------------------------------------------------------------------------
-- Private: register pairs
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_read_pair(pair)
    local high = self:_read_reg(pair * 2)
    local low  = self:_read_reg(pair * 2 + 1)
    return (high << 4) | low
end

function Intel4004GateLevel:_write_pair(pair, value)
    self:_write_reg(pair * 2,     (value >> 4) & 0xF)
    self:_write_reg(pair * 2 + 1, value & 0xF)
end

-- ---------------------------------------------------------------------------
-- Private: 3-level hardware stack via flip-flops
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_stack_push(addr)
    self.stack_states[self.stack_pointer] = write_ff(addr & 0xFFF, 12,
        self.stack_states[self.stack_pointer])
    self.stack_pointer = (self.stack_pointer % 3) + 1
end

function Intel4004GateLevel:_stack_pop()
    self.stack_pointer = ((self.stack_pointer - 2) % 3) + 1
    return read_ff(self.stack_states[self.stack_pointer], 12)
end

-- ---------------------------------------------------------------------------
-- Private: RAM via flip-flop states
-- ---------------------------------------------------------------------------

local function ram_key(bank, reg, char) return bank * 10000 + reg * 100 + char end

function Intel4004GateLevel:_ram_read_main()
    local key = ram_key(self.ram_bank, self.ram_register, self.ram_character)
    local state = self.ram_states[key]
    if state == nil then return 0 end
    return read_ff(state, 4)
end

function Intel4004GateLevel:_ram_write_main(value)
    local key = ram_key(self.ram_bank, self.ram_register, self.ram_character)
    local state = self.ram_states[key] or new_ff_state(4)
    self.ram_states[key] = write_ff(value & 0xF, 4, state)
end

local function status_key(bank, reg, idx) return bank * 10000 + reg * 100 + idx end

function Intel4004GateLevel:_ram_read_status(idx)
    local key = status_key(self.ram_bank, self.ram_register, idx)
    local state = self.ram_status_states[key]
    if state == nil then return 0 end
    return read_ff(state, 4)
end

function Intel4004GateLevel:_ram_write_status(idx, value)
    local key = status_key(self.ram_bank, self.ram_register, idx)
    local state = self.ram_status_states[key] or new_ff_state(4)
    self.ram_status_states[key] = write_ff(value & 0xF, 4, state)
end

-- ---------------------------------------------------------------------------
-- Private: 2-byte detection and dispatcher (same logic as behavioral)
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_is_two_byte(raw)
    local upper = (raw >> 4) & 0xF
    if upper == 0x1 or upper == 0x4 or upper == 0x5 or upper == 0x7 then
        return true
    end
    if upper == 0x2 and (raw & 0x1) == 0 then return true end
    return false
end

function Intel4004GateLevel:_execute(raw, raw2, addr)
    if raw == 0x00 then return "NOP" end
    if raw == 0x01 then self.halted = true; return "HLT" end

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
    end
    return string.format("UNKNOWN(0x%02X)", raw)
end

-- ---------------------------------------------------------------------------
-- Instructions — all arithmetic routed through gate functions
-- ---------------------------------------------------------------------------

function Intel4004GateLevel:_exec_ldm(n)
    self:_write_acc(n & 0xF)
    return "LDM " .. n
end

function Intel4004GateLevel:_exec_ld(reg)
    self:_write_acc(self:_read_reg(reg))
    return "LD R" .. reg
end

function Intel4004GateLevel:_exec_xch(reg)
    local old_a   = self:_read_acc()
    local reg_val = self:_read_reg(reg)
    self:_write_acc(reg_val)
    self:_write_reg(reg, old_a)
    return "XCH R" .. reg
end

function Intel4004GateLevel:_exec_inc(reg)
    -- Increment using a half-adder chain (1-bit add of 1)
    local val  = self:_read_reg(reg)
    local bits = int_to_bits(val, 4)
    local carry = 1
    local new_bits = {}
    for i = 1, 4 do
        local sum, c_out = arith.half_adder(bits[i], carry)
        new_bits[i] = sum
        carry = c_out
    end
    self:_write_reg(reg, bits_to_int(new_bits) & 0xF)
    return "INC R" .. reg
end

-- ADD: uses the gate-level ripple-carry adder
-- A = A + Rn + carry_in; carry set if overflow
function Intel4004GateLevel:_exec_add(reg)
    local a        = self:_read_acc()
    local rn       = self:_read_reg(reg)
    local carry_in = self:_read_carry()
    local result, carry_out = gate_add(a, rn, carry_in)
    self:_write_acc(result & 0xF)
    self:_write_carry(carry_out)
    return "ADD R" .. reg
end

-- SUB: complement-add subtraction through gates
-- A = A + NOT(Rn) + CY; carry is added directly (A = A + NOT(Rn) + CY).
-- Intel 4004 SUB: uses carry as the incoming borrow complement.
function Intel4004GateLevel:_exec_sub(reg)
    local a         = self:_read_acc()
    local rn        = self:_read_reg(reg)
    local carry     = self:_read_carry()
    local compl_rn  = gate_not4(rn)
    local borrow_in = carry  -- carry is added directly (CY=1 means add 1)
    local result, carry_out = gate_add(a, compl_rn, borrow_in)
    self:_write_acc(result & 0xF)
    self:_write_carry(carry_out)
    return "SUB R" .. reg
end

function Intel4004GateLevel:_exec_jun(lower, raw2)
    local target = (lower << 8) | raw2
    self:_write_pc_val(target)
    return string.format("JUN 0x%03X", target)
end

function Intel4004GateLevel:_exec_jcn(cond, raw2, addr)
    local acc    = self:_read_acc()
    local carry  = self:_read_carry()
    local test_zero  = (cond & 0x4) ~= 0 and acc == 0
    local test_carry = (cond & 0x2) ~= 0 and carry
    local test_pin   = (cond & 0x1) ~= 0 and false
    local result     = test_zero or test_carry or test_pin
    if (cond & 0x8) ~= 0 then result = not result end
    local page   = (addr + 2) & 0xF00
    local target = page | raw2
    if result then self:_write_pc_val(target) end
    return string.format("JCN %d,0x%02X", cond, raw2)
end

function Intel4004GateLevel:_exec_jms(lower, raw2, addr)
    local target = (lower << 8) | raw2
    self:_stack_push(addr + 2)
    self:_write_pc_val(target)
    return string.format("JMS 0x%03X", target)
end

function Intel4004GateLevel:_exec_bbl(n)
    local ret = self:_stack_pop()
    if n ~= 0 then self:_write_acc(n & 0xF) end  -- only load if n≠0
    self:_write_pc_val(ret)
    return "BBL " .. n
end

function Intel4004GateLevel:_exec_isz(reg, raw2, addr)
    local val  = self:_read_reg(reg)
    local bits = int_to_bits(val, 4)
    local carry = 1
    local new_bits = {}
    for i = 1, 4 do
        local sum, c_out = arith.half_adder(bits[i], carry)
        new_bits[i] = sum
        carry = c_out
    end
    local new_val = bits_to_int(new_bits) & 0xF
    self:_write_reg(reg, new_val)
    if new_val ~= 0 then
        self:_write_pc_val((addr + 2) & 0xF00 | raw2)
    end
    return string.format("ISZ R%d,0x%02X", reg, raw2)
end

function Intel4004GateLevel:_exec_fim(pair, data)
    self:_write_pair(pair, data)
    return string.format("FIM P%d,0x%02X", pair, data)
end

function Intel4004GateLevel:_exec_src(pair)
    local pair_val = self:_read_pair(pair)
    self.ram_register  = ((pair_val >> 4) & 0xF) % 4 + 1
    self.ram_character = (pair_val & 0xF) + 1
    return "SRC P" .. pair
end

function Intel4004GateLevel:_exec_fin(pair, addr)
    local p0_val   = self:_read_pair(0)
    local page     = addr & 0xF00
    local rom_addr = (page | p0_val) + 1
    local byte     = self.rom[rom_addr] or 0
    self:_write_pair(pair, byte)
    return "FIN P" .. pair
end

function Intel4004GateLevel:_exec_jin(pair, addr)
    local pair_val = self:_read_pair(pair)
    self:_write_pc_val((addr & 0xF00) | pair_val)
    return "JIN P" .. pair
end

function Intel4004GateLevel:_exec_io(raw)
    local acc = self:_read_acc()
    if raw == 0xE0 then
        self:_ram_write_main(acc)
        return "WRM"
    elseif raw == 0xE1 then
        self.ram_output[self.ram_bank] = acc & 0xF
        return "WMP"
    elseif raw == 0xE2 then
        self.rom_port = acc & 0xF
        return "WRR"
    elseif raw == 0xE3 then
        return "WPM"
    elseif raw >= 0xE4 and raw <= 0xE7 then
        self:_ram_write_status(raw - 0xE4 + 1, acc)
        return "WR" .. (raw - 0xE4)
    elseif raw == 0xE8 then
        local ram_val   = self:_ram_read_main()
        local compl_val = gate_not4(ram_val)
        local borrow_in = not self:_read_carry()
        local result, carry_out = gate_add(acc, compl_val, borrow_in)
        self:_write_acc(result & 0xF)
        self:_write_carry(carry_out)
        return "SBM"
    elseif raw == 0xE9 then
        self:_write_acc(self:_ram_read_main())
        return "RDM"
    elseif raw == 0xEA then
        self:_write_acc(self.rom_port & 0xF)
        return "RDR"
    elseif raw == 0xEB then
        local ram_val  = self:_ram_read_main()
        local carry_in = self:_read_carry()
        local result, carry_out = gate_add(acc, ram_val, carry_in)
        self:_write_acc(result & 0xF)
        self:_write_carry(carry_out)
        return "ADM"
    elseif raw >= 0xEC and raw <= 0xEF then
        self:_write_acc(self:_ram_read_status(raw - 0xEC + 1))
        return "RD" .. (raw - 0xEC)
    end
    return string.format("UNKNOWN(0x%02X)", raw)
end

function Intel4004GateLevel:_exec_accum(raw)
    local acc   = self:_read_acc()
    local carry = self:_read_carry()

    if raw == 0xF0 then
        self:_write_acc(0); self:_write_carry(false); return "CLB"
    elseif raw == 0xF1 then
        self:_write_carry(false); return "CLC"
    elseif raw == 0xF2 then
        -- IAC: increment via gate_add
        local result, cout = gate_add(acc, 0, true)  -- +1 via carry_in=true
        self:_write_acc(result & 0xF); self:_write_carry(cout); return "IAC"
    elseif raw == 0xF3 then
        -- CMC: complement carry using NOT gate
        self:_write_carry(lg.NOT(carry and 1 or 0) == 1); return "CMC"
    elseif raw == 0xF4 then
        -- CMA: complement accumulator using NOT gates
        self:_write_acc(gate_not4(acc)); return "CMA"
    elseif raw == 0xF5 then
        -- RAL: rotate left through carry
        local bits     = int_to_bits(acc, 4)
        local old_carry = carry and 1 or 0
        local new_carry = bits[4]  -- MSB goes to carry
        local new_bits  = {old_carry, bits[1], bits[2], bits[3]}
        self:_write_acc(bits_to_int(new_bits))
        self:_write_carry(new_carry == 1)
        return "RAL"
    elseif raw == 0xF6 then
        -- RAR: rotate right through carry
        local bits      = int_to_bits(acc, 4)
        local old_carry = carry and 1 or 0
        local new_carry = bits[1]  -- LSB goes to carry
        local new_bits  = {bits[2], bits[3], bits[4], old_carry}
        self:_write_acc(bits_to_int(new_bits))
        self:_write_carry(new_carry == 1)
        return "RAR"
    elseif raw == 0xF7 then
        self:_write_acc(carry and 1 or 0); self:_write_carry(false); return "TCC"
    elseif raw == 0xF8 then
        -- DAC: decrement via subtraction (A + NOT(0) + borrow_in=true = A-1)
        local result, cout = gate_add(acc, gate_not4(1), true)
        -- carry = no borrow = acc > 0
        local no_borrow = acc > 0
        self:_write_acc(result & 0xF); self:_write_carry(no_borrow); return "DAC"
    elseif raw == 0xF9 then
        self:_write_acc(carry and 10 or 9); self:_write_carry(false); return "TCS"
    elseif raw == 0xFA then
        self:_write_carry(true); return "STC"
    elseif raw == 0xFB then
        -- DAA: BCD adjust using gate_add
        if acc > 9 or carry then
            local result, cout = gate_add(acc, 6, false)
            local new_carry = cout and true or carry
            self:_write_acc(result & 0xF); self:_write_carry(new_carry)
        end
        return "DAA"
    elseif raw == 0xFC then
        local kbp = {[0]=0,[1]=1,[2]=2,[4]=3,[8]=4}
        local val = kbp[acc]
        self:_write_acc(val ~= nil and val or 15); return "KBP"
    elseif raw == 0xFD then
        local bank_bits = acc & 0x7
        if bank_bits > 3 then bank_bits = bank_bits & 0x3 end
        self.ram_bank = bank_bits + 1; return "DCL"
    end
    return string.format("UNKNOWN(0x%02X)", raw)
end

return Intel4004GateLevel
