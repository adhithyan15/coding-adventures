-- java_lexer — Tokenizes Java source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar to configure the tokenizer.
--
-- # What is Java tokenization?
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
-- Earlier versions of this module read `code/grammars/java/java<version>.tokens`
-- off disk at runtime, using a path that walked outside this package's own
-- directory into the monorepo. That works when running from a checkout of
-- the monorepo, but a published LuaRocks package does not include
-- `code/grammars/` — installing this rock and calling `tokenize` would raise
-- a file-not-found error.
--
-- Instead, each supported Java version's grammar is compiled ahead of time
-- (via `grammar-tools compile-tokens`) into a `_grammar_<version>.lua`
-- sibling module that embeds the parsed `TokenGrammar` as native Lua data.
-- Those modules ship with this package like any other source file, so the
-- rock is self-contained.
--
-- # Version routing
--
-- When `version` is nil or "" → uses the Java 21 grammar (default, latest LTS)
-- When `version` is "1.0"    → uses the Java 1.0 grammar.
-- When `version` is "8"      → uses the Java 8 grammar.
-- ... etc.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid Java versions
-- =========================================================================
--
-- Java is versioned by release number. The version strings we accept
-- correspond to the precompiled grammar modules below:
--
--   "1.0" — Java 1.0  (1996): the original release.
--   "1.1" — Java 1.1  (1997): inner classes.
--   "1.4" — Java 1.4  (2002): assert keyword.
--   "5"   — Java 5    (2004): generics, annotations, enums.
--   "7"   — Java 7    (2011): try-with-resources, diamond operator.
--   "8"   — Java 8    (2014): lambdas, default methods, streams.
--   "10"  — Java 10   (2018): local variable type inference (var).
--   "14"  — Java 14   (2020): switch expressions, records (preview).
--   "17"  — Java 17   (2021): sealed classes, pattern matching.
--   "21"  — Java 21   (2023): virtual threads, record patterns.
--   nil / "" — defaults to Java 21 (latest LTS).

local VALID_JAVA_VERSIONS = {
    ["1.0"] = true, ["1.1"] = true, ["1.4"] = true,
    ["5"]   = true, ["7"]   = true, ["8"]   = true,
    ["10"]  = true, ["14"]  = true, ["17"]  = true,
    ["21"]  = true,
}

local DEFAULT_VERSION = "21"

-- =========================================================================
-- Precompiled grammars
-- =========================================================================
--
-- Each entry maps a version string to the `require`d module produced by
-- `grammar-tools compile-tokens code/grammars/java/java<version>.tokens`.
-- The module exposes a `token_grammar()` constructor; we call it lazily
-- (and cache the result) so grammar construction only happens for versions
-- actually used.

local compiled_grammars = {
    ["1.0"] = require("coding_adventures.java_lexer._grammar_1_0"),
    ["1.1"] = require("coding_adventures.java_lexer._grammar_1_1"),
    ["1.4"] = require("coding_adventures.java_lexer._grammar_1_4"),
    ["5"]   = require("coding_adventures.java_lexer._grammar_5"),
    ["7"]   = require("coding_adventures.java_lexer._grammar_7"),
    ["8"]   = require("coding_adventures.java_lexer._grammar_8"),
    ["10"]  = require("coding_adventures.java_lexer._grammar_10"),
    ["14"]  = require("coding_adventures.java_lexer._grammar_14"),
    ["17"]  = require("coding_adventures.java_lexer._grammar_17"),
    ["21"]  = require("coding_adventures.java_lexer._grammar_21"),
}

local _grammar_cache = {}

--- Resolve a version string (or nil/"") to a valid, known version.
--
-- @param version string|nil  The Java version tag, or nil/empty for default (21).
-- @return string             A key present in VALID_JAVA_VERSIONS.
local function resolve_version(version)
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_JAVA_VERSIONS[version] then
        error(
            "java_lexer: unknown Java version '" .. version .. "'. " ..
            "Valid values are: 1.0, 1.1, 1.4, 5, 7, 8, 10, 14, 17, 21, or nil/\"\" for default (21)."
        )
    end

    return version
end

--- Build (or return the cached) `TokenGrammar` for a specific version.
--
-- @param version string|nil  The Java version tag (see resolve_version).
-- @return TokenGrammar       The Java token grammar.
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

--- Tokenize a Java source string.
--
-- @param source  string       The Java text to tokenize.
-- @param version string|nil   Java version: "1.0", "1.1", "1.4", "5", "7",
--                             "8", "10", "14", "17", "21", or nil/"" for
--                             default (21).
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (default):
--
--   local java_lexer = require("coding_adventures.java_lexer")
--   local tokens = java_lexer.tokenize("int x = 1;")
--
-- Example (versioned):
--
--   local tokens = java_lexer.tokenize("int x = 1;", "1.0")
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

--- Create a GrammarLexer for a Java source string without tokenizing yet.
--
-- @param source  string       The Java text to lex.
-- @param version string|nil   Java version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly built) TokenGrammar for Java.
--
-- @param version string|nil  Java version tag (see tokenize for valid values).
-- @return TokenGrammar       The Java token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
