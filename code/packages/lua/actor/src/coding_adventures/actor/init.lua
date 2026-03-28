-- coding_adventures.actor
-- ============================================================================
--
-- THE ACTOR MODEL FOR CONCURRENT COMPUTATION
--
-- The Actor Model, introduced by Carl Hewitt in 1973, is a mathematical model
-- of concurrent computation. In this model, the fundamental unit of
-- computation is an ACTOR — an entity that:
--
--   1. Has its own private STATE (nobody else can read or write it directly)
--   2. Communicates only through MESSAGES (no shared memory)
--   3. Can, in response to a message:
--      - Change its own state
--      - Send messages to other actors
--      - Create new actors
--      - Stop itself
--
-- This design eliminates the need for locks, mutexes, and semaphores — the
-- root causes of most concurrency bugs (race conditions, deadlocks).
--
-- ARCHITECTURE
-- ------------
-- ActorSystem
--   The top-level container. Holds a registry of actors and a global message
--   queue. In a real system, this would schedule actors across threads or
--   processes. Here, we run synchronously for simplicity and testability.
--
-- Actor (internal)
--   Stored in the system's actor registry. Tracks:
--     id            — unique string identifier
--     state         — the actor's current private state (any Lua value)
--     behavior      — function(state, message) → ActorResult
--     stopped       — boolean flag: true means the actor is dead
--
-- ActorResult
--   The return value from a behavior function. It tells the system what to do:
--     new_state       — the actor's updated state
--     messages_to_send — list of [target_id, message] pairs to dispatch
--     actors_to_create — list of ActorSpec tables to spawn
--     stop            — if true, halt this actor after this message
--
-- ActorSpec
--   Description of a new actor to create:
--     actor_id      — the id for the new actor
--     initial_state — its starting state
--     behavior      — its behavior function
--
-- MESSAGE QUEUE
-- -------------
-- The queue is a simple Lua array (FIFO). Each entry is { target_id, message }.
-- system:run() drains the queue until it is empty. New messages added during
-- processing are appended to the tail and will be processed in the same run.
--
-- DEAD LETTERS
-- ------------
-- If a message is sent to an actor that doesn't exist or has been stopped,
-- the message is recorded in system.dead_letters for inspection/debugging.
-- This mirrors Akka's DeadLetterChannel.
--
-- SINGLE-THREADED SEMANTICS
-- -------------------------
-- Because Lua is single-threaded, system:run() processes messages one at a
-- time in queue order. This gives sequential, deterministic, testable behavior.
-- Real actor systems (Erlang, Akka) use true concurrency, but the API is the
-- same.
--
-- Usage:
--   local Actor = require("coding_adventures.actor")
--   local system = Actor.ActorSystem.new()
--   local id = system:spawn("counter", 0, function(state, msg)
--       if msg.type == "inc" then
--           return Actor.ActorResult.new({ new_state = state + 1 })
--       end
--   end)
--   system:send(id, {type="inc"})
--   system:run()
--   print(system:get_state(id))  -- 1
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- ActorResult
-- ---------------------------------------------------------------------------

M.ActorResult = {}
M.ActorResult.__index = M.ActorResult

-- ActorResult.new(opts)
--   Create an ActorResult with sensible defaults.
--
--   opts.new_state        — the actor's next state (required; no default)
--   opts.messages_to_send — list of {target_id, message} pairs (default {})
--   opts.actors_to_create — list of ActorSpec tables (default {})
--   opts.stop             — boolean; if true, actor is stopped (default false)
--
--   Returns: an ActorResult table
function M.ActorResult.new(opts)
    opts = opts or {}
    return setmetatable({
        new_state        = opts.new_state,
        messages_to_send = opts.messages_to_send or {},
        actors_to_create = opts.actors_to_create or {},
        stop             = opts.stop or false,
    }, M.ActorResult)
end

-- ---------------------------------------------------------------------------
-- ActorSpec
-- ---------------------------------------------------------------------------
--
-- A plain table describing a new actor to spawn:
--   { actor_id = "...", initial_state = ..., behavior = function(...) end }
--
-- No constructor needed — just use a table literal.

-- ---------------------------------------------------------------------------
-- ActorSystem
-- ---------------------------------------------------------------------------

M.ActorSystem = {}
M.ActorSystem.__index = M.ActorSystem

-- ActorSystem.new()
--   Create a new actor system with no actors and an empty message queue.
--
--   Fields:
--     actors      — map from id (string) → actor table
--     queue       — FIFO array of { target_id, message }
--     dead_letters — array of { target_id, message } for undeliverable messages
function M.ActorSystem.new()
    return setmetatable({
        actors       = {},
        queue        = {},
        dead_letters = {},
    }, M.ActorSystem)
end

-- system:spawn(id, initial_state, behavior)
--   Register a new actor in the system.
--
--   Parameters:
--     id            (string)   — unique identifier for this actor
--     initial_state (any)      — the actor's starting state
--     behavior      (function) — function(state, msg) → ActorResult
--
--   Returns: `id` (for convenient chaining: local id = system:spawn(...))
--
--   Raises an error if an actor with the same id already exists.
function M.ActorSystem:spawn(id, initial_state, behavior)
    assert(type(id) == "string", "ActorSystem:spawn — id must be a string")
    assert(self.actors[id] == nil,
        "ActorSystem:spawn — actor '" .. id .. "' already exists")
    self.actors[id] = {
        id      = id,
        state   = initial_state,
        behavior = behavior,
        stopped = false,
    }
    return id
end

-- system:send(target_id, message)
--   Enqueue a message for delivery to actor `target_id`.
--
--   The message is not delivered immediately — it waits until system:run()
--   is called. This mirrors real actor systems where message delivery is
--   asynchronous.
function M.ActorSystem:send(target_id, message)
    self.queue[#self.queue + 1] = { target_id, message }
end

-- system:run()
--   Process all queued messages until the queue is empty.
--
--   For each message in the queue:
--     1. Find the target actor.
--     2. If the actor doesn't exist or is stopped, add to dead_letters.
--     3. Otherwise, call behavior(state, msg) to get an ActorResult.
--     4. Apply the result:
--        a. Update the actor's state to result.new_state
--        b. Enqueue each message in result.messages_to_send
--        c. Spawn each actor in result.actors_to_create
--        d. If result.stop is true, mark the actor as stopped
--
--   New messages added during step 4b are processed in the SAME run.
--   This guarantees that system:run() fully drains all work triggered by
--   the current batch.
function M.ActorSystem:run()
    local i = 1
    while i <= #self.queue do
        local entry = self.queue[i]
        local target_id = entry[1]
        local message   = entry[2]
        i = i + 1

        local actor = self.actors[target_id]

        -- Dead-letter: no such actor, or actor has stopped
        if actor == nil or actor.stopped then
            self.dead_letters[#self.dead_letters + 1] = { target_id, message }
        else
            -- Invoke the behavior with the current state and message
            local result = actor.behavior(actor.state, message)

            if result ~= nil then
                -- Update state
                actor.state = result.new_state

                -- Enqueue outgoing messages
                if result.messages_to_send then
                    for _, pair in ipairs(result.messages_to_send) do
                        self:send(pair[1], pair[2])
                    end
                end

                -- Spawn new actors
                if result.actors_to_create then
                    for _, spec in ipairs(result.actors_to_create) do
                        self:spawn(spec.actor_id, spec.initial_state, spec.behavior)
                    end
                end

                -- Stop the actor if requested
                if result.stop then
                    actor.stopped = true
                end
            end
        end
    end
    -- Clear the queue now that it's fully drained
    self.queue = {}
end

-- system:get_state(id)
--   Return the current state of actor `id`.
--   Raises an error if the actor does not exist.
function M.ActorSystem:get_state(id)
    local actor = self.actors[id]
    assert(actor ~= nil, "ActorSystem:get_state — no actor with id '" .. tostring(id) .. "'")
    return actor.state
end

-- system:is_stopped(id)
--   Return true if the actor with `id` has been stopped, false otherwise.
--   Raises an error if the actor does not exist.
function M.ActorSystem:is_stopped(id)
    local actor = self.actors[id]
    assert(actor ~= nil, "ActorSystem:is_stopped — no actor with id '" .. tostring(id) .. "'")
    return actor.stopped
end

-- ---------------------------------------------------------------------------

return M
