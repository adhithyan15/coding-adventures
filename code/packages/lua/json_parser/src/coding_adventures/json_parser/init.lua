-- json_parser -- Builds an AST from JSON text using the grammar-driven engine
-- ===========================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer, above the lexer and grammar_tools
-- packages and alongside other language parsers.
--
-- # What does a parser do?
--
-- A lexer breaks raw text into a flat stream of tokens:
--
--   '{"key": 42}'  →  LBRACE STRING COLON NUMBER RBRACE EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the input:
--
--   value
--   └── object
--       ├── LBRACE  "{"
--       ├── pair
--       │   ├── STRING  '"key"'
--       │   ├── COLON   ":"
--       │   └── value
--       │       └── NUMBER  "42"
--       └── RBRACE  "}"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, serializers, validators) walk the AST rather than
-- re-parsing the text.
--
-- # Grammar
--
-- The JSON grammar is defined in `code/grammars/json.grammar`:
--
--   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
--   object = LBRACE [ pair { COMMA pair } ] RBRACE ;
--   pair   = STRING COLON value ;
--   array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
--
-- There are exactly four rules.  The grammar is recursive: `value`
-- references `object` and `array`, both of which reference `value` again.
-- This mutual recursion allows arbitrarily deep nesting.
--
-- # Architecture
--
-- 1. **Tokenize** — call `json_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # GrammarParser and ASTNode
--
-- `GrammarParser.new(tokens, grammar)` returns a parser instance.
-- Calling `:parse()` returns either:
--   (node, nil)    — success; `node` is the root ASTNode
--   (nil, errmsg)  — failure; `errmsg` is a human-readable error string
--
-- ASTNode fields:
--   node.rule_name   — which grammar rule produced this node ("value", "pair", …)
--   node.children    — array of child ASTNodes and/or Token tables
--   node:is_leaf()   — true when the node wraps exactly one token
--   node:token()     — the wrapped token (only valid when is_leaf() is true)
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/json_parser/src/coding_adventures/json_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   json_parser/        (1)
--   coding_adventures/  (2)
--   src/                (3)
--   json_parser/        (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/json.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local json_lexer    = require("coding_adventures.json_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by json_lexer (which navigates
-- to json.tokens).  We do the same to reach json.grammar.

--- Return the directory portion of a file path (no trailing slash).
-- Example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua prepends "@" to the source path in debug info — we strip it.
-- When busted runs tests with a relative path containing ".." the
-- dirname-only approach produces a path that collapses to "." after
-- up() steps, so the grammar file cannot be found.  We resolve to an
-- absolute path via "cd <dir> && pwd" to give up() an absolute anchor.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    -- Security: io.popen is used only with fixed built-in commands ("cd"/"pwd"),
    -- never with user-controlled input. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- The current approach is safe: no user input reaches the shell.
    -- Updated: 2026-04-10.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd == "" then
            -- LuaFileSystem gives an absolute cwd without spawning a subprocess.
            -- busted (our test runner) depends on lfs, so it is available at test time.
            local ok, lfs = pcall(require, "lfs")
            if ok and lfs and lfs.currentdir then
                cwd = lfs.currentdir() or ""
            end
        end
        if cwd == "" then
            -- Try `pwd` — works in POSIX shells and MSYS/Git-Bash on Windows.
            -- Safe: fixed command with no user input — no injection risk.
            local h = io.popen("pwd")
            if h then
                local line = (h:read("*l") or ""):gsub("%c+$", "")
                h:close()
                -- Convert MSYS/Git-Bash path "/d/foo" → "D:/foo" so Windows
                -- io.open() can find the file (it uses Win32 file APIs).
                line = line:gsub("^/(%a)/", function(d) return d:upper() .. ":/" end)
                cwd = line
            end
        end
        if cwd == "" then
            -- Last resort: cmd.exe `cd` builtin (native Windows cmd context).
            -- Safe: fixed command with no user input — no injection risk.
            local h = io.popen("cd")
            if h then
                cwd = (h:read("*l") or ""):gsub("%c+$", "")
                h:close()
            end
        end
        if cwd ~= "" then
            cwd = cwd:gsub("\\", "/"):gsub("%c+$", "")
            dir = cwd .. "/" .. dir
            -- Normalise .. and . segments so dirname-based traversal works
            -- correctly when the source was loaded via a relative package.path
            -- entry (e.g. "../src/?.lua" from a tests/ subdirectory).
            local is_abs = dir:sub(1, 1) == "/"
            local parts = {}
            for seg in dir:gmatch("[^/]+") do
                if seg == ".." then
                    if #parts > 0 then table.remove(parts) end
                elseif seg ~= "." then
                    table.insert(parts, seg)
                end
            end
            dir = (is_abs and "/" or "") .. table.concat(parts, "/")
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = dirname(result)
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The parser grammar is loaded from disk once and cached.  Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar, avoiding
-- repeated file I/O and repeated rule compilation.

local _grammar_cache = nil

--- Load and parse `json.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed JSON parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/json.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "json_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "json_parser: failed to parse json.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a JSON source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `json_lexer.tokenize`.
--   2. Loads the JSON parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "value"` (the first rule in the
-- JSON grammar).
--
-- @param source string  The JSON text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local json_parser = require("coding_adventures.json_parser")
--   local ast = json_parser.parse('{"x": 42}')
--   -- ast.rule_name  → "value"
--   -- #ast.children  → 1  (an "object" node)
function M.parse(source)
    local tokens = json_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("json_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a JSON source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode (`GrammarParser.new_with_trace`) or to inspect the token
-- stream before parsing.
--
-- @param source string   The JSON text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = json_parser.create_parser('{"x":1}')
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = json_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for JSON.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or to check how many rules the grammar has.
--
-- @return ParserGrammar  The parsed JSON parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
