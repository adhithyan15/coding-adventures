-- clock -- the heartbeat of every digital circuit
--
-- Every sequential circuit in a computer -- flip-flops, registers, counters,
-- CPU pipeline stages, GPU cores -- is driven by a clock signal. The clock
-- is a square wave that alternates between 0 and 1:
--
--     +--+  +--+  +--+  +--+
--     |  |  |  |  |  |  |  |
--  ---+  +--+  +--+  +--+  +--
--
-- On each rising edge (0->1), flip-flops capture their inputs. This is
-- what makes synchronous digital logic work -- everything happens in
-- lockstep, driven by the clock.
--
-- In real hardware:
--   - CPU clock: 3-5 GHz (3-5 billion cycles per second)
--   - GPU clock: 1-2 GHz
--   - Memory clock: 4-8 GHz (DDR5)
--   - The clock frequency is the single most important performance number
--
-- Why does the clock matter?
--
-- Without a clock, digital circuits would be chaotic. Imagine a chain of
-- logic gates where each gate has a slightly different propagation delay.
-- Without synchronization, signals would arrive at different times and
-- produce garbage. The clock solves this by saying: "Everyone, capture
-- your inputs NOW." This is called synchronous design.
--
-- The clock period must be long enough for the slowest signal path to
-- settle. This slowest path is called the "critical path," and it
-- determines the maximum clock frequency.
--
-- Half-cycles and edges
--
-- A single clock cycle has two halves:
--
--   Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
--   Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too
--
-- "DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
-- actually runs at 3200 MHz but transfers data on both rising and falling
-- edges, achieving 6400 MT/s (megatransfers per second).
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 8 in the computing stack.

-- ---------------------------------------------------------------------------
-- Validation helpers
-- ---------------------------------------------------------------------------
-- These small functions enforce preconditions at the API boundary so that
-- callers get clear error messages instead of mysterious nil dereferences
-- deep inside the implementation.

--- Asserts that a value is a positive integer. Errors with a descriptive
--- message identifying the parameter name if the check fails.
local function assert_positive_integer(value, name)
    if type(value) ~= "number" then
        error(name .. " must be a number, got " .. type(value), 3)
    end
    if value ~= math.floor(value) then
        error(name .. " must be an integer, got " .. tostring(value), 3)
    end
    if value <= 0 then
        error(name .. " must be positive, got " .. tostring(value), 3)
    end
end

--- Asserts that a value is a non-negative integer (>= 0).
local function assert_non_negative_integer(value, name)
    if type(value) ~= "number" then
        error(name .. " must be a number, got " .. type(value), 3)
    end
    if value ~= math.floor(value) then
        error(name .. " must be an integer, got " .. tostring(value), 3)
    end
    if value < 0 then
        error(name .. " must be non-negative, got " .. tostring(value), 3)
    end
end

--- Asserts that a value is a function.
local function assert_function(value, name)
    if type(value) ~= "function" then
        error(name .. " must be a function, got " .. type(value), 3)
    end
end

-- ---------------------------------------------------------------------------
-- ClockEdge -- a record of one transition
-- ---------------------------------------------------------------------------
-- Every time the clock ticks, it produces an edge. An edge captures:
--   - Which cycle we are in (cycles count from 1)
--   - The current signal level (0 or 1)
--   - Whether this was a rising edge (0->1) or falling edge (1->0)
--
-- Think of it like a timestamp in a logic analyzer trace. In real hardware,
-- an oscilloscope captures exactly these properties: the signal level at
-- each point in time, and whether the signal is transitioning up or down.

local ClockEdge = {}
ClockEdge.__index = ClockEdge

--- Creates a new ClockEdge record.
---
--- @param cycle number Which cycle this edge belongs to (starts at 1)
--- @param value number Current level after the transition (0 or 1)
--- @param is_rising boolean True if this was a 0->1 transition
--- @param is_falling boolean True if this was a 1->0 transition
--- @return table A ClockEdge instance
function ClockEdge.new(cycle, value, is_rising, is_falling)
    -- Validate inputs: edges are immutable records, so we check once at
    -- construction time rather than on every access.
    assert_non_negative_integer(cycle, "cycle")

    if value ~= 0 and value ~= 1 then
        error("value must be 0 or 1, got " .. tostring(value), 2)
    end
    if type(is_rising) ~= "boolean" then
        error("is_rising must be a boolean, got " .. type(is_rising), 2)
    end
    if type(is_falling) ~= "boolean" then
        error("is_falling must be a boolean, got " .. type(is_falling), 2)
    end

    local self = setmetatable({}, ClockEdge)
    self.cycle = cycle
    self.value = value
    self.is_rising = is_rising
    self.is_falling = is_falling
    return self
end

-- ---------------------------------------------------------------------------
-- Clock -- the main square-wave generator
-- ---------------------------------------------------------------------------
-- The clock maintains a cycle count and alternates between low (0) and
-- high (1) on each tick. Components connect to the clock and react to
-- edges (transitions).
--
-- A complete cycle is: low -> high -> low (two ticks).
--
-- Example usage:
--
--   local clk = Clock.new(1000000)  -- 1 MHz
--   local edge = clk:tick()         -- rising edge, cycle 1
--   edge = clk:tick()               -- falling edge, cycle 1
--   edge = clk:tick()               -- rising edge, cycle 2
--
-- The observer pattern (listeners) allows components to react to clock
-- edges without polling. This mirrors how real hardware works: the clock
-- wire is physically connected to every component's clock input pin.

local Clock = {}
Clock.__index = Clock

--- Creates a new Clock with the given frequency in Hz.
---
--- The clock starts at value 0 (low), cycle 0, with no ticks elapsed.
--- This is the state of a real oscillator before it starts oscillating.
---
--- @param frequency_hz number Clock frequency in Hz (must be a positive integer)
--- @return table A Clock instance
function Clock.new(frequency_hz)
    assert_positive_integer(frequency_hz, "frequency_hz")

    local self = setmetatable({}, Clock)

    -- Public fields
    self.frequency_hz = frequency_hz  -- Clock frequency in Hz
    self.cycle = 0                     -- Current cycle count (increments on rising edges)
    self.value = 0                     -- Current signal level (0 or 1)

    -- Private fields (by convention; Lua has no true private)
    self._total_ticks = 0              -- Total half-cycles elapsed
    self._listeners = {}               -- Registered edge listener functions

    return self
end

--- Advances one half-cycle and returns the edge that occurred.
---
--- The clock alternates like a toggle switch:
---   - If currently 0, goes to 1 (rising edge, new cycle starts)
---   - If currently 1, goes to 0 (falling edge, cycle ends)
---
--- After toggling, all registered listeners are notified with the
--- edge record. This is how connected components "see" the clock.
---
--- @return table A ClockEdge describing the transition
function Clock:tick()
    -- Save the old value so we can determine the edge direction.
    -- This is the same logic as a toggle flip-flop: Q_next = NOT Q.
    local old_value = self.value
    self.value = 1 - self.value
    self._total_ticks = self._total_ticks + 1

    -- Detect edge direction. A rising edge is a 0->1 transition;
    -- a falling edge is a 1->0 transition.
    local is_rising = old_value == 0 and self.value == 1
    local is_falling = old_value == 1 and self.value == 0

    -- Cycle count increments on each rising edge.
    -- This means cycle 1 starts at the first rising edge, cycle 2 at
    -- the second rising edge, etc. The falling edge belongs to the
    -- same cycle as the preceding rising edge.
    if is_rising then
        self.cycle = self.cycle + 1
    end

    local edge = ClockEdge.new(self.cycle, self.value, is_rising, is_falling)

    -- Notify all listeners -- this is the observer pattern.
    -- In real hardware, the clock signal propagates electrically to
    -- every connected component simultaneously. We simulate this by
    -- calling each listener function in order.
    for _, listener in ipairs(self._listeners) do
        listener(edge)
    end

    return edge
end

--- Executes one complete cycle (rising + falling edge).
---
--- A full cycle is two ticks:
---  1. Rising edge (0 -> 1): the "active" half
---  2. Falling edge (1 -> 0): the "idle" half
---
--- @return table, table The rising edge and falling edge
function Clock:full_cycle()
    local rising = self:tick()
    local falling = self:tick()
    return rising, falling
end

--- Executes N complete cycles and returns all edges.
---
--- Since each cycle has two edges (rising + falling), running N cycles
--- produces 2N edges total. This is useful for simulation: "run the clock
--- for 100 cycles and collect what happened."
---
--- @param cycles number Number of complete cycles to run (positive integer)
--- @return table Array of ClockEdge records (length = 2 * cycles)
function Clock:run(cycles)
    assert_positive_integer(cycles, "cycles")

    local edges = {}
    for _ = 1, cycles do
        local r, f = self:full_cycle()
        edges[#edges + 1] = r
        edges[#edges + 1] = f
    end
    return edges
end

--- Adds a function to be called on every clock edge.
---
--- In real hardware, this is like connecting a wire from the clock
--- to a component's clock input pin. The listener receives a ClockEdge
--- record on every tick.
---
--- @param listener function A function that accepts a ClockEdge
function Clock:register_listener(listener)
    assert_function(listener, "listener")
    self._listeners[#self._listeners + 1] = listener
end

--- Removes a previously registered listener by index.
---
--- Since Lua functions have identity (each closure is unique), we identify
--- listeners by their position in the listener list. Use the index
--- corresponding to registration order (1-based, Lua convention).
---
--- @param index number 1-based index of the listener to remove
--- @return boolean, string|nil True on success, or false + error message
function Clock:unregister_listener(index)
    assert_positive_integer(index, "index")

    if index > #self._listeners then
        error(
            "listener index " .. index .. " out of range [1, " .. #self._listeners .. "]",
            2
        )
    end

    table.remove(self._listeners, index)
    return true
end

--- Returns the number of registered listeners.
---
--- @return number
function Clock:listener_count()
    return #self._listeners
end

--- Restores the clock to its initial state.
---
--- Sets the value back to 0, cycle count to 0, and tick count to 0.
--- Listeners are preserved -- only the timing state is reset.
--- This is like hitting the reset button on an oscillator.
function Clock:reset()
    self.cycle = 0
    self.value = 0
    self._total_ticks = 0
end

--- Returns the clock period in nanoseconds.
---
--- The period is the time for one complete cycle (rising + falling).
--- For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.
---
--- Formula: period_ns = 1e9 / frequency_hz
---
--- @return number Period in nanoseconds
function Clock:period_ns()
    return 1e9 / self.frequency_hz
end

--- Returns the total number of half-cycles elapsed.
---
--- @return number
function Clock:total_ticks()
    return self._total_ticks
end

-- ---------------------------------------------------------------------------
-- ClockDivider -- frequency division
-- ---------------------------------------------------------------------------
-- In hardware, clock dividers are used to generate slower clocks from
-- a fast master clock. For example, a 1 GHz CPU clock might be divided
-- by 4 to get a 250 MHz bus clock.
--
-- How it works:
--   - Count rising edges from the source clock
--   - Every `divisor` rising edges, generate one full cycle on the output
--
-- Real-world uses:
--   - CPU-to-bus clock ratio (e.g., CPU at 4 GHz, bus at 1 GHz)
--   - USB clock derivation from system clock
--   - Audio sample rate generation from master clock

local ClockDivider = {}
ClockDivider.__index = ClockDivider

--- Creates a clock divider that produces a slower clock from a source.
---
--- The divisor must be >= 2 (dividing by 1 is a no-op and likely a bug).
--- The output clock's frequency is set to source.frequency_hz / divisor.
---
--- The divider automatically registers itself as a listener on the
--- source clock, so it starts working immediately.
---
--- @param source table A Clock instance (the faster source)
--- @param divisor number Division factor (integer >= 2)
--- @return table A ClockDivider instance
function ClockDivider.new(source, divisor)
    -- Validate the source is a Clock instance (duck-type check).
    if type(source) ~= "table" or type(source.tick) ~= "function" then
        error("source must be a Clock instance", 2)
    end
    if type(divisor) ~= "number" or divisor ~= math.floor(divisor) then
        error("divisor must be an integer, got " .. tostring(divisor), 2)
    end
    if divisor < 2 then
        error("divisor must be >= 2, got " .. tostring(divisor), 2)
    end

    local self = setmetatable({}, ClockDivider)

    self.source = source
    self.divisor = divisor
    -- The output clock runs at a divided-down frequency.
    -- math.floor handles the case where the division is not exact
    -- (e.g., 1000 / 3 = 333 Hz, not 333.33).
    self.output = Clock.new(math.floor(source.frequency_hz / divisor))
    self._counter = 0  -- Rising edge counter

    -- Register ourselves as a listener on the source clock.
    -- Every time the source ticks, our _on_edge method is called.
    source:register_listener(function(edge) self:_on_edge(edge) end)

    return self
end

--- Internal handler called on every source clock edge.
---
--- We only count rising edges. When we have counted `divisor` rising
--- edges, we generate one complete output cycle (rising + falling).
--- This is exactly how a hardware frequency divider works: it counts
--- input transitions and toggles the output every N counts.
---
--- @param edge table A ClockEdge from the source clock
function ClockDivider:_on_edge(edge)
    if edge.is_rising then
        self._counter = self._counter + 1
        if self._counter >= self.divisor then
            self._counter = 0
            self.output:tick()  -- rising
            self.output:tick()  -- falling
        end
    end
end

-- ---------------------------------------------------------------------------
-- MultiPhaseClock -- non-overlapping phase generation
-- ---------------------------------------------------------------------------
-- Used in CPU pipelines where different stages need offset clocks.
-- A 4-phase clock generates 4 non-overlapping clock signals, each
-- active for 1/4 of the master cycle.
--
-- Timing diagram for a 4-phase clock:
--
--   Source:   _|^|_|^|_|^|_|^|_
--   Phase 0:  _|^|___|___|___|_
--   Phase 1:  _|___|^|___|___|_
--   Phase 2:  _|___|___|^|___|_
--   Phase 3:  _|___|___|___|^|_
--
-- On each rising edge of the source, exactly ONE phase is active (1)
-- and all others are inactive (0). The active phase rotates.
--
-- Real-world uses:
--   - Classic RISC pipelines (fetch, decode, execute, writeback)
--   - DRAM refresh timing
--   - Multiplexed bus access

local MultiPhaseClock = {}
MultiPhaseClock.__index = MultiPhaseClock

--- Creates a multi-phase clock from a source clock.
---
--- The number of phases must be >= 2. The multi-phase clock registers
--- itself as a listener on the source clock and starts working immediately.
---
--- @param source table A Clock instance (the master clock)
--- @param phases number Number of phases (integer >= 2)
--- @return table A MultiPhaseClock instance
function MultiPhaseClock.new(source, phases)
    -- Validate the source is a Clock instance (duck-type check).
    if type(source) ~= "table" or type(source.tick) ~= "function" then
        error("source must be a Clock instance", 2)
    end
    if type(phases) ~= "number" or phases ~= math.floor(phases) then
        error("phases must be an integer, got " .. tostring(phases), 2)
    end
    if phases < 2 then
        error("phases must be >= 2, got " .. tostring(phases), 2)
    end

    local self = setmetatable({}, MultiPhaseClock)

    self.source = source
    self.phases = phases
    self.active_phase = 0  -- 0-indexed to match Go implementation

    -- Initialize all phase values to 0 (inactive).
    -- We use 1-based indexing for the internal array (Lua convention)
    -- but the public API uses 0-based phase indices to match the Go port
    -- and to align with hardware conventions (phase 0, phase 1, ...).
    self._phase_values = {}
    for i = 1, phases do
        self._phase_values[i] = 0
    end

    -- Register as a listener on the source clock.
    source:register_listener(function(edge) self:_on_edge(edge) end)

    return self
end

--- Returns the current value of phase N.
---
--- The phase index is 0-based (phase 0 through phase (phases-1)).
--- Returns 1 if the phase is active, 0 if inactive.
---
--- @param index number 0-based phase index
--- @return number 0 or 1
function MultiPhaseClock:get_phase(index)
    if type(index) ~= "number" or index ~= math.floor(index) then
        error("index must be an integer, got " .. tostring(index), 2)
    end
    if index < 0 or index >= self.phases then
        error(
            "phase index " .. index .. " out of range [0, " .. (self.phases - 1) .. "]",
            2
        )
    end
    -- Convert from 0-based public API to 1-based internal array.
    return self._phase_values[index + 1]
end

--- Internal handler called on every source clock edge.
---
--- On rising edges, we rotate the active phase. Only one phase
--- is high at any time -- this is the "non-overlapping" property
--- that prevents pipeline hazards.
---
--- @param edge table A ClockEdge from the source clock
function MultiPhaseClock:_on_edge(edge)
    if edge.is_rising then
        -- Reset all phases to 0 (inactive).
        for i = 1, self.phases do
            self._phase_values[i] = 0
        end
        -- Activate the current phase (convert 0-based to 1-based).
        self._phase_values[self.active_phase + 1] = 1
        -- Rotate to next phase using modular arithmetic.
        -- This creates the round-robin pattern: 0, 1, 2, ..., N-1, 0, 1, ...
        self.active_phase = (self.active_phase + 1) % self.phases
    end
end

-- ---------------------------------------------------------------------------
-- Module export table
-- ---------------------------------------------------------------------------
-- We export all three types plus a version string. The module follows the
-- Lua convention of returning a table that serves as the public API.

return {
    VERSION = "0.1.0",

    -- Core types
    Clock = Clock,
    ClockEdge = ClockEdge,
    ClockDivider = ClockDivider,
    MultiPhaseClock = MultiPhaseClock,
}
