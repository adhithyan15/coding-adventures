-- progress_bar — Terminal progress bar for tracking build execution.
--
-- This package is a Lua port of the Go progress-bar package from the
-- coding-adventures monorepo. It provides a text-based progress bar that
-- tracks operations and renders status updates to a writer (any object with
-- a :write() method, such as io.stderr).
--
-- # The postal worker analogy (adapted from the Go version)
--
-- Imagine a post office with a single clerk (the draw method) and a counter
-- (the Tracker object). Workers walk up one at a time (synchronous calls)
-- and hand over a slip (an event). The clerk updates the scoreboard
-- (the progress bar) immediately.
--
-- Unlike the Go version, which uses goroutines and channels for concurrent
-- event processing, this Lua version is synchronous. There are no background
-- threads — each call to :send() immediately updates state and redraws.
-- This is the right design for Lua, which is single-threaded by default.
--
-- # Usage
--
-- Flat (simple) mode:
--
--   local progress = require("coding_adventures.progress_bar")
--   local t = progress.new(21, io.stderr, "")
--   t:start()
--   t:send({ type = progress.STARTED, name = "pkg-a" })
--   t:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
--   t:send({ type = progress.SKIPPED, name = "pkg-b" })
--   t:stop()
--
-- Hierarchical mode (e.g., build levels):
--
--   local parent = progress.new(3, io.stderr, "Level")
--   parent:start()
--   local child = parent:child(7, "Package")
--   child:send({ type = progress.STARTED, name = "pkg-a" })
--   child:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
--   child:finish()   -- advances parent by 1
--   parent:stop()

local progress_bar = {}

-- ---------------------------------------------------------------------------
-- Module metadata
-- ---------------------------------------------------------------------------

progress_bar.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Event types — what can happen to a tracked item
-- ---------------------------------------------------------------------------

-- EventType constants distinguish the three things that can happen to an item.
--
-- Think of it like a traffic light:
--
--   STARTED  = green  (item is actively being processed)
--   FINISHED = red    (item is done — success or failure)
--   SKIPPED  = yellow (item was bypassed without processing)
--
-- We use simple string constants rather than numeric enums. Strings are
-- easier to debug ("STARTED" is more readable than 0 in a stack trace)
-- and Lua's string interning means equality checks are just pointer
-- comparisons — no performance penalty.

progress_bar.STARTED = "STARTED"   -- An item began processing (now "in-flight").
progress_bar.FINISHED = "FINISHED" -- An item completed (success or failure).
progress_bar.SKIPPED = "SKIPPED"   -- An item was skipped without processing.

-- ---------------------------------------------------------------------------
-- Tracker — the progress bar engine
-- ---------------------------------------------------------------------------

-- The Tracker is the main object. It receives events and renders a text-based
-- progress bar. In this Lua version, all updates are synchronous — calling
-- :send() immediately updates state and redraws.
--
-- # State tracking
--
-- The tracker maintains:
--
--   completed — count of items that are FINISHED or SKIPPED
--   building  — set of item names currently in-flight (STARTED but not FINISHED)
--   total     — the target count (set at creation time)
--
-- Truth table for state transitions:
--
--   Event     | completed | building
--   ----------+-----------+---------
--   STARTED   | unchanged | add name
--   FINISHED  | +1        | remove name
--   SKIPPED   | +1        | unchanged

local Tracker = {}
Tracker.__index = Tracker

-- new creates a Tracker that expects `total` items and writes to `writer`.
--
-- The `writer` must be any object with a :write(str) method. In practice,
-- this is usually io.stderr (for terminal output) or a table with a custom
-- :write() for testing.
--
-- The optional `label` parameter adds a prefix to the display line
-- (e.g., "Level" produces "Level 2/3 [####....] ..."). Pass "" or nil
-- for no label (flat mode).
--
-- Parameters:
--   total  (number)  — how many items to track
--   writer (table)   — object with :write(str) method
--   label  (string)  — optional prefix label ("" or nil for flat mode)
--
-- Returns:
--   A new Tracker instance.
function progress_bar.new(total, writer, label)
    local self = setmetatable({}, Tracker)
    self.total = total
    self.completed = 0
    self.building = {}         -- set: name -> true
    self.writer = writer
    self.label = label or ""
    self.start_time = nil      -- set by :start()
    self.parent = nil           -- set by :child() on the child
    self._started = false       -- guard: has :start() been called?
    return self
end

-- start initializes the tracker and records the start time.
--
-- Call this once before sending any events. The start time is used
-- to compute elapsed time shown in the progress bar.
--
-- In the Go version, Start() launches a background goroutine. Here,
-- it simply records the timestamp — all rendering happens synchronously
-- in :send().
function Tracker:start()
    self.start_time = os.clock()
    self._started = true
end

-- send submits an event to the tracker.
--
-- This immediately updates internal state and redraws the progress bar.
-- Unlike the Go version (which writes to a buffered channel), this is
-- synchronous — the caller blocks until the draw is complete.
--
-- The event table has three fields:
--
--   type   — one of progress_bar.STARTED, FINISHED, SKIPPED
--   name   — human-readable identifier (e.g., "python/logic-gates")
--   status — outcome label, only meaningful for FINISHED events
--            (e.g., "built", "failed", "cached")
--
-- If the tracker has not been started, send is a no-op. This is a
-- deliberate design choice matching the Go version: callers can
-- unconditionally call send without checking state.
function Tracker:send(event)
    if not self._started then
        return
    end

    -- Update state based on event type.
    --
    -- This mirrors the Go render() goroutine's switch statement,
    -- but happens inline instead of in a background loop.
    if event.type == progress_bar.STARTED then
        self.building[event.name] = true
    elseif event.type == progress_bar.FINISHED then
        self.building[event.name] = nil
        self.completed = self.completed + 1
    elseif event.type == progress_bar.SKIPPED then
        self.completed = self.completed + 1
    end

    self:_draw()
end

-- child creates a nested sub-tracker for hierarchical progress.
--
-- The child shares the parent's writer and start time. When the child
-- calls :finish(), it advances the parent's completed count by 1.
--
-- Example: a build system has 3 dependency levels, each with N packages.
-- The parent tracks levels (total=3, label="Level"), and each child
-- tracks packages within that level (total=N, label="Package").
--
--   parent = progress.new(3, io.stderr, "Level")
--   child = parent:child(7, "Package")
--   -- Display: Level 1/3  [####....] 3/7  Building: pkg-a  (2.1s)
--
-- Parameters:
--   total (number) — how many items the child tracks
--   label (string) — label for the child (e.g., "Package")
--
-- Returns:
--   A new Tracker instance linked to this parent.
function Tracker:child(total, label)
    local c = progress_bar.new(total, self.writer, label)
    c.start_time = self.start_time
    c.parent = self
    c._started = true
    return c
end

-- finish marks this child tracker as complete and advances the parent
-- tracker by one. Call this when all items in the child are done.
--
-- In the Go version, Finish() closes the channel and waits for the
-- renderer goroutine. Here, we just do a final draw and notify the parent.
function Tracker:finish()
    -- Final draw to ensure the bar shows the completed state.
    self:_draw()

    -- Notify the parent that this level/child is done.
    if self.parent then
        self.parent:send({
            type = progress_bar.FINISHED,
            name = self.label,
        })
    end
end

-- stop shuts down the tracker. It performs a final draw and writes a
-- newline so the last progress line is preserved in terminal scrollback.
--
-- In the Go version, Stop() closes the channel and waits for the
-- renderer goroutine to drain. Here, we just finalize output.
function Tracker:stop()
    self:_draw()
    self.writer:write("\n")
end

-- ---------------------------------------------------------------------------
-- Internal: rendering
-- ---------------------------------------------------------------------------

-- _draw composes and writes one progress line to the writer.
--
-- The line format depends on whether we have a parent (hierarchical)
-- or not (flat):
--
-- Flat:
--
--   [########............]  7/21  Building: pkg-a, pkg-b  (12.3s)
--
-- Hierarchical:
--
--   Level 2/3  [####................]  5/12  Building: pkg-a  (8.2s)
--
-- The bar uses Unicode block characters:
--
--   U+2588 (full block) — filled portion
--   U+2591 (light shade) — empty portion
--
-- We use \r (carriage return) to overwrite the current line. This works
-- on all platforms — Windows cmd, PowerShell, Git Bash, and Unix terminals.
-- No ANSI escape codes needed.
function Tracker:_draw()
    local elapsed = 0.0
    if self.start_time then
        elapsed = os.clock() - self.start_time
    end

    -- --- Build the progress bar ---
    --
    -- The bar is 20 characters wide. The number of filled characters is
    -- proportional to completed/total:
    --
    --   filled = (completed * 20) / total
    --
    -- Integer division (via math.floor) naturally rounds down, so the bar
    -- only shows 100% when all items are truly complete.
    local bar_width = 20
    local filled = 0
    if self.total > 0 then
        filled = math.floor((self.completed * bar_width) / self.total)
    end
    if filled > bar_width then
        filled = bar_width
    end

    -- Build the bar string using Unicode block characters.
    -- "\u{2588}" is the filled block, "\u{2591}" is the empty block.
    local bar = string.rep("\u{2588}", filled) .. string.rep("\u{2591}", bar_width - filled)

    -- --- Build the in-flight names list ---
    local activity = format_activity(self.building, self.completed, self.total)

    -- --- Compose the line ---
    local line
    if self.parent then
        -- Hierarchical: show parent label and count.
        -- +1 because this child is the "current" one being processed.
        local parent_completed = self.parent.completed + 1
        line = string.format("\r%s %d/%d  [%s]  %d/%d  %s  (%.1fs)",
            self.parent.label, parent_completed, self.parent.total,
            bar, self.completed, self.total, activity, elapsed)
    elseif self.label ~= "" then
        -- Labeled flat tracker (used as parent — shows own state).
        line = string.format("\r%s %d/%d  [%s]  %s  (%.1fs)",
            self.label, self.completed, self.total, bar, activity, elapsed)
    else
        -- Flat mode: just the bar.
        line = string.format("\r[%s]  %d/%d  %s  (%.1fs)",
            bar, self.completed, self.total, activity, elapsed)
    end

    -- Pad to 80 characters to overwrite any previous longer line.
    -- Lua's string.format %-80s left-aligns and pads with spaces.
    self.writer:write(string.format("%-80s", line))
end

-- ---------------------------------------------------------------------------
-- Internal: activity formatting
-- ---------------------------------------------------------------------------

-- format_activity builds the "Building: pkg-a, pkg-b" or "waiting..."
-- or "done" string from the current in-flight set.
--
-- The rules:
--
--   | In-flight count | Completed vs Total | Output                       |
--   |-----------------|--------------------|------------------------------|
--   | 0               | completed < total  | "waiting..."                 |
--   | 0               | completed == total | "done"                       |
--   | 1-3             | any                | "Building: a, b, c"          |
--   | 4+              | any                | "Building: a, b, c +N more"  |
--
-- Names are sorted alphabetically for deterministic output — this matches
-- the Go version and makes testing predictable.
function format_activity(building, completed, total)
    -- Count items in the building set.
    -- Lua tables don't have a built-in length for hash-style tables,
    -- so we must iterate to count.
    local names = {}
    for name, _ in pairs(building) do
        names[#names + 1] = name
    end

    if #names == 0 then
        if completed >= total then
            return "done"
        end
        return "waiting..."
    end

    -- Sort alphabetically for deterministic output.
    table.sort(names)

    local max_names = 3
    if #names <= max_names then
        return "Building: " .. table.concat(names, ", ")
    end

    -- Show first 3 names plus "+N more" suffix.
    local shown = {}
    for i = 1, max_names do
        shown[i] = names[i]
    end
    return string.format("Building: %s +%d more",
        table.concat(shown, ", "), #names - max_names)
end

-- Export format_activity for testing.
-- In real usage you'd never call this directly, but exposing it lets
-- tests verify the formatting logic in isolation (matching the Go tests).
progress_bar._format_activity = format_activity

-- Export the Tracker metatable so tests can verify types.
progress_bar._Tracker = Tracker

return progress_bar
