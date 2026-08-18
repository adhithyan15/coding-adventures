-- lisp_parser -- Builds an AST from Lisp/Scheme source using the grammar-driven engine
-- ======================================================================================
--
-- This package is part of the coding-adventures monorepo.  It sits above the
-- `lisp_lexer` and `grammar_tools` packages and uses the `GrammarParser` from
-- the `parser` package to produce Abstract Syntax Trees from S-expressions.
--
-- # Why Lisp is special
--
-- Most programming languages require elaborate grammars with many rules to
-- handle operator precedence, statement forms, expressions, declarations, etc.
-- Lisp has none of this.  The entire Lisp grammar — the syntax of the whole
-- language — fits in six rules:
--
--   program   = { sexpr } ;
--   sexpr     = atom | list | quoted ;
--   atom      = NUMBER | SYMBOL | STRING ;
--   list      = LPAREN list_body RPAREN ;
--   list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
--   quoted    = QUOTE sexpr ;
--
-- This radical simplicity is not accidental.  John McCarthy designed Lisp
-- so that the structure of programs mirrors the structure of data.  An S-
-- expression is simultaneously a program you can run and a list you can
-- manipulate.  The macros you write are ordinary Lisp functions that receive
-- and return S-expressions — the same S-expressions that are your code.
--
-- This property (code = data = code) is called **homoiconicity** and is the
-- source of Lisp's legendary expressiveness.
--
-- # What the parser produces
--
-- Given:  (define x 42)
--
-- Token stream:
--   LPAREN SYMBOL("define") SYMBOL("x") NUMBER("42") RPAREN EOF
--
-- AST:
--   program
--   └── sexpr
--       └── list
--           ├── LPAREN   "("
--           ├── list_body
--           │   ├── sexpr → atom → SYMBOL  "define"
--           │   ├── sexpr → atom → SYMBOL  "x"
--           │   └── sexpr → atom → NUMBER  "42"
--           └── RPAREN   ")"
--
-- The tree faithfully represents the recursive structure of the input.
-- Downstream evaluators walk this tree to execute Lisp programs.
--
-- # Understanding the DOT rule
--
-- `list_body = [ sexpr { sexpr } [ DOT sexpr ] ]`
--
-- This says: a list body is optionally:
--   - One or more S-expressions (the "proper list" part)
--   - Optionally followed by DOT and another S-expression (the "cdr" value)
--
-- So:
--   (1 2 3)        → list_body with three sexprs, no dot
--   (1 2 . 3)      → list_body with two sexprs, then DOT, then sexpr "3"
--   (a . b)        → list_body with one sexpr, then DOT, then sexpr "b"
--   ()             → empty list_body (the optional part is absent)
--
-- # Understanding QUOTE expansion
--
-- `quoted = QUOTE sexpr`
--
-- The tick prefix 'x is reader shorthand for (quote x).  The grammar captures
-- it as a `quoted` node containing the QUOTE token and the following sexpr.
-- A Lisp evaluator that walks this AST will then expand `quoted` nodes into
-- `(quote ...)` calls at evaluation time.
--
-- # Architecture
--
-- 1. **Tokenize** — call `lisp_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — require the pre-compiled `_grammar` module and call
--    its `parser_grammar()` function to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # Grammar source
--
-- The parser grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `lisp.grammar` is pre-compiled (via
-- `grammar-tools compile-grammar`) into `_grammar.lua`, a plain Lua module
-- that embeds the ParserGrammar as native Lua data structures. That module
-- ships as part of this package, so `require()` always finds it.

local lisp_lexer = require("coding_adventures.lisp_lexer")
local parser_pkg = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `parser_grammar()` result cached in a module-level variable. Repeated
-- calls to `parse()` or `create_parser()` reuse the cached grammar.

local _grammar_cache = nil

--- Return the (cached) ParserGrammar for Lisp.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `parser_grammar()`. On subsequent calls, returns the cached
-- ParserGrammar object immediately.
-- @return ParserGrammar  The Lisp parser grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.lisp_parser._grammar")
    _grammar_cache = compiled.parser_grammar()
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Lisp source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `lisp_lexer.tokenize`.
--   2. Loads the Lisp parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- Lisp grammar).  A program contains zero or more `sexpr` children.
--
-- @param source string  The Lisp text to parse.
-- @return ASTNode       Root of the AST (rule_name == "program").
-- @error                Raises an error on lexer or parser failure.
--
-- Examples:
--
--   local lisp_parser = require("coding_adventures.lisp_parser")
--
--   -- Parse a single expression
--   local ast = lisp_parser.parse("(+ 1 2)")
--   -- ast.rule_name → "program"
--
--   -- Parse a multi-expression program
--   local ast = lisp_parser.parse("(define x 42) (display x)")
--   -- ast.rule_name  → "program"
--   -- #ast.children  → 2 sexpr nodes
function M.parse(source)
    local tokens  = lisp_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp      = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("lisp_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Lisp source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Lisp text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = lisp_parser.create_parser("(+ 1 2)")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens  = lisp_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Lisp.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or to check how many rules the grammar has.
--
-- @return ParserGrammar  The parsed Lisp parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
