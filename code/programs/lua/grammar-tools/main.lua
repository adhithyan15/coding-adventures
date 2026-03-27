#!/usr/bin/env lua
-- grammar-tools — validate .tokens and .grammar files from the command line.
-- ==========================================================================
--
-- This program wraps the `coding_adventures.grammar_tools` Lua library and
-- exposes three commands:
--
--   grammar-tools validate <tokens-file> <grammar-file>
--   grammar-tools validate-tokens <tokens-file>
--   grammar-tools validate-grammar <grammar-file>
--   grammar-tools --help
--
-- Because there is no cli-builder package for Lua, argument parsing is done
-- manually using the standard `arg` table (identical convention to all other
-- Lua programs in this monorepo).
--
-- === Why a program at all? ===
--
-- The library already works perfectly as a Lua module.  The CLI layer adds
-- one thing: a standard shell interface so CI scripts, Makefiles, and other
-- tools can call it without writing Lua code.  Every language in this monorepo
-- ships the same three commands so that grammar authors can use whichever
-- language runtime they have available.
--
-- === Exit codes ===
--
--   0 — All checks passed
--   1 — One or more validation errors found
--   2 — Usage error (wrong number of arguments, unknown command)

-- ---------------------------------------------------------------------------
-- Module search path — resolve the grammar_tools library from this program's
-- location so the program works regardless of where it is invoked from.
-- ---------------------------------------------------------------------------

-- arg[0] is the path to main.lua (e.g., /path/to/grammar-tools/main.lua).
-- We derive the directory from it and add the library's src directory to the
-- Lua module path.
local function script_dir()
    local src = arg[0] or "main.lua"
    -- Remove the filename to get the directory.
    return src:match("^(.*[/\\])") or "./"
end

local DIR = script_dir()
-- Add the grammar_tools package source tree to the module path.
package.path = DIR .. "../../../packages/lua/grammar_tools/src/?.lua;"
             .. DIR .. "../../../packages/lua/grammar_tools/src/?/init.lua;"
             .. package.path

local grammar_tools = require("coding_adventures.grammar_tools")

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

--- Count issues that are real errors (not warnings).
--
-- Issues beginning with "Warning" are informational only.  This convention is
-- shared across all language implementations so output can be compared in CI.
local function count_errors(issues)
    local n = 0
    for _, issue in ipairs(issues) do
        if not issue:match("^Warning") then
            n = n + 1
        end
    end
    return n
end

--- Print each issue with two-space indentation.
local function print_issues(issues)
    for _, issue in ipairs(issues) do
        print("  " .. issue)
    end
end

--- Extract the filename from a path (last component after / or \).
local function basename(path)
    return path:match("[^/\\]+$") or path
end

-- ---------------------------------------------------------------------------
-- validate_command — validate a (.tokens, .grammar) pair
-- ---------------------------------------------------------------------------

--- Validate a .tokens file and a .grammar file together.
--
-- Returns 0 on success, 1 on any error.
local function validate_command(tokens_path, grammar_path)
    local total_errors = 0

    -- Step 1: tokens file ---------------------------------------------------
    io.write("Validating " .. basename(tokens_path) .. " ... ")

    local tf, err = io.open(tokens_path, "r")
    if not tf then
        print("ERROR")
        print("  " .. (err or "could not open file"))
        return 1
    end
    local tokens_source = tf:read("*a")
    tf:close()

    local ok, token_grammar = pcall(grammar_tools.parse_token_grammar, tokens_source)
    if not ok then
        print("PARSE ERROR")
        print("  " .. tostring(token_grammar))
        return 1
    end

    local token_issues = grammar_tools.validate_token_grammar(token_grammar)
    local token_errors = count_errors(token_issues)
    local n_tokens = #token_grammar.definitions
    local n_skip   = #token_grammar.skip_definitions
    local n_error_defs = #token_grammar.error_definitions

    if token_errors > 0 then
        print(token_errors .. " error(s)")
        print_issues(token_issues)
        total_errors = total_errors + token_errors
    else
        local parts = { n_tokens .. " tokens" }
        if n_skip > 0 then parts[#parts+1] = n_skip .. " skip" end
        if n_error_defs > 0 then parts[#parts+1] = n_error_defs .. " error" end
        print("OK (" .. table.concat(parts, ", ") .. ")")
    end

    -- Step 2: grammar file --------------------------------------------------
    io.write("Validating " .. basename(grammar_path) .. " ... ")

    local gf, gerr = io.open(grammar_path, "r")
    if not gf then
        print("ERROR")
        print("  " .. (gerr or "could not open file"))
        return 1
    end
    local grammar_source = gf:read("*a")
    gf:close()

    local ok2, parser_grammar = pcall(grammar_tools.parse_parser_grammar, grammar_source)
    if not ok2 then
        print("PARSE ERROR")
        print("  " .. tostring(parser_grammar))
        return 1
    end

    local tnames = token_grammar:token_names()
    local parser_issues = grammar_tools.validate_parser_grammar(parser_grammar, tnames)
    local parser_errors = count_errors(parser_issues)
    local n_rules = #parser_grammar.rules

    if parser_errors > 0 then
        print(parser_errors .. " error(s)")
        print_issues(parser_issues)
        total_errors = total_errors + parser_errors
    else
        print("OK (" .. n_rules .. " rules)")
    end

    -- Step 3: cross-validation ----------------------------------------------
    io.write("Cross-validating ... ")

    local cross_issues = grammar_tools.cross_validate(token_grammar, parser_grammar)
    local cross_errors = count_errors(cross_issues)
    local cross_warnings = #cross_issues - cross_errors

    if cross_errors > 0 then
        print(cross_errors .. " error(s)")
        print_issues(cross_issues)
        total_errors = total_errors + cross_errors
    elseif cross_warnings > 0 then
        print("OK (" .. cross_warnings .. " warning(s))")
        print_issues(cross_issues)
    else
        print("OK")
    end

    if total_errors > 0 then
        print("\nFound " .. total_errors .. " error(s). Fix them and try again.")
        return 1
    else
        print("\nAll checks passed.")
        return 0
    end
end

-- ---------------------------------------------------------------------------
-- validate_tokens_only — validate just a .tokens file
-- ---------------------------------------------------------------------------

--- Validate only a .tokens file.  Returns 0 on success, 1 on error.
local function validate_tokens_only(tokens_path)
    io.write("Validating " .. basename(tokens_path) .. " ... ")

    local tf, err = io.open(tokens_path, "r")
    if not tf then
        print("ERROR")
        print("  " .. (err or "could not open file"))
        return 1
    end
    local tokens_source = tf:read("*a")
    tf:close()

    local ok, token_grammar = pcall(grammar_tools.parse_token_grammar, tokens_source)
    if not ok then
        print("PARSE ERROR")
        print("  " .. tostring(token_grammar))
        return 1
    end

    local issues = grammar_tools.validate_token_grammar(token_grammar)
    local errors = count_errors(issues)
    local n_tokens = #token_grammar.definitions

    if errors > 0 then
        print(errors .. " error(s)")
        print_issues(issues)
        print("\nFound " .. errors .. " error(s). Fix them and try again.")
        return 1
    else
        print("OK (" .. n_tokens .. " tokens)")
        print("\nAll checks passed.")
        return 0
    end
end

-- ---------------------------------------------------------------------------
-- validate_grammar_only — validate just a .grammar file
-- ---------------------------------------------------------------------------

--- Validate only a .grammar file (no token names available, so undefined
-- token-reference checks are skipped).  Returns 0 on success, 1 on error.
local function validate_grammar_only(grammar_path)
    io.write("Validating " .. basename(grammar_path) .. " ... ")

    local gf, err = io.open(grammar_path, "r")
    if not gf then
        print("ERROR")
        print("  " .. (err or "could not open file"))
        return 1
    end
    local grammar_source = gf:read("*a")
    gf:close()

    local ok, parser_grammar = pcall(grammar_tools.parse_parser_grammar, grammar_source)
    if not ok then
        print("PARSE ERROR")
        print("  " .. tostring(parser_grammar))
        return 1
    end

    -- Pass nil for token_names so undefined-token-reference checks are skipped.
    local issues = grammar_tools.validate_parser_grammar(parser_grammar, nil)
    local errors = count_errors(issues)
    local n_rules = #parser_grammar.rules

    if errors > 0 then
        print(errors .. " error(s)")
        print_issues(issues)
        print("\nFound " .. errors .. " error(s). Fix them and try again.")
        return 1
    else
        print("OK (" .. n_rules .. " rules)")
        print("\nAll checks passed.")
        return 0
    end
end

-- ---------------------------------------------------------------------------
-- compile_tokens_command — compile a .tokens file to Lua source code
-- ---------------------------------------------------------------------------

--- Compile a .tokens file to Lua source code.
--
-- Reads and parses the .tokens file, then calls compile_token_grammar to
-- produce Lua source code that embeds the grammar as native Lua data.
-- Generated code is written to output_path (if given) or printed to stdout.
-- Status messages go to stderr.
--
-- Returns 0 on success, 1 on error.
local function compile_tokens_command(tokens_path, output_path)
    local tf, err = io.open(tokens_path, "r")
    if not tf then
        io.stderr:write("Error: cannot open '" .. tokens_path .. "': " .. (err or "unknown error") .. "\n")
        return 1
    end
    local source = tf:read("*a")
    tf:close()

    local ok, token_grammar = pcall(grammar_tools.parse_token_grammar, source)
    if not ok then
        io.stderr:write("Parse error in '" .. tokens_path .. "': " .. tostring(token_grammar) .. "\n")
        return 1
    end

    local code = grammar_tools.compile_token_grammar(token_grammar, basename(tokens_path))

    if output_path then
        local out, werr = io.open(output_path, "w")
        if not out then
            io.stderr:write("Error: cannot write '" .. output_path .. "': " .. (werr or "unknown error") .. "\n")
            return 1
        end
        out:write(code)
        out:close()
        io.stderr:write("Compiled " .. basename(tokens_path) .. " → " .. output_path .. "\n")
    else
        io.write(code)
        io.stderr:write("Compiled " .. basename(tokens_path) .. " to stdout.\n")
    end
    return 0
end

-- ---------------------------------------------------------------------------
-- compile_grammar_command — compile a .grammar file to Lua source code
-- ---------------------------------------------------------------------------

--- Compile a .grammar file to Lua source code.
--
-- Reads and parses the .grammar file, then calls compile_parser_grammar to
-- produce Lua source code that embeds the grammar as native Lua data.
-- Generated code is written to output_path (if given) or printed to stdout.
-- Status messages go to stderr.
--
-- Returns 0 on success, 1 on error.
local function compile_grammar_command(grammar_path, output_path)
    local gf, err = io.open(grammar_path, "r")
    if not gf then
        io.stderr:write("Error: cannot open '" .. grammar_path .. "': " .. (err or "unknown error") .. "\n")
        return 1
    end
    local source = gf:read("*a")
    gf:close()

    local ok, parser_grammar = pcall(grammar_tools.parse_parser_grammar, source)
    if not ok then
        io.stderr:write("Parse error in '" .. grammar_path .. "': " .. tostring(parser_grammar) .. "\n")
        return 1
    end

    local code = grammar_tools.compile_parser_grammar(parser_grammar, basename(grammar_path))

    if output_path then
        local out, werr = io.open(output_path, "w")
        if not out then
            io.stderr:write("Error: cannot write '" .. output_path .. "': " .. (werr or "unknown error") .. "\n")
            return 1
        end
        out:write(code)
        out:close()
        io.stderr:write("Compiled " .. basename(grammar_path) .. " → " .. output_path .. "\n")
    else
        io.write(code)
        io.stderr:write("Compiled " .. basename(grammar_path) .. " to stdout.\n")
    end
    return 0
end

-- ---------------------------------------------------------------------------
-- dispatch — route command + files to the right function
-- ---------------------------------------------------------------------------

--- Dispatch a command name and file list.  Returns an exit code (0, 1, 2).
--
-- output_path is optional; when provided it is passed to the compile commands.
local function dispatch(command, files, output_path)
    if command == "validate" then
        if #files ~= 2 then
            io.stderr:write("Error: 'validate' requires exactly two files: <tokens> <grammar>\n")
            return 2
        end
        return validate_command(files[1], files[2])

    elseif command == "validate-tokens" then
        if #files ~= 1 then
            io.stderr:write("Error: 'validate-tokens' requires exactly one file: <tokens>\n")
            return 2
        end
        return validate_tokens_only(files[1])

    elseif command == "validate-grammar" then
        if #files ~= 1 then
            io.stderr:write("Error: 'validate-grammar' requires exactly one file: <grammar>\n")
            return 2
        end
        return validate_grammar_only(files[1])

    elseif command == "compile-tokens" then
        if #files ~= 1 then
            io.stderr:write("Error: 'compile-tokens' requires exactly one file: <tokens>\n")
            return 2
        end
        return compile_tokens_command(files[1], output_path)

    elseif command == "compile-grammar" then
        if #files ~= 1 then
            io.stderr:write("Error: 'compile-grammar' requires exactly one file: <grammar>\n")
            return 2
        end
        return compile_grammar_command(files[1], output_path)

    else
        io.stderr:write("Error: unknown command '" .. tostring(command) .. "'\n")
        return 2
    end
end

-- ---------------------------------------------------------------------------
-- print_usage — help text
-- ---------------------------------------------------------------------------

local function print_usage()
    print("Usage: lua main.lua <command> [options] [files...]")
    print()
    print("Commands:")
    print("  validate <tokens> <grammar>   Validate a token/grammar pair")
    print("  validate-tokens <tokens>      Validate just a .tokens file")
    print("  validate-grammar <grammar>    Validate just a .grammar file")
    print("  compile-tokens <tokens>       Compile a .tokens file to Lua source")
    print("  compile-grammar <grammar>     Compile a .grammar file to Lua source")
    print()
    print("Options:")
    print("  -o, --output <file>           Write generated code to file (compile commands only)")
    print()
    print("Exit codes:")
    print("  0   All checks passed / code compiled")
    print("  1   One or more errors found")
    print("  2   Usage error")
end

-- ---------------------------------------------------------------------------
-- main — parse arg[] and dispatch
-- ---------------------------------------------------------------------------

--- Top-level entry point.  Returns an exit code.
local function main()
    -- When loaded as a module by tests, arg may be nil.
    if arg == nil then return 0 end

    -- arg[1] = first user argument (command), arg[0] = script name.
    if arg[1] == nil or arg[1] == "--help" or arg[1] == "-h" or arg[1] == "help" then
        print_usage()
        return 0
    end

    local command = arg[1]
    local files = {}
    local output_path = nil
    local i = 2
    while i <= #arg do
        local a = arg[i]
        if a == "-o" or a == "--output" then
            i = i + 1
            output_path = arg[i]
        else
            files[#files + 1] = a
        end
        i = i + 1
    end

    return dispatch(command, files, output_path)
end

-- ---------------------------------------------------------------------------
-- Export public API for testing, then run main when executed directly.
-- ---------------------------------------------------------------------------

local M = {
    validate_command      = validate_command,
    validate_tokens_only  = validate_tokens_only,
    validate_grammar_only = validate_grammar_only,
    compile_tokens_command  = compile_tokens_command,
    compile_grammar_command = compile_grammar_command,
    dispatch              = dispatch,
    main                  = main,
}

-- When executed as a script (not required as a module), run main().
-- The heuristic: if arg[0] ends with "main.lua" we are being run directly.
if arg and arg[0] and arg[0]:match("main%.lua$") then
    os.exit(main())
end

return M
