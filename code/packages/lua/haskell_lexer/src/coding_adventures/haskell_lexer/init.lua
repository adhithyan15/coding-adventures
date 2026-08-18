-- haskell_lexer — Tokenizes Haskell source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar to configure the lexer.
--
-- # What is Haskell tokenization?
--
-- Given the input:  int x = 42;
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INT,        "int",  1:1)
--   Token(NAME,       "x",    1:5)
--   Token(EQUALS,     "=",    1:7)
--   Token(NUMBER,     "42",   1:9)
--   Token(SEMICOLON,  ";",    1:11)
--   Token(EOF,        "",     1:12)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in the grammar). The parser never sees them.
--
-- # Architecture
--
-- This module:
--   1. Selects the correct *precompiled* grammar module for the requested
--      version (see "Why precompiled grammars?" below).
--   2. Builds (and caches) a `TokenGrammar` from it via `token_grammar()`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Why precompiled grammars?
--
-- Earlier versions of this module read `code/grammars/haskell/haskell<version>.tokens`
-- off disk at runtime, using a path that walked outside this package's own
-- directory into the monorepo. That works when running from a checkout of
-- the monorepo, but a published LuaRocks package does not include
-- `code/grammars/` — installing this rock and calling `tokenize` would raise
-- a file-not-found error.
--
-- Instead, each supported Haskell version's grammar is compiled ahead of
-- time (via `grammar-tools compile-tokens`) into a `_grammar_<version>.lua`
-- sibling module that embeds the parsed `TokenGrammar` as native Lua data.
-- Those modules ship with this package like any other source file, so the
-- rock is self-contained.
--
-- # Version routing
--
-- When `version` is nil or "" → uses the Haskell 2010 grammar (default,
--   the Haskell 2010 Report, the latest standardized revision)
-- When `version` is "1.0"    → uses the Haskell 1.0 grammar.
-- When `version` is "98"     → uses the Haskell 98 grammar.
-- ... etc.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid Haskell versions
-- =========================================================================
--
-- Haskell is versioned by report/release. The version strings we accept
-- correspond to the precompiled grammar modules below:
--
--   "1.0"  — Haskell 1.0  (1990): the original report.
--   "1.1"  — Haskell 1.1  (1991): first revision.
--   "1.2"  — Haskell 1.2  (1992): second revision.
--   "1.3"  — Haskell 1.3  (1996): monadic I/O, strictness annotations.
--   "1.4"  — Haskell 1.4  (1997): newtype, standard libraries reorganized.
--   "98"   — Haskell 98   (1998): the stable, standardized language report.
--   "2010" — Haskell 2010 (2010): FFI, hierarchical modules, language pragmas.
--   nil / "" — defaults to Haskell 2010 (latest standardized revision).

local VALID_HASKELL_VERSIONS = {
    ["1.0"] = true, ["1.1"] = true, ["1.2"] = true,
    ["1.3"] = true, ["1.4"] = true, ["98"] = true,
    ["2010"] = true,
}

local DEFAULT_VERSION = "2010"

-- =========================================================================
-- Precompiled grammars
-- =========================================================================
--
-- Each entry maps a version string to the `require`d module produced by
-- `grammar-tools compile-tokens code/grammars/haskell/haskell<version>.tokens`.
-- The module exposes a `token_grammar()` constructor; we call it lazily
-- (and cache the result) so grammar construction only happens for versions
-- actually used.

local compiled_grammars = {
    ["1.0"]  = require("coding_adventures.haskell_lexer._grammar_1_0"),
    ["1.1"]  = require("coding_adventures.haskell_lexer._grammar_1_1"),
    ["1.2"]  = require("coding_adventures.haskell_lexer._grammar_1_2"),
    ["1.3"]  = require("coding_adventures.haskell_lexer._grammar_1_3"),
    ["1.4"]  = require("coding_adventures.haskell_lexer._grammar_1_4"),
    ["98"]   = require("coding_adventures.haskell_lexer._grammar_98"),
    ["2010"] = require("coding_adventures.haskell_lexer._grammar_2010"),
}

local _grammar_cache = {}

--- Resolve a version string (or nil/"") to a valid, known version.
--
-- @param version string|nil  The Haskell version tag, or nil/empty for default (2010).
-- @return string             A key present in VALID_HASKELL_VERSIONS.
local function resolve_version(version)
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_HASKELL_VERSIONS[version] then
        error(
            "haskell_lexer: unknown Haskell version '" .. version .. "'. " ..
            "Valid values are: 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21, or nil/\"\" for default (21)."
        )
    end

    return version
end

--- Build (or return the cached) `TokenGrammar` for a specific version.
--
-- @param version string|nil  The Haskell version tag (see resolve_version).
-- @return TokenGrammar       The Haskell token grammar.
local function get_grammar(version)
    local key = resolve_version(version)
    if _grammar_cache[key] == nil then
        _grammar_cache[key] = compiled_grammars[key].token_grammar()
    end
    return _grammar_cache[key]
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Haskell source string.
--
-- @param source  string       The Haskell text to tokenize.
-- @param version string|nil   Haskell version: "1.0", "1.1", "1.2", "1.3",
--                             "1.4", "98", "2010", or nil/"" for default
--                             (2010).
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (default):
--
--   local haskell_lexer = require("coding_adventures.haskell_lexer")
--   local tokens = haskell_lexer.tokenize("int x = 1;")
--
-- Example (versioned):
--
--   local tokens = haskell_lexer.tokenize("int x = 1;", "98")
function M.tokenize(source, version)
    local grammar = get_grammar(version)
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

--- Create a GrammarLexer for a Haskell source string without tokenizing yet.
--
-- @param source  string       The Haskell text to lex.
-- @param version string|nil   Haskell version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly built) TokenGrammar for Haskell.
--
-- @param version string|nil  Haskell version tag (see tokenize for valid values).
-- @return TokenGrammar       The Haskell token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
