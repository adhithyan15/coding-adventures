-- sql_lexer -- Tokenizes SQL text using the grammar-driven infrastructure
-- =========================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `sql.tokens` grammar file to configure the tokenizer.
--
-- # What is SQL tokenization?
--
-- Given the input:  SELECT * FROM users WHERE id = 1
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(SELECT,  "SELECT",  1:1)
--   Token(STAR,    "*",       1:8)
--   Token(FROM,    "FROM",    1:10)
--   Token(NAME,    "users",   1:15)
--   Token(WHERE,   "WHERE",   1:21)
--   Token(NAME,    "id",      1:27)
--   Token(EQUALS,  "=",       1:30)
--   Token(NUMBER,  "1",       1:32)
--   Token(EOF,     "",        1:33)
--
-- Whitespace and comments are silently consumed (the `sql.tokens` grammar
-- declares them as skip patterns). The parser never sees whitespace tokens.
--
-- # SQL-specific lexer concerns
--
-- **Case-insensitive keywords** — The `sql.tokens` grammar has
-- `@case_insensitive true`, meaning keyword literals like "SELECT" match
-- `select`, `SELECT`, or `Select`. The grammar tools infrastructure handles
-- case folding when building the GrammarLexer.
--
-- **Keywords vs identifiers** — The grammar lists keywords (SELECT, FROM,
-- WHERE, etc.) that must match before the generic `NAME` pattern. The
-- GrammarLexer tries definitions in order, so keywords take priority.
--
-- **Operator ordering** — Longer operators must come before shorter ones:
--   `<=` before `<`,  `>=` before `>`,  `!=` before nothing.
-- The grammar handles this via ordering.
--
-- **NEQ_ANSI alias** — `<>` (ANSI SQL inequality) is aliased to NOT_EQUALS
-- so a parser only needs to handle one token type for both `!=` and `<>`.
--
-- **STRING alias** — `STRING_SQ` (single-quoted) and `QUOTED_ID` (backtick-
-- quoted identifier) are aliased to STRING and NAME respectively so the
-- grammar can reference a single type.
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar` module (generated ahead of
--      time from `sql.tokens` via `grammar-tools compile-tokens`), which
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

--- Return the (cached) TokenGrammar for SQL.
-- @return TokenGrammar  The compiled SQL token grammar.
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.sql_lexer._grammar").token_grammar()
    end
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a SQL source string.
--
-- Loads the `sql.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Keywords are case-insensitive: "select", "SELECT", and "Select" all
-- produce a SELECT token. The token value preserves the original source
-- casing.
--
-- SQL token types produced:
--
--   NAME           — identifiers: column names, table names, aliases
--   NUMBER         — integer and decimal literals: 42, 3.14
--   STRING         — single-quoted string literals: 'hello'
--                    (aliased from STRING_SQ in grammar)
--   SELECT, FROM, WHERE, GROUP, BY, HAVING, ORDER, LIMIT, OFFSET
--   INSERT, INTO, VALUES, UPDATE, SET, DELETE
--   CREATE, DROP, TABLE, IF, EXISTS
--   NOT, AND, OR, NULL, IS, IN, BETWEEN, LIKE, AS, DISTINCT
--   ALL, UNION, INTERSECT, EXCEPT
--   JOIN, INNER, LEFT, RIGHT, OUTER, CROSS, FULL, ON
--   ASC, DESC, TRUE, FALSE
--   CASE, WHEN, THEN, ELSE, END
--   PRIMARY, KEY, UNIQUE, DEFAULT
--   LESS_EQUALS, GREATER_EQUALS, NOT_EQUALS  — <=, >=, != (and <>)
--   EQUALS, LESS_THAN, GREATER_THAN          — =, <, >
--   PLUS, MINUS, STAR, SLASH, PERCENT        — arithmetic operators
--   LPAREN, RPAREN, COMMA, SEMICOLON, DOT    — delimiters
--   EOF                                      — end of input
--
-- Whitespace, line comments (-- ...), and block comments (/* ... */) are
-- consumed silently via the skip patterns in `sql.tokens`.
--
-- @param source string  The SQL text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local sql_lexer = require("coding_adventures.sql_lexer")
--   local tokens = sql_lexer.tokenize("SELECT * FROM users")
--   -- tokens[1].type  → "SELECT"
--   -- tokens[1].value → "SELECT"
--   -- tokens[2].type  → "STAR"
--   -- tokens[3].type  → "FROM"
--   -- tokens[4].type  → "NAME"
--   -- tokens[4].value → "users"
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

--- Return the cached (or freshly loaded) TokenGrammar for SQL.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed SQL token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
