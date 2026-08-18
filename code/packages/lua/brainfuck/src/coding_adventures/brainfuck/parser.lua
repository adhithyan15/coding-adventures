-- brainfuck.parser — Builds an AST from Brainfuck text using the grammar-driven engine
-- =====================================================================================
--
-- This module is the parsing layer for the Brainfuck front-end pipeline.
-- It sits above the lexer:
--
--   brainfuck.lexer   → flat token list
--          |
--          v
--   brainfuck.parser  → structured AST
--
-- # What does parsing add over tokenization?
--
-- The lexer turns a flat string into a flat list of tokens:
--
--   "++[>+<-]"  →  INC INC LOOP_START RIGHT INC LEFT DEC LOOP_END EOF
--
-- The parser turns that flat list into a tree capturing the *structure*:
--
--   program
--     instruction → command(INC)
--     instruction → command(INC)
--     instruction → loop
--       LOOP_START
--       instruction → command(RIGHT)
--       instruction → command(INC)
--       instruction → command(LEFT)
--       instruction → command(DEC)
--       LOOP_END
--
-- This tree is what downstream tools (interpreters, code generators,
-- visualizers) work with. Walking the tree is much cleaner than manually
-- tracking bracket depth during interpretation.
--
-- # Grammar (from brainfuck.grammar)
--
--   program     = { instruction } ;
--   instruction = loop | command ;
--   loop        = LOOP_START { instruction } LOOP_END ;
--   command     = RIGHT | LEFT | INC | DEC | OUTPUT | INPUT ;
--
-- There are exactly 4 rules. The grammar is recursive: loop contains
-- { instruction }, and instruction can be a loop again. This handles
-- arbitrarily deep nesting.
--
-- # ASTNode fields
--
--   node.rule_name   — which grammar rule produced this node
--                      ("program", "instruction", "loop", "command")
--   node.children    — array of child ASTNodes and/or Token tables
--   node:is_leaf()   — true when the node wraps exactly one token
--   node:token()     — the wrapped token (only valid when is_leaf() is true)
--
-- # Grammar source
--
-- The parser grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `brainfuck.grammar` is pre-compiled (via
-- `grammar-tools compile-grammar`) into `_grammar_parser.lua`, a plain Lua
-- module that embeds the ParserGrammar as native Lua data structures. That
-- module ships as part of this package, so `require()` always finds it.

local brainfuck_lexer = require("coding_adventures.brainfuck.lexer")
local parser_pkg      = require("coding_adventures.parser")

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

--- Return the (cached) ParserGrammar for Brainfuck.
-- On the first call, requires the pre-compiled `_grammar_parser` module
-- and invokes `parser_grammar()`. Subsequent calls return the cache.
-- @return ParserGrammar  The Brainfuck parser grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.brainfuck._grammar_parser")
    _grammar_cache = compiled.parser_grammar()
    return _grammar_cache
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Brainfuck source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `brainfuck_lexer.tokenize`.
--   2. Loads the Brainfuck parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in
-- the Brainfuck grammar). Its children are `instruction` nodes.
--
-- @param source string  The Brainfuck text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on parser failure (unmatched brackets).
--
-- Example:
--
--   local bf_parser = require("coding_adventures.brainfuck.parser")
--   local ast = bf_parser.parse("++[>+<-]")
--   -- ast.rule_name  → "program"
function M.parse(source)
    local tokens = brainfuck_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("brainfuck.parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Brainfuck source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Brainfuck text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = bf_parser.create_parser("++[>+<-]")
--   local ast, err = p:parse()
--   if err then error(err) end
function M.create_parser(source)
    local tokens = brainfuck_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Brainfuck.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check how many rules the grammar has.
--
-- @return ParserGrammar  The parsed Brainfuck parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
