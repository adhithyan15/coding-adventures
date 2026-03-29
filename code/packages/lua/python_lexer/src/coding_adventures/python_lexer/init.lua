-- python_lexer — Tokenizes Python source using the grammar-driven infrastructure
-- ================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `python.tokens` grammar file to configure the tokenizer.
--
-- # What is Python tokenization?
--
-- Given the input:  x = 42
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(NAME,    "x",  1:1)
--   Token(EQUALS,  "=",  1:3)
--   Token(NUMBER,  "42", 1:5)
--   Token(EOF,     "",   1:7)
--
-- Whitespace between tokens is silently consumed (declared as skip patterns
-- in `python.tokens`). The parser never sees ordinary whitespace.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `python.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/python_lexer/src/coding_adventures/python_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/python.tokens`.
--
-- Directory structure from script_dir upward:
--   python_lexer/    (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   python_lexer/        (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/python.tokens
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   NUMBER  — integer literals (e.g. 42, 0)
--   STRING  — double-quoted string literals
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   IF, ELSE, ELIF, WHILE, FOR, DEF, RETURN, CLASS, IMPORT, FROM,
--   AS, TRUE, FALSE, NONE
--
-- Operators and delimiters:
--   EQUALS_EQUALS, EQUALS,
--   PLUS, MINUS, STAR, SLASH,
--   LPAREN, RPAREN, COMMA, COLON

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (without trailing slash).
-- For example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string The full file path.
-- @return string     The directory portion.
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua embeds the source path in the chunk debug info with a leading "@".
-- We strip that prefix to get the real filesystem path.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- If the path is relative, resolve it against CWD so that dirname
    -- navigation works correctly when tests run from a subdirectory
    -- (e.g., busted adds ../src to package.path, making paths relative).
    if src:sub(1, 1) ~= "/" and src:sub(2, 2) ~= ":"           then
        local f = io.popen("pwd")
        local cwd = f and f:read("*l") or "."
        if f then f:close() end
        src = cwd .. "/" .. src
    end
    return dirname(src)
end

--- Walk up `levels` directory levels from `path`.
-- Each call to this function strips one path component.
-- For example: up("/a/b/c", 2) → "/a"
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string        Resulting directory.
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
-- The grammar is read from disk exactly once and cached in a module-level
-- variable. Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = nil

--- Load and parse the `python.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Python token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/python_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/python_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/python.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "python_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("python_lexer: failed to parse python.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Python source string.
--
-- Loads the `python.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `python.tokens`.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source string  The Python text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local py_lexer = require("coding_adventures.python_lexer")
--   local tokens = py_lexer.tokenize("def foo(x):")
--   -- tokens[1].type  → "DEF"
--   -- tokens[1].value → "def"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "foo"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    return gl:tokenize()
end

--- Return the cached (or freshly loaded) TokenGrammar for Python.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Python token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
