-- ============================================================================
-- Tests for event_loop — event emitter and tick scheduler
-- ============================================================================
--
-- ## Testing Strategy
--
-- 1. on / emit — basic register-and-fire.
-- 2. once — fires exactly once.
-- 3. off — removes handlers correctly.
-- 4. Multiple handlers for same event all fire.
-- 5. on_tick / tick — fires tick handlers with correct delta_time.
-- 6. run — fires the right number of ticks.
-- 7. step — convenience alias for one tick.
-- 8. elapsed_time and tick_count are updated correctly.
-- 9. Handlers registered during emit do not fire for the current emit.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local EventLoop = require("coding_adventures.event_loop")

describe("EventLoop", function()

    -- -----------------------------------------------------------------------
    -- new()
    -- -----------------------------------------------------------------------
    describe("new()", function()

        it("starts with zero elapsed_time and tick_count", function()
            local loop = EventLoop.new()
            assert.are.equal(0.0, loop.elapsed_time)
            assert.are.equal(0,   loop.tick_count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- on / emit
    -- -----------------------------------------------------------------------
    describe("on() and emit()", function()

        it("fires registered handler when event is emitted", function()
            local loop = EventLoop.new()
            local fired = false
            loop:on("test", function(_data) fired = true end)
            loop:emit("test", {})
            assert.is_true(fired)
        end)

        it("passes data to the handler", function()
            local loop = EventLoop.new()
            local received = nil
            loop:on("msg", function(data) received = data.value end)
            loop:emit("msg", { value = 42 })
            assert.are.equal(42, received)
        end)

        it("does nothing when no handlers registered", function()
            local loop = EventLoop.new()
            assert.has_no.errors(function()
                loop:emit("nonexistent", {})
            end)
        end)

        it("multiple handlers for the same event all fire in order", function()
            local loop = EventLoop.new()
            local calls = {}
            loop:on("ev", function(_) table.insert(calls, "first")  end)
            loop:on("ev", function(_) table.insert(calls, "second") end)
            loop:on("ev", function(_) table.insert(calls, "third")  end)
            loop:emit("ev", nil)
            assert.are.same({"first", "second", "third"}, calls)
        end)

        it("handlers for different events are independent", function()
            local loop = EventLoop.new()
            local a_count, b_count = 0, 0
            loop:on("a", function(_) a_count = a_count + 1 end)
            loop:on("b", function(_) b_count = b_count + 1 end)
            loop:emit("a", nil)
            loop:emit("a", nil)
            loop:emit("b", nil)
            assert.are.equal(2, a_count)
            assert.are.equal(1, b_count)
        end)

        it("emitting same event multiple times fires handler each time", function()
            local loop = EventLoop.new()
            local count = 0
            loop:on("tick", function(_) count = count + 1 end)
            loop:emit("tick", nil)
            loop:emit("tick", nil)
            loop:emit("tick", nil)
            assert.are.equal(3, count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- once
    -- -----------------------------------------------------------------------
    describe("once()", function()

        it("fires exactly once even when emitted multiple times", function()
            local loop = EventLoop.new()
            local count = 0
            loop:once("ev", function(_) count = count + 1 end)
            loop:emit("ev", nil)
            loop:emit("ev", nil)
            loop:emit("ev", nil)
            assert.are.equal(1, count)
        end)

        it("passes data to the one-shot handler", function()
            local loop = EventLoop.new()
            local received = nil
            loop:once("init", function(data) received = data.version end)
            loop:emit("init", { version = "1.0" })
            assert.are.equal("1.0", received)
        end)

        it("does not affect other persistent handlers on the same event", function()
            local loop = EventLoop.new()
            local once_count = 0
            local always_count = 0
            loop:once("ev",   function(_) once_count   = once_count   + 1 end)
            loop:on("ev",     function(_) always_count = always_count + 1 end)
            loop:emit("ev", nil)
            loop:emit("ev", nil)
            assert.are.equal(1, once_count)
            assert.are.equal(2, always_count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- off
    -- -----------------------------------------------------------------------
    describe("off()", function()

        it("removes all handlers when callback is nil", function()
            local loop = EventLoop.new()
            local count = 0
            loop:on("ev", function(_) count = count + 1 end)
            loop:on("ev", function(_) count = count + 1 end)
            loop:off("ev")
            loop:emit("ev", nil)
            assert.are.equal(0, count)
        end)

        it("removes only the specified handler", function()
            local loop = EventLoop.new()
            local a_count, b_count = 0, 0
            local function handler_a(_) a_count = a_count + 1 end
            local function handler_b(_) b_count = b_count + 1 end
            loop:on("ev", handler_a)
            loop:on("ev", handler_b)
            loop:off("ev", handler_a)
            loop:emit("ev", nil)
            assert.are.equal(0, a_count)
            assert.are.equal(1, b_count)
        end)

        it("off on nonexistent event does not error", function()
            local loop = EventLoop.new()
            assert.has_no.errors(function()
                loop:off("no_such_event")
            end)
        end)

        it("off with specific handler on nonexistent event does not error", function()
            local loop = EventLoop.new()
            assert.has_no.errors(function()
                loop:off("no_such_event", function() end)
            end)
        end)

        it("removes only the first occurrence when handler is registered twice", function()
            local loop = EventLoop.new()
            local count = 0
            local function handler(_) count = count + 1 end
            loop:on("ev", handler)
            loop:on("ev", handler)
            loop:off("ev", handler)
            loop:emit("ev", nil)
            -- One handler remains after removing the first occurrence.
            assert.are.equal(1, count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- on_tick / tick
    -- -----------------------------------------------------------------------
    describe("on_tick() and tick()", function()

        it("fires tick handler on each tick call", function()
            local loop = EventLoop.new()
            local count = 0
            loop:on_tick(function(_dt) count = count + 1 end)
            loop:tick()
            loop:tick()
            loop:tick()
            assert.are.equal(3, count)
        end)

        it("passes delta_time to tick handlers", function()
            local loop = EventLoop.new()
            local received_dts = {}
            loop:on_tick(function(dt) table.insert(received_dts, dt) end)
            loop:tick(0.5)
            loop:tick(1.0)
            loop:tick(0.25)
            assert.are.same({0.5, 1.0, 0.25}, received_dts)
        end)

        it("uses default delta_time of 1.0 when not specified", function()
            local loop = EventLoop.new()
            local received = nil
            loop:on_tick(function(dt) received = dt end)
            loop:tick()
            assert.are.equal(1.0, received)
        end)

        it("multiple tick handlers all fire in order", function()
            local loop = EventLoop.new()
            local order = {}
            loop:on_tick(function(_) table.insert(order, "A") end)
            loop:on_tick(function(_) table.insert(order, "B") end)
            loop:tick()
            assert.are.same({"A", "B"}, order)
        end)

        it("updates elapsed_time correctly", function()
            local loop = EventLoop.new()
            loop:tick(0.5)
            loop:tick(0.25)
            loop:tick(1.0)
            assert.are.equal(1.75, loop.elapsed_time)
        end)

        it("increments tick_count on each tick", function()
            local loop = EventLoop.new()
            loop:tick()
            loop:tick()
            loop:tick()
            assert.are.equal(3, loop.tick_count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- run
    -- -----------------------------------------------------------------------
    describe("run()", function()

        it("runs the specified number of ticks", function()
            local loop = EventLoop.new()
            local count = 0
            loop:on_tick(function(_) count = count + 1 end)
            loop:run(7)
            assert.are.equal(7, count)
        end)

        it("uses the provided delta_time for each tick", function()
            local loop = EventLoop.new()
            loop:run(4, 0.25)
            assert.are.equal(1.0, loop.elapsed_time)
            assert.are.equal(4,   loop.tick_count)
        end)

        it("defaults to 1 tick and delta_time=1.0 when called with no args", function()
            local loop = EventLoop.new()
            loop:run()
            assert.are.equal(1.0, loop.elapsed_time)
            assert.are.equal(1,   loop.tick_count)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- step
    -- -----------------------------------------------------------------------
    describe("step()", function()

        it("runs exactly one tick", function()
            local loop = EventLoop.new()
            local count = 0
            loop:on_tick(function(_) count = count + 1 end)
            loop:step()
            assert.are.equal(1, count)
        end)

        it("passes delta_time to handlers", function()
            local loop = EventLoop.new()
            local received = nil
            loop:on_tick(function(dt) received = dt end)
            loop:step(0.016)
            assert.are.equal(0.016, received)
        end)

    end)

    -- -----------------------------------------------------------------------
    -- Interaction between emit and tick
    -- -----------------------------------------------------------------------
    describe("emit and tick together", function()

        it("tick handlers and event handlers coexist independently", function()
            local loop = EventLoop.new()
            local tick_count  = 0
            local event_count = 0
            loop:on_tick(function(_) tick_count  = tick_count  + 1 end)
            loop:on("hit",  function(_) event_count = event_count + 1 end)
            loop:run(3)
            loop:emit("hit", nil)
            loop:emit("hit", nil)
            assert.are.equal(3, tick_count)
            assert.are.equal(2, event_count)
        end)

    end)

end)
