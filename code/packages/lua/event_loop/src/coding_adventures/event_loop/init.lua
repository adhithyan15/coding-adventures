-- ============================================================================
-- event_loop — A simple event emitter and tick-based scheduler
-- ============================================================================
--
-- An event loop is the heartbeat of any interactive application.  It sits
-- at the top of the program and repeatedly checks "did anything happen?",
-- then dispatches whatever happened to the code that cares about it.
--
-- ## Two Models of Event Loops
--
-- ### Pull-based (polling) loops — used in games and simulations
--
--   while running:
--       tick(delta_time)        -- advance simulation clock
--       process_events()        -- check keyboard, network, timers…
--
-- ### Push-based (emitter) loops — used in GUI frameworks and servers
--
--   on("click",  function(e) … end)
--   on("resize", function(e) … end)
--   emit("click", {x=5, y=10})   -- fires all "click" handlers
--
-- This module implements **both** in one lightweight package:
--
--   - `emit` / `on` / `once` / `off` — push-based event emitter
--   - `on_tick` / `tick` / `run`    — pull-based tick scheduler
--
-- ## Why Have Both?
--
-- Real applications mix the two styles. A game loop calls `tick()` every
-- frame (pull), while keyboard and network events are best handled as
-- `emit("key", {key="Space"})` (push).  This module supports both patterns
-- with a unified API.
--
-- ## Event Emitter Pattern
--
-- The event emitter pattern decouples event *producers* from event
-- *consumers*:
--
--   Producer: emit("damage", {amount = 10, source = "sword"})
--   Consumer: on("damage", function(data) hp = hp - data.amount end)
--
-- The producer doesn't know how many consumers there are; consumers don't
-- know who is emitting.  This is the Observer pattern.
--
-- ## Usage
--
--   local EventLoop = require("coding_adventures.event_loop")
--
--   local loop = EventLoop.new()
--
--   loop:on("greet", function(data)
--       print("Hello, " .. data.name)
--   end)
--
--   loop:emit("greet", {name = "World"})   -- prints "Hello, World"
--
--   loop:once("startup", function(data)
--       print("Started with config: " .. tostring(data.version))
--   end)
--
--   loop:on_tick(function(dt)
--       -- called every tick with the delta-time in seconds
--   end)
--
--   loop:run(10)   -- run 10 ticks
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Constructor
-- ============================================================================

--- EventLoop.new creates a new, empty event loop.
--
-- The loop starts with:
--   - No registered event handlers.
--   - No registered tick handlers.
--   - elapsed_time = 0.
--   - tick_count   = 0.
--
-- @return  table  A new EventLoop instance.
function M.new()
    local self = {
        -- _handlers maps event_name → array of {fn = function, once = bool}
        _handlers    = {},
        -- _tick_handlers is an array of {fn = function, once = bool}
        _tick_handlers = {},
        -- elapsed_time accumulates the delta_times passed to tick()
        elapsed_time = 0.0,
        -- tick_count tracks how many ticks have been processed
        tick_count   = 0,
    }
    setmetatable(self, { __index = M })
    return self
end

-- ============================================================================
-- on — Register a persistent event handler
-- ============================================================================

--- on registers a callback to be called every time event_name is emitted.
--
-- Multiple handlers can be registered for the same event.  They are called
-- in the order they were registered.
--
-- This is the Observer pattern: the event loop is the *subject*, and the
-- callbacks are the *observers*.
--
-- @param event_name  string    The name of the event to listen for.
-- @param callback    function  function(data) called on each emit.
function M:on(event_name, callback)
    if not self._handlers[event_name] then
        self._handlers[event_name] = {}
    end
    table.insert(self._handlers[event_name], { fn = callback, once = false })
end

-- ============================================================================
-- once — Register a one-shot event handler
-- ============================================================================

--- once registers a callback that fires exactly once, then removes itself.
--
-- This is useful for "startup" or "first-frame" logic that should only run
-- once, such as initialising resources or logging a message on first use.
--
-- ## Implementation Note
--
-- The callback is wrapped in a table with once = true.  When emit() fires
-- the handler, it removes it from the handler list before calling the
-- callback, so even if the callback re-emits the same event, it won't
-- fire again.
--
-- @param event_name  string    The name of the event to listen for.
-- @param callback    function  function(data) called exactly once.
function M:once(event_name, callback)
    if not self._handlers[event_name] then
        self._handlers[event_name] = {}
    end
    table.insert(self._handlers[event_name], { fn = callback, once = true })
end

-- ============================================================================
-- off — Remove event handlers
-- ============================================================================

--- off removes event handlers for event_name.
--
-- ## Overloads
--
--   off(event_name)            — remove ALL handlers for this event.
--   off(event_name, callback)  — remove only the specified handler function.
--
-- When removing a specific callback, only the first occurrence is removed
-- (to match the behaviour of languages like JavaScript that do the same).
--
-- @param event_name  string             The event to modify.
-- @param callback    function | nil     Specific handler to remove, or nil to remove all.
function M:off(event_name, callback)
    if not self._handlers[event_name] then return end

    if callback == nil then
        -- Remove all handlers for this event.
        self._handlers[event_name] = {}
    else
        -- Remove the first handler whose fn matches callback.
        local handlers = self._handlers[event_name]
        for i = 1, #handlers do
            if handlers[i].fn == callback then
                table.remove(handlers, i)
                break
            end
        end
    end
end

-- ============================================================================
-- emit — Fire an event
-- ============================================================================

--- emit fires all handlers registered for event_name, passing data to each.
--
-- Handlers are called in registration order.  If a handler was registered
-- with `once`, it is removed before being called (so it cannot fire again
-- even if the callback re-emits the same event).
--
-- ## What Is `data`?
--
-- data can be any Lua value: a table, a number, a string, or nil.  The
-- convention is to pass a table (struct-like) so callers can add fields
-- without breaking existing handlers:
--
--   emit("damage", {amount = 10, type = "fire"})
--
-- @param event_name  string  The event to fire.
-- @param data        any     Payload passed to each handler.
function M:emit(event_name, data)
    local handlers = self._handlers[event_name]
    if not handlers or #handlers == 0 then return end

    -- Iterate over a snapshot so that removing `once` handlers during
    -- iteration does not skip or double-fire any handler.
    local snapshot = {}
    for i = 1, #handlers do snapshot[i] = handlers[i] end

    -- Process once-handlers: remove them from the live list first.
    for _, h in ipairs(snapshot) do
        if h.once then
            M.off(self, event_name, h.fn)
        end
    end

    -- Call all handlers in the snapshot.
    for _, h in ipairs(snapshot) do
        h.fn(data)
    end
end

-- ============================================================================
-- on_tick — Register a tick handler
-- ============================================================================

--- on_tick registers a callback to be called on every tick.
--
-- Tick handlers receive the delta_time (seconds since last tick) as their
-- argument.  This is the standard game-loop pattern for time-based updates:
--
--   loop:on_tick(function(dt)
--       position = position + velocity * dt
--   end)
--
-- @param callback  function  function(delta_time) called on each tick.
function M:on_tick(callback)
    table.insert(self._tick_handlers, { fn = callback, once = false })
end

-- ============================================================================
-- tick — Advance time by one step
-- ============================================================================

--- tick advances the event loop by one time step, firing all tick handlers.
--
-- The delta_time argument represents the elapsed time since the previous
-- tick (in seconds, or whatever unit the caller chooses).  All tick handlers
-- receive this value.
--
-- After calling all tick handlers, `elapsed_time` is increased by delta_time
-- and `tick_count` is incremented.
--
-- @param delta_time  number  Time step (default 1.0 if omitted).
function M:tick(delta_time)
    delta_time = delta_time or 1.0

    -- Snapshot to allow handlers to safely register/unregister during a tick.
    local snapshot = {}
    for i = 1, #self._tick_handlers do snapshot[i] = self._tick_handlers[i] end

    for _, h in ipairs(snapshot) do
        h.fn(delta_time)
    end

    self.elapsed_time = self.elapsed_time + delta_time
    self.tick_count   = self.tick_count + 1
end

-- ============================================================================
-- run — Execute n ticks
-- ============================================================================

--- run executes n_ticks ticks with a fixed delta_time.
--
-- This is the simplest possible main-loop driver.  For a game with a 60 Hz
-- fixed timestep, you would call run(60, 1/60) once per second.
--
-- @param n_ticks     int     Number of ticks to run (default 1).
-- @param delta_time  number  Time step per tick (default 1.0).
function M:run(n_ticks, delta_time)
    n_ticks    = n_ticks    or 1
    delta_time = delta_time or 1.0
    for _ = 1, n_ticks do
        self:tick(delta_time)
    end
end

-- ============================================================================
-- step — Execute exactly one tick
-- ============================================================================

--- step runs exactly one tick.  A convenience alias for run(1, delta_time).
--
-- @param delta_time  number  Time step (default 1.0).
function M:step(delta_time)
    self:tick(delta_time or 1.0)
end

return M
