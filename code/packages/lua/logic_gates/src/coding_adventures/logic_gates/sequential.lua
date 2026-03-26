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

local gates = require("coding_adventures.logic_gates.gates")

local sequential = {}

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
function sequential.new_flip_flop_state(master_q, master_q_bar, slave_q, slave_q_bar)
    return {
        master_q = master_q or 0,
        master_q_bar = master_q_bar or 1,
        slave_q = slave_q or 0,
        slave_q_bar = slave_q_bar or 1,
    }
end

--- Create a new counter state.
--
-- @param width number The number of bits in the counter.
-- @return table The counter state table with zeroed bits.
function sequential.new_counter_state(width)
    if width < 1 then
        error("logic_gates: Counter width must be at least 1")
    end
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
function sequential.SRLatch(set, reset, q, q_bar)
    for _, pair in ipairs({{"set", set}, {"reset", reset}, {"q", q}, {"q_bar", q_bar}}) do
        if pair[2] ~= 0 and pair[2] ~= 1 then
            error(string.format("logic_gates: %s must be 0 or 1, got %s", pair[1], tostring(pair[2])))
        end
    end

    -- The SR latch equations (NOR-based):
    --   Q     = NOR(Reset, Q-bar)
    --   Q-bar = NOR(Set,   Q)
    --
    -- We iterate until the circuit reaches a stable state.
    local current_q = q
    local current_q_bar = q_bar

    for _ = 1, 10 do
        local new_q = gates.NOR(reset, current_q_bar)
        local new_q_bar = gates.NOR(set, new_q)

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
function sequential.DLatch(data, enable, q, q_bar)
    for _, pair in ipairs({{"data", data}, {"enable", enable}, {"q", q}, {"q_bar", q_bar}}) do
        if pair[2] ~= 0 and pair[2] ~= 1 then
            error(string.format("logic_gates: %s must be 0 or 1, got %s", pair[1], tostring(pair[2])))
        end
    end

    local set = gates.AND(data, enable)
    local reset = gates.AND(gates.NOT(data), enable)

    return sequential.SRLatch(set, reset, q, q_bar)
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
function sequential.DFlipFlop(data, clock, state)
    if data ~= 0 and data ~= 1 then
        error(string.format("logic_gates: data must be 0 or 1, got %s", tostring(data)))
    end
    if clock ~= 0 and clock ~= 1 then
        error(string.format("logic_gates: clock must be 0 or 1, got %s", tostring(clock)))
    end

    if state == nil then
        state = sequential.new_flip_flop_state()
    end

    -- Master latch: enabled when clock = 1
    local master_q, master_q_bar = sequential.DLatch(
        data, clock, state.master_q, state.master_q_bar
    )

    -- Slave latch: enabled when clock = 0 (NOT clock)
    local slave_q, slave_q_bar = sequential.DLatch(
        master_q, gates.NOT(clock), state.slave_q, state.slave_q_bar
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
function sequential.Register(data, clock, state)
    if clock ~= 0 and clock ~= 1 then
        error("logic_gates: clock must be 0 or 1, got " .. tostring(clock))
    end

    local n = #data
    if n == 0 then
        error("logic_gates: Register requires at least 1 bit of data")
    end

    -- Initialize state if nil.
    if state == nil then
        state = {}
        for i = 1, n do
            state[i] = sequential.new_flip_flop_state()
        end
    end

    if #state ~= n then
        error("logic_gates: Register data and state length mismatch")
    end

    local outputs = {}
    local new_state = {}

    for i = 1, n do
        local q, _, ns = sequential.DFlipFlop(data[i], clock, state[i])
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
function sequential.ShiftRegister(serial_in, clock, state, direction)
    if serial_in ~= 0 and serial_in ~= 1 then
        error("logic_gates: serial_in must be 0 or 1, got " .. tostring(serial_in))
    end
    if clock ~= 0 and clock ~= 1 then
        error("logic_gates: clock must be 0 or 1, got " .. tostring(clock))
    end
    if direction ~= "left" and direction ~= "right" then
        error('logic_gates: ShiftRegister direction must be "left" or "right"')
    end
    if state == nil or #state == 0 then
        error("logic_gates: ShiftRegister requires non-empty state")
    end

    local n = #state
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
            local q, _, ns = sequential.DFlipFlop(data_in, clock, state[i])
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
        local q, _, ns = sequential.DFlipFlop(data_in, clock, state[i])
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
function sequential.Counter(clock, reset, state)
    if clock ~= 0 and clock ~= 1 then
        error("logic_gates: clock must be 0 or 1, got " .. tostring(clock))
    end
    if reset ~= 0 and reset ~= 1 then
        error("logic_gates: reset must be 0 or 1, got " .. tostring(reset))
    end
    if state == nil then
        error("logic_gates: Counter requires non-nil state")
    end

    local width = state.width
    if width < 1 then
        error("logic_gates: Counter width must be at least 1")
    end

    -- Initialize bits if empty.
    if #state.bits == 0 then
        for i = 1, width do
            state.bits[i] = 0
        end
    end

    -- Asynchronous reset: immediately clear all bits.
    if reset == 1 then
        local new_bits = {}
        for i = 1, width do
            new_bits[i] = 0
        end
        return new_bits, { bits = new_bits, width = width }
    end

    -- On clock = 0, hold current value.
    if clock == 0 then
        local output = {}
        for i = 1, width do
            output[i] = state.bits[i]
        end
        return output, { bits = output, width = width }
    end

    -- Increment using ripple carry.
    local new_bits = {}
    local carry = 1

    for i = 1, width do
        new_bits[i] = gates.XOR(state.bits[i], carry)
        carry = gates.AND(state.bits[i], carry)
    end

    return new_bits, { bits = new_bits, width = width }
end

return sequential
