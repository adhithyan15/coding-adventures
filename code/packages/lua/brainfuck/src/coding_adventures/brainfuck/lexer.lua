-- brainfuck.lexer — Tokenizes Brainfuck source text using the grammar infrastructure
-- ==================================================================================
--
-- This module is the tokenization layer for the Brainfuck front-end pipeline.
-- It is a thin wrapper around the grammar-driven `GrammarLexer` from the
-- `coding_adventures.lexer` package, loading the `brainfuck.tokens` grammar
-- file to configure the tokenizer.
--
-- # What is Brainfuck tokenization?
--
-- Given the input:  ++[>+<-]
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(INC,        "+",  1:1)
--   Token(INC,        "+",  1:2)
--   Token(LOOP_START, "[",  1:3)
--   Token(RIGHT,      ">",  1:4)
--   Token(INC,        "+",  1:5)
--   Token(LEFT,       "<",  1:6)
--   Token(DEC,        "-",  1:7)
--   Token(LOOP_END,   "]",  1:8)
--   Token(EOF,        "",   1:9)
--
-- Comment characters (everything that is not `><+-.,[]`) are silently consumed
-- by the `brainfuck.tokens` grammar's skip: section. The parser only ever
-- receives the eight command tokens and EOF.
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar_tokens` module (once, cached).
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Grammar source
--
-- The token grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `brainfuck.tokens` is pre-compiled (via
-- `grammar-tools compile-tokens`) into `_grammar_tokens.lua`, a plain Lua
-- module that embeds the TokenGrammar as native Lua data structures. That
-- module ships as part of this package, so `require()` always finds it.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `token_grammar()` result cached in a module-level variable. Subsequent
-- calls to `tokenize` reuse the cached grammar.

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Brainfuck.
-- On the first call, requires the pre-compiled `_grammar_tokens` module
-- and invokes `token_grammar()`. On subsequent calls, returns the cached
-- TokenGrammar object immediately.
-- @return TokenGrammar  The Brainfuck token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.brainfuck._grammar_tokens")
    _grammar_cache = compiled.token_grammar()
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Brainfuck source string.
--
-- Loads the `brainfuck.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Comment characters (any character that is not `><+-.,[]`) are consumed
-- silently via the skip patterns in `brainfuck.tokens`. The caller receives
-- only meaningful tokens: RIGHT, LEFT, INC, DEC, OUTPUT, INPUT,
-- LOOP_START, LOOP_END, EOF.
--
-- Unlike JSON, Brainfuck tokenization never raises an error on unexpected
-- characters — every non-command character is a valid comment.
--
-- @param source string  The Brainfuck text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
--
-- Example:
--
--   local bf_lexer = require("coding_adventures.brainfuck.lexer")
--   local tokens = bf_lexer.tokenize("++[>+<-]")
--   -- tokens[1].type  → "INC"
--   -- tokens[1].value → "+"
--   -- tokens[3].type  → "LOOP_START"
--   -- tokens[3].value → "["
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

--- Create a GrammarLexer for a Brainfuck source string without immediately tokenizing.
--
-- Use this when you want fine-grained control over lexing — for example,
-- to tokenize incrementally or to access the raw GrammarLexer object.
--
-- @param source string      The Brainfuck text to tokenize.
-- @return GrammarLexer      An initialized lexer, ready to call :tokenize().
--
-- Example:
--
--   local lx = bf_lexer.create_lexer("++")
--   local raw_tokens = lx:tokenize()
function M.create_lexer(source)
    local grammar = get_grammar()
    return lexer_pkg.GrammarLexer.new(source, grammar)
end

--- Return the cached (or freshly loaded) TokenGrammar for Brainfuck.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Brainfuck token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
