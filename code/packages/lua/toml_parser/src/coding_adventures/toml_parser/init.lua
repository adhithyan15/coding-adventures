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
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/toml_parser/src/coding_adventures/toml_parser/init.lua
--
-- Walking 6 levels up reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   toml_parser/        (1)
--   coding_adventures/  (2)
--   src/                (3)
--   toml_parser/        (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/toml.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local toml_lexer    = require("coding_adventures.toml_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (no trailing slash).
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- When busted runs tests with a relative path containing ".." the
-- dirname-only approach produces a path that collapses to "." after
-- up() steps, so the grammar file cannot be found.  We resolve to an
-- absolute path via "cd <dir> && pwd" to give up() an absolute anchor.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    -- Normalize Windows backslashes to forward slashes for cross-platform
    -- path handling (on Linux/macOS this is a no-op).
    src = src:gsub("\\", "/")
    local dir = src:match("(.+)/[^/]+$") or "."
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string
-- @param levels number
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = result .. "/.."
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================

local _grammar_cache = nil

--- Load and parse `toml.grammar`, with caching.
-- @return ParserGrammar
-- @error  Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/toml.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "toml_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "toml_parser: failed to parse toml.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
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
