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
--   1. Selects the pre-compiled grammar module for the requested version.
--   2. Constructs it once per version (cached) via its `token_grammar()`
--      function — see "Compiled grammars" below.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
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

local lexer_pkg = require("coding_adventures.lexer")

-- =========================================================================
-- Compiled grammars
-- =========================================================================
--
-- Historically this module read `.tokens` grammar files from `code/grammars/`
-- at runtime via `io.open`, walking outside this package's own directory
-- into the monorepo. That works when running inside the monorepo checkout,
-- but a published LuaRocks package does not ship `code/grammars/`, so
-- `luarocks install` + first use would raise a file-not-found error.
--
-- Instead, each grammar is now pre-compiled to a native Lua data structure
-- (via `code/programs/lua/grammar-tools`) and checked in as a sibling
-- `_grammar_<version>.lua` file, `require`d like any other module. No
-- runtime file I/O, no path traversal outside the package.
--
-- Python version strings contain dots ("2.7", "3.12"), which are not
-- valid in Lua module names, so the filenames/module names use
-- underscores in place of dots while the lookup KEY below stays exactly
-- "2.7", "3.12", etc. to match the public API.

local compiled_grammars = {
    ["2.7"]  = require("coding_adventures.python_lexer._grammar_2_7"),
    ["3.0"]  = require("coding_adventures.python_lexer._grammar_3_0"),
    ["3.6"]  = require("coding_adventures.python_lexer._grammar_3_6"),
    ["3.8"]  = require("coding_adventures.python_lexer._grammar_3_8"),
    ["3.10"] = require("coding_adventures.python_lexer._grammar_3_10"),
    ["3.12"] = require("coding_adventures.python_lexer._grammar_3_12"),
}

local M = {}
M.VERSION = "0.1.0"

-- DefaultVersion is the Python version used when no version is specified.
M.DEFAULT_VERSION = "3.12"

-- SupportedVersions lists all Python versions with grammar files.
M.SUPPORTED_VERSIONS = {"2.7", "3.0", "3.6", "3.8", "3.10", "3.12"}

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- Grammars are constructed once per version and cached in a module-level
-- table. Subsequent calls to `tokenize` reuse the cached grammar.

-- Per-version grammar cache. Keys are version strings (e.g., "3.12"),
-- values are parsed TokenGrammar objects. Once a grammar is constructed for
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

--- Load and parse a versioned Python grammar, with per-version caching.
-- On the first call for a given version, invokes the compiled module's
-- `token_grammar()` constructor. On subsequent calls, returns the cached
-- TokenGrammar object.
-- @param version string|nil  Python version (e.g., "3.12"). Defaults to DEFAULT_VERSION.
-- @return TokenGrammar  The parsed Python token grammar.
local function get_grammar(version)
    local v = resolve_version(version)

    if _grammar_cache[v] then
        return _grammar_cache[v]
    end

    local module = compiled_grammars[v]
    if not module then
        error(
            "python_lexer: unknown Python version '" .. tostring(v) .. "'. " ..
            "Valid values are: 2.7, 3.0, 3.6, 3.8, 3.10, 3.12."
        )
    end

    local grammar = module.token_grammar()
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
