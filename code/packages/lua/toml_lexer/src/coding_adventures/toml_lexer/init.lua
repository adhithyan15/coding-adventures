-- toml_lexer -- Tokenizes TOML text using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `toml.tokens` grammar file to configure the tokenizer.
--
-- # What is TOML tokenization?
--
-- TOML (Tom's Obvious, Minimal Language) is a configuration file format
-- designed to be easy to read. Given the input:
--
--   [server]
--   host = "localhost"
--   port = 8080
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(LBRACKET,   "[",           1:1)
--   Token(BARE_KEY,   "server",      1:2)
--   Token(RBRACKET,   "]",           1:8)
--   Token(NEWLINE,    "\n",          1:9)  ← TOML is newline-sensitive
--   Token(BARE_KEY,   "host",        2:1)
--   Token(EQUALS,     "=",           2:6)
--   Token(BASIC_STRING, '"localhost"', 2:8)
--   Token(NEWLINE,    "\n",          2:19)
--   Token(BARE_KEY,   "port",        3:1)
--   Token(EQUALS,     "=",           3:6)
--   Token(INTEGER,    "8080",        3:8)
--   Token(EOF,        "",            4:1)
--
-- # TOML-specific lexer concerns
--
-- **Newlines are significant** — Unlike JSON or SQL, TOML key-value pairs
-- are terminated by newlines. The `toml.tokens` grammar therefore skips only
-- horizontal whitespace (spaces and tabs). Newlines are emitted as NEWLINE
-- tokens so that a parser can use them as statement terminators.
--
-- **Ordering matters** — `toml.tokens` places more-specific patterns before
-- less-specific ones. For example:
--   - Multi-line strings (""" and ''') must come before single-line strings
--   - Date/time patterns (1979-05-27) must come before BARE_KEY and INTEGER
--   - Floats must come before integers (3.14 would match INTEGER(3) DOT otherwise)
--   - Boolean literals (true/false) must come before BARE_KEY
--
-- **FLOAT alias** — FLOAT_SPECIAL, FLOAT_EXP, and FLOAT_DEC all emit as FLOAT.
-- **INTEGER alias** — HEX_INTEGER, OCT_INTEGER, BIN_INTEGER all emit as INTEGER.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `toml.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/toml_lexer/src/coding_adventures/toml_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path to this file.
-- We strip the leading `@` Lua adds to source paths, then walk up 6
-- directory levels to reach the repo root (`code/`), then descend into
-- `grammars/toml.tokens`.
--
-- Directory structure from script_dir upward:
--   toml_lexer/          (1) — coding_adventures/toml_lexer/
--   coding_adventures/   (2)
--   src/                 (3)
--   toml_lexer/          (4) — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/toml.tokens

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

--- Load and parse the `toml.tokens` grammar, with caching.
-- On the first call, opens and parses the file. On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed TOML token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/toml_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/toml_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/toml.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "toml_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("toml_lexer: failed to parse toml.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a TOML source string.
--
-- Loads the `toml.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- TOML-specific token types produced:
--
--   BASIC_STRING, ML_BASIC_STRING  — double-quoted strings
--   LITERAL_STRING, ML_LITERAL_STRING — single-quoted strings (no escapes)
--   INTEGER                        — decimal, hex (0x), octal (0o), binary (0b)
--   FLOAT                          — decimal, scientific, inf, nan
--   TRUE, FALSE                    — boolean literals
--   OFFSET_DATETIME, LOCAL_DATETIME, LOCAL_DATE, LOCAL_TIME — date/time values
--   BARE_KEY                       — unquoted key names (letters, digits, -, _)
--   EQUALS, DOT, COMMA             — structural punctuation
--   LBRACKET, RBRACKET             — [ ] (tables and arrays)
--   LBRACE, RBRACE                 — { } (inline tables)
--
-- Horizontal whitespace (spaces and tabs) and TOML comments (#...) are
-- consumed silently via the skip patterns in `toml.tokens`. Newlines
-- are NOT skipped — they appear as NEWLINE tokens when present.
--
-- @param source string  The TOML text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local toml_lexer = require("coding_adventures.toml_lexer")
--   local tokens = toml_lexer.tokenize('key = "value"')
--   -- tokens[1].type  → "BARE_KEY"
--   -- tokens[1].value → "key"
--   -- tokens[2].type  → "EQUALS"
--   -- tokens[3].type  → "BASIC_STRING"
--   -- tokens[3].value → '"value"'
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        tokens[#tokens + 1] = {
            type  = tok.type_name,
            value = tok.value,
            line  = tok.line,
            col   = tok.column,
        }
    end
    return tokens
end

--- Return the cached (or freshly loaded) TokenGrammar for TOML.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed TOML token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
