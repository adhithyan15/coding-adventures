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
--   1. Requires the pre-compiled `_grammar` module (generated ahead of
--      time from `toml.tokens` via `grammar-tools compile-tokens`), which
--      embeds the TokenGrammar as native Lua data — no disk I/O.
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for TOML.
-- @return TokenGrammar  The compiled TOML token grammar.
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.toml_lexer._grammar").token_grammar()
    end
    return _grammar_cache
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
