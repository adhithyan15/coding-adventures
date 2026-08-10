-- lattice_lexer — Tokenizes Lattice source using the grammar-driven infrastructure
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo. It is a thin
-- wrapper around the grammar-driven `GrammarLexer` from the `lexer` package,
-- loading a generated payload of the canonical `lattice.tokens` grammar to
-- configure the tokenizer without ambient filesystem access.
--
-- # What is Lattice?
--
-- Lattice is a CSS superset language that adds:
--   - Variables:        $color, $font-size
--   - Mixins:           @mixin, @include
--   - Control flow:     @if, @else, @for, @each
--   - Functions:        @function, @return
--   - Modules:          @use
--   - Nesting:          .parent { .child { ... } }
--   - Placeholder selectors: %button-base (for @extend)
--   - Single-line comments: // to end of line
--   - Comparison operators: ==, !=, >=, <= (for @if conditions)
--   - Variable flags:   !default, !global
--
-- Every valid CSS file is valid Lattice. Lattice adds tokens on top of
-- the CSS token set without modifying any existing CSS token behaviour.
--
-- # What is Lattice tokenization?
--
-- Given the input:  $color: #ff0000;
--
-- The lexer produces:
--
--   Token(VARIABLE,   "$color",  1:1)
--   Token(COLON,      ":",       1:7)
--   Token(HASH,       "#ff0000", 1:9)
--   Token(SEMICOLON,  ";",       1:16)
--   Token(EOF,        "",        1:17)
--
-- # Escape handling
--
-- `lattice.tokens` declares `escapes: none`, which tells the GrammarLexer
-- to strip the surrounding quotes from STRING tokens but leave the string
-- content as raw text (no escape-sequence processing). CSS escape sequences
-- use a different format from JSON (\26 vs \n) and are a semantic concern
-- to be handled post-parse, not at the lexer level.
--
-- # Architecture
--
-- This module:
--   1. Loads the checked-in grammar payload generated from
--      `code/grammars/lattice/lattice.tokens`.
--   2. Parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Token types produced
--
-- Lattice-specific tokens (new):
--   VARIABLE        — $color, $font-size
--   PLACEHOLDER     — %button-base, %flex-center
--   EQUALS_EQUALS   — ==
--   NOT_EQUALS      — !=
--   GREATER_EQUALS  — >=
--   LESS_EQUALS     — <=
--   BANG_DEFAULT    — !default
--   BANG_GLOBAL     — !global
--
-- Shared with CSS:
--   STRING          — "hello", 'world' (via -> STRING alias)
--   DIMENSION       — 10px, 1.5em, -2rem
--   PERCENTAGE      — 50%, 100%
--   NUMBER          — 3.14, -1, 0
--   HASH            — #ff0000, #abc
--   AT_KEYWORD      — @media, @mixin, @include, @if
--   URL_TOKEN       — url(https://example.com)
--   FUNCTION        — rgb(, calc(, var(
--   CDO             — <!--
--   CDC             — -->
--   UNICODE_RANGE   — U+0041, U+0041-005A
--   CUSTOM_PROPERTY — --primary-color
--   IDENT           — red, serif, auto
--   COLON_COLON     — ::
--   TILDE_EQUALS    — ~=
--   PIPE_EQUALS     — |=
--   CARET_EQUALS    — ^=
--   DOLLAR_EQUALS   — $=
--   STAR_EQUALS     — *=
--   LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET
--   SEMICOLON, COLON, COMMA, DOT
--   PLUS, GREATER, LESS, TILDE, STAR, PIPE
--   BANG, SLASH, EQUALS, AMPERSAND, MINUS

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")
local grammar_data  = require("coding_adventures.lattice_lexer.grammar_data")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The bundled grammar payload is parsed exactly once and cached in a
-- module-level variable. Subsequent calls reuse the cached grammar.

local _grammar_cache = nil

--- Load and parse the bundled `lattice.tokens` grammar, with caching.
-- @return TokenGrammar  The parsed Lattice token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local grammar, parse_err = grammar_tools.parse_token_grammar(grammar_data)
    if not grammar then
        error("lattice_lexer: failed to parse lattice.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Lattice source string.
--
-- Loads the `lattice.tokens` grammar (cached after first call) and feeds
-- the source to a `GrammarLexer`. Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Because `lattice.tokens` declares `escapes: none`, STRING token values
-- retain their surrounding quote characters and any escape sequences as
-- raw text. CSS escape decoding (e.g. \26 → &) is a semantic concern
-- handled after parsing, not at the lexer level.
--
-- Whitespace and comments (// line comments and /* block comments */) are
-- consumed silently via the skip patterns in `lattice.tokens`.
--
-- @param source string  The Lattice text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local lattice_lexer = require("coding_adventures.lattice_lexer")
--   local tokens = lattice_lexer.tokenize("$color: #ff0000;")
--   -- tokens[1].type  → "VARIABLE"
--   -- tokens[1].value → "$color"
--   -- tokens[2].type  → "COLON"
--   -- tokens[3].type  → "HASH"
--   -- tokens[3].value → "#ff0000"
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

--- Return the cached (or freshly loaded) TokenGrammar for Lattice.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Lattice token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
