-- silent_waiting — A no-op Waiting plug-in for the REPL
--
-- # What Is a Waiting Plug-in?
--
-- When a REPL evaluates user input it may take noticeable time — imagine a
-- language that compiles and runs a program. During that wait, users benefit
-- from visual feedback: a spinner, a progress bar, an animation.
--
-- The Waiting plug-in models that feedback mechanism. Its contract is four
-- methods:
--
--   waiting.start()         → state
--     Called once just before evaluation begins.
--     Returns an opaque "state" value that carries whatever the animation
--     needs across ticks (frame counter, start time, etc.).
--
--   waiting.tick(state)     → state
--     Called repeatedly while waiting. Each call should advance the animation
--     one frame and return the new state.
--     In Lua, because eval is synchronous (see note below), tick is never
--     called between start() and stop(). The method exists so plug-ins written
--     for other runtimes work here too.
--
--   waiting.tick_ms()       → integer
--     How many milliseconds the caller should sleep between ticks.
--     Ignored in this implementation but used by richer hosts.
--
--   waiting.stop(state)     → nil
--     Called once after evaluation completes.
--     Use this to erase the spinner or print a newline.
--
-- # On Lua's Synchronous Eval
--
-- Lua has coroutines but not true threads. Coroutines are cooperative: only
-- one runs at a time, and they must explicitly yield. This means that once
-- we call language.eval(), no other code runs until eval() returns.
--
-- Consequence: waiting.tick() can never be called between start() and stop()
-- in a standard Lua host. A multi-threaded host (e.g., via LuaJIT's thread
-- library) could call tick() from a second thread, but we make no such
-- assumption here.
--
-- This is an honest limitation documented rather than papered over. The
-- interface remains correct: a richer host can call tick() if it has a way
-- to do so; a standard host simply doesn't.
--
-- # SilentWaiting
--
-- SilentWaiting is the default: every method is a no-op. It produces no
-- output and keeps no state (using nil as the state token). It is the right
-- default for:
--
--   - Batch mode (no terminal to display on)
--   - Testing (where animations would clutter output)
--   - Fast evaluators (where the delay is imperceptible)

local SilentWaiting = {}

-- start() — begin the waiting animation.
--
-- Returns:
--   nil — SilentWaiting keeps no state, so nil is the state token.
function SilentWaiting.start()
    -- Nothing to set up.
    return nil
end

-- tick(state) — advance the animation by one frame.
--
-- Parameters:
--   state — the opaque state returned by start() or the previous tick().
--
-- Returns:
--   state — the (unchanged) state token, for the caller to pass to next tick.
--
-- Note: In a real spinner this would update a frame counter and re-draw.
-- SilentWaiting ignores both the input and produces no output.
function SilentWaiting.tick(state)
    -- Nothing to advance.
    return state
end

-- tick_ms() — how long to sleep between ticks, in milliseconds.
--
-- Returns:
--   integer — 100 ms is a conventional default (10 ticks per second), which
--   is fast enough to look smooth without hammering the CPU.
function SilentWaiting.tick_ms()
    return 100
end

-- stop(state) — end the waiting animation.
--
-- Parameters:
--   state — the opaque state, in case cleanup needs it.
--
-- Returns:
--   nil
function SilentWaiting.stop(_state)
    -- Nothing to tear down.
    return nil
end

return SilentWaiting
