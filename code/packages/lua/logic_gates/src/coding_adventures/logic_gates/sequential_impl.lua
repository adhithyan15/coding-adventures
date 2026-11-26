-- sequential.lua — Sequential Logic Circuits That Remember
-- ========================================================
--
-- Everything in gates.lua is "combinational" logic: the output depends
-- ONLY on the current inputs. There is no memory, no history, no state.
--
-- Sequential logic is different: the output depends on current inputs
-- AND previous state. Sequential circuits can REMEMBER. This is the
-- fundamental difference between a calculator (combinational) and a
-- computer (sequential).
--
-- The key insight: by feeding a gate's output back to its own input,
-- we create a circuit that can hold a value indefinitely.
--
-- The memory hierarchy (simplest to most complex):
--
--   1. SR Latch      — remembers one bit (set/reset interface)
--   2. D Latch       — remembers one bit (data interface, transparent)
--   3. D Flip-Flop   — remembers one bit (data interface, edge-triggered)
--   4. Register      — remembers N bits (parallel flip-flops)
--   5. Shift Register — moves bits left/right on each clock
--   6. Counter       — counts up on each clock pulse

-- This file returns a FACTORY: gates.lua calls it once with the private kit
-- (error minting, validation, the ORIGINAL gate functions).  Everything the
-- circuits need is copied into locals at that moment; the kit is never read
-- again, so nothing reachable afterwards can change behaviour.
return function(kit)

local sequential = {}

-- =========================================================================
-- Validation: one identity for every bit, bounded widths, fresh results
-- =========================================================================
--
-- Everything here REUSES gates.lua's validator, so a sequential circuit
-- rejects exactly what a combinational gate rejects: the integer 0 or 1,
-- never the float 1.0 (in Lua 5.3+ `1.0 == 1` is true, so a `~=` check
-- silently accepts it).  Errors are structured tables with a stable `code`:
--
--   INVALID_BIT    a bit (input or stored state) is not integer 0/1
--   INVALID_WIDTH  a width is not an integer in range
--   INVALID_SHAPE  a state/data container is not the expected table shape
--
-- Widths are checked BEFORE any loop or allocation, so a hostile
-- `width = 1e9` costs one comparison, not a gigabyte.  Every function
-- returns FRESH tables of canonical integers, never a caller's table, so
-- later mutation by the caller cannot corrupt (or alias) circuit state.

--- Largest data width (register / shift register bit count).
sequential.MAX_WIDTH = 65536
--- Largest counter width (a counter value must fit a 64-bit integer).
sequential.MAX_COUNTER_WIDTH = 64

-- Captured internals
-- ------------------
--
-- Every collaborator AND every standard-library function used below is bound
-- to a local when this file loads, so neither rebinding an export nor
-- replacing a global (`_G.ipairs`, `string.format`, ...) afterwards can change
-- how the circuits behave or make validation fail open.
local mtype = math.type
local rawget, rawequal, getmetatable, type = rawget, rawequal, getmetatable, type
local ipairs = ipairs
local sformat = string.format

local NOT, AND, XOR, NOR = kit.NOT, kit.AND, kit.XOR, kit.NOR
local check_bit = kit.validate_bit
local describe = kit.describe
local snapshot_list, snapshot_bits = kit.snapshot_list, kit.snapshot_bits
local MAX_WIDTH, MAX_COUNTER_WIDTH = sequential.MAX_WIDTH, sequential.MAX_COUNTER_WIDTH

local raise, INVALID_WIDTH, INVALID_SHAPE = kit.raise, kit.INVALID_WIDTH, kit.INVALID_SHAPE
local readonly = kit.readonly
local function raise_invalid_width(param, detail) raise(INVALID_WIDTH, param, detail) end
local function raise_invalid_shape(param, detail) raise(INVALID_SHAPE, param, detail) end

local function check_width(w, max, name)
    if type(w) ~= "number" or mtype(w) ~= "integer" then
        raise_invalid_width(name, "must be an integer, got " .. describe(w))
    end
    if w < 1 or w > max then
        raise_invalid_width(name, sformat("must be in 1..%d, got %d", max, w))
    end
    return w
end

local FF_FIELDS = { "master_q", "master_q_bar", "slave_q", "slave_q_bar" }

--- Validate one flip-flop state table; returns a fresh canonical copy.
local function check_ff_state(state, name)
    if type(state) ~= "table" then
        raise_invalid_shape(name, "flip-flop state must be a table, got " .. describe(state))
    end
    -- Plain data only: no metatable.
    if not rawequal(getmetatable(state), nil) then
        raise_invalid_shape(name, "flip-flop state must be a plain table without a metatable")
    end
    local out = {}
    for _, f in ipairs(FF_FIELDS) do
        out[f] = check_bit(rawget(state, f), name .. "." .. f)
    end
    return out
end

--- Validate a list of flip-flop states (1..MAX_WIDTH entries).
-- A strict snapshot (plain table, keys exactly 1..n, n capped) is taken first.
local function check_ff_list(state, name)
    local items, n = snapshot_list(state, MAX_WIDTH, name)
    if n < 1 then
        raise_invalid_shape(name, "must be non-empty")
    end
    local out = {}
    for i = 1, n do
        out[i] = check_ff_state(items[i], name .. "[" .. i .. "]")
    end
    return out, n
end

-- =========================================================================
-- State Constructors
-- =========================================================================

--- Create a new flip-flop state. A master-slave flip-flop has TWO latches.
--
-- In Lua, state is a table rather than a struct. We use a constructor
-- function to ensure all fields are initialized correctly.
--
-- @param master_q number Initial master Q (default 0).
-- @param master_q_bar number Initial master Q-bar (default 1).
-- @param slave_q number Initial slave Q (default 0).
-- @param slave_q_bar number Initial slave Q-bar (default 1).
-- @return table The flip-flop state table.
local function new_flip_flop_state(master_q, master_q_bar, slave_q, slave_q_bar)
    if master_q == nil then master_q = 0 end
    if master_q_bar == nil then master_q_bar = 1 end
    if slave_q == nil then slave_q = 0 end
    if slave_q_bar == nil then slave_q_bar = 1 end
    return {
        master_q = check_bit(master_q, "master_q"),
        master_q_bar = check_bit(master_q_bar, "master_q_bar"),
        slave_q = check_bit(slave_q, "slave_q"),
        slave_q_bar = check_bit(slave_q_bar, "slave_q_bar"),
    }
end

--- Create a new counter state.
--
-- @param width number The number of bits in the counter.
-- @return table The counter state table with zeroed bits.
local function new_counter_state(width)
    check_width(width, MAX_COUNTER_WIDTH, "width")
    local bits = {}
    for i = 1, width do
        bits[i] = 0
    end
    return {
        bits = bits,
        width = width,
    }
end

-- =========================================================================
-- SR Latch — The Simplest Memory Element
-- =========================================================================
--
-- An SR (Set-Reset) latch is built from two NOR gates whose outputs
-- feed back into each other's inputs, creating a stable feedback loop.
--
-- Truth table:
--
--   S | R | Q    | Q-bar | Action
--   --|---|------|-------|--------
--   0 | 0 | hold | hold  | Remember (no change)
--   1 | 0 |  1   |  0    | Set (Q becomes 1)
--   0 | 1 |  0   |  1    | Reset (Q becomes 0)
--   1 | 1 |  0   |  0    | Invalid! (violates Q-bar = NOT Q)

--- SRLatch simulates one evaluation of an SR latch.
--
-- @param set number The Set input (0 or 1).
-- @param reset number The Reset input (0 or 1).
-- @param q number Current Q output.
-- @param q_bar number Current Q-bar output.
-- @return number, number The new Q and Q-bar.
local function SRLatch(set, reset, q, q_bar)
    set = check_bit(set, "set")
    reset = check_bit(reset, "reset")
    q = check_bit(q, "q")
    q_bar = check_bit(q_bar, "q_bar")

    -- The SR latch equations (NOR-based):
    --   Q     = NOR(Reset, Q-bar)
    --   Q-bar = NOR(Set,   Q)
    --
    -- We iterate until the circuit reaches a stable state.
    local current_q = q
    local current_q_bar = q_bar

    for _ = 1, 10 do
        local new_q = NOR(reset, current_q_bar)
        local new_q_bar = NOR(set, new_q)

        if new_q == current_q and new_q_bar == current_q_bar then
            break
        end
        current_q = new_q
        current_q_bar = new_q_bar
    end

    return current_q, current_q_bar
end

-- =========================================================================
-- D Latch — Taming the SR Latch
-- =========================================================================
--
-- The D (Data) latch solves the SR latch's problems by generating S and R
-- from a single data input D and an enable signal:
--
--   Set   = Data AND Enable
--   Reset = NOT(Data) AND Enable
--
-- When Enable = 1: Q follows D (transparent).
-- When Enable = 0: Q holds its value (opaque).
-- The invalid S=1, R=1 state can never occur.

--- DLatch simulates one evaluation of a D latch.
--
-- @param data number The data input (0 or 1).
-- @param enable number Enable signal (0 or 1).
-- @param q number Current Q output.
-- @param q_bar number Current Q-bar output.
-- @return number, number The new Q and Q-bar.
local function DLatch(data, enable, q, q_bar)
    data = check_bit(data, "data")
    enable = check_bit(enable, "enable")
    q = check_bit(q, "q")
    q_bar = check_bit(q_bar, "q_bar")

    local set = AND(data, enable)
    local reset = AND(NOT(data), enable)

    return SRLatch(set, reset, q, q_bar)
end

-- =========================================================================
-- D Flip-Flop — Edge-Triggered Memory
-- =========================================================================
--
-- The D flip-flop uses the master-slave technique: two D latches in
-- series with opposite enable signals.
--
--   Clock = 1: Master captures Data, Slave holds
--   Clock = 0: Master holds, Slave passes to output
--
-- The output changes only once per clock cycle (at the falling edge).

--- DFlipFlop simulates one clock phase of a master-slave D flip-flop.
--
-- @param data number The data input (0 or 1).
-- @param clock number The clock signal (0 or 1).
-- @param state table|nil FlipFlopState (nil for initial state).
-- @return number, number, table The Q, Q-bar, and new state.
local function DFlipFlop(data, clock, state)
    data = check_bit(data, "data")
    clock = check_bit(clock, "clock")

    if state == nil then
        state = new_flip_flop_state()
    else
        state = check_ff_state(state, "state")
    end

    -- Master latch: enabled when clock = 1
    local master_q, master_q_bar = DLatch(
        data, clock, state.master_q, state.master_q_bar
    )

    -- Slave latch: enabled when clock = 0 (NOT clock)
    local slave_q, slave_q_bar = DLatch(
        master_q, NOT(clock), state.slave_q, state.slave_q_bar
    )

    local new_state = {
        master_q = master_q,
        master_q_bar = master_q_bar,
        slave_q = slave_q,
        slave_q_bar = slave_q_bar,
    }

    return slave_q, slave_q_bar, new_state
end

-- =========================================================================
-- Register — N Bits of Parallel Storage
-- =========================================================================
--
-- A register is N flip-flops sharing the same clock signal. Each
-- flip-flop stores one bit. A 64-bit CPU register is literally
-- 64 flip-flops sharing a clock.

--- Register simulates an N-bit register (N parallel D flip-flops).
--
-- @param data table N-bit input data (each element 0 or 1).
-- @param clock number The shared clock signal.
-- @param state table|nil List of FlipFlopStates (nil for initial state).
-- @return table, table The N-bit output and new states.
local function Register(data, clock, state)
    clock = check_bit(clock, "clock")

    -- Strict snapshot: plain table, keys exactly 1..n, n <= MAX_WIDTH, every
    -- element an integer 0/1.  Holes and truncation are errors, not silent.
    local n
    data, n = snapshot_bits(data, MAX_WIDTH, "data")
    if n == 0 then
        raise_invalid_shape("data", "Register requires at least 1 bit of data")
    end

    -- Initialize state if nil; otherwise validate a fresh copy up front.
    if state == nil then
        state = {}
        for i = 1, n do
            state[i] = new_flip_flop_state()
        end
    else
        local sn
        state, sn = check_ff_list(state, "state")
        if sn ~= n then
            raise_invalid_shape("state", "Register data and state length mismatch")
        end
    end

    local outputs = {}
    local new_state = {}

    for i = 1, n do
        local q, _, ns = DFlipFlop(data[i], clock, state[i])
        outputs[i] = q
        new_state[i] = ns
    end

    return outputs, new_state
end

-- =========================================================================
-- Shift Register — Moving Bits Along a Chain
-- =========================================================================
--
-- A shift register is a chain of flip-flops where each one feeds into
-- the next. On each clock cycle, every bit shifts one position.
--
-- Left shift:  serialIn -> [0] -> [1] -> [2] -> serialOut
-- Right shift: serialOut <- [0] <- [1] <- [2] <- serialIn

--- ShiftRegister simulates one clock cycle of an N-bit shift register.
--
-- @param serial_in number The bit entering the register.
-- @param clock number The clock signal.
-- @param state table List of FlipFlopStates (must be non-empty).
-- @param direction string "left" or "right".
-- @return table, number, table The outputs, serial_out, and new states.
local function ShiftRegister(serial_in, clock, state, direction)
    serial_in = check_bit(serial_in, "serial_in")
    clock = check_bit(clock, "clock")
    if direction ~= "left" and direction ~= "right" then
        raise_invalid_shape("direction", 'ShiftRegister direction must be "left" or "right"')
    end
    -- Validates every flip-flop field up front, so current_outputs below
    -- holds only canonical integers and serial_out can never be nil/garbage.
    local n
    state, n = check_ff_list(state, "state")

    local outputs = {}
    local new_state = {}

    -- Capture current outputs before shifting.
    local current_outputs = {}
    for i = 1, n do
        current_outputs[i] = state[i].slave_q
    end

    if direction == "left" then
        -- Left shift: bit 1 gets serial_in, bit i gets old bit i-1.
        -- Serial out is the MSB (last bit).
        for i = 1, n do
            local data_in
            if i == 1 then
                data_in = serial_in
            else
                data_in = current_outputs[i - 1]
            end
            local q, _, ns = DFlipFlop(data_in, clock, state[i])
            outputs[i] = q
            new_state[i] = ns
        end
        return outputs, current_outputs[n], new_state
    end

    -- Right shift: last bit gets serial_in, bit i gets old bit i+1.
    -- Serial out is the LSB (first bit).
    for i = n, 1, -1 do
        local data_in
        if i == n then
            data_in = serial_in
        else
            data_in = current_outputs[i + 1]
        end
        local q, _, ns = DFlipFlop(data_in, clock, state[i])
        outputs[i] = q
        new_state[i] = ns
    end
    return outputs, current_outputs[1], new_state
end

-- =========================================================================
-- Counter — A Self-Incrementing Register
-- =========================================================================
--
-- A binary counter adds 1 to its current value on each clock pulse.
-- When all bits are 1, it wraps around to all 0s.
--
-- The incrementer uses ripple carry:
--   new_bit[i] = XOR(old_bit[i], carry[i])
--   carry[i+1] = AND(old_bit[i], carry[i])
--   carry[0] = 1 (we're adding 1)

--- Counter simulates one clock cycle of an N-bit binary counter.
--
-- @param clock number The clock signal (0 or 1).
-- @param reset number When 1, resets counter to 0.
-- @param state table CounterState (must not be nil).
-- @return table, table The N-bit count (LSB first) and new state.
local function Counter(clock, reset, state)
    clock = check_bit(clock, "clock")
    reset = check_bit(reset, "reset")
    if type(state) ~= "table" then
        raise_invalid_shape("state", "Counter requires a state table, got " .. describe(state))
    end
    if not rawequal(getmetatable(state), nil) then
        raise_invalid_shape("state", "Counter state must be a plain table without a metatable")
    end

    -- Width first (bounded 1..64), then the bits table is checked to be
    -- exactly that long, then every bit -- on EVERY path (reset and hold
    -- included), so corrupt state is never passed through.
    local width = check_width(rawget(state, "width"), MAX_COUNTER_WIDTH, "state.width")
    local old, count = snapshot_bits(rawget(state, "bits"), MAX_COUNTER_WIDTH, "state.bits")
    if count ~= width then
        raise_invalid_shape("state.bits", sformat(
            "length %d does not match state.width %d", count, width))
    end

    local new_bits = {}

    if reset == 1 then
        -- Asynchronous reset: immediately clear all bits.
        for i = 1, width do new_bits[i] = 0 end
    elseif clock == 0 then
        -- On clock = 0, hold current value (a validated fresh copy).
        for i = 1, width do new_bits[i] = old[i] end
    else
        -- Increment using ripple carry.
        local carry = 1
        for i = 1, width do
            new_bits[i] = XOR(old[i], carry)
            carry = AND(old[i], carry)
        end
    end

    -- Output and state get independent tables so they never alias.
    local state_bits = {}
    for i = 1, width do state_bits[i] = new_bits[i] end
    return new_bits, { bits = state_bits, width = width }
end

sequential.new_flip_flop_state = new_flip_flop_state
sequential.new_counter_state = new_counter_state
sequential.SRLatch = SRLatch
sequential.DLatch = DLatch
sequential.DFlipFlop = DFlipFlop
sequential.Register = Register
sequential.ShiftRegister = ShiftRegister
sequential.Counter = Counter

return readonly(sequential)
end
