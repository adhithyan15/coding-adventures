-- python_parser -- Builds an AST from Python text using the grammar-driven engine
-- ==================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside javascript_parser, json_parser,
-- and sql_parser, above the lexer, grammar_tools, and python_lexer packages.
--
-- # What does a Python parser do?
--
-- A lexer breaks raw Python source into a flat token stream:
--
--   'x = 5'
--   →  NAME("x") EQUALS("=") NUMBER("5") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── assignment
--           ├── NAME    "x"
--           ├── EQUALS  "="
--           └── expression
--               └── term
--                   └── factor
--                       └── NUMBER  "5"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, transpilers, linters) walk the AST rather than re-parsing.
--
-- # Python grammar
--
-- The Python grammar is defined in `code/grammars/python.grammar`.
-- The grammar covers a focused subset:
--
--   program      = { statement } ;
--   statement    = assignment | expression_stmt ;
--   assignment   = NAME EQUALS expression ;
--   expression_stmt = expression ;
--   expression   = term { ( PLUS | MINUS ) term } ;
--   term         = factor { ( STAR | SLASH ) factor } ;
--   factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
--
-- This grammar handles:
--   - Assignments: x = 5
--   - Arithmetic expressions: 1 + 2 * 3 (respects precedence via term/factor)
--   - Parenthesized groups: (a + b) * c
--   - Expression statements: just an expression on its own line
--
-- # Architecture
--
-- 1. **Tokenize** — call `python_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # GrammarParser and ASTNode
--
-- `GrammarParser.new(tokens, grammar)` returns a parser instance.
-- Calling `:parse()` returns either:
--   (node, nil)    — success; `node` is the root ASTNode
--   (nil, errmsg)  — failure; `errmsg` is a human-readable error string
--
-- ASTNode fields:
--   node.rule_name  — which grammar rule produced this node ("program", …)
--   node.children   — array of child ASTNodes and/or Token tables
--   node:is_leaf()  — true when the node wraps exactly one token
--   node:token()    — the wrapped token (only valid when is_leaf() is true)
--
-- # Operator precedence
--
-- The grammar encodes Python operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, parenthesized expressions (highest)
--
-- This means "1 + 2 * 3" correctly parses as "1 + (2 * 3)":
--
--   expression
--   ├── term → factor → NUMBER "1"
--   ├── PLUS "+"
--   └── term
--       ├── factor → NUMBER "2"
--       ├── STAR "*"
--       └── factor → NUMBER "3"
--

local python_lexer  = require("coding_adventures.python_lexer")
local parser_pkg    = require("coding_adventures.parser")

-- =========================================================================
-- Compiled grammar
-- =========================================================================
--
-- Historically this module read `python.grammar` from `code/grammars/` at
-- runtime via `io.open`, walking outside this package's own directory into
-- the monorepo. That works when running inside the monorepo checkout, but
-- a published LuaRocks package does not ship `code/grammars/`, so
-- `luarocks install` + first use would raise a file-not-found error.
--
-- Instead, the grammar is now pre-compiled to a native Lua data structure
-- (via `code/programs/lua/grammar-tools`) and checked in as a sibling
-- `_grammar_default.lua` file, `require`d like any other module. No
-- runtime file I/O, no path traversal outside the package.
--
-- Unlike python_lexer, python_parser is NOT versioned: `parse()` and
-- `create_parser()` take no version argument and always use this single
-- grammar, regardless of which Python version `python_lexer.tokenize` was
-- asked to lex with. This mirrors the pre-existing (already asymmetric)
-- behavior of this package -- confirmed by reading the original
-- `get_grammar()`, which always opened the same unversioned
-- `python.grammar` file.

local compiled_grammar_module = require("coding_adventures.python_parser._grammar_default")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The parser grammar is constructed once and cached.  Repeated calls to
-- `parse()` or `create_parser()` reuse the cached grammar.

local _grammar_cache = nil

--- Load `python.grammar`'s compiled form, with caching.
-- On the first call, invokes the compiled module's `parser_grammar()`
-- constructor and caches the result.
-- @return ParserGrammar  The parsed Python parser grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    _grammar_cache = compiled_grammar_module.parser_grammar()
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Python source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `python_lexer.tokenize`.
--   2. Loads the Python parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- Python grammar).
--
-- The grammar supports:
--   - Assignments: `x = 5`
--   - Arithmetic: `1 + 2 * 3` (correct precedence via term/factor layering)
--   - Parenthesized expressions: `(a + b) * c`
--   - Expression statements
--
-- @param source string  The Python text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local python_parser = require("coding_adventures.python_parser")
--   local ast = python_parser.parse("x = 5")
--   -- ast.rule_name  → "program"
--   -- contains statement → assignment
function M.parse(source)
    local tokens = python_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("python_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Python source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Python text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = python_parser.create_parser("x = 1")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = python_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Python.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed Python parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
