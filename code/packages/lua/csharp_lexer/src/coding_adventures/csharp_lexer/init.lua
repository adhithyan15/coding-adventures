-- csharp_lexer — Tokenizes C# source using the grammar-driven infrastructure
-- ============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar to configure the tokenizer.
--
-- # What is C# tokenization?
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
--      version (see "Version routing" below).
--   2. Builds (and caches) a `TokenGrammar` from it via `token_grammar()`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Why precompiled grammars?
--
-- Earlier versions of this module read `code/grammars/csharp/csharp<version>.tokens`
-- off disk at runtime, using a path that walked outside this package's own
-- directory into the monorepo. That works when running from a checkout of
-- the monorepo, but a published LuaRocks package does not include
-- `code/grammars/` — installing this rock and calling `tokenize_csharp` would
-- raise a file-not-found error.
--
-- Instead, each supported C# version's grammar is compiled ahead of time (via
-- `grammar-tools compile-tokens`) into a `_grammar_<version>.lua` sibling
-- module that embeds the parsed `TokenGrammar` as native Lua data. Those
-- modules ship with this package like any other source file, so the rock is
-- self-contained.
--
-- # Version routing
--
-- When `version` is nil or "" → uses the C# 12.0 grammar (default, latest
--   stable release).
-- When `version` is "1.0"    → uses the C# 1.0 grammar.
-- When `version` is "8.0"    → uses the C# 8.0 grammar.
-- ... etc.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Valid C# versions
-- =========================================================================
--
-- C# is versioned by language release. The version strings we accept
-- correspond to the precompiled grammar modules below:
--
--   "1.0"  — C# 1.0  (2002): the original .NET 1.0 release. Classes,
--             interfaces, structs, delegates, events, enums, basic OOP.
--   "2.0"  — C# 2.0  (2005): generics, nullable types, anonymous methods,
--             iterators (yield), partial types.
--   "3.0"  — C# 3.0  (2007): LINQ, lambda expressions, extension methods,
--             anonymous types, var keyword, auto-properties.
--   "4.0"  — C# 4.0  (2010): dynamic typing, named/optional arguments,
--             generic covariance and contravariance.
--   "5.0"  — C# 5.0  (2012): async/await, caller info attributes.
--   "6.0"  — C# 6.0  (2015): string interpolation, null-conditional operators,
--             expression-bodied members, nameof, using static.
--   "7.0"  — C# 7.0  (2017): tuples, pattern matching (is), local functions,
--             out variables, ref returns, discards (_).
--   "8.0"  — C# 8.0  (2019): nullable reference types, switch expressions,
--             default interface members, ranges and indices (..  ^).
--   "9.0"  — C# 9.0  (2020): records, init-only setters, top-level statements,
--             pattern matching improvements, nint/nuint native types.
--   "10.0" — C# 10.0 (2021): record structs, global using, file-scoped
--             namespace, extended property patterns.
--   "11.0" — C# 11.0 (2022): raw string literals, list patterns, required
--             members, generic attributes, file-local types.
--   "12.0" — C# 12.0 (2023): primary constructors on classes/structs,
--             collection expressions, inline arrays, alias any type.
--   nil / "" — defaults to C# 12.0 (latest stable).

local VALID_CSHARP_VERSIONS = {
    ["1.0"]  = true, ["2.0"]  = true, ["3.0"]  = true,
    ["4.0"]  = true, ["5.0"]  = true, ["6.0"]  = true,
    ["7.0"]  = true, ["8.0"]  = true, ["9.0"]  = true,
    ["10.0"] = true, ["11.0"] = true, ["12.0"] = true,
}

local DEFAULT_VERSION = "12.0"

-- =========================================================================
-- Precompiled grammars
-- =========================================================================
--
-- Each entry maps a version string to the `require`d module produced by
-- `grammar-tools compile-tokens code/grammars/csharp/csharp<version>.tokens`.
-- The module exposes a `token_grammar()` constructor; we call it lazily (and
-- cache the result) so grammar construction only happens for versions
-- actually used.

local compiled_grammars = {
    ["1.0"]  = require("coding_adventures.csharp_lexer._grammar_1_0"),
    ["2.0"]  = require("coding_adventures.csharp_lexer._grammar_2_0"),
    ["3.0"]  = require("coding_adventures.csharp_lexer._grammar_3_0"),
    ["4.0"]  = require("coding_adventures.csharp_lexer._grammar_4_0"),
    ["5.0"]  = require("coding_adventures.csharp_lexer._grammar_5_0"),
    ["6.0"]  = require("coding_adventures.csharp_lexer._grammar_6_0"),
    ["7.0"]  = require("coding_adventures.csharp_lexer._grammar_7_0"),
    ["8.0"]  = require("coding_adventures.csharp_lexer._grammar_8_0"),
    ["9.0"]  = require("coding_adventures.csharp_lexer._grammar_9_0"),
    ["10.0"] = require("coding_adventures.csharp_lexer._grammar_10_0"),
    ["11.0"] = require("coding_adventures.csharp_lexer._grammar_11_0"),
    ["12.0"] = require("coding_adventures.csharp_lexer._grammar_12_0"),
}

local _grammar_cache = {}

--- Resolve a version string (or nil/"") to a valid, known version.
--
-- @param version string|nil  The C# version tag, or nil/empty for default (12.0).
-- @return string             A key present in VALID_CSHARP_VERSIONS.
local function resolve_version(version)
    if not version or version == "" then
        version = DEFAULT_VERSION
    end

    if not VALID_CSHARP_VERSIONS[version] then
        error(
            "csharp_lexer: unknown C# version '" .. version .. "'. " ..
            "Valid values are: 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, " ..
            "9.0, 10.0, 11.0, 12.0, or nil/\"\" for default (12.0)."
        )
    end

    return version
end

--- Build (or return the cached) `TokenGrammar` for a specific version.
--
-- Caching matters: for programs that tokenize many C# snippets, we don't
-- want to reconstruct the grammar's Lua tables on every call.
--
-- @param version string|nil  The C# version tag (see resolve_version).
-- @return TokenGrammar       The C# token grammar.
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

--- Tokenize a C# source string.
--
-- @param source  string       The C# text to tokenize.
-- @param version string|nil   C# version: "1.0", "2.0", "3.0", "4.0", "5.0",
--                             "6.0", "7.0", "8.0", "9.0", "10.0", "11.0",
--                             "12.0", or nil/"" for default (12.0).
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (default):
--
--   local csharp_lexer = require("coding_adventures.csharp_lexer")
--   local tokens = csharp_lexer.tokenize_csharp("int x = 1;")
--
-- Example (versioned):
--
--   local tokens = csharp_lexer.tokenize_csharp("int x = 1;", "8.0")
function M.tokenize_csharp(source, version)
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

--- Create a GrammarLexer for a C# source string without tokenizing yet.
--
-- This is useful when you need the lexer object directly — for example,
-- to configure it, to stream tokens lazily, or to pass it to another tool.
--
-- @param source  string       The C# text to lex.
-- @param version string|nil   C# version tag (see tokenize_csharp for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
function M.create_csharp_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly built) TokenGrammar for C#.
--
-- Useful for inspecting what tokens the grammar defines, or for passing
-- the grammar to other infrastructure components.
--
-- @param version string|nil  C# version tag (see tokenize_csharp for valid values).
-- @return TokenGrammar       The C# token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
