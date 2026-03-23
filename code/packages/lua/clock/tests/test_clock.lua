-- Tests for the clock package.
--
-- These tests verify the fundamental clock behavior: signal toggling,
-- edge detection, cycle counting, listener notification, frequency
-- division, and multi-phase generation.
--
-- Every test mirrors (and extends) the Go test suite to ensure the Lua
-- port is behavior-identical.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local clock_mod = require("coding_adventures.clock")
local Clock = clock_mod.Clock
local ClockEdge = clock_mod.ClockEdge
local ClockDivider = clock_mod.ClockDivider
local MultiPhaseClock = clock_mod.MultiPhaseClock

-- ===========================================================================
-- ClockEdge
-- ===========================================================================

describe("ClockEdge", function()

    it("stores fields correctly", function()
        local edge = ClockEdge.new(3, 1, true, false)
        assert.are.equal(3, edge.cycle)
        assert.are.equal(1, edge.value)
        assert.is_true(edge.is_rising)
        assert.is_false(edge.is_falling)
    end)

    it("accepts cycle 0", function()
        local edge = ClockEdge.new(0, 0, false, false)
        assert.are.equal(0, edge.cycle)
    end)

    it("rejects non-integer cycle", function()
        assert.has_error(function()
            ClockEdge.new(1.5, 0, false, false)
        end)
    end)

    it("rejects negative cycle", function()
        assert.has_error(function()
            ClockEdge.new(-1, 0, false, false)
        end)
    end)

    it("rejects invalid value", function()
        assert.has_error(function()
            ClockEdge.new(1, 2, true, false)
        end)
    end)

    it("rejects non-boolean is_rising", function()
        assert.has_error(function()
            ClockEdge.new(1, 0, 1, false)
        end)
    end)

    it("rejects non-boolean is_falling", function()
        assert.has_error(function()
            ClockEdge.new(1, 0, false, "no")
        end)
    end)

    it("rejects string cycle", function()
        assert.has_error(function()
            ClockEdge.new("abc", 0, false, false)
        end)
    end)
end)

-- ===========================================================================
-- Clock -- basic behavior
-- ===========================================================================

describe("Clock", function()

    describe("constructor", function()

        it("starts at value 0", function()
            local clk = Clock.new(1000000)
            assert.are.equal(0, clk.value)
        end)

        it("starts at cycle 0", function()
            local clk = Clock.new(1000000)
            assert.are.equal(0, clk.cycle)
        end)

        it("starts with zero ticks", function()
            local clk = Clock.new(1000000)
            assert.are.equal(0, clk:total_ticks())
        end)

        it("stores custom frequency", function()
            local clk = Clock.new(3000000000)
            assert.are.equal(3000000000, clk.frequency_hz)
        end)

        it("rejects zero frequency", function()
            assert.has_error(function()
                Clock.new(0)
            end)
        end)

        it("rejects negative frequency", function()
            assert.has_error(function()
                Clock.new(-100)
            end)
        end)

        it("rejects non-integer frequency", function()
            assert.has_error(function()
                Clock.new(1.5)
            end)
        end)

        it("rejects string frequency", function()
            assert.has_error(function()
                Clock.new("fast")
            end)
        end)

        it("rejects nil frequency", function()
            assert.has_error(function()
                Clock.new(nil)
            end)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Tick behavior
    -- -----------------------------------------------------------------------

    describe("tick", function()

        it("first tick is a rising edge", function()
            local clk = Clock.new(1000000)
            local edge = clk:tick()
            assert.is_true(edge.is_rising)
            assert.is_false(edge.is_falling)
            assert.are.equal(1, edge.value)
            assert.are.equal(1, clk.value)
        end)

        it("second tick is a falling edge", function()
            local clk = Clock.new(1000000)
            clk:tick()  -- rising
            local edge = clk:tick()
            assert.is_false(edge.is_rising)
            assert.is_true(edge.is_falling)
            assert.are.equal(0, edge.value)
        end)

        it("alternates correctly over 10 ticks", function()
            local clk = Clock.new(1000000)
            for i = 1, 10 do
                local edge = clk:tick()
                if i % 2 == 1 then
                    assert.is_true(edge.is_rising, "tick " .. i .. " should be rising")
                else
                    assert.is_true(edge.is_falling, "tick " .. i .. " should be falling")
                end
            end
        end)

        it("increments cycle on rising edges", function()
            local clk = Clock.new(1000000)

            local edge1 = clk:tick()  -- rising
            assert.are.equal(1, edge1.cycle)
            assert.are.equal(1, clk.cycle)

            local edge2 = clk:tick()  -- falling
            assert.are.equal(1, edge2.cycle)
            assert.are.equal(1, clk.cycle)

            local edge3 = clk:tick()  -- rising
            assert.are.equal(2, edge3.cycle)
            assert.are.equal(2, clk.cycle)
        end)

        it("increments tick count on every tick", function()
            local clk = Clock.new(1000000)
            clk:tick()
            assert.are.equal(1, clk:total_ticks())
            clk:tick()
            assert.are.equal(2, clk:total_ticks())
            clk:tick()
            assert.are.equal(3, clk:total_ticks())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- full_cycle
    -- -----------------------------------------------------------------------

    describe("full_cycle", function()

        it("returns rising then falling", function()
            local clk = Clock.new(1000000)
            local rising, falling = clk:full_cycle()
            assert.is_true(rising.is_rising)
            assert.is_true(falling.is_falling)
        end)

        it("ends at value 0", function()
            local clk = Clock.new(1000000)
            clk:full_cycle()
            assert.are.equal(0, clk.value)
        end)

        it("increments cycle count to 1", function()
            local clk = Clock.new(1000000)
            clk:full_cycle()
            assert.are.equal(1, clk.cycle)
        end)

        it("produces two ticks", function()
            local clk = Clock.new(1000000)
            clk:full_cycle()
            assert.are.equal(2, clk:total_ticks())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- run
    -- -----------------------------------------------------------------------

    describe("run", function()

        it("produces correct edge count", function()
            local clk = Clock.new(1000000)
            local edges = clk:run(5)
            assert.are.equal(10, #edges)
        end)

        it("edges alternate rising/falling", function()
            local clk = Clock.new(1000000)
            local edges = clk:run(3)
            for i, edge in ipairs(edges) do
                if i % 2 == 1 then
                    assert.is_true(edge.is_rising, "edge " .. i .. " should be rising")
                else
                    assert.is_true(edge.is_falling, "edge " .. i .. " should be falling")
                end
            end
        end)

        it("sets final cycle count correctly", function()
            local clk = Clock.new(1000000)
            clk:run(7)
            assert.are.equal(7, clk.cycle)
        end)

        it("rejects zero cycles", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:run(0)
            end)
        end)

        it("rejects negative cycles", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:run(-1)
            end)
        end)

        it("rejects non-integer cycles", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:run(2.5)
            end)
        end)

        it("produces 14 ticks for 7 cycles", function()
            local clk = Clock.new(1000000)
            clk:run(7)
            assert.are.equal(14, clk:total_ticks())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Listeners
    -- -----------------------------------------------------------------------

    describe("listeners", function()

        it("listener is called on tick", function()
            local clk = Clock.new(1000000)
            local received = {}
            clk:register_listener(function(edge)
                received[#received + 1] = edge
            end)
            clk:tick()
            assert.are.equal(1, #received)
            assert.is_true(received[1].is_rising)
        end)

        it("listener sees all edges", function()
            local clk = Clock.new(1000000)
            local received = {}
            clk:register_listener(function(edge)
                received[#received + 1] = edge
            end)
            clk:run(3)
            assert.are.equal(6, #received)
        end)

        it("multiple listeners all receive edges", function()
            local clk = Clock.new(1000000)
            local a, b = {}, {}
            clk:register_listener(function(edge) a[#a + 1] = edge end)
            clk:register_listener(function(edge) b[#b + 1] = edge end)
            clk:tick()
            assert.are.equal(1, #a)
            assert.are.equal(1, #b)
        end)

        it("unregister stops notification", function()
            local clk = Clock.new(1000000)
            local received = {}
            clk:register_listener(function(edge)
                received[#received + 1] = edge
            end)
            clk:tick()  -- 1 edge received
            clk:unregister_listener(1)  -- 1-based index
            clk:tick()  -- should NOT be received
            assert.are.equal(1, #received)
        end)

        it("unregister out of range errors", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:unregister_listener(1)  -- no listeners
            end)
        end)

        it("unregister zero index errors", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:unregister_listener(0)
            end)
        end)

        it("unregister negative index errors", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:unregister_listener(-1)
            end)
        end)

        it("listener_count tracks correctly", function()
            local clk = Clock.new(1000000)
            assert.are.equal(0, clk:listener_count())
            clk:register_listener(function() end)
            assert.are.equal(1, clk:listener_count())
            clk:register_listener(function() end)
            assert.are.equal(2, clk:listener_count())
        end)

        it("rejects non-function listener", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:register_listener("not a function")
            end)
        end)

        it("rejects nil listener", function()
            local clk = Clock.new(1000000)
            assert.has_error(function()
                clk:register_listener(nil)
            end)
        end)

        it("unregister with non-integer errors", function()
            local clk = Clock.new(1000000)
            clk:register_listener(function() end)
            assert.has_error(function()
                clk:unregister_listener(1.5)
            end)
        end)

        it("listener_count decrements after unregister", function()
            local clk = Clock.new(1000000)
            clk:register_listener(function() end)
            clk:register_listener(function() end)
            assert.are.equal(2, clk:listener_count())
            clk:unregister_listener(1)
            assert.are.equal(1, clk:listener_count())
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Reset
    -- -----------------------------------------------------------------------

    describe("reset", function()

        it("resets value to 0", function()
            local clk = Clock.new(1000000)
            clk:tick()
            clk:reset()
            assert.are.equal(0, clk.value)
        end)

        it("resets cycle to 0", function()
            local clk = Clock.new(1000000)
            clk:run(5)
            clk:reset()
            assert.are.equal(0, clk.cycle)
        end)

        it("resets ticks to 0", function()
            local clk = Clock.new(1000000)
            clk:run(5)
            clk:reset()
            assert.are.equal(0, clk:total_ticks())
        end)

        it("preserves listeners", function()
            local clk = Clock.new(1000000)
            local received = {}
            clk:register_listener(function(edge)
                received[#received + 1] = edge
            end)
            clk:run(3)  -- 6 edges
            clk:reset()  -- listeners preserved
            clk:tick()   -- 1 more edge
            assert.are.equal(7, #received)
        end)

        it("preserves frequency", function()
            local clk = Clock.new(5000000)
            clk:run(10)
            clk:reset()
            assert.are.equal(5000000, clk.frequency_hz)
        end)

        it("allows normal operation after reset", function()
            local clk = Clock.new(1000000)
            clk:run(5)
            clk:reset()
            local edge = clk:tick()
            assert.is_true(edge.is_rising)
            assert.are.equal(1, edge.cycle)
        end)
    end)

    -- -----------------------------------------------------------------------
    -- Period calculation
    -- -----------------------------------------------------------------------

    describe("period_ns", function()

        it("1 MHz = 1000 ns", function()
            local clk = Clock.new(1000000)
            assert.are.equal(1000.0, clk:period_ns())
        end)

        it("1 GHz = 1 ns", function()
            local clk = Clock.new(1000000000)
            assert.are.equal(1.0, clk:period_ns())
        end)

        it("3 GHz ~ 0.333 ns", function()
            local clk = Clock.new(3000000000)
            local expected = 1e9 / 3000000000
            assert.is_true(math.abs(clk:period_ns() - expected) < 1e-10)
        end)

        it("100 Hz = 10000000 ns", function()
            local clk = Clock.new(100)
            assert.are.equal(10000000.0, clk:period_ns())
        end)
    end)
end)

-- ===========================================================================
-- ClockDivider
-- ===========================================================================

describe("ClockDivider", function()

    it("divides by 2", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 2)
        master:run(4)
        assert.are.equal(2, divider.output.cycle)
    end)

    it("divides by 4", function()
        local master = Clock.new(1000000000)
        local divider = ClockDivider.new(master, 4)
        master:run(8)
        assert.are.equal(2, divider.output.cycle)
    end)

    it("sets output frequency correctly", function()
        local master = Clock.new(1000000000)
        local divider = ClockDivider.new(master, 4)
        assert.are.equal(250000000, divider.output.frequency_hz)
    end)

    it("output value returns to 0 after full output cycle", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 2)
        master:run(2)
        assert.are.equal(0, divider.output.value)
    end)

    it("rejects divisor of 1", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            ClockDivider.new(master, 1)
        end)
    end)

    it("rejects divisor of 0", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            ClockDivider.new(master, 0)
        end)
    end)

    it("rejects negative divisor", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            ClockDivider.new(master, -1)
        end)
    end)

    it("rejects non-integer divisor", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            ClockDivider.new(master, 2.5)
        end)
    end)

    it("rejects nil source", function()
        assert.has_error(function()
            ClockDivider.new(nil, 2)
        end)
    end)

    it("rejects non-Clock source", function()
        assert.has_error(function()
            ClockDivider.new("not a clock", 2)
        end)
    end)

    it("output has correct tick count", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 3)
        master:run(6)
        -- 6 master rising edges / 3 = 2 output cycles = 4 output ticks
        assert.are.equal(4, divider.output:total_ticks())
    end)

    it("divider output can have its own listeners", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 2)
        local received = {}
        divider.output:register_listener(function(edge)
            received[#received + 1] = edge
        end)
        master:run(4)
        -- 4 master rising edges / 2 = 2 output cycles = 4 output ticks
        assert.are.equal(4, #received)
    end)

    it("handles non-exact division (floor)", function()
        -- 1000 Hz / 3 = 333 Hz (floored)
        local master = Clock.new(1000)
        local divider = ClockDivider.new(master, 3)
        assert.are.equal(333, divider.output.frequency_hz)
    end)

    it("divides by large divisor", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 100)
        master:run(300)
        assert.are.equal(3, divider.output.cycle)
    end)
end)

-- ===========================================================================
-- MultiPhaseClock
-- ===========================================================================

describe("MultiPhaseClock", function()

    it("all phases start at 0", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)
        for i = 0, 3 do
            assert.are.equal(0, mpc:get_phase(i))
        end
    end)

    it("first rising edge activates phase 0", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)
        master:tick()  -- rising edge
        assert.are.equal(1, mpc:get_phase(0))
        for i = 1, 3 do
            assert.are.equal(0, mpc:get_phase(i))
        end
    end)

    it("phases rotate on rising edges", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)

        for expected_phase = 0, 3 do
            master:tick()  -- rising
            for p = 0, 3 do
                local expected = (p == expected_phase) and 1 or 0
                assert.are.equal(expected, mpc:get_phase(p),
                    "phase " .. p .. " expected " .. expected ..
                    " when active phase is " .. expected_phase)
            end
            master:tick()  -- falling (no change to phases)
        end
    end)

    it("phases wrap around", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 3)

        -- 3 rising edges cycle through phases 0, 1, 2
        for _ = 1, 3 do
            master:full_cycle()
        end

        -- 4th rising edge should activate phase 0 again
        master:tick()
        assert.are.equal(1, mpc:get_phase(0))
        assert.are.equal(0, mpc:get_phase(1))
        assert.are.equal(0, mpc:get_phase(2))
    end)

    it("at most one phase is active at any time", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)

        for _ = 1, 20 do
            master:tick()
            local active_count = 0
            for i = 0, 3 do
                if mpc:get_phase(i) == 1 then
                    active_count = active_count + 1
                end
            end
            assert.is_true(active_count <= 1, "more than one phase active")
        end
    end)

    it("rejects phases < 2", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            MultiPhaseClock.new(master, 1)
        end)
    end)

    it("rejects phases = 0", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            MultiPhaseClock.new(master, 0)
        end)
    end)

    it("rejects negative phases", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            MultiPhaseClock.new(master, -2)
        end)
    end)

    it("rejects non-integer phases", function()
        local master = Clock.new(1000000)
        assert.has_error(function()
            MultiPhaseClock.new(master, 3.5)
        end)
    end)

    it("two-phase clock works", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 2)

        master:tick()  -- rising -> phase 0 active
        assert.are.equal(1, mpc:get_phase(0))
        assert.are.equal(0, mpc:get_phase(1))

        master:tick()  -- falling -> no change
        master:tick()  -- rising -> phase 1 active
        assert.are.equal(0, mpc:get_phase(0))
        assert.are.equal(1, mpc:get_phase(1))
    end)

    it("rejects nil source", function()
        assert.has_error(function()
            MultiPhaseClock.new(nil, 4)
        end)
    end)

    it("rejects non-Clock source", function()
        assert.has_error(function()
            MultiPhaseClock.new({}, 4)
        end)
    end)

    it("get_phase rejects out of range index", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)
        assert.has_error(function()
            mpc:get_phase(4)
        end)
        assert.has_error(function()
            mpc:get_phase(-1)
        end)
    end)

    it("get_phase rejects non-integer index", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 4)
        assert.has_error(function()
            mpc:get_phase(1.5)
        end)
    end)

    it("active_phase tracks correctly", function()
        local master = Clock.new(1000000)
        local mpc = MultiPhaseClock.new(master, 3)
        assert.are.equal(0, mpc.active_phase)
        master:tick()  -- rising: activates phase 0, then rotates to 1
        assert.are.equal(1, mpc.active_phase)
        master:tick()  -- falling: no change
        assert.are.equal(1, mpc.active_phase)
        master:tick()  -- rising: activates phase 1, then rotates to 2
        assert.are.equal(2, mpc.active_phase)
    end)
end)

-- ===========================================================================
-- Module-level tests
-- ===========================================================================

describe("clock module", function()

    it("has a version", function()
        assert.are.equal("0.1.0", clock_mod.VERSION)
    end)

    it("exports Clock", function()
        assert.is_not_nil(clock_mod.Clock)
    end)

    it("exports ClockEdge", function()
        assert.is_not_nil(clock_mod.ClockEdge)
    end)

    it("exports ClockDivider", function()
        assert.is_not_nil(clock_mod.ClockDivider)
    end)

    it("exports MultiPhaseClock", function()
        assert.is_not_nil(clock_mod.MultiPhaseClock)
    end)
end)

-- ===========================================================================
-- Integration tests
-- ===========================================================================

describe("integration", function()

    it("divider with multi-phase on output", function()
        -- A realistic scenario: master clock -> divider -> multi-phase
        -- This models a CPU where the master clock is divided down and then
        -- split into pipeline phases.
        local master = Clock.new(4000000000)  -- 4 GHz master
        local divider = ClockDivider.new(master, 4)  -- 1 GHz divided
        local mpc = MultiPhaseClock.new(divider.output, 4)  -- 4-phase pipeline

        -- Run 16 master cycles. This produces 4 divider output cycles,
        -- which rotates through all 4 phases once.
        master:run(16)

        -- After 16 master rising edges / 4 = 4 output cycles.
        -- 4 output rising edges rotate through phases 0,1,2,3
        -- and wrap back to 0. So active_phase should be back at 0.
        assert.are.equal(0, mpc.active_phase)
        assert.are.equal(4, divider.output.cycle)
    end)

    it("listener on source and divider both fire", function()
        local master = Clock.new(1000000)
        local master_edges = {}
        master:register_listener(function(edge) master_edges[#master_edges + 1] = edge end)

        local divider = ClockDivider.new(master, 2)
        local output_edges = {}
        divider.output:register_listener(function(edge)
            output_edges[#output_edges + 1] = edge
        end)

        master:run(4)
        assert.are.equal(8, #master_edges)   -- 4 cycles * 2 ticks
        assert.are.equal(4, #output_edges)   -- 2 output cycles * 2 ticks
    end)

    it("chained dividers", function()
        -- 8 GHz -> /2 -> 4 GHz -> /2 -> 2 GHz
        local master = Clock.new(8000000000)
        local div1 = ClockDivider.new(master, 2)
        local div2 = ClockDivider.new(div1.output, 2)

        master:run(8)
        -- 8 master rising edges / 2 = 4 div1 output cycles
        -- 4 div1 rising edges / 2 = 2 div2 output cycles
        assert.are.equal(4, div1.output.cycle)
        assert.are.equal(2, div2.output.cycle)
        assert.are.equal(2000000000, div2.output.frequency_hz)
    end)

    it("reset does not affect divider listeners", function()
        local master = Clock.new(1000000)
        local divider = ClockDivider.new(master, 2)
        master:run(4)
        master:reset()
        -- The divider listener is still registered; running again should work
        master:run(4)
        -- Total: 4 output cycles from first run + 4 from second = 8
        -- But the divider counter does NOT reset, so it continues counting.
        -- After reset, master value is 0 again. First tick is rising.
        -- The divider internal counter still has its state from before.
        assert.are.equal(4, divider.output.cycle)
    end)
end)
