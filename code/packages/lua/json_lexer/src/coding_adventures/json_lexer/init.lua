-- json_lexer -- Tokenizes JSON text using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading a bundled projection of `json.tokens` to configure the tokenizer.
--
-- # What is JSON tokenization?
--
-- Given the input:  {"key": 42, "ok": true}
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(LBRACE,  "{",     1:1)
--   Token(STRING,  '"key"', 1:2)
--   Token(COLON,   ":",     1:7)
--   Token(NUMBER,  "42",    1:9)
--   Token(COMMA,   ",",     1:11)
--   Token(STRING,  '"ok"',  1:13)
--   Token(COLON,   ":",     1:17)
--   Token(TRUE,    "true",  1:19)
--   Token(RBRACE,  "}",     1:23)
--   Token(EOF,     "",      1:24)
--
-- Whitespace is silently consumed (the `json.tokens` grammar declares it
-- as a skip pattern). The parser never sees whitespace tokens.
--
-- # Architecture
--
-- This module:
--   1. Loads the checked-in Lua projection of the canonical `json.tokens`.
--   2. Parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")
local grammar_data  = require("coding_adventures.json_lexer.grammar_data")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The bundled grammar payload is parsed exactly once and cached in a
-- module-level variable. Subsequent calls reuse the cached grammar.

local _grammar_cache = nil

--- Load and parse the bundled `json.tokens` grammar, with caching.
-- @return TokenGrammar  The parsed JSON token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local grammar, parse_err = grammar_tools.parse_token_grammar(grammar_data)
    if not grammar then
        error("json_lexer: failed to parse json.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a JSON source string.
--
-- Loads the `json.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`.  Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `json.tokens`.
-- The caller receives only meaningful tokens: STRING, NUMBER, TRUE, FALSE,
-- NULL, LBRACE, RBRACE, LBRACKET, RBRACKET, COLON, COMMA, EOF.
--
-- @param source string  The JSON text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local json_lexer = require("coding_adventures.json_lexer")
--   local tokens = json_lexer.tokenize('{"x": 1}')
--   -- tokens[1].type  → "LBRACE"
--   -- tokens[1].value → "{"
--   -- tokens[2].type  → "STRING"
--   -- tokens[2].value → '"x"'
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

--- Return the cached (or freshly loaded) TokenGrammar for JSON.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed JSON token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
