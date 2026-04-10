-- sql_parser -- Builds an AST from SQL text using the grammar-driven engine
-- ===========================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer, above the lexer, grammar_tools, and
-- sql_lexer packages, and alongside other language parsers such as json_parser
-- and toml_parser.
--
-- # What does a SQL parser do?
--
-- A lexer breaks raw SQL text into a flat stream of tokens:
--
--   'SELECT name FROM users WHERE age > 18'
--   →  SELECT NAME FROM NAME WHERE NAME GREATER_THAN NUMBER EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the SQL statement:
--
--   program
--   └── statement
--       └── select_stmt
--           ├── SELECT  "SELECT"
--           ├── select_list
--           │   └── select_item
--           │       └── expr → … → column_ref → NAME "name"
--           ├── FROM    "FROM"
--           ├── table_ref → table_name → NAME "users"
--           └── where_clause
--               ├── WHERE  "WHERE"
--               └── expr → … → comparison
--                   ├── additive → … → column_ref → NAME "age"
--                   ├── cmp_op → GREATER_THAN ">"
--                   └── additive → … → NUMBER "18"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (query planners, evaluators, formatters) walk the AST rather than
-- re-parsing the text.
--
-- # SQL grammar
--
-- The SQL grammar is defined in `code/grammars/sql.grammar`.  The entry
-- point is `program`, which allows one or more semicolon-separated statements.
-- Supported statement types:
--
--   SELECT   col1, col2 FROM table [WHERE expr] [GROUP BY] [HAVING] …
--   INSERT   INTO table VALUES (…)
--   UPDATE   table SET col = expr [WHERE expr]
--   DELETE   FROM table [WHERE expr]
--   CREATE   TABLE name (col_def, …)
--   DROP     TABLE [IF EXISTS] name
--
-- Expressions support full operator precedence:
--
--   OR < AND < NOT < comparison < + - < * / % < unary - < primary
--
-- # Architecture
--
-- 1. **Tokenize** — call `sql_lexer.tokenize(source)` to get a token list.
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
--   node.rule_name  — which grammar rule produced this node ("program", …)
--   node.children   — array of child ASTNodes and/or Token tables
--   node:is_leaf()  — true when the node wraps exactly one token
--   node:token()    — the wrapped token (only valid when is_leaf() is true)
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/sql_parser/src/coding_adventures/sql_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   sql_parser/         (1)
--   coding_adventures/  (2)
--   src/                (3)
--   sql_parser/         (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/sql.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local sql_lexer     = require("coding_adventures.sql_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by json_parser and toml_parser.
-- We navigate up 6 levels from this file's directory to reach `code/`,
-- then descend into `grammars/sql.grammar`.

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
            -- Safe fallback: fixed built-in command, no user input — no injection risk.
            -- On Windows `cd` prints the current directory; on POSIX `pwd` does the same.
            local is_win = package.config:sub(1, 1) == "\\"
            local h = is_win and io.popen("cd") or io.popen("pwd")
            if h then
                local line = h:read("*l") or ""
                h:close()
                cwd = line:gsub("%c+$", "")
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

--- Load and parse `sql.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed SQL parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/sql.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "sql_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "sql_parser: failed to parse sql.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a SQL source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `sql_lexer.tokenize`.
--   2. Loads the SQL parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- SQL grammar).
--
-- SQL keywords are case-insensitive (handled by the lexer).
-- Whitespace and comments are stripped by the lexer before parsing.
--
-- @param source string  The SQL text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local sql_parser = require("coding_adventures.sql_parser")
--   local ast = sql_parser.parse("SELECT * FROM users")
--   -- ast.rule_name  → "program"
--   -- contains statement → select_stmt
function M.parse(source)
    local tokens = sql_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("sql_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a SQL source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The SQL text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = sql_parser.create_parser("SELECT 1")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = sql_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for SQL.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed SQL parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
