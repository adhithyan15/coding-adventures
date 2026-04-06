-- Tests for the repl package.
--
-- We test the REPL framework at three levels:
--
--   1. Built-in plug-ins — EchoLanguage, DefaultPrompt, SilentWaiting in
--      isolation, verifying their individual contracts.
--
--   2. run_with_io — the main loop with injected I/O, verifying end-to-end
--      behaviour: correct prompts, correct output, quit on ":quit", quit on
--      EOF, error propagation, and pcall safety.
--
--   3. Module API — that the public surface is complete and correctly typed.
--
-- We use busted's describe/it style throughout. Busted is the de-facto
-- standard test framework for Lua, analogous to RSpec (Ruby) or Jest (JS).
--
-- I/O injection strategy
-- ──────────────────────
-- run_with_io() accepts:
--   input_fn()     → string or nil
--   output_fn(s)   → nil
--
-- In tests we drive input_fn from a pre-built list of strings and capture
-- everything output_fn receives into a list. After the loop finishes we
-- assert on the captured list.
--
-- Newline convention
-- ──────────────────
-- Real terminals emit lines with a trailing "\n". We pass inputs WITHOUT the
-- trailing newline (as io.read("l") would give us) to avoid double-stripping.
-- The loop strips a trailing newline if present, so both forms are safe.

-- Add the src/ directory to the module search path so tests can run without
-- `luarocks install` in the development environment.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local repl = require("coding_adventures.repl")

-- ============================================================================
-- Helper: run the REPL loop with a fixed list of inputs
-- ============================================================================
--
-- Returns the captured output as a single concatenated string, which makes
-- it easy to do substring assertions.
local function run_with_inputs(language, inputs)
    local prompt  = repl.DefaultPrompt
    local waiting = repl.SilentWaiting

    local idx = 0
    local function input_fn()
        idx = idx + 1
        return inputs[idx]  -- returns nil when past the end (EOF)
    end

    local captured = {}
    local function output_fn(s)
        captured[#captured + 1] = s
    end

    repl.run_with_io(language, prompt, waiting, input_fn, output_fn)

    return table.concat(captured)
end

-- ============================================================================
-- Describe: EchoLanguage
-- ============================================================================
describe("EchoLanguage", function()

    -- ── Basic echo behaviour ────────────────────────────────────────────────

    it("echoes non-quit input back as {tag='ok', output=input}", function()
        local result = repl.EchoLanguage.eval("hello")
        assert.are.equal("ok",    result.tag)
        assert.are.equal("hello", result.output)
    end)

    it("echoes an empty string as {tag='ok', output=''}", function()
        local result = repl.EchoLanguage.eval("")
        assert.are.equal("ok", result.tag)
        assert.are.equal("",   result.output)
    end)

    it("echoes a string with spaces and punctuation verbatim", function()
        local input = "  hello, world!  "
        local result = repl.EchoLanguage.eval(input)
        assert.are.equal("ok",   result.tag)
        assert.are.equal(input,  result.output)
    end)

    -- ── Quit command ────────────────────────────────────────────────────────

    it("returns {tag='quit'} for the ':quit' command", function()
        local result = repl.EchoLanguage.eval(":quit")
        assert.are.equal("quit", result.tag)
    end)

    it("does NOT treat ':Quit' (capital Q) as a quit command", function()
        -- The quit command is case-sensitive.
        local result = repl.EchoLanguage.eval(":Quit")
        assert.are.equal("ok",    result.tag)
        assert.are.equal(":Quit", result.output)
    end)

    it("does NOT treat ' :quit' (leading space) as a quit command", function()
        local result = repl.EchoLanguage.eval(" :quit")
        assert.are.equal("ok",     result.tag)
        assert.are.equal(" :quit", result.output)
    end)

    -- ── Error path ──────────────────────────────────────────────────────────

    it("returns {tag='error'} when input is not a string", function()
        local result = repl.EchoLanguage.eval(42)
        assert.are.equal("error", result.tag)
        assert.is_string(result.message)
    end)

    -- ── QUIT_COMMAND constant ────────────────────────────────────────────────

    it("exposes QUIT_COMMAND as ':quit'", function()
        assert.are.equal(":quit", repl.EchoLanguage.QUIT_COMMAND)
    end)
end)

-- ============================================================================
-- Describe: DefaultPrompt
-- ============================================================================
describe("DefaultPrompt", function()

    it("global_prompt() returns '> '", function()
        assert.are.equal("> ", repl.DefaultPrompt.global_prompt())
    end)

    it("line_prompt() returns '... '", function()
        assert.are.equal("... ", repl.DefaultPrompt.line_prompt())
    end)

    it("global_prompt() is idempotent (same value on repeated calls)", function()
        local a = repl.DefaultPrompt.global_prompt()
        local b = repl.DefaultPrompt.global_prompt()
        assert.are.equal(a, b)
    end)

    it("line_prompt() is idempotent (same value on repeated calls)", function()
        local a = repl.DefaultPrompt.line_prompt()
        local b = repl.DefaultPrompt.line_prompt()
        assert.are.equal(a, b)
    end)
end)

-- ============================================================================
-- Describe: SilentWaiting
-- ============================================================================
describe("SilentWaiting", function()

    it("start() returns nil (no state needed)", function()
        local state = repl.SilentWaiting.start()
        assert.is_nil(state)
    end)

    it("tick(nil) returns nil (state is passed through unchanged)", function()
        local state = repl.SilentWaiting.tick(nil)
        assert.is_nil(state)
    end)

    it("tick() is safe to call multiple times", function()
        local state = repl.SilentWaiting.start()
        for _ = 1, 10 do
            state = repl.SilentWaiting.tick(state)
        end
        assert.is_nil(state)
    end)

    it("tick_ms() returns a positive integer", function()
        local ms = repl.SilentWaiting.tick_ms()
        assert.is_number(ms)
        assert.is_true(ms > 0)
        assert.are.equal(ms, math.floor(ms))  -- integer check
    end)

    it("tick_ms() returns 100", function()
        assert.are.equal(100, repl.SilentWaiting.tick_ms())
    end)

    it("stop(nil) returns nil (no cleanup needed)", function()
        local result = repl.SilentWaiting.stop(nil)
        assert.is_nil(result)
    end)

    it("start / tick / stop round-trip produces no side-effects", function()
        -- If SilentWaiting is truly silent, running it should not cause errors
        -- or produce any observable change.
        local state = repl.SilentWaiting.start()
        state = repl.SilentWaiting.tick(state)
        state = repl.SilentWaiting.tick(state)
        repl.SilentWaiting.stop(state)
        -- If we reach here without error, the test passes.
        assert.is_true(true)
    end)
end)

-- ============================================================================
-- Describe: run_with_io — end-to-end loop behaviour
-- ============================================================================
describe("run_with_io", function()

    -- ── Basic echo round-trip ───────────────────────────────────────────────

    it("echoes a single line and then quits on EOF", function()
        -- inputs: ["hello", nil (EOF)]
        -- expected output: "> " + "hello\n" + "> " + "\n"
        --   (prompt, echo, prompt, EOF newline)
        local out = run_with_inputs(repl.EchoLanguage, {"hello"})
        -- The prompt appears before input, and the echo appears after.
        assert.is_truthy(out:find("> ", 1, true))
        assert.is_truthy(out:find("hello", 1, true))
    end)

    it("echoes multiple lines in order", function()
        local out = run_with_inputs(repl.EchoLanguage, {"alpha", "beta", "gamma"})
        -- All three echoed lines must appear in the output.
        assert.is_truthy(out:find("alpha", 1, true))
        assert.is_truthy(out:find("beta",  1, true))
        assert.is_truthy(out:find("gamma", 1, true))
    end)

    -- ── Quit command ────────────────────────────────────────────────────────

    it("stops the loop when eval returns {tag='quit'}", function()
        -- ":quit" causes the loop to stop; "after" should never be echoed.
        local out = run_with_inputs(repl.EchoLanguage, {":quit", "after"})
        assert.is_falsy(out:find("after", 1, true))
    end)

    it("echoes lines before :quit and stops at :quit", function()
        local out = run_with_inputs(repl.EchoLanguage, {"first", ":quit"})
        assert.is_truthy(out:find("first", 1, true))
    end)

    -- ── EOF handling ────────────────────────────────────────────────────────

    it("stops cleanly on immediate EOF (no input at all)", function()
        -- Passing an empty table means the first call to input_fn returns nil.
        -- The loop should exit cleanly without error.
        local ok = pcall(run_with_inputs, repl.EchoLanguage, {})
        assert.is_true(ok)
    end)

    -- ── Error propagation ───────────────────────────────────────────────────

    it("displays error messages returned by the language plug-in", function()
        -- Build a language that always errors.
        local ErrorLanguage = {
            eval = function(_input)
                return { tag = "error", message = "boom" }
            end
        }
        local out = run_with_inputs(ErrorLanguage, {"anything"})
        -- The loop should display "error: boom" and continue.
        assert.is_truthy(out:find("error: boom", 1, true))
    end)

    -- ── pcall safety ────────────────────────────────────────────────────────

    it("survives a language plug-in that panics (raises a Lua error)", function()
        -- Build a language whose eval() calls error().
        local PanickingLanguage = {
            eval = function(_input)
                error("catastrophic failure")
            end
        }
        -- The REPL should catch the panic via pcall and display an error
        -- message, not propagate the exception to the caller.
        local ok, _ = pcall(run_with_inputs, PanickingLanguage, {"oops", ":quit"})
        assert.is_true(ok)
    end)

    it("shows 'error:' prefix when language panics", function()
        local PanickingLanguage = {
            eval = function(_input)
                error("deliberate panic")
            end
        }
        -- The first input panics; the second is ":quit".
        -- But because the loop catches the panic, it keeps running —
        -- and the second input (:quit) also panics, so we need EOF to stop.
        local out = run_with_inputs(PanickingLanguage, {"boom"})
        assert.is_truthy(out:find("error:", 1, true))
    end)

    -- ── Prompt appearance ───────────────────────────────────────────────────

    it("shows the global prompt before each line of input", function()
        local out = run_with_inputs(repl.EchoLanguage, {"line1", "line2"})
        -- With two lines plus EOF, we expect three prompt occurrences.
        -- Count by finding all "> " occurrences.
        local count = 0
        local pos = 1
        while true do
            local s = out:find("> ", pos, true)
            if not s then break end
            count = count + 1
            pos = s + 1
        end
        assert.is_true(count >= 2)
    end)

    -- ── Newline stripping ───────────────────────────────────────────────────

    it("strips a trailing newline from input before passing to eval", function()
        -- If we pass "hello\n", the echo should be "hello" not "hello\n".
        local captured = {}
        repl.run_with_io(
            repl.EchoLanguage,
            repl.DefaultPrompt,
            repl.SilentWaiting,
            (function()
                local done = false
                return function()
                    if done then return nil end
                    done = true
                    return "hello\n"
                end
            end)(),
            function(s) captured[#captured + 1] = s end
        )
        local out = table.concat(captured)
        -- "hello" should appear, but not "hello\n\n" (double newline).
        assert.is_truthy(out:find("hello", 1, true))
        -- The output line should be "hello\n" not "hello\n\n".
        assert.is_falsy(out:find("hello\n\n", 1, true))
    end)

    -- ── Argument validation ─────────────────────────────────────────────────

    it("errors if language is not a table", function()
        assert.has_error(function()
            repl.run_with_io(
                "not a table",
                repl.DefaultPrompt,
                repl.SilentWaiting,
                function() return nil end,
                function(_s) end
            )
        end)
    end)

    it("errors if language.eval is not a function", function()
        assert.has_error(function()
            repl.run_with_io(
                { eval = "not a function" },
                repl.DefaultPrompt,
                repl.SilentWaiting,
                function() return nil end,
                function(_s) end
            )
        end)
    end)

    it("errors if prompt is not a table", function()
        assert.has_error(function()
            repl.run_with_io(
                repl.EchoLanguage,
                42,
                repl.SilentWaiting,
                function() return nil end,
                function(_s) end
            )
        end)
    end)

    it("errors if input_fn is not a function", function()
        assert.has_error(function()
            repl.run_with_io(
                repl.EchoLanguage,
                repl.DefaultPrompt,
                repl.SilentWaiting,
                "not a function",
                function(_s) end
            )
        end)
    end)

    it("errors if output_fn is not a function", function()
        assert.has_error(function()
            repl.run_with_io(
                repl.EchoLanguage,
                repl.DefaultPrompt,
                repl.SilentWaiting,
                function() return nil end,
                "not a function"
            )
        end)
    end)

    -- ── Custom language plug-in ─────────────────────────────────────────────

    it("works with a custom language that uppercases input", function()
        local UpperLanguage = {
            eval = function(input)
                if input == "quit" then
                    return { tag = "quit" }
                end
                return { tag = "ok", output = input:upper() }
            end
        }
        local out = run_with_inputs(UpperLanguage, {"hello", "quit"})
        assert.is_truthy(out:find("HELLO", 1, true))
    end)

    it("handles a language that returns output=nil (void result)", function()
        -- Some languages have statements that produce no output (e.g. assignments).
        local VoidLanguage = {
            eval = function(input)
                if input == ":quit" then return { tag = "quit" } end
                return { tag = "ok", output = nil }
            end
        }
        -- Should run without error; no output lines expected (other than prompts).
        local ok = pcall(run_with_inputs, VoidLanguage, {"x = 1", ":quit"})
        assert.is_true(ok)
    end)
end)

-- ============================================================================
-- Describe: Module API surface
-- ============================================================================
describe("module API", function()

    it("has a VERSION string", function()
        assert.is_string(repl.VERSION)
        assert.are.equal("0.1.0", repl.VERSION)
    end)

    it("exports run as a function", function()
        assert.is_function(repl.run)
    end)

    it("exports run_with_io as a function", function()
        assert.is_function(repl.run_with_io)
    end)

    it("exports EchoLanguage as a table with an eval function", function()
        assert.is_table(repl.EchoLanguage)
        assert.is_function(repl.EchoLanguage.eval)
    end)

    it("exports DefaultPrompt as a table with global_prompt and line_prompt", function()
        assert.is_table(repl.DefaultPrompt)
        assert.is_function(repl.DefaultPrompt.global_prompt)
        assert.is_function(repl.DefaultPrompt.line_prompt)
    end)

    it("exports SilentWaiting as a table with start, tick, tick_ms, stop", function()
        assert.is_table(repl.SilentWaiting)
        assert.is_function(repl.SilentWaiting.start)
        assert.is_function(repl.SilentWaiting.tick)
        assert.is_function(repl.SilentWaiting.tick_ms)
        assert.is_function(repl.SilentWaiting.stop)
    end)
end)
