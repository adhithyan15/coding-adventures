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

-- DefaultVersion is the Python version used when no version is specified.
M.DEFAULT_VERSION = "3.12"

-- SupportedVersions lists all Python versions with grammar files.
M.SUPPORTED_VERSIONS = {"2.7", "3.0", "3.6", "3.8", "3.10", "3.12"}

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

-- Per-version grammar cache. Keys are version strings (e.g., "3.12"),
-- values are parsed TokenGrammar objects. Once a grammar is parsed for
-- a given version, it is reused for all subsequent calls.
local _grammar_cache = {}

--- Resolve the version string. If nil or empty, returns DEFAULT_VERSION.
-- @param version string|nil  The version string to resolve.
-- @return string             The resolved version string.
local function resolve_version(version)
    if not version or version == "" then
        return M.DEFAULT_VERSION
    end
    return version
end

--- Return the path to the versioned grammar file.
-- @param version string  The Python version (e.g., "3.12").
-- @return string         Absolute path to the .tokens file.
local function grammar_path(version)
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    return repo_root .. "/grammars/python/python" .. version .. ".tokens"
end

--- Load and parse a versioned Python grammar, with per-version caching.
-- On the first call for a given version, opens and parses the file.
-- On subsequent calls, returns the cached TokenGrammar object.
-- @param version string|nil  Python version (e.g., "3.12"). Defaults to DEFAULT_VERSION.
-- @return TokenGrammar  The parsed Python token grammar.
local function get_grammar(version)
    local v = resolve_version(version)

    if _grammar_cache[v] then
        return _grammar_cache[v]
    end

    local tokens_path = grammar_path(v)

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
        error("python_lexer: failed to parse python" .. v .. ".tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache[v] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Python source string using a versioned grammar.
--
-- Loads the grammar for the given Python version (cached after first call)
-- and feeds the source to a `GrammarLexer`. Returns the complete flat
-- token list, including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in the grammar.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source  string      The Python text to tokenize.
-- @param version string|nil  Python version (e.g., "3.12"). Defaults to DEFAULT_VERSION.
-- @return table              Array of Token objects (type, value, line, col).
-- @error                     Raises an error on unexpected characters.
--
-- Example:
--
--   local py_lexer = require("coding_adventures.python_lexer")
--   local tokens = py_lexer.tokenize("def foo(x):", "3.12")
--   local tokens = py_lexer.tokenize("def foo(x):")  -- defaults to 3.12
function M.tokenize(source, version)
    local grammar = get_grammar(version)
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

--- Return the cached (or freshly loaded) TokenGrammar for a Python version.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @param version string|nil  Python version (e.g., "3.12"). Defaults to DEFAULT_VERSION.
-- @return TokenGrammar  The parsed Python token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
