-- Tests for the Parrot REPL program.
--
-- We load main.lua as a module (NOT executing it as a script) and exercise:
--
--   1. The ParrotPrompt object directly — verifying strings.
--   2. The full REPL loop with injected I/O — verifying end-to-end behaviour.
--
-- # How tests avoid triggering main()
--
-- main.lua guards its entry point with:
--
--   if arg and arg[0] and arg[0]:match("main%.lua$") then main() end
--
-- When busted runs, arg[0] is the busted executable path, not "main.lua", so
-- the guard never fires and require("main") is safe.
--
-- # Module path setup
--
-- The busted runner is invoked from the tests/ directory (cd tests && busted).
-- We must add two directories to package.path:
--
--   1. The parent directory (..) so that require("main") finds main.lua.
--   2. The repl package's src/ directory so that main.lua can require the
--      REPL framework modules.
--
-- We compute paths relative to ".." (the parrot/ directory) using the
-- platform separator obtained from package.config.

local sep = package.config:sub(1, 1)  -- "/" on Unix, "\" on Windows

-- Add the parrot/ directory (parent of tests/) so require("main") works.
package.path = ".." .. sep .. "?.lua;"
            .. ".." .. sep .. "?" .. sep .. "init.lua;"
            .. package.path

-- Add the repl package src/ directory so require("coding_adventures.repl.*") works.
-- Path relative to tests/ is: ../../../../packages/lua/repl/src/
local repl_src = ".." .. sep .. ".." .. sep .. ".." .. sep .. ".."
              .. sep .. "packages" .. sep .. "lua" .. sep .. "repl" .. sep .. "src"

package.path = repl_src .. sep .. "?.lua;"
            .. repl_src .. sep .. "?" .. sep .. "init.lua;"
            .. package.path

-- Load the parrot program as a module. This also loads the repl framework
-- internally. The returned table gives us access to ParrotPrompt, Loop, etc.
local Parrot = require("main")
local Loop   = Parrot.Loop
local Echo   = Parrot.Echo
local Silent = Parrot.Silent

-- ============================================================================
-- Helper: run_parrot
--
-- Runs the Parrot REPL loop with a controlled list of input strings, collects
-- all output into a table, and returns it.
--
-- Parameters:
--   ...  — variadic strings; each becomes one line of input.
--           Pass nil explicitly to simulate EOF before a :quit.
--           The list ends naturally when all values are consumed (next call
--           returns nil = EOF).
--
-- Returns:
--   table of strings — every string passed to output_fn, in order.
--
-- Design: we use a queue backed by a numeric index. Advancing past the end
-- returns nil, which the loop treats as EOF. This mirrors how real programs
-- handle piped input running out of data.
-- ============================================================================

local function run_parrot(...)
    local inputs = { ... }
    local output = {}
    local idx    = 1

    Loop.run_with_io(
        Echo,
        Parrot.ParrotPrompt.new(),
        Silent,
        function()
            local val = inputs[idx]
            idx = idx + 1
            return val  -- nil when past the end of the table
        end,
        function(text)
            table.insert(output, text)
        end,
        { mode = "sync" }
    )

    return output
end

-- ============================================================================
-- Helper: joined_output
--
-- Joins all output strings into a single string, making substring searches
-- easy without needing to know which chunk a particular text appeared in.
-- ============================================================================

local function joined_output(out)
    return table.concat(out, "")
end

-- ============================================================================
-- ParrotPrompt unit tests
--
-- Test the prompt object in isolation, without running the full loop.
-- ============================================================================

describe("ParrotPrompt", function()

    local prompt

    before_each(function()
        prompt = Parrot.ParrotPrompt.new()
    end)

    -- Test 1: global_prompt returns a string
    it("global_prompt returns a string", function()
        local gp = prompt:global_prompt()
        assert.is_string(gp)
    end)

    -- Test 2: global_prompt contains the word "Parrot"
    it("global_prompt mentions Parrot", function()
        local gp = prompt:global_prompt()
        assert.truthy(gp:find("Parrot"),
            "expected 'Parrot' in global_prompt, got: " .. gp)
    end)

    -- Test 3: global_prompt contains quit instruction
    it("global_prompt mentions :quit", function()
        local gp = prompt:global_prompt()
        assert.truthy(gp:find(":quit"),
            "expected ':quit' in global_prompt, got: " .. gp)
    end)

    -- Test 4: line_prompt returns a string
    it("line_prompt returns a string", function()
        local lp = prompt:line_prompt()
        assert.is_string(lp)
    end)

    -- Test 5: line_prompt is non-empty
    it("line_prompt is non-empty", function()
        local lp = prompt:line_prompt()
        assert.truthy(#lp > 0, "line_prompt should not be empty")
    end)

    -- Test 6: global_prompt and line_prompt are different strings
    it("global_prompt and line_prompt differ", function()
        local gp = prompt:global_prompt()
        local lp = prompt:line_prompt()
        assert.not_equal(gp, lp)
    end)

    -- Test 7: new() returns distinct instances
    it("new() creates independent instances", function()
        local p1 = Parrot.ParrotPrompt.new()
        local p2 = Parrot.ParrotPrompt.new()
        -- Both should behave the same (same strings) but be different tables.
        assert.not_equal(p1, p2)
        assert.equal(p1:global_prompt(), p2:global_prompt())
    end)

end)

-- ============================================================================
-- REPL loop integration tests
--
-- These tests exercise the full loop by injecting inputs and capturing output.
-- ============================================================================

describe("Parrot REPL loop", function()

    -- Test 8: echoes basic input
    it("echoes basic input back", function()
        local out = run_parrot("hello", ":quit")
        local full = joined_output(out)
        -- The loop should have printed "hello\n" as the eval result.
        assert.truthy(full:find("hello\n"),
            "expected 'hello' in output, got: " .. full)
    end)

    -- Test 9: :quit ends the session without echoing ":quit"
    it(":quit ends the session", function()
        local out = run_parrot(":quit")
        local full = joined_output(out)
        -- ":quit" is the quit sentinel; it should NOT appear as echoed output.
        assert.falsy(full:find(":quit\n"),
            "':quit' should not be echoed as output")
    end)

    -- Test 10: EOF (no more input) exits gracefully
    it("exits gracefully on EOF", function()
        -- Provide one input then let the queue run dry (EOF).
        -- The loop should exit without error.
        assert.has_no.errors(function()
            run_parrot("test input")
        end)
    end)

    -- Test 11: multiple inputs are all echoed
    it("echoes multiple inputs in order", function()
        local out = run_parrot("alpha", "beta", "gamma", ":quit")
        local full = joined_output(out)
        assert.truthy(full:find("alpha\n"), "expected 'alpha' in output")
        assert.truthy(full:find("beta\n"),  "expected 'beta' in output")
        assert.truthy(full:find("gamma\n"), "expected 'gamma' in output")
    end)

    -- Test 12: empty string is echoed back
    it("echoes empty string", function()
        local out = run_parrot("", ":quit")
        -- EchoLanguage returns { tag = "ok", output = "" } for empty input.
        -- The loop prints output .. "\n" only if output ~= nil.
        -- An empty string is not nil, so "\n" should appear in output.
        local found_newline = false
        for _, v in ipairs(out) do
            if v == "\n" then found_newline = true end
        end
        assert.is_true(found_newline,
            "empty input should produce a newline in output")
    end)

    -- Test 13: output contains the prompt text on each iteration
    it("prints the prompt before each input", function()
        -- The loop calls prompt.global_prompt() on EVERY iteration, so we
        -- should see prompt text in the output for each line we feed.
        local out = run_parrot("hello", ":quit")
        local full = joined_output(out)
        -- "Parrot" appears in the prompt; it should appear at least once.
        assert.truthy(full:find("Parrot"),
            "expected prompt text in output, got: " .. full)
    end)

    -- Test 14: session ends after :quit even with more input available
    it("stops after :quit even if more input is queued", function()
        -- Feed :quit first, then more lines. Those lines should NOT be echoed.
        local out = run_parrot(":quit", "should-not-appear", "also-not-this")
        local full = joined_output(out)
        assert.falsy(full:find("should%-not%-appear"),
            "input after :quit should not be processed")
    end)

    -- Test 15: pure EOF (no :quit) also terminates cleanly
    it("terminates cleanly on pure EOF without :quit", function()
        local out = run_parrot("line one", "line two")
        -- Both lines should have been echoed (no :quit means we hit EOF).
        local full = joined_output(out)
        assert.truthy(full:find("line one\n"))
        assert.truthy(full:find("line two\n"))
    end)

    -- Test 16: output does not contain the raw :quit command echoed back
    it("does not echo :quit as output text", function()
        local out = run_parrot("hello", ":quit", "world")
        local full = joined_output(out)
        -- The echo of the word ":quit" should never appear.
        assert.falsy(full:find(":quit\n"),
            "':quit' should not appear as echoed output")
    end)

    -- Test 17: sync mode is accepted without error
    it("sync mode runs without error", function()
        assert.has_no.errors(function()
            run_parrot(":quit")
        end)
    end)

    -- Test 18: async mode raises an error
    it("async mode raises an error", function()
        assert.has_error(function()
            Loop.run_with_io(
                Echo,
                Parrot.ParrotPrompt.new(),
                Silent,
                function() return nil end,
                function(_) end,
                { mode = "async" }
            )
        end)
    end)

    -- Test 19: special characters are echoed verbatim
    it("echoes strings with special characters", function()
        local special = "hello & world | pipe < > redirect"
        local out = run_parrot(special, ":quit")
        local full = joined_output(out)
        -- Use a plain string find (no pattern interpretation) by escaping.
        assert.truthy(full:find("hello & world", 1, true),
            "special characters should be echoed unchanged")
    end)

    -- Test 20: run_parrot returns a table (output collector works)
    it("run_parrot returns a table of output strings", function()
        local out = run_parrot(":quit")
        assert.is_table(out)
    end)

end)
