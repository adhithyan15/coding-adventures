#!/usr/bin/env lua
-- Parrot REPL — the world's simplest REPL.
--
-- Whatever you type, I repeat back. Type :quit to exit.
--
-- This program demonstrates the coding-adventures REPL framework by wiring
-- three plug-in objects together:
--
--   EchoLanguage   — evaluates input by echoing it back unchanged.
--                    Special case: ":quit" signals the loop to stop.
--   ParrotPrompt   — provides parrot-themed prompts and banner text.
--   SilentWaiting  — shows nothing while "evaluating" (a no-op spinner).
--
-- # Why only sync mode?
--
-- Lua's coroutines are cooperative: once language.eval() is called, no other
-- Lua code runs until eval() returns. There is no mechanism in standard Lua
-- to run code concurrently without an external library (e.g., luv/libuv for
-- async I/O, or LuaJIT's OS-thread support). We intentionally avoid external
-- event-loop dependencies to keep the framework self-contained.
--
-- Passing opts.mode = "async" to the loop will raise an explicit error rather
-- than silently misbehaving — fail loud and early.

-- ============================================================================
-- Module path setup
--
-- When Lua loads a file, the `package.path` variable controls where `require`
-- looks for modules. The default path does not include our monorepo layout, so
-- we extend it here before any require calls.
--
-- We need to find the repl package source tree, which lives at:
--
--   <this-file>/../../../packages/lua/repl/src/
--
-- We derive the directory of this file from arg[0] (the script path) when
-- running as a program, or from debug.getinfo when required as a module.
-- ============================================================================

local function this_dir()
    -- When executed as a script: arg[0] holds the path to main.lua.
    if arg and arg[0] then
        -- Remove the filename portion to get the directory.
        local dir = arg[0]:match("^(.*)[/\\]")
        return dir or "."
    end
    -- When required as a module (e.g., from tests): debug.getinfo gives us
    -- the source path with a leading "@" that we strip.
    local src = debug.getinfo(1, "S").source
    if src:sub(1, 1) == "@" then
        local dir = src:sub(2):match("^(.*)[/\\]")
        return dir or "."
    end
    return "."
end

local DIR = this_dir()
local SEP = package.config:sub(1, 1)  -- "/" on Unix, "\" on Windows

-- Path to the repl package's src/ directory, relative to this file's location.
-- We traverse: parrot/ → lua/ → programs/ → code/ → (root) → packages/lua/repl/src/
local repl_src = DIR
    .. SEP .. ".." .. SEP .. ".." .. SEP .. ".."
    .. SEP .. "packages" .. SEP .. "lua" .. SEP .. "repl" .. SEP .. "src"

-- Prepend our paths so they take priority over anything in the default path.
package.path = repl_src .. SEP .. "?.lua;"
            .. repl_src .. SEP .. "?" .. SEP .. "init.lua;"
            .. package.path

-- ============================================================================
-- Imports
--
-- We load only the pieces we actually use rather than the umbrella init.lua.
-- This makes dependencies explicit and avoids loading default_prompt.lua,
-- which we are replacing with our own ParrotPrompt.
-- ============================================================================

local Loop   = require("coding_adventures.repl.loop")
local Echo   = require("coding_adventures.repl.echo_language")
local Silent = require("coding_adventures.repl.silent_waiting")

-- ============================================================================
-- ParrotPrompt
--
-- Implements the Prompt interface expected by the REPL loop:
--
--   prompt.global_prompt() → string
--     Called before each READ step (i.e., once per input line). In a richer
--     REPL this would show a persistent banner or session header. Here we
--     display it per-line, which is what the loop expects — it calls
--     global_prompt() on every iteration.
--
--   prompt.line_prompt() → string
--     Called on continuation lines (multi-line input, e.g., incomplete
--     expressions). EchoLanguage never produces continuation lines, but we
--     implement the method to satisfy the interface contract.
--
-- A parrot's defining trait is repetition — it says back whatever it hears.
-- The emoji and themed wording reinforce this personality.
-- ============================================================================

local ParrotPrompt = {}
ParrotPrompt.__index = ParrotPrompt

-- new() → ParrotPrompt instance
--
-- ParrotPrompt is stateless: every instance behaves identically. We still
-- use __index / new() to follow the conventional Lua OOP pattern used
-- throughout this codebase.
function ParrotPrompt.new()
    return setmetatable({}, ParrotPrompt)
end

-- global_prompt() → string
--
-- The banner shown before each input line. Because the REPL loop calls this
-- on every iteration, we keep it short: just a labeled prompt rather than a
-- multi-line startup message. This is consistent with how DefaultPrompt works
-- in the repl package.
function ParrotPrompt:global_prompt()
    return "Parrot REPL - I repeat everything you say! (:quit to exit)\n"
        .. "\240\159\166\156 > "
    -- The bytes \240\159\166\156 are the UTF-8 encoding of the 🦜 parrot emoji.
    -- We write it as bytes rather than the literal glyph because some Lua
    -- builds and Windows terminals handle source-level emoji inconsistently.
    -- Any compliant UTF-8 terminal will render it correctly.
end

-- line_prompt() → string
--
-- Shown on continuation lines (when the user has begun a multi-line expression
-- and the evaluator needs more input before it can evaluate). EchoLanguage
-- always returns a result immediately, so this prompt is never displayed in
-- practice — but we implement it correctly to satisfy the Prompt interface.
function ParrotPrompt:line_prompt()
    return "\240\159\166\156 . "
    -- The ". " suffix mirrors the conventional "... " continuation prompt,
    -- shortened to keep it visually distinct from the primary prompt.
end

-- ============================================================================
-- main — run the Parrot REPL on standard I/O
--
-- We wire together the three plug-in objects and pass standard I/O functions
-- to run_with_io. This is the normal entry point when the program is executed
-- from the command line.
--
-- Using run_with_io (instead of the simpler run()) gives us explicit control
-- over I/O, and keeps the same code path that tests exercise.
-- ============================================================================

local function main()
    Loop.run_with_io(
        Echo,                      -- language: echo input back, quit on :quit
        ParrotPrompt.new(),        -- prompt:   parrot-themed prompts
        Silent,                    -- waiting:  silent no-op (eval is instant)
        function()                 -- input_fn: read one line from stdin
            -- io.read("l") returns the line WITHOUT the trailing newline,
            -- or nil on EOF (Ctrl-D on Unix, Ctrl-Z on Windows, pipe end).
            return io.read("l")
        end,
        function(text)             -- output_fn: write to stdout, no extra newline
            io.write(text)
            -- Flush immediately so the prompt appears before the user types.
            -- Without flush(), output may sit in the buffer until the program
            -- exits — which would make the REPL feel completely broken.
            io.flush()
        end,
        { mode = "sync" }          -- opts: sync is the only supported mode
    )
end

-- ============================================================================
-- Entry point guard
--
-- Lua has no built-in "am I the main script?" mechanism, but `arg` provides
-- one idiomatically:
--
--   When Lua executes a file directly: arg[0] is the script path.
--   When another file does require("main"): arg[0] is the OUTER script's path,
--   but more importantly, `arg` is the global arg table of the OUTER script.
--
-- We use a pattern match on arg[0] to check if THIS file is the top-level
-- script. The match "main%.lua$" handles both bare ("main.lua") and path-
-- qualified ("/some/path/main.lua") invocations.
--
-- Tests do: local Parrot = require("main")
-- The test runner (busted) sets arg[0] to the busted executable path, which
-- does NOT end in "main.lua", so the guard prevents main() from running.
-- ============================================================================

if arg and arg[0] and arg[0]:match("main%.lua$") then
    main()
end

-- Export for testing.
-- Tests import this module and call ParrotPrompt directly to verify prompt
-- strings, and call run_parrot() with injected I/O to verify loop behaviour.
return {
    ParrotPrompt = ParrotPrompt,
    main         = main,
    Loop         = Loop,
    Echo         = Echo,
    Silent       = Silent,
}
