-- Tests for progress-bar.
--
-- This test suite mirrors the Go test suite (progress_test.go) to ensure
-- the Lua port has equivalent behavior. We use busted as the test framework.
--
-- Because the Lua version is synchronous (no goroutines), we don't need
-- sleep() calls or concurrency tests. Instead, we can directly inspect
-- the output buffer after each :send() call.

-- Add src/ to the module search path so we can require the package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local progress = require("coding_adventures.progress_bar")

-- ---------------------------------------------------------------------------
-- Helper: a string buffer that captures :write() calls
-- ---------------------------------------------------------------------------

-- MockWriter acts as an io.Writer substitute. It accumulates everything
-- written to it in a string buffer, which tests can inspect.
--
-- This is the Lua equivalent of Go's bytes.Buffer.
local function MockWriter()
    local self = { buffer = "" }
    function self:write(str)
        self.buffer = self.buffer .. str
    end
    return self
end

-- ---------------------------------------------------------------------------
-- Helper: run a tracker and return its output
-- ---------------------------------------------------------------------------

-- run_tracker creates a Tracker backed by a MockWriter, sends the given
-- events, then stops it and returns everything written to the buffer.
--
-- This mirrors the Go runTracker helper.
local function run_tracker(total, label, events)
    local writer = MockWriter()
    local tracker = progress.new(total, writer, label)
    tracker:start()
    if events then
        for _, e in ipairs(events) do
            tracker:send(e)
        end
    end
    tracker:stop()
    return writer.buffer
end

-- ---------------------------------------------------------------------------
-- Tests for module metadata
-- ---------------------------------------------------------------------------

describe("progress-bar module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", progress.VERSION)
    end)

    it("exports event type constants", function()
        assert.are.equal("STARTED", progress.STARTED)
        assert.are.equal("FINISHED", progress.FINISHED)
        assert.are.equal("SKIPPED", progress.SKIPPED)
    end)

    it("exports the new constructor", function()
        assert.is_function(progress.new)
    end)

    it("exports _format_activity for testing", function()
        assert.is_function(progress._format_activity)
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for Tracker construction
-- ---------------------------------------------------------------------------

describe("Tracker construction", function()
    it("creates a tracker with correct total", function()
        local writer = MockWriter()
        local tracker = progress.new(10, writer, "")
        assert.are.equal(10, tracker.total)
    end)

    it("creates a tracker with zero completed", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        assert.are.equal(0, tracker.completed)
    end)

    it("creates a tracker with empty building set", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        -- building should be an empty table
        local count = 0
        for _ in pairs(tracker.building) do count = count + 1 end
        assert.are.equal(0, count)
    end)

    it("stores the label", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "Level")
        assert.are.equal("Level", tracker.label)
    end)

    it("defaults label to empty string when nil", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, nil)
        assert.are.equal("", tracker.label)
    end)

    it("uses the Tracker metatable", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        assert.are.equal(progress._Tracker, getmetatable(tracker))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for event counting and basic rendering
-- ---------------------------------------------------------------------------

describe("empty tracker", function()
    -- TestEmptyTracker: a tracker with zero events renders a zeroed-out bar
    -- with "waiting..." status.
    it("shows 0/N counter", function()
        local out = run_tracker(5, "", nil)
        assert.truthy(out:find("0/5", 1, true))
    end)

    it("shows 'waiting...' for idle state", function()
        local out = run_tracker(5, "", nil)
        assert.truthy(out:find("waiting...", 1, true))
    end)
end)

describe("STARTED event", function()
    -- TestStartedEvent: a STARTED event adds the item name to the
    -- "Building:" display without incrementing the completed counter.
    it("does not increment completed counter", function()
        local out = run_tracker(5, "", {
            { type = progress.STARTED, name = "pkg-a" },
        })
        assert.truthy(out:find("0/5", 1, true))
    end)

    it("shows the item name in the building list", function()
        local out = run_tracker(5, "", {
            { type = progress.STARTED, name = "pkg-a" },
        })
        assert.truthy(out:find("pkg-a", 1, true))
    end)

    it("shows 'Building:' prefix", function()
        local out = run_tracker(5, "", {
            { type = progress.STARTED, name = "pkg-a" },
        })
        assert.truthy(out:find("Building:", 1, true))
    end)
end)

describe("FINISHED event", function()
    -- TestFinishedEvent: a FINISHED event increments the completed counter
    -- and removes the item from the building set.
    it("increments completed counter", function()
        local out = run_tracker(1, "", {
            { type = progress.STARTED, name = "pkg-a" },
            { type = progress.FINISHED, name = "pkg-a", status = "built" },
        })
        assert.truthy(out:find("1/1", 1, true))
    end)

    it("shows 'done' when all items complete", function()
        local out = run_tracker(1, "", {
            { type = progress.STARTED, name = "pkg-a" },
            { type = progress.FINISHED, name = "pkg-a", status = "built" },
        })
        assert.truthy(out:find("done", 1, true))
    end)

    it("removes item from building set", function()
        local writer = MockWriter()
        local tracker = progress.new(2, writer, "")
        tracker:start()
        tracker:send({ type = progress.STARTED, name = "pkg-a" })
        tracker:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
        -- After finishing, pkg-a should not be in the building set
        assert.is_nil(tracker.building["pkg-a"])
    end)
end)

describe("SKIPPED event", function()
    -- TestSkippedEvent: a SKIPPED event increments the completed counter
    -- without going through the building state.
    it("increments completed counter", function()
        local out = run_tracker(3, "", {
            { type = progress.SKIPPED, name = "pkg-b" },
        })
        assert.truthy(out:find("1/3", 1, true))
    end)

    it("does not add to building set", function()
        local writer = MockWriter()
        local tracker = progress.new(3, writer, "")
        tracker:start()
        tracker:send({ type = progress.SKIPPED, name = "pkg-b" })
        assert.is_nil(tracker.building["pkg-b"])
    end)
end)

describe("mixed events", function()
    -- TestMixedEvents: a realistic sequence with some started+finished,
    -- some skipped.
    it("counts all completed items correctly", function()
        local out = run_tracker(3, "", {
            { type = progress.SKIPPED, name = "pkg-a" },
            { type = progress.SKIPPED, name = "pkg-b" },
            { type = progress.STARTED, name = "pkg-c" },
            { type = progress.FINISHED, name = "pkg-c", status = "built" },
        })
        assert.truthy(out:find("3/3", 1, true))
    end)

    it("shows 'done' when all items processed", function()
        local out = run_tracker(3, "", {
            { type = progress.SKIPPED, name = "pkg-a" },
            { type = progress.SKIPPED, name = "pkg-b" },
            { type = progress.STARTED, name = "pkg-c" },
            { type = progress.FINISHED, name = "pkg-c", status = "built" },
        })
        assert.truthy(out:find("done", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for bar rendering
-- ---------------------------------------------------------------------------

describe("bar characters", function()
    -- TestBarCharacters: the bar contains Unicode block characters.
    it("contains filled block character", function()
        local out = run_tracker(4, "", {
            { type = progress.SKIPPED, name = "a" },
            { type = progress.SKIPPED, name = "b" },
        })
        -- 2/4 = 50% -> 10 filled, 10 empty
        assert.truthy(out:find("\u{2588}"))
    end)

    it("contains empty block character", function()
        local out = run_tracker(4, "", {
            { type = progress.SKIPPED, name = "a" },
            { type = progress.SKIPPED, name = "b" },
        })
        assert.truthy(out:find("\u{2591}"))
    end)
end)

describe("bar fully filled", function()
    -- TestBarFullyFilled: the bar is 100% filled when all items are complete.
    it("shows 20 filled blocks", function()
        local out = run_tracker(1, "", {
            { type = progress.SKIPPED, name = "a" },
        })
        local full_bar = string.rep("\u{2588}", 20)
        assert.truthy(out:find(full_bar, 1, true))
    end)
end)

describe("bar empty", function()
    -- TestBarEmpty: the bar is 0% filled when no items are complete.
    it("shows 20 empty blocks", function()
        local out = run_tracker(5, "", nil)
        local empty_bar = string.rep("\u{2591}", 20)
        assert.truthy(out:find(empty_bar, 1, true))
    end)
end)

describe("bar proportional fill", function()
    it("fills half the bar at 50%", function()
        local out = run_tracker(4, "", {
            { type = progress.SKIPPED, name = "a" },
            { type = progress.SKIPPED, name = "b" },
        })
        -- 2/4 = 50% -> 10 filled, 10 empty
        local half_bar = string.rep("\u{2588}", 10) .. string.rep("\u{2591}", 10)
        assert.truthy(out:find(half_bar, 1, true))
    end)

    it("fills quarter of bar at 25%", function()
        local out = run_tracker(4, "", {
            { type = progress.SKIPPED, name = "a" },
        })
        -- 1/4 = 25% -> 5 filled, 15 empty
        local quarter_bar = string.rep("\u{2588}", 5) .. string.rep("\u{2591}", 15)
        assert.truthy(out:find(quarter_bar, 1, true))
    end)

    it("caps filled at bar width when completed exceeds total", function()
        -- Edge case: more events than total shouldn't overflow the bar.
        local out = run_tracker(2, "", {
            { type = progress.SKIPPED, name = "a" },
            { type = progress.SKIPPED, name = "b" },
            { type = progress.SKIPPED, name = "c" },  -- exceeds total
        })
        local full_bar = string.rep("\u{2588}", 20)
        assert.truthy(out:find(full_bar, 1, true))
    end)
end)

describe("bar with zero total", function()
    -- Edge case: total of 0 should not cause division by zero.
    it("shows empty bar without crashing", function()
        local out = run_tracker(0, "", nil)
        local empty_bar = string.rep("\u{2591}", 20)
        assert.truthy(out:find(empty_bar, 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for name truncation
-- ---------------------------------------------------------------------------

describe("name truncation", function()
    -- TestNameTruncation: when more than 3 items are in-flight, only the
    -- first 3 (alphabetically) are shown with a "+N more" suffix.
    it("shows first 3 names alphabetically with +N more", function()
        local out = run_tracker(10, "", {
            { type = progress.STARTED, name = "delta" },
            { type = progress.STARTED, name = "alpha" },
            { type = progress.STARTED, name = "charlie" },
            { type = progress.STARTED, name = "bravo" },
            { type = progress.STARTED, name = "echo" },
        })
        assert.truthy(out:find("alpha", 1, true))
        assert.truthy(out:find("bravo", 1, true))
        assert.truthy(out:find("charlie", 1, true))
        assert.truthy(out:find("+2 more", 1, true))
    end)

    -- TestThreeNamesNoTruncation: exactly 3 in-flight items are shown
    -- without the "+N more" suffix.
    it("shows all 3 names without truncation suffix", function()
        local out = run_tracker(10, "", {
            { type = progress.STARTED, name = "a" },
            { type = progress.STARTED, name = "b" },
            { type = progress.STARTED, name = "c" },
        })
        assert.falsy(out:find("more", 1, true))
    end)

    it("shows single name without truncation", function()
        local out = run_tracker(10, "", {
            { type = progress.STARTED, name = "solo" },
        })
        assert.truthy(out:find("Building: solo", 1, true))
        assert.falsy(out:find("more", 1, true))
    end)

    it("shows two names without truncation", function()
        local out = run_tracker(10, "", {
            { type = progress.STARTED, name = "zeta" },
            { type = progress.STARTED, name = "alpha" },
        })
        -- Should be sorted: alpha, zeta
        assert.truthy(out:find("alpha", 1, true))
        assert.truthy(out:find("zeta", 1, true))
        assert.falsy(out:find("more", 1, true))
    end)

    it("shows +1 more for 4 items", function()
        local out = run_tracker(10, "", {
            { type = progress.STARTED, name = "a" },
            { type = progress.STARTED, name = "b" },
            { type = progress.STARTED, name = "c" },
            { type = progress.STARTED, name = "d" },
        })
        assert.truthy(out:find("+1 more", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for elapsed time
-- ---------------------------------------------------------------------------

describe("elapsed time", function()
    -- TestElapsedTimeFormat: elapsed time appears in the output in the
    -- expected format (parenthesized, with 's' suffix).
    it("shows elapsed time with 's)' suffix", function()
        local out = run_tracker(1, "", nil)
        assert.truthy(out:find("s)", 1, true))
    end)

    it("shows elapsed time in parentheses", function()
        local out = run_tracker(1, "", nil)
        -- Should match pattern like (0.0s) or (1.2s)
        assert.truthy(out:find("%(.*s%)"))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for labeled (flat) mode
-- ---------------------------------------------------------------------------

describe("labeled tracker", function()
    -- TestLabeledTracker: a label prefix appears in the output.
    it("shows the label prefix", function()
        local out = run_tracker(3, "Level", {
            { type = progress.SKIPPED, name = "a" },
        })
        assert.truthy(out:find("Level", 1, true))
    end)

    it("shows the counter with label", function()
        local out = run_tracker(3, "Level", {
            { type = progress.SKIPPED, name = "a" },
        })
        assert.truthy(out:find("1/3", 1, true))
    end)

    it("shows label before the bar", function()
        local out = run_tracker(3, "Level", {
            { type = progress.SKIPPED, name = "a" },
        })
        -- Label should appear before the bar bracket
        local label_pos = out:find("Level", 1, true)
        local bar_pos = out:find("%[")
        assert.truthy(label_pos)
        assert.truthy(bar_pos)
        assert.truthy(label_pos < bar_pos)
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for hierarchical progress
-- ---------------------------------------------------------------------------

describe("hierarchical progress", function()
    -- TestHierarchicalProgress: a child tracker shows the parent's label
    -- and count alongside the child's progress.
    it("shows parent label in child output", function()
        local writer = MockWriter()
        local parent = progress.new(3, writer, "Level")
        parent:start()

        local child = parent:child(2, "Package")
        child:send({ type = progress.STARTED, name = "pkg-a" })
        child:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
        child:send({ type = progress.SKIPPED, name = "pkg-b" })
        child:finish()

        parent:stop()

        local out = writer.buffer
        assert.truthy(out:find("Level", 1, true))
    end)

    it("shows item names in child output", function()
        local writer = MockWriter()
        local parent = progress.new(3, writer, "Level")
        parent:start()

        local child = parent:child(2, "Package")
        child:send({ type = progress.STARTED, name = "pkg-a" })
        child:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
        child:finish()

        parent:stop()

        local out = writer.buffer
        assert.truthy(out:find("pkg-a", 1, true))
    end)

    -- TestHierarchicalParentAdvances: calling :finish() on a child
    -- advances the parent's completed count.
    it("advances parent completed count on child finish", function()
        local writer = MockWriter()
        local parent = progress.new(2, writer, "Level")
        parent:start()

        local child1 = parent:child(1, "Pkg")
        child1:send({ type = progress.SKIPPED, name = "a" })
        child1:finish()

        local child2 = parent:child(1, "Pkg")
        child2:send({ type = progress.SKIPPED, name = "b" })
        child2:finish()

        parent:stop()

        local out = writer.buffer
        assert.truthy(out:find("2/2", 1, true))
    end)

    it("child shares parent start time", function()
        local writer = MockWriter()
        local parent = progress.new(2, writer, "Level")
        parent:start()

        local child = parent:child(1, "Pkg")
        assert.are.equal(parent.start_time, child.start_time)

        child:finish()
        parent:stop()
    end)

    it("child shares parent writer", function()
        local writer = MockWriter()
        local parent = progress.new(2, writer, "Level")
        parent:start()

        local child = parent:child(1, "Pkg")
        assert.are.equal(writer, child.writer)

        child:finish()
        parent:stop()
    end)

    it("child is automatically started", function()
        local writer = MockWriter()
        local parent = progress.new(2, writer, "Level")
        parent:start()

        local child = parent:child(1, "Pkg")
        assert.is_true(child._started)

        child:finish()
        parent:stop()
    end)

    it("shows parent count as current+1 for active child", function()
        local writer = MockWriter()
        local parent = progress.new(3, writer, "Level")
        parent:start()

        -- First child — parent.completed is 0, display should show 1/3
        local child = parent:child(1, "Pkg")
        child:send({ type = progress.SKIPPED, name = "a" })

        -- The last draw from child should contain "Level 1/3"
        assert.truthy(writer.buffer:find("Level 1/3", 1, true))

        child:finish()
        parent:stop()
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for :send() before :start()
-- ---------------------------------------------------------------------------

describe("send before start", function()
    it("is a no-op", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        -- Send without calling :start() first
        tracker:send({ type = progress.STARTED, name = "pkg-a" })
        -- Nothing should have been written
        assert.are.equal("", writer.buffer)
    end)

    it("does not modify state", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        tracker:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
        assert.are.equal(0, tracker.completed)
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for carriage return
-- ---------------------------------------------------------------------------

describe("carriage return", function()
    it("each draw starts with \\r for line overwriting", function()
        local out = run_tracker(3, "", {
            { type = progress.SKIPPED, name = "a" },
        })
        assert.truthy(out:find("\r", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for output padding
-- ---------------------------------------------------------------------------

describe("output padding", function()
    it("pads output to at least 80 characters per draw", function()
        local writer = MockWriter()
        local tracker = progress.new(1, writer, "")
        tracker:start()
        tracker:send({ type = progress.SKIPPED, name = "a" })

        -- The last write before stop should be padded.
        -- Each draw produces a string formatted with %-80s.
        -- We check that the output contains spaces for padding.
        local out = writer.buffer
        -- The output should contain trailing spaces from padding.
        assert.truthy(#out >= 80)
        tracker:stop()
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for :stop()
-- ---------------------------------------------------------------------------

describe("stop", function()
    it("appends a newline at the end", function()
        local out = run_tracker(1, "", nil)
        assert.truthy(out:sub(-1) == "\n")
    end)

    it("performs a final draw before newline", function()
        local out = run_tracker(1, "", {
            { type = progress.SKIPPED, name = "a" },
        })
        -- Should show the final state (1/1 done) before the newline
        assert.truthy(out:find("1/1", 1, true))
        assert.truthy(out:find("done", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for format_activity helper (direct unit tests)
-- ---------------------------------------------------------------------------

describe("format_activity", function()
    -- TestFormatActivityEmpty: empty building set with incomplete items
    -- returns "waiting..."
    it("returns 'waiting...' when empty and not done", function()
        local result = progress._format_activity({}, 0, 5)
        assert.are.equal("waiting...", result)
    end)

    -- TestFormatActivityDone: empty building set with all items complete
    -- returns "done"
    it("returns 'done' when empty and all complete", function()
        local result = progress._format_activity({}, 5, 5)
        assert.are.equal("done", result)
    end)

    it("returns 'done' when completed exceeds total", function()
        local result = progress._format_activity({}, 6, 5)
        assert.are.equal("done", result)
    end)

    -- TestFormatActivityOneItem: single in-flight item
    it("returns 'Building: name' for single item", function()
        local result = progress._format_activity({ alpha = true }, 0, 5)
        assert.are.equal("Building: alpha", result)
    end)

    it("returns sorted names for two items", function()
        local result = progress._format_activity(
            { zeta = true, alpha = true }, 0, 5)
        assert.are.equal("Building: alpha, zeta", result)
    end)

    it("returns sorted names for three items", function()
        local result = progress._format_activity(
            { charlie = true, alpha = true, bravo = true }, 0, 5)
        assert.are.equal("Building: alpha, bravo, charlie", result)
    end)

    -- TestFormatActivityTruncated: more than 3 items shows "+N more"
    it("truncates with '+N more' for 5 items", function()
        local building = {
            alpha = true, bravo = true, charlie = true,
            delta = true, echo = true,
        }
        local result = progress._format_activity(building, 0, 10)
        assert.truthy(result:find("+2 more", 1, true))
        assert.truthy(result:find("^Building: alpha"))
    end)

    it("truncates with '+1 more' for 4 items", function()
        local building = {
            alpha = true, bravo = true, charlie = true, delta = true,
        }
        local result = progress._format_activity(building, 0, 10)
        assert.truthy(result:find("+1 more", 1, true))
    end)

    it("shows exactly 3 names in truncated output", function()
        local building = {
            alpha = true, bravo = true, charlie = true,
            delta = true, echo = true, foxtrot = true,
        }
        local result = progress._format_activity(building, 0, 10)
        assert.truthy(result:find("alpha", 1, true))
        assert.truthy(result:find("bravo", 1, true))
        assert.truthy(result:find("charlie", 1, true))
        assert.falsy(result:find("delta", 1, true))
        assert.falsy(result:find("echo", 1, true))
        assert.falsy(result:find("foxtrot", 1, true))
        assert.truthy(result:find("+3 more", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for line format (flat vs hierarchical vs labeled)
-- ---------------------------------------------------------------------------

describe("line format", function()
    it("flat mode starts with \\r[ (bar first)", function()
        local out = run_tracker(3, "", {
            { type = progress.SKIPPED, name = "a" },
        })
        -- Flat mode: \r[bar]  N/M  activity  (Xs)
        assert.truthy(out:find("\r%["))
    end)

    it("labeled mode starts with \\rLabel", function()
        local out = run_tracker(3, "Level", {
            { type = progress.SKIPPED, name = "a" },
        })
        assert.truthy(out:find("\rLevel"))
    end)

    it("hierarchical mode shows parent label and child bar", function()
        local writer = MockWriter()
        local parent = progress.new(3, writer, "Level")
        parent:start()

        local child = parent:child(5, "Package")
        child:send({ type = progress.SKIPPED, name = "a" })

        local out = writer.buffer
        -- Should contain parent label, parent counter display,
        -- child bar, and child counter
        assert.truthy(out:find("Level", 1, true))
        assert.truthy(out:find("1/5", 1, true))

        child:finish()
        parent:stop()
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for multiple sequential operations
-- ---------------------------------------------------------------------------

describe("sequential operations", function()
    it("tracks multiple start-finish pairs", function()
        local out = run_tracker(3, "", {
            { type = progress.STARTED, name = "pkg-a" },
            { type = progress.FINISHED, name = "pkg-a", status = "built" },
            { type = progress.STARTED, name = "pkg-b" },
            { type = progress.FINISHED, name = "pkg-b", status = "built" },
            { type = progress.STARTED, name = "pkg-c" },
            { type = progress.FINISHED, name = "pkg-c", status = "built" },
        })
        assert.truthy(out:find("3/3", 1, true))
        assert.truthy(out:find("done", 1, true))
    end)

    it("handles interleaved starts and finishes", function()
        local writer = MockWriter()
        local tracker = progress.new(3, writer, "")
        tracker:start()

        tracker:send({ type = progress.STARTED, name = "a" })
        tracker:send({ type = progress.STARTED, name = "b" })
        -- Both should be in building
        assert.is_true(tracker.building["a"] == true)
        assert.is_true(tracker.building["b"] == true)

        tracker:send({ type = progress.FINISHED, name = "a", status = "ok" })
        -- Only b should remain
        assert.is_nil(tracker.building["a"])
        assert.is_true(tracker.building["b"] == true)
        assert.are.equal(1, tracker.completed)

        tracker:send({ type = progress.FINISHED, name = "b", status = "ok" })
        assert.is_nil(tracker.building["b"])
        assert.are.equal(2, tracker.completed)

        tracker:stop()
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for edge cases
-- ---------------------------------------------------------------------------

describe("edge cases", function()
    it("handles finishing an item that was never started", function()
        -- This shouldn't crash — FINISHED on unknown name just increments
        -- completed (the delete from building is a no-op for missing keys).
        local out = run_tracker(2, "", {
            { type = progress.FINISHED, name = "phantom", status = "ok" },
        })
        assert.truthy(out:find("1/2", 1, true))
    end)

    it("handles total of 1", function()
        local out = run_tracker(1, "", {
            { type = progress.SKIPPED, name = "only" },
        })
        assert.truthy(out:find("1/1", 1, true))
        assert.truthy(out:find("done", 1, true))
    end)

    it("handles very large total", function()
        local out = run_tracker(10000, "", {
            { type = progress.SKIPPED, name = "first" },
        })
        assert.truthy(out:find("1/10000", 1, true))
    end)

    it("handles empty name", function()
        local out = run_tracker(1, "", {
            { type = progress.STARTED, name = "" },
        })
        -- Should not crash; building set has "" as a key
        assert.truthy(out:find("Building:", 1, true))
    end)

    it("handles special characters in name", function()
        local out = run_tracker(1, "", {
            { type = progress.STARTED, name = "pkg/sub-module" },
        })
        assert.truthy(out:find("pkg/sub-module", 1, true))
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for multiple children in sequence
-- ---------------------------------------------------------------------------

describe("multiple children", function()
    it("each child finish advances parent", function()
        local writer = MockWriter()
        local parent = progress.new(3, writer, "Level")
        parent:start()

        for i = 1, 3 do
            local child = parent:child(1, "Pkg")
            child:send({ type = progress.SKIPPED, name = "item-" .. i })
            child:finish()
        end

        -- Parent should now be at 3/3
        assert.are.equal(3, parent.completed)
        parent:stop()
    end)
end)

-- ---------------------------------------------------------------------------
-- Tests for draw state after each event type
-- ---------------------------------------------------------------------------

describe("internal state tracking", function()
    it("STARTED adds name to building set", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        tracker:start()
        tracker:send({ type = progress.STARTED, name = "foo" })
        assert.is_true(tracker.building["foo"] == true)
        tracker:stop()
    end)

    it("FINISHED removes name from building set", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        tracker:start()
        tracker:send({ type = progress.STARTED, name = "foo" })
        tracker:send({ type = progress.FINISHED, name = "foo", status = "ok" })
        assert.is_nil(tracker.building["foo"])
        tracker:stop()
    end)

    it("SKIPPED does not affect building set", function()
        local writer = MockWriter()
        local tracker = progress.new(5, writer, "")
        tracker:start()
        tracker:send({ type = progress.SKIPPED, name = "foo" })
        assert.is_nil(tracker.building["foo"])
        tracker:stop()
    end)

    it("completed count matches expected after mixed events", function()
        local writer = MockWriter()
        local tracker = progress.new(10, writer, "")
        tracker:start()
        tracker:send({ type = progress.SKIPPED, name = "a" })     -- +1
        tracker:send({ type = progress.SKIPPED, name = "b" })     -- +1
        tracker:send({ type = progress.STARTED, name = "c" })     -- no change
        tracker:send({ type = progress.FINISHED, name = "c" })    -- +1
        tracker:send({ type = progress.STARTED, name = "d" })     -- no change
        tracker:send({ type = progress.STARTED, name = "e" })     -- no change
        tracker:send({ type = progress.FINISHED, name = "d" })    -- +1
        assert.are.equal(4, tracker.completed)
        tracker:stop()
    end)
end)
