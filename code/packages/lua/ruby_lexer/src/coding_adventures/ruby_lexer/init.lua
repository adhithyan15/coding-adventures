-- ruby_lexer — Tokenizes Ruby source using the grammar-driven infrastructure
-- ==============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `ruby.tokens` grammar file to configure the tokenizer.
--
-- # What is Ruby tokenization?
--
-- Given the input:  def greet(name)
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(DEF,    "def",   1:1)
--   Token(NAME,   "greet", 1:5)
--   Token(LPAREN, "(",     1:10)
--   Token(NAME,   "name",  1:11)
--   Token(RPAREN, ")",     1:15)
--   Token(EOF,    "",      1:16)
--
-- Whitespace is silently consumed (declared as skip patterns in
-- `ruby.tokens`). The parser never sees ordinary whitespace.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `ruby.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/ruby_lexer/src/coding_adventures/ruby_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/ruby.tokens`.
--
-- Directory structure from script_dir upward:
--   ruby_lexer/          (1) — module dir
--   coding_adventures/   (2)
--   src/                 (3)
--   ruby_lexer/          (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/ruby.tokens
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   NUMBER  — integer literals (e.g. 42, 0)
--   STRING  — double-quoted string literals
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   IF, ELSE, ELSIF, END, WHILE, FOR, DO, DEF, RETURN, CLASS, MODULE,
--   REQUIRE, PUTS, TRUE, FALSE, NIL, AND, OR, NOT, THEN, UNLESS, UNTIL,
--   YIELD, BEGIN, RESCUE, ENSURE
--
-- Multi-char operators (must match before single-char versions):
--   EQUALS_EQUALS, DOT_DOT, HASH_ROCKET, NOT_EQUALS,
--   LESS_EQUALS, GREATER_EQUALS
--
-- Single-char operators and delimiters:
--   EQUALS, PLUS, MINUS, STAR, SLASH,
--   LESS_THAN, GREATER_THAN,
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
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    -- Extract the directory portion of the source path (may be relative
    -- and may contain .. when busted uses ../src in package.path).
    local dir = src:match("(.+)/[^/]+$") or "."
    -- Resolve to an absolute normalised path. Using 'cd dir && pwd' correctly
    -- resolves any .. components -- unlike string-based dirname traversal.
    -- Skip on Windows drive paths (C:\...) and fall back to the raw string.
    -- Security: Do not attempt shell-based path resolution via io.popen.
    -- Passing unsanitised directory strings to a shell command introduces
    -- OS command injection risk (path could contain single-quotes or shell
    -- metacharacters). String-based path arithmetic in up_n_levels works
    -- correctly for both absolute and relative source paths.
    -- Fixed: 2026-04-10 security review.
    return dir
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

--- Load and parse the `ruby.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Ruby token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/ruby_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/ruby_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/ruby.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "ruby_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("ruby_lexer: failed to parse ruby.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Ruby source string.
--
-- Loads the `ruby.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `ruby.tokens`.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source string  The Ruby text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local rb_lexer = require("coding_adventures.ruby_lexer")
--   local tokens = rb_lexer.tokenize("def greet(name)")
--   -- tokens[1].type  → "DEF"
--   -- tokens[1].value → "def"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "greet"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        if tok.type_name ~= "NEWLINE" then
            tokens[#tokens + 1] = {
                type  = tok.type_name,
                value = tok.value,
                line  = tok.line,
                col   = tok.column,
            }
        end
    end
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for Ruby.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Ruby token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
