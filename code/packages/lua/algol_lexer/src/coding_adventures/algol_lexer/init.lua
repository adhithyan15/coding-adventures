-- algol_lexer -- Tokenizes ALGOL 60 source text using the grammar-driven infrastructure
-- =====================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `algol.tokens` grammar file to configure the tokenizer.
--
-- # What is ALGOL 60?
--
-- ALGOL 60 (ALGOrithmic Language, 1960) was the first programming language to be
-- formally specified using BNF (Backus-Naur Form). It introduced block structure,
-- lexical scoping, recursion, and the call stack — concepts every modern language
-- inherits. It is the ancestor of Pascal, C, Ada, and Simula (the first OOP language).
--
-- # What is ALGOL 60 tokenization?
--
-- Given the input:  begin integer x; x := 42 end
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(BEGIN,       "begin",   1:1)
--   Token(INTEGER,     "integer", 1:7)
--   Token(IDENT,       "x",       1:15)
--   Token(SEMICOLON,   ";",       1:16)
--   Token(IDENT,       "x",       1:18)
--   Token(ASSIGN,      ":=",      1:20)
--   Token(INTEGER_LIT, "42",      1:23)
--   Token(END,         "end",     1:26)
--   Token(EOF,         "",        1:29)
--
-- Whitespace is silently consumed (the `algol.tokens` grammar declares it
-- as a skip pattern). Comments (`comment ... ;`) are also silently consumed.
--
-- Keywords are case-insensitive: `BEGIN`, `Begin`, and `begin` all produce
-- a BEGIN token. The value field preserves the original case.
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar` module (once, cached).
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Grammar source
--
-- The token grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `algol60.tokens` is pre-compiled (via
-- `grammar-tools compile-tokens`) into `_grammar.lua`, a plain Lua module
-- that embeds the TokenGrammar as native Lua data structures. That module
-- ships as part of this package, so `require()` always finds it.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `token_grammar()` result cached in a module-level table. Subsequent
-- calls to `tokenize` reuse the cached grammar.

local _grammar_cache = {}

local function normalize_version(version)
    if version == nil or version == "" then
        return "algol60"
    end
    if version ~= "algol60" then
        error("algol_lexer: unknown ALGOL version '" .. tostring(version) .. "' (valid: algol60)")
    end
    return version
end

--- Return the (cached) TokenGrammar for the given ALGOL version.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `token_grammar()`.  On subsequent calls, returns the cached
-- TokenGrammar object immediately.
-- @return TokenGrammar  The ALGOL 60 token grammar.
local function get_grammar(version)
    version = normalize_version(version)
    if _grammar_cache[version] then
        return _grammar_cache[version]
    end

    local compiled = require("coding_adventures.algol_lexer._grammar")
    _grammar_cache[version] = compiled.token_grammar()
    return _grammar_cache[version]
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize an ALGOL 60 source string.
--
-- Loads the `algol.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`.  Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace and comments (`comment ... ;`) are consumed silently via
-- the skip patterns in `algol.tokens`.  The caller receives only
-- meaningful tokens.
--
-- Keywords are case-insensitive.  The `value` field of a keyword token
-- preserves the original source text.  The `type` field is normalized
-- to the keyword name in uppercase (e.g., "BEGIN", "END", "IF").
--
-- @param source string  The ALGOL 60 text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local algol_lexer = require("coding_adventures.algol_lexer")
--   local tokens = algol_lexer.tokenize("begin integer x; x := 42 end")
--   -- tokens[1].type  → "BEGIN"
--   -- tokens[1].value → "begin"
--   -- tokens[3].type  → "NAME"
--   -- tokens[3].value → "x"
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

--- Return the cached (or freshly loaded) TokenGrammar for ALGOL 60.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed ALGOL 60 token grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
