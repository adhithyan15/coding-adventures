-- ruby_lexer — Tokenizes Ruby source using the grammar-driven infrastructure
-- ==============================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `ruby.tokens` grammar file to configure the tokenizer.
--
-- # What is Ruby tokenization?
--
-- Given the input:  def greet(name)
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(DEF,    "def",   1:1)
--   Token(NAME,   "greet", 1:5)
--   Token(LPAREN, "(",     1:10)
--   Token(NAME,   "name",  1:11)
--   Token(RPAREN, ")",     1:15)
--   Token(EOF,    "",      1:16)
--
-- Whitespace is silently consumed (declared as skip patterns in
-- `ruby.tokens`). The parser never sees ordinary whitespace.
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar` module (generated ahead of
--      time from `ruby.tokens` via `grammar-tools compile-tokens`), which
--      embeds the TokenGrammar as native Lua data — no disk I/O.
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   NUMBER  — integer literals (e.g. 42, 0)
--   STRING  — double-quoted string literals
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   IF, ELSE, ELSIF, END, WHILE, FOR, DO, DEF, RETURN, CLASS, MODULE,
--   REQUIRE, PUTS, TRUE, FALSE, NIL, AND, OR, NOT, THEN, UNLESS, UNTIL,
--   YIELD, BEGIN, RESCUE, ENSURE
--
-- Multi-char operators (must match before single-char versions):
--   EQUALS_EQUALS, DOT_DOT, HASH_ROCKET, NOT_EQUALS,
--   LESS_EQUALS, GREATER_EQUALS
--
-- Single-char operators and delimiters:
--   EQUALS, PLUS, MINUS, STAR, SLASH,
--   LESS_THAN, GREATER_THAN,
--   LPAREN, RPAREN, COMMA, COLON

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The grammar is embedded as native Lua data in the pre-compiled
-- `_grammar` module (generated ahead of time from `ruby.tokens` via
-- `grammar-tools compile-tokens`). require() caches modules on its own,
-- so we only need to cache the *called* TokenGrammar object, not the
-- module itself.

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Ruby.
-- @return TokenGrammar  The compiled Ruby token grammar.
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.ruby_lexer._grammar").token_grammar()
    end
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Ruby source string.
--
-- Loads the `ruby.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace is consumed silently via the skip patterns in `ruby.tokens`.
-- The caller receives only meaningful tokens: NAME (and keyword subtypes),
-- NUMBER, STRING, operators, delimiters, and EOF.
--
-- @param source string  The Ruby text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local rb_lexer = require("coding_adventures.ruby_lexer")
--   local tokens = rb_lexer.tokenize("def greet(name)")
--   -- tokens[1].type  → "DEF"
--   -- tokens[1].value → "def"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "greet"
function M.tokenize(source)
    local grammar = get_grammar()
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

--- Return the cached (or freshly loaded) TokenGrammar for Ruby.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Ruby token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
