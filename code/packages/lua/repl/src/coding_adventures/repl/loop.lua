-- loop — The core Read-Eval-Print Loop engine
--
-- # What Is a REPL?
--
-- A REPL (Read-Eval-Print Loop) is the interactive shell at the heart of most
-- interpreted languages. It has four steps, repeated forever:
--
--   1. READ   — display a prompt, then read a line of input from the user
--   2. EVAL   — pass that input to the language evaluator
--   3. PRINT  — display the result (or error) to the user
--   4. LOOP   — go back to step 1
--
-- The loop terminates when:
--   - The evaluator returns { tag = "quit" }
--   - The input source signals EOF (input_fn returns nil)
--
-- # Plug-in Architecture
--
-- Rather than hard-coding I/O and language behaviour, this loop accepts three
-- plug-in objects and two I/O functions:
--
--   language  — defines what "evaluate" means (see echo_language.lua)
--   prompt    — provides the prompt strings (see default_prompt.lua)
--   waiting   — animates while eval runs (see silent_waiting.lua)
--   input_fn  — returns the next line of input, or nil for EOF
--   output_fn — writes a string to the user's screen
--
-- Separating these concerns makes the loop testable (inject fake I/O, inject
-- an echo language) and reusable (swap in readline, swap in Python, etc.).
--
-- # The Waiting Protocol in Lua
--
-- Ideally, waiting.tick() would be called repeatedly while eval runs,
-- animating a spinner. However, Lua's standard coroutines are cooperative:
-- once language.eval() is called, no other code runs until it returns.
--
-- We are honest about this limitation:
--
--   waiting.start()   ← called before eval
--   language.eval()   ← runs to completion (no ticks possible)
--   waiting.stop()    ← called after eval
--
-- This matches the documented contract exactly. A host with true threads could
-- call tick() from a background thread; the interface supports that without
-- any changes here.
--
-- # Exception Safety
--
-- We wrap the eval call in pcall(). If the language plug-in panics (raises a
-- Lua error), we catch it and return { tag = "error", message = ... } rather
-- than crashing the REPL. This mirrors how real shells survive bad evaluators.

-- ============================================================================
-- Internal helper: safely call language.eval
-- ============================================================================
--
-- pcall(f, ...) returns:
--   true,  result   — if f(...) returned normally
--   false, errmsg   — if f(...) raised an error (via error() or assert())
--
-- We convert the pcall result into our tagged union so the loop doesn't need
-- to know about pcall at all.
local function safe_eval(language, input)
    local ok, result = pcall(language.eval, input)
    if not ok then
        -- result is the error message string (or object) from the panic.
        -- Convert to string in case it's a table or other type.
        return { tag = "error", message = tostring(result) }
    end
    -- Validate that the language returned a proper result table.
    if type(result) ~= "table" or type(result.tag) ~= "string" then
        return {
            tag = "error",
            message = "language.eval returned an invalid result (expected table with tag)"
        }
    end
    return result
end

-- ============================================================================
-- run_with_io — the main REPL loop with injected I/O
-- ============================================================================
--
-- This is the primary function. It accepts explicit I/O functions, making it
-- fully testable without a terminal.
--
-- Parameters:
--   language   (table)    — Language plug-in; must have an .eval(input) method
--   prompt     (table)    — Prompt plug-in; must have .global_prompt() and
--                           .line_prompt() methods
--   waiting    (table|nil)— Waiting plug-in; must have .start(), .tick(s),
--                           .tick_ms(), and .stop(s) methods. May be nil in
--                           sync mode (a silent no-op is used internally).
--   input_fn   (function) — Called with no args; returns a string (next line)
--                           or nil (EOF / end of input)
--   output_fn  (function) — Called with a string; writes it to the user.
--                           Should NOT add a trailing newline (we add \n where
--                           needed so the caller controls line endings).
--   opts       (table|nil)— Optional options table.
--                           opts.mode = "sync"  (default) — synchronous eval
--                           opts.mode = "async" — raises an error immediately,
--                             because Lua's stdlib has no native threads and
--                             cannot support true async evaluation.
--
-- Returns: nil
--
-- The loop runs until:
--   - input_fn() returns nil  (EOF — e.g., user pressed Ctrl-D)
--   - eval returns { tag = "quit" }
--
-- # Mode Support
--
-- Lua's standard coroutines are cooperative — once language.eval() is called,
-- no other code runs until it returns. There is therefore no mechanism to
-- evaluate expressions asynchronously in standard Lua without external
-- libraries (e.g., lua-ev, Luvit, or LuaJIT with OS threads).
--
-- We make this constraint explicit via the mode option:
--
--   opts = { mode = "sync" }    -- the only supported mode; this is the default
--   opts = { mode = "async" }   -- raises an error immediately at startup
--
-- The "async" path exists so that code written to be multi-runtime-aware
-- (e.g., shared glue code calling both a Lua REPL and a Python REPL that does
-- support async) gets a clear, actionable error rather than silently falling
-- back to broken behaviour.
local function run_with_io(language, prompt, waiting, input_fn, output_fn, opts)
    -- ── Mode check ────────────────────────────────────────────────────────
    --
    -- Resolve mode from opts (default: "sync"). Reject "async" immediately
    -- because standard Lua cannot support non-blocking eval — raising here
    -- gives the caller a clear error rather than subtly wrong behaviour.
    local mode = "sync"
    if opts ~= nil then
        assert(type(opts) == "table", "opts must be a table or nil")
        if opts.mode ~= nil then
            mode = opts.mode
        end
    end

    if mode == "async" then
        error(
            "async mode is not supported in the Lua REPL implementation.\n" ..
            "Use mode = 'sync' instead."
        )
    end

    assert(mode == "sync", "opts.mode must be 'sync' or 'async' (got: " ..
           tostring(mode) .. ")")

    -- ── Validate plug-ins at entry to give clear error messages ───────────
    assert(type(language) == "table",  "language must be a table")
    assert(type(language.eval) == "function", "language.eval must be a function")
    assert(type(prompt) == "table",    "prompt must be a table")
    assert(type(prompt.global_prompt) == "function",
           "prompt.global_prompt must be a function")
    assert(type(prompt.line_prompt) == "function",
           "prompt.line_prompt must be a function")
    -- waiting is optional in sync mode. When nil we supply a minimal no-op
    -- shim so the rest of the loop code can call waiting.start() / .stop()
    -- unconditionally without defensive checks scattered everywhere.
    if waiting == nil then
        waiting = {
            start   = function() return nil end,
            tick    = function(s) return s   end,
            tick_ms = function() return 100  end,
            stop    = function(_s) end,
        }
    else
        assert(type(waiting) == "table",   "waiting must be a table")
        assert(type(waiting.start) == "function",  "waiting.start must be a function")
        assert(type(waiting.tick) == "function",   "waiting.tick must be a function")
        assert(type(waiting.tick_ms) == "function","waiting.tick_ms must be a function")
        assert(type(waiting.stop) == "function",   "waiting.stop must be a function")
    end
    assert(type(input_fn) == "function",  "input_fn must be a function")
    assert(type(output_fn) == "function", "output_fn must be a function")

    -- The main loop. We use a flag rather than break-from-middle so that the
    -- control flow is easy to follow and easy to test.
    local running = true

    while running do
        -- ── STEP 1: READ ──────────────────────────────────────────────────
        --
        -- Show the global prompt, then read a line.
        --
        -- We do NOT add a newline after the prompt string because the user's
        -- input will appear on the same line (the terminal's echo mechanism
        -- handles cursor positioning).
        output_fn(prompt.global_prompt())

        local input = input_fn()

        -- nil means EOF (Ctrl-D on Unix, Ctrl-Z on Windows, or end of pipe).
        -- Exit the loop cleanly.
        if input == nil then
            -- Print a newline so the next shell prompt appears on a fresh line.
            output_fn("\n")
            running = false

        else
            -- Strip trailing newline if present. input_fn implementations vary:
            -- some strip it, some don't. We normalise here.
            --
            -- The pattern "%\n$" matches a literal newline at the end of the
            -- string. gsub returns the modified string plus a count; we only
            -- want the string.
            input = input:gsub("\n$", "")

            -- ── STEP 2: EVAL ──────────────────────────────────────────────
            --
            -- Notify the waiting plug-in that we are about to evaluate.
            -- In standard Lua this is purely informational — no ticks will
            -- fire because eval is synchronous. A true-threaded host could
            -- arrange ticks here.
            local wait_state = waiting.start()

            -- Evaluate the input through the language plug-in, with pcall
            -- protection against panics.
            local result = safe_eval(language, input)

            -- Eval has finished. Let the waiting plug-in clean up (erase
            -- spinner, print a newline, etc.).
            waiting.stop(wait_state)

            -- ── STEP 3: PRINT ─────────────────────────────────────────────
            --
            -- Dispatch on the result tag. The three legal tags are:
            --   "ok"    — success, output may be nil (void result)
            --   "error" — evaluation failed, show message
            --   "quit"  — exit the loop

            if result.tag == "ok" then
                -- Only print something if the evaluator produced output.
                -- A void expression (e.g., a statement) returns output=nil.
                if result.output ~= nil then
                    output_fn(tostring(result.output) .. "\n")
                end

            elseif result.tag == "error" then
                -- Show the error message. We prefix with "error: " to make it
                -- visually distinct from normal output.
                output_fn("error: " .. tostring(result.message) .. "\n")

            elseif result.tag == "quit" then
                -- The language (or the user via ":quit") has signalled exit.
                running = false

            else
                -- Unknown tag — the language plug-in is misbehaving. Report
                -- it as an error and keep running (best-effort recovery).
                output_fn("error: unknown result tag: " ..
                          tostring(result.tag) .. "\n")
            end

            -- ── STEP 4: LOOP ──────────────────────────────────────────────
            --
            -- Control returns to the top of the while loop automatically.
        end
    end
end

-- ============================================================================
-- run — convenience wrapper using stdio
-- ============================================================================
--
-- Most callers just want a REPL on the terminal. This wrapper provides the
-- standard I/O functions so callers don't have to.
--
-- Parameters:
--   language (table)    — Language plug-in (required)
--   prompt   (table)    — Prompt plug-in (optional, defaults to DefaultPrompt)
--   waiting  (table)    — Waiting plug-in (optional, defaults to SilentWaiting)
--   opts     (table|nil)— Options table forwarded to run_with_io.
--                         opts.mode = "sync" (default) or "async" (errors).
--
-- The defaults are loaded lazily here rather than at the top of the module.
-- This avoids circular-require issues and keeps each module self-contained.
local function run(language, prompt, waiting, opts)
    -- Load defaults only when needed.
    local default_prompt  = prompt   or require("coding_adventures.repl.default_prompt")
    local default_waiting = waiting  or require("coding_adventures.repl.silent_waiting")

    -- Standard input: io.read("l") reads one line without the newline.
    -- Returns nil at EOF.
    local function stdio_input()
        return io.read("l")
    end

    -- Standard output: io.write writes without an implicit newline.
    local function stdio_output(s)
        io.write(s)
        -- Flush immediately so the prompt appears before the user types.
        io.flush()
    end

    run_with_io(language, default_prompt, default_waiting,
                stdio_input, stdio_output, opts)
end

-- ============================================================================
-- Public API
-- ============================================================================

return {
    run         = run,
    run_with_io = run_with_io,
}
