-- javascript_lexer — Tokenizes JavaScript source using the grammar-driven infrastructure
-- ======================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the appropriate grammar file to configure the tokenizer.
--
-- # What is JavaScript tokenization?
--
-- Given the input:  const x = 42;
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(CONST,      "const", 1:1)
--   Token(NAME,       "x",     1:7)
--   Token(EQUALS,     "=",     1:9)
--   Token(NUMBER,     "42",    1:11)
--   Token(SEMICOLON,  ";",     1:13)
--   Token(EOF,        "",      1:14)
--
-- Whitespace and comments are silently consumed (declared as skip patterns
-- in the grammar file). The parser never sees them.
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
-- # Version routing
--
-- When `version` is nil or "" → uses the generic/unified grammar.
-- When `version` is "es1"     → uses the ECMAScript 1 grammar.
-- When `version` is "es2015"  → uses the ECMAScript 2015 grammar.
-- ... etc.
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   NUMBER  — integer literals (e.g. 42, 0xFF)
--   STRING  — double-quoted string literals
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
--   CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF,
--   TRUE, FALSE, NULL, UNDEFINED
--
-- Operators and delimiters:
--   STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
--   LESS_EQUALS, GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS, STAR,
--   SLASH, LESS_THAN, GREATER_THAN, BANG,
--   LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET,
--   COMMA, COLON, SEMICOLON, DOT

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
-- Regenerate with (from code/programs/lua/grammar-tools):
--   compile_tokens_command("../../../grammars/ecmascript/es2015.tokens", ...)

local compiled_grammars = {
    [""]       = require("coding_adventures.javascript_lexer._grammar_default"),
    ["es1"]    = require("coding_adventures.javascript_lexer._grammar_es1"),
    ["es3"]    = require("coding_adventures.javascript_lexer._grammar_es3"),
    ["es5"]    = require("coding_adventures.javascript_lexer._grammar_es5"),
    ["es2015"] = require("coding_adventures.javascript_lexer._grammar_es2015"),
    ["es2016"] = require("coding_adventures.javascript_lexer._grammar_es2016"),
    ["es2017"] = require("coding_adventures.javascript_lexer._grammar_es2017"),
    ["es2018"] = require("coding_adventures.javascript_lexer._grammar_es2018"),
    ["es2019"] = require("coding_adventures.javascript_lexer._grammar_es2019"),
    ["es2020"] = require("coding_adventures.javascript_lexer._grammar_es2020"),
    ["es2021"] = require("coding_adventures.javascript_lexer._grammar_es2021"),
    ["es2022"] = require("coding_adventures.javascript_lexer._grammar_es2022"),
    ["es2023"] = require("coding_adventures.javascript_lexer._grammar_es2023"),
    ["es2024"] = require("coding_adventures.javascript_lexer._grammar_es2024"),
    ["es2025"] = require("coding_adventures.javascript_lexer._grammar_es2025"),
}

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid ECMAScript / JavaScript versions
-- =========================================================================
--
-- JavaScript is standardized as ECMAScript. The version strings we accept
-- match the grammar files under code/grammars/ecmascript/:
--
--   "es1"    — ECMAScript 1  (1997): original standardization.
--   "es3"    — ECMAScript 3  (1999): try/catch, regex literals.
--   "es5"    — ECMAScript 5  (2009): strict mode, JSON, Array extras.
--   "es2015" — ECMAScript 6  (2015): let/const, arrow functions, classes.
--   "es2016" — ECMAScript 7  (2016): exponentiation operator.
--   "es2017" — ECMAScript 8  (2017): async/await.
--   "es2018" — ECMAScript 9  (2018): rest/spread properties.
--   "es2019" — ECMAScript 10 (2019): flat, flatMap.
--   "es2020" — ECMAScript 11 (2020): nullish coalescing, optional chaining.
--   "es2021" — ECMAScript 12 (2021): logical assignment, numeric separators.
--   "es2022" — ECMAScript 13 (2022): class fields, top-level await.
--   "es2023" — ECMAScript 14 (2023): array findLast, change array by copy.
--   "es2024" — ECMAScript 15 (2024): Object.groupBy, Promise.withResolvers.
--   "es2025" — ECMAScript 16 (2025): import attributes, RegExp.escape.
--   nil / "" — Generic JavaScript (uses the unified javascript.tokens grammar).

local VALID_JS_VERSIONS = {
    ["es1"]    = true, ["es3"]    = true, ["es5"]    = true,
    ["es2015"] = true, ["es2016"] = true, ["es2017"] = true,
    ["es2018"] = true, ["es2019"] = true, ["es2020"] = true,
    ["es2021"] = true, ["es2022"] = true, ["es2023"] = true,
    ["es2024"] = true, ["es2025"] = true,
}

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- Grammars are cached per version string. The cache key is the version
-- string (or "" for generic). Each compiled grammar module's
-- `token_grammar()` constructor is only invoked once per version; the
-- resulting TokenGrammar is memoized in `_grammar_cache`.

local _grammar_cache = {}

--- Resolve a version string to its entry in `compiled_grammars`.
--
-- Version routing logic:
--   - nil or ""  →  generic (unified) grammar
--   - "es1"      →  ECMAScript 1 grammar
--   - "es2015"   →  ECMAScript 2015 grammar
--   - ...etc.
--
-- If an unrecognized version string is passed, we raise an error immediately.
--
-- @param version string|nil  The ECMAScript version tag, or nil/empty for generic.
-- @return string             The resolved lookup key into `compiled_grammars`.
local function resolve_version(version)
    -- Generic (no version specified) — use the unified grammar.
    if not version or version == "" then
        return ""
    end

    -- Validate the version string before looking it up.
    if not VALID_JS_VERSIONS[version] then
        error(
            "javascript_lexer: unknown ECMAScript version '" .. version .. "'. " ..
            "Valid values are: es1, es3, es5, es2015..es2025, or nil/\"\" for generic."
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
-- @param version string|nil  The ECMAScript version tag (see resolve_version).
-- @return TokenGrammar       The parsed JavaScript token grammar.
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

--- Tokenize a JavaScript source string.
--
-- Loads the grammar for the requested ECMAScript version (cached after first
-- call) and feeds the source to a `GrammarLexer`. Returns the complete flat
-- token list, including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in the grammar file.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source  string       The JavaScript text to tokenize.
-- @param version string|nil   ECMAScript version: "es1", "es3", "es5",
--                             "es2015".."es2025", or nil/"" for generic.
-- @return table               Array of Token objects (type, value, line, col).
-- @error                      Raises an error on unexpected characters or
--                             unknown version string.
--
-- Example (generic):
--
--   local js_lexer = require("coding_adventures.javascript_lexer")
--   local tokens = js_lexer.tokenize("const x = 1;")
--   -- tokens[1].type  → "CONST"
--   -- tokens[1].value → "const"
--
-- Example (versioned):
--
--   local tokens = js_lexer.tokenize("var x = 1;", "es1")
--   -- Uses code/grammars/ecmascript/es1.tokens
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

--- Create a GrammarLexer for a JavaScript source string without tokenizing yet.
--
-- Returns the initialized `GrammarLexer` instance; call `:tokenize()` to run it.
-- Useful when you need to configure the lexer before consuming tokens, or to
-- measure performance without counting grammar-load time.
--
-- @param source  string       The JavaScript text to lex.
-- @param version string|nil   ECMAScript version tag (see tokenize for valid values).
-- @return GrammarLexer        An initialized lexer, ready to call `:tokenize()`.
--
-- Example:
--
--   local gl = js_lexer.create_lexer("var x = 1;", "es5")
--   local raw = gl:tokenize()
function M.create_lexer(source, version)
    local grammar = get_grammar(version)
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for JavaScript.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @param version string|nil  ECMAScript version tag (see tokenize for valid values).
-- @return TokenGrammar       The parsed JavaScript token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
