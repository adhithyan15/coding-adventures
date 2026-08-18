-- toml_parser -- Builds an AST from TOML text using the grammar-driven engine
-- ===========================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside json_parser, above the
-- toml_lexer, parser, and grammar_tools packages.
--
-- # What is TOML?
--
-- TOML (Tom's Obvious, Minimal Language) is a configuration file format
-- designed to be unambiguous and easy to read. A typical TOML file:
--
--   [server]
--   host = "localhost"
--   port = 8080
--   debug = true
--
-- # What does this parser produce?
--
-- The lexer turns the text into a flat token stream:
--
--   LBRACKET BARE_KEY("server") RBRACKET NEWLINE
--   BARE_KEY("host") EQUALS BASIC_STRING('"localhost"') NEWLINE
--   BARE_KEY("port") EQUALS INTEGER("8080") NEWLINE
--   BARE_KEY("debug") EQUALS TRUE("true") NEWLINE EOF
--
-- The parser turns the token stream into a tree:
--
--   document
--   ├── expression
--   │   └── table_header
--   │       ├── LBRACKET "["
--   │       ├── key
--   │       │   └── simple_key → BARE_KEY "server"
--   │       └── RBRACKET "]"
--   ├── NEWLINE
--   ├── expression
--   │   └── keyval
--   │       ├── key → simple_key → BARE_KEY "host"
--   │       ├── EQUALS "="
--   │       └── value → BASIC_STRING '"localhost"'
--   └── … etc.
--
-- # Grammar
--
-- The TOML grammar is defined in `code/grammars/toml.grammar`.  It has
-- ~12 rules, far more than JSON's 4 rules.  The entry point is `document`.
--
--   document = { NEWLINE | expression } ;
--   expression = array_table_header | table_header | keyval ;
--   keyval = key EQUALS value ;
--   key = simple_key { DOT simple_key } ;
--   simple_key = BARE_KEY | BASIC_STRING | LITERAL_STRING | … ;
--   table_header = LBRACKET key RBRACKET ;
--   array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
--   value = BASIC_STRING | … | array | inline_table ;
--   array = LBRACKET array_values RBRACKET ;
--   array_values = { NEWLINE } [ value { … } ] ;
--   inline_table = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
--
-- # Key design decision: NEWLINEs are significant in TOML
--
-- Unlike JSON, TOML key-value pairs are terminated by newlines.  The
-- `toml.grammar` references NEWLINE, so the `GrammarParser` automatically
-- preserves NEWLINE tokens instead of skipping them.
--
-- # Architecture
--
-- 1. **Tokenize** — call `toml_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.
--
-- # Grammar loading
--
-- The parser grammar is compiled ahead of time from `toml.grammar` via
-- `grammar-tools compile-grammar` and required as native Lua data — no
-- disk I/O at runtime.

local toml_lexer = require("coding_adventures.toml_lexer")
local parser_pkg = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The parser grammar is embedded as native Lua data in the pre-compiled
-- `_grammar` module (generated ahead of time from `toml.grammar` via
-- `grammar-tools compile-grammar`).

local _grammar_cache = nil

--- Return the (cached) ParserGrammar for TOML.
-- @return ParserGrammar
local function get_grammar()
    if not _grammar_cache then
        _grammar_cache = require("coding_adventures.toml_parser._grammar").parser_grammar()
    end
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a TOML source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `toml_lexer.tokenize`.
--   2. Loads the TOML parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "document"` (the first rule in the
-- TOML grammar).
--
-- TOML is newline-sensitive.  The GrammarParser auto-detects this from the
-- grammar (since `document` and `array_values` reference NEWLINE), so
-- NEWLINE tokens are preserved and not skipped.
--
-- @param source string  The TOML text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local toml_parser = require("coding_adventures.toml_parser")
--   local ast = toml_parser.parse('[server]\nhost = "localhost"\n')
--   -- ast.rule_name  → "document"
function M.parse(source)
    local tokens = toml_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("toml_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a TOML source string without immediately parsing.
--
-- @param source string   The TOML text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = toml_parser.create_parser('key = "value"\n')
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = toml_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for TOML.
--
-- @return ParserGrammar
function M.get_grammar()
    return get_grammar()
end

return M
