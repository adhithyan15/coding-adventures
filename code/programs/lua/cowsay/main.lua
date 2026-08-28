#!/usr/bin/env lua
-- cowsay (Lua) — entry point
-- ================================================================
--
-- Thin CLI wiring: parse argv against code/specs/cowsay.json via
-- CliBuilder, resolve the parsed flags/arguments into an invocation table,
-- and hand off to the sibling cowsay.lua module for the actual formatting +
-- paint-vm-ascii render. See code/specs/cowsay-paintvm-pipeline.md for the
-- design, and cowsay.lua's own doc comment for how this port's logic maps
-- onto code/programs/go/cowsay/main.go and
-- code/programs/perl/cowsay/lib/CodingAdventures/Cowsay.pm.
--
-- Lua's `arg` global already excludes the program name the way this
-- package's cli_builder.Parser.parse expects ("argv WITHOUT argv[0]") --
-- `arg[0]` is the script path, `arg[1..n]` are the real CLI arguments, so
-- `arg` is passed straight through with no placeholder prepended (unlike
-- the C#/F# CliBuilder ports, which need a program-name placeholder; see
-- known_gotchas in .claude/cowsay-paintvm-loop-state.json).

-- ============================================================================
-- Module path setup
--
-- Same technique code/programs/lua/parrot/main.lua already uses: derive
-- this file's own directory from arg[0] (running as a script) or
-- debug.getinfo (required as a module, e.g. from tests), then prepend the
-- sibling packages' src/ directories to package.path before requiring them.
-- ============================================================================

local function this_dir()
    if arg and arg[0] then
        local dir = arg[0]:match("^(.*)[/\\]")
        return dir or "."
    end
    local src = debug.getinfo(1, "S").source
    if src:sub(1, 1) == "@" then
        local dir = src:sub(2):match("^(.*)[/\\]")
        return dir or "."
    end
    return "."
end

local DIR = this_dir()
local SEP = package.config:sub(1, 1) -- "/" on Unix, "\" on Windows

-- Path to the sibling packages' src/ directories, relative to this file's
-- location. We traverse: cowsay/ -> lua/ -> programs/ -> code/ -> (root) ->
-- packages/lua/<pkg>/src/.
local function sibling_src(pkg)
    return DIR .. SEP .. ".." .. SEP .. ".." .. SEP .. ".."
        .. SEP .. "packages" .. SEP .. "lua" .. SEP .. pkg .. SEP .. "src"
end

for _, pkg in ipairs({ "cli_builder", "paint_instructions", "paint_vm_ascii" }) do
    local src = sibling_src(pkg)
    package.path = src .. SEP .. "?.lua;" .. src .. SEP .. "?" .. SEP .. "init.lua;" .. package.path
end

-- This program's own directory, so `require("cowsay")` finds cowsay.lua
-- regardless of the caller's current working directory.
package.path = DIR .. SEP .. "?.lua;" .. package.path

-- ============================================================================
-- Imports
-- ============================================================================

local cli = require("coding_adventures.cli_builder")
local cowsay = require("cowsay")

-- ============================================================================
-- stdin TTY detection
-- ================================================================
--
-- When no message argument is given, cowsay falls back to reading stdin --
-- but only when stdin is piped/redirected, not when it's an interactive
-- terminal (matching every already-merged port: Go checks
-- os.ModeCharDevice, Perl checks `-t STDIN`). Lua's standard library has no
-- portable TTY-detection primitive, so this shells out to the POSIX `test
-- -t 0` idiom on POSIX platforms. On Windows (no `test` command), or if the
-- check is otherwise inconclusive, this falls back to attempting the read
-- rather than guessing wrong in the other direction — matching
-- code/programs/lua/parrot's precedent of Lua's interactive/stdin-driven
-- programs already being excluded from Windows CI (see parrot's
-- BUILD_windows), so this narrow edge case (a real interactive Windows
-- terminal session with no message argument and no piped input) isn't
-- exercised by this repo's automated tests either way.
local function stdin_is_tty()
    if SEP == "\\" then
        return false
    end
    local ok = os.execute("test -t 0 2>/dev/null")
    return ok == true
end

-- ============================================================================
-- main
-- ============================================================================

local function main()
    local repo_root = cowsay.find_repo_root(DIR)
    local spec_path = repo_root .. SEP .. "code" .. SEP .. "specs" .. SEP .. "cowsay.json"
    local cows_dir = repo_root .. SEP .. "code" .. SEP .. "specs" .. SEP .. "cows"

    local ok, result = pcall(cli.parse, spec_path, arg)
    if not ok then
        io.stderr:write("cowsay: " .. tostring(result) .. "\n")
        os.exit(1)
    end

    if result.type == "help" then
        print(result.text)
        os.exit(0)
    elseif result.type == "version" then
        print(result.version)
        os.exit(0)
    elseif result.type == "error" then
        for _, e in ipairs(result.errors) do
            io.stderr:write(e.message .. "\n")
        end
        os.exit(1)
    end

    local flags = result.flags
    local arguments = result.arguments

    if cowsay.is_list_requested(flags) then
        for _, name in ipairs(cowsay.list_cow_files(cows_dir)) do
            print(name)
        end
        os.exit(0)
    end

    local message = cowsay.resolve_message_from_arguments(arguments)
    if message == nil then
        if stdin_is_tty() then
            os.exit(0)
        end
        local stdin_content = io.read("a") or ""
        message = stdin_content:match("^%s*(.-)%s*$")
    end

    if message == "" then
        os.exit(0)
    end

    local invocation = cowsay.build_invocation(message, flags)

    -- Rendering (in particular, load_cow's file I/O and paint_vm_ascii's
    -- own error() calls on malformed geometry) can raise. A raw Lua
    -- traceback would leak internal file paths to the user; report a clean
    -- "cowsay: ..." message on stderr instead, mirroring the fix Java's
    -- port needed after /security-review flagged an uncaught IOException
    -- leaking a raw stack trace (see .claude/cowsay-paintvm-loop-state.json,
    -- java-cowsay's notes).
    local render_ok, rendered = pcall(cowsay.render, invocation, cows_dir)
    if not render_ok then
        io.stderr:write("cowsay: " .. tostring(rendered) .. "\n")
        os.exit(1)
    end

    print(rendered)
end

-- ============================================================================
-- Entry point guard (see code/programs/lua/parrot/main.lua for the same
-- pattern and its rationale: busted's arg[0] never ends in "main.lua", so
-- `require("main")` from a test never triggers main()).
-- ============================================================================

if arg and arg[0] and arg[0]:match("main%.lua$") then
    main()
end

return {
    main = main,
    this_dir = this_dir,
    stdin_is_tty = stdin_is_tty,
}
