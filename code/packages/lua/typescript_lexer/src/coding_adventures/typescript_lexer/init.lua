-- typescript_lexer — Tokenizes TypeScript source using the grammar-driven infrastructure
-- =======================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `typescript.tokens` grammar file to configure the tokenizer.
--
-- # What is TypeScript tokenization?
--
-- TypeScript is a strict superset of JavaScript. Every valid JavaScript
-- program is also valid TypeScript. TypeScript adds:
--   - Type annotations: `let x: number = 1`
--   - Interfaces: `interface Foo { bar: string }`
--   - Generics: `Array<number>`
--   - Access modifiers: `public`, `private`, `protected`
--   - `enum`, `type`, `namespace`, `declare`, `readonly`
--   - Abstract classes, `implements`, `extends`
--   - Type utilities: `keyof`, `infer`, `never`, `unknown`
--   - Primitive type keywords: `any`, `void`, `number`, `string`,
--     `boolean`, `object`, `symbol`, `bigint`
--
-- Given the input:  interface Foo { bar: number; }
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INTERFACE, "interface", 1:1)
--   Token(NAME,      "Foo",       1:11)
--   Token(LBRACE,    "{",         1:15)
--   Token(NAME,      "bar",       1:17)
--   Token(COLON,     ":",         1:20)
--   Token(NUMBER_KW, "number",    1:22)   -- keyword, not a number literal
--   Token(SEMICOLON, ";",         1:28)
--   Token(RBRACE,    "}",         1:30)
--   Token(EOF,       "",          1:31)
--
-- Whitespace is silently consumed (declared as skip patterns in
-- `typescript.tokens`). The parser never sees whitespace tokens.
--
-- # Architecture
--
-- This module:
--   1. Selects the pre-compiled grammar module for the requested version.
--   2. Constructs it once per version (cached) via its `token_grammar()`
--      function — see "Compiled grammars" below.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.

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
-- TypeScript version strings contain dots ("ts1.0", "ts5.8"), which are
-- not valid in Lua module names, so the filenames/module names use
-- underscores in place of dots (e.g. "ts1.0" -> _grammar_ts1_0.lua) while
-- the lookup KEY below stays exactly "ts1.0" to match the public API.

local compiled_grammars = {
    [""]      = require("coding_adventures.typescript_lexer._grammar_default"),
    ["ts1.0"] = require("coding_adventures.typescript_lexer._grammar_ts1_0"),
    ["ts2.0"] = require("coding_adventures.typescript_lexer._grammar_ts2_0"),
    ["ts3.0"] = require("coding_adventures.typescript_lexer._grammar_ts3_0"),
    ["ts4.0"] = require("coding_adventures.typescript_lexer._grammar_ts4_0"),
    ["ts5.0"] = require("coding_adventures.typescript_lexer._grammar_ts5_0"),
    ["ts5.8"] = require("coding_adventures.typescript_lexer._grammar_ts5_8"),
}

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid TypeScript versions
-- =========================================================================
--
-- TypeScript has had several major releases. The canonical version strings
-- we accept are:
--
--   "ts1.0"  — TypeScript 1.0 (April 2014): initial public release.
--   "ts2.0"  — TypeScript 2.0 (September 2016): non-nullable types.
--   "ts3.0"  — TypeScript 3.0 (July 2018): project references, tuples.
--   "ts4.0"  — TypeScript 4.0 (August 2020): variadic tuple types.
--   "ts5.0"  — TypeScript 5.0 (March 2023): decorators (Stage 3).
--   "ts5.8"  — TypeScript 5.8 (February 2025): granular control-flow.
--   nil / "" — Generic TypeScript (uses the latest stable grammar).
--
-- Each version maps to a grammar file under:
--   code/grammars/typescript/<version>.tokens

local VALID_TS_VERSIONS = {
    ["ts1.0"] = true,
    ["ts2.0"] = true,
    ["ts3.0"] = true,
    ["ts4.0"] = true,
    ["ts5.0"] = true,
    ["ts5.8"] = true,
}

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- Grammars are constructed once per version and cached in a module-level
-- table. Subsequent calls to `tokenize` reuse the cached grammar.

-- Cache keyed by version string (or "" for generic).
local _grammar_cache = {}

--- Resolve a version string to its entry in `compiled_grammars`.
--
-- Version routing logic:
--   - nil or ""  →  generic (unified) grammar
--   - "ts1.0"    →  TypeScript 1.0 grammar
--   - "ts2.0"    →  TypeScript 2.0 grammar
--   - ...etc.
--
-- If an unrecognized version string is passed we raise an error immediately
-- rather than silently falling back, which would hide bugs in callers.
--
-- @param version string|nil  The TypeScript version tag, or nil/empty for generic.
-- @return string             The resolved lookup key into `compiled_grammars`.
local function resolve_version(version)
    -- Generic (no version specified) — use the unified grammar.
    if not version or version == "" then
        return ""
    end

    -- Validate the version string before looking it up.
    if not VALID_TS_VERSIONS[version] then
        error(
            "typescript_lexer: unknown TypeScript version '" .. version .. "'. " ..
            "Valid values are: ts1.0, ts2.0, ts3.0, ts4.0, ts5.0, ts5.8, or nil/\"\" for generic."
        )
    end

    return version
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- On the first call for a given version, invokes the compiled module's
-- `token_grammar()` constructor and stores the result in `_grammar_cache`.
-- On subsequent calls for the same version, returns the cached object
-- immediately.
--
-- @param version string|nil  The TypeScript version tag (see resolve_version).
-- @return TokenGrammar       The parsed TypeScript token grammar.
local function get_grammar(version)
    local key = resolve_version(version)
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local grammar = compiled_grammars[key].token_grammar()
    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a TypeScript source string.
--
-- Loads the grammar for the requested TypeScript version (cached after first
-- call) and feeds the source to a `GrammarLexer`. Returns the complete flat
-- token list, including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in the grammar file.
-- The caller receives only meaningful tokens.
--
-- TypeScript is a superset of JavaScript, so all JavaScript tokens are
-- recognized plus TypeScript-specific keywords: INTERFACE, TYPE, ENUM,
-- NAMESPACE, DECLARE, READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT,
-- IMPLEMENTS, EXTENDS, KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID,
-- NUMBER (keyword), STRING (keyword), BOOLEAN, OBJECT, SYMBOL, BIGINT.
--
-- @param source  string       The TypeScript text to tokenize.
-- @param version string|nil   TypeScript version: "ts1.0", "ts2.0", "ts3.0",
--                             "ts4.0", "ts5.0", "ts5.8", or nil/"" for generic.
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (generic):
--
--   local ts_lexer = require("coding_adventures.typescript_lexer")
--   local tokens = ts_lexer.tokenize("interface Foo { x: number }")
--   -- tokens[1].type  → "INTERFACE"
--   -- tokens[1].value → "interface"
--
-- Example (versioned):
--
--   local tokens = ts_lexer.tokenize("let x: number = 1;", "ts5.0")
--   -- Uses code/grammars/typescript/ts5.0.tokens
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

--- Create a GrammarLexer for a TypeScript source string without tokenizing yet.
--
-- Returns the initialized `GrammarLexer` instance; call `:tokenize()` to run it.
-- Useful when you need to configure the lexer before consuming tokens, or to
-- measure performance without counting grammar-load time.
--
-- @param source  string       The TypeScript text to lex.
-- @param version string|nil   TypeScript version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
--
-- Example:
--
--   local gl = ts_lexer.create_lexer("let x = 1;", "ts5.8")
--   local raw = gl:tokenize()
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for TypeScript.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @param version string|nil  TypeScript version tag (see tokenize for valid values).
-- @return TokenGrammar       The parsed TypeScript token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
