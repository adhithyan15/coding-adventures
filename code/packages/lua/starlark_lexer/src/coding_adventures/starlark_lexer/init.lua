-- starlark_lexer — Tokenizes Starlark source using the grammar-driven infrastructure
-- ====================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading the `starlark.tokens` grammar file to configure the tokenizer.
--
-- # What is Starlark?
--
-- Starlark is a deterministic subset of Python designed for use as a
-- configuration language (famously used in Bazel BUILD files). It is
-- syntactically similar to Python but with significant differences:
--   - No while loops (no general iteration)
--   - No classes or class definitions
--   - No try/except/raise
--   - No global/nonlocal
--   - Significant indentation (like Python)
--   - Certain Python keywords are reserved but disallowed
--
-- # What is Starlark tokenization?
--
-- Given the input:  def foo(x):
--
-- The lexer produces a flat stream of typed tokens:
--
--   Token(DEF,    "def",  1:1)
--   Token(NAME,   "foo",  1:5)
--   Token(LPAREN, "(",    1:8)
--   Token(NAME,   "x",    1:9)
--   Token(RPAREN, ")",    1:10)
--   Token(COLON,  ":",    1:11)
--   Token(EOF,    "",     1:12)
--
-- Whitespace between tokens is silently consumed (declared as skip patterns
-- in `starlark.tokens`). The parser never sees ordinary whitespace.
--
-- # Indentation mode
--
-- `starlark.tokens` declares `mode: indentation`, which activates the
-- Python-style INDENT/DEDENT/NEWLINE token emission in the GrammarLexer.
-- This means:
--   - NEWLINE is emitted at each logical line boundary
--   - INDENT is emitted when indentation level increases
--   - DEDENT is emitted (possibly multiple times) when it decreases
--   - INDENT/DEDENT/NEWLINE are suppressed inside (), [], {}
--
-- # Architecture
--
-- This module:
--   1. Requires the pre-compiled `_grammar` module (generated ahead of
--      time from `starlark.tokens` via `grammar-tools compile-tokens`),
--      which embeds the TokenGrammar as native Lua data — no disk I/O.
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Token types produced
--
-- From regex definitions:
--   NAME    — identifiers and keywords (before keyword promotion)
--   INT     — integer literals (hex, octal, decimal) via -> INT aliases
--   FLOAT   — floating-point literals
--   STRING  — all string variants (single, double, triple, raw, bytes) via -> STRING
--
-- From keyword definitions (NAME tokens promoted to keyword types):
--   AND, BREAK, CONTINUE, DEF, ELIF, ELSE, FOR, IF, IN, LAMBDA,
--   LOAD, NOT, OR, PASS, RETURN, TRUE, FALSE, NONE
--
-- Three-character augmented assignment operators:
--   DOUBLE_STAR_EQUALS, LEFT_SHIFT_EQUALS, RIGHT_SHIFT_EQUALS, FLOOR_DIV_EQUALS
--
-- Two-character operators:
--   DOUBLE_STAR, FLOOR_DIV, LEFT_SHIFT, RIGHT_SHIFT,
--   EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS, GREATER_EQUALS,
--   PLUS_EQUALS, MINUS_EQUALS, STAR_EQUALS, SLASH_EQUALS,
--   PERCENT_EQUALS, AMP_EQUALS, PIPE_EQUALS, CARET_EQUALS
--
-- Single-character operators:
--   PLUS, MINUS, STAR, SLASH, PERCENT, EQUALS,
--   LESS_THAN, GREATER_THAN, AMP, PIPE, CARET, TILDE
--
-- Delimiters:
--   LPAREN, RPAREN, LBRACKET, RBRACKET, LBRACE, RBRACE,
--   COMMA, COLON, SEMICOLON, DOT
--
-- Indentation tokens (emitted by mode: indentation):
--   INDENT, DEDENT, NEWLINE
--
-- Reserved words cause a lexer error if used as identifiers.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Starlark.
-- @return TokenGrammar  The compiled Starlark token grammar.
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.starlark_lexer._grammar").token_grammar()
    end
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Starlark source string.
--
-- Loads the `starlark.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Because `starlark.tokens` uses `mode: indentation`, the GrammarLexer
-- automatically emits INDENT, DEDENT, and NEWLINE tokens at logical line
-- boundaries. INDENT/DEDENT/NEWLINE are suppressed inside (), [], {}.
--
-- Whitespace and comments are consumed silently via the skip patterns in
-- `starlark.tokens`.
--
-- @param source string  The Starlark text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters or reserved
--                       keywords used as identifiers.
--
-- Example:
--
--   local starlark_lexer = require("coding_adventures.starlark_lexer")
--   local tokens = starlark_lexer.tokenize("def foo(x):")
--   -- tokens[1].type  → "DEF"
--   -- tokens[1].value → "def"
--   -- tokens[2].type  → "NAME"
--   -- tokens[2].value → "foo"
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

--- Return the cached (or freshly loaded) TokenGrammar for Starlark.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Starlark token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
