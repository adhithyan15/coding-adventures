-- repl — A pluggable Read-Eval-Print Loop framework for Lua
--
-- # Overview
--
-- This package provides a complete, extensible REPL (Read-Eval-Print Loop)
-- framework. A REPL is the interactive shell found in every interpreter:
-- Lua's `lua -i`, Python's `python3`, Ruby's `irb`, Node's `node`. You type
-- an expression, the shell evaluates it, prints the result, and waits for
-- more.
--
-- The framework's design philosophy is *dependency injection*: every
-- behavioural concern is a plug-in, passed in at runtime. Nothing is
-- hard-coded. This makes the framework:
--
--   - Testable: inject fake I/O to test the loop without a terminal
--   - Reusable: drop in any language evaluator
--   - Observable: swap in a spinner or progress bar without touching loop code
--
-- # The Three Plug-in Interfaces
--
-- Lua has no formal interface system (no abstract classes, no traits). We
-- document our interfaces as conventions — a table with specific fields and
-- method signatures. The REPL loop validates these at startup.
--
-- ## Language
--
--   A table with one method:
--
--     language.eval(input) → result
--
--   Where result is one of:
--     { tag = "ok",    output = string_or_nil }   — success
--     { tag = "error", message = string        }   — failure
--     { tag = "quit"                           }   — exit the REPL
--
--   The tagged-union pattern is idiomatic Lua for discriminated results.
--
-- ## Prompt
--
--   A table with two methods:
--
--     prompt.global_prompt() → string   -- shown before each new expression
--     prompt.line_prompt()   → string   -- shown on continuation lines
--
-- ## Waiting
--
--   A table with four methods:
--
--     waiting.start()       → state     -- called before eval
--     waiting.tick(state)   → state     -- called each animation frame
--     waiting.tick_ms()     → integer   -- ms to sleep between ticks
--     waiting.stop(state)   → nil       -- called after eval
--
--   Important: In standard Lua, language.eval() is synchronous — once called,
--   no other code runs until it returns. Therefore tick() is never called
--   between start() and stop() in a single-threaded Lua host. The interface
--   is designed to work correctly in both threaded and non-threaded hosts.
--
-- # I/O Injection
--
--   run_with_io() accepts two additional parameters:
--
--     input_fn()     → string or nil   -- read next line; nil = EOF
--     output_fn(s)   → nil             -- write string s to the user
--
--   run() uses io.read / io.write for convenience.
--
-- # Package Layout
--
--   repl/
--     init.lua           — This file: public API, re-exports everything
--     loop.lua           — The run / run_with_io engine
--     echo_language.lua  — Built-in Language: echoes input back
--     default_prompt.lua — Built-in Prompt: "> " and "... "
--     silent_waiting.lua — Built-in Waiting: all no-ops
--
-- # Quick Start
--
--   local repl = require("coding_adventures.repl")
--
--   -- Use defaults: EchoLanguage + DefaultPrompt + SilentWaiting + stdio
--   repl.run(repl.EchoLanguage)
--
--   -- Inject custom I/O for testing
--   local inputs = {"hello", "world", ":quit"}
--   local i = 0
--   local outputs = {}
--   repl.run_with_io(
--       repl.EchoLanguage,
--       repl.DefaultPrompt,
--       repl.SilentWaiting,
--       function() i = i + 1; return inputs[i] end,
--       function(s) outputs[#outputs + 1] = s end
--   )

local loop_mod    = require("coding_adventures.repl.loop")
local echo_mod    = require("coding_adventures.repl.echo_language")
local prompt_mod  = require("coding_adventures.repl.default_prompt")
local waiting_mod = require("coding_adventures.repl.silent_waiting")

return {
    VERSION = "0.1.0",

    -- ── Core loop functions ────────────────────────────────────────────────

    -- run(language, prompt, waiting)
    --   Start an interactive REPL on stdio.
    --   language is required. prompt and waiting default to the built-ins.
    run = loop_mod.run,

    -- run_with_io(language, prompt, waiting, input_fn, output_fn)
    --   Start a REPL with injected I/O. Useful for testing and embedding.
    run_with_io = loop_mod.run_with_io,

    -- ── Built-in plug-ins ──────────────────────────────────────────────────

    -- EchoLanguage — evaluates ":quit" as a quit signal; echoes everything else
    EchoLanguage = echo_mod,

    -- DefaultPrompt — uses "> " for the global prompt and "... " for continuation
    DefaultPrompt = prompt_mod,

    -- SilentWaiting — all no-ops; tick_ms() returns 100
    SilentWaiting = waiting_mod,
}
