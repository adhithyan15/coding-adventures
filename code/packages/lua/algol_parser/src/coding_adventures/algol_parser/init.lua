-- algol_parser -- Builds an AST from ALGOL 60 text using the grammar-driven engine
-- =================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer, above the lexer and grammar_tools
-- packages and alongside other language parsers.
--
-- # What does a parser do?
--
-- A lexer breaks raw text into a flat stream of tokens:
--
--   'begin integer x; x := 42 end'
--   →  BEGIN INTEGER IDENT SEMICOLON IDENT ASSIGN INTEGER_LIT END EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the input:
--
--   program
--   └── block
--       ├── BEGIN          "begin"
--       ├── declaration
--       │   └── type_decl
--       │       ├── type
--       │       │   └── INTEGER  "integer"
--       │       └── ident_list
--       │           └── IDENT    "x"
--       ├── SEMICOLON      ";"
--       ├── statement
--       │   └── unlabeled_stmt
--       │       └── assign_stmt
--       │           ├── left_part
--       │           │   ├── variable → IDENT "x"
--       │           │   └── ASSIGN   ":="
--       │           └── expression
--       │               └── arith_expr → INTEGER_LIT "42"
--       └── END            "end"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (interpreters, code generators, analyzers) walk the AST.
--
-- # Grammar
--
-- The ALGOL 60 grammar is defined in `code/grammars/algol.grammar`.
-- Entry point: `program`, which is a single `block`.
--
-- Key features of ALGOL 60 that make its grammar interesting:
--   - Block structure with declarations before statements
--   - Dangling-else resolved at grammar level (unlabeled_stmt excludes cond_stmt)
--   - Exponentiation is LEFT-associative: 2^3^4 = (2^3)^4 = 4096
--   - Call-by-name is the default parameter passing mode
--   - Arrays can have dynamic (runtime-evaluated) bounds
--
-- # Architecture
--
-- 1. **Tokenize** — call `algol_lexer.tokenize(source)` to get a token list.
-- 2. **Load grammar** — require the pre-compiled `_grammar` module and call
--    its `parser_grammar()` function to get a `ParserGrammar` with `.rules`.
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
--   node.rule_name   — which grammar rule produced this node ("program", "block", …)
--   node.children    — array of child ASTNodes and/or Token tables
--   node:is_leaf()   — true when the node wraps exactly one token
--   node:token()     — the wrapped token (only valid when is_leaf() is true)
--
-- # Grammar source
--
-- The parser grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `algol60.grammar` is pre-compiled (via
-- `grammar-tools compile-grammar`) into `_grammar.lua`, a plain Lua module
-- that embeds the ParserGrammar as native Lua data structures. That module
-- ships as part of this package, so `require()` always finds it.

local algol_lexer = require("coding_adventures.algol_lexer")
local parser_pkg  = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `parser_grammar()` result cached in a module-level table. Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar.

local _grammar_cache = {}

local function normalize_version(version)
    if version == nil or version == "" then
        return "algol60"
    end
    if version ~= "algol60" then
        error("algol_parser: unknown ALGOL version '" .. tostring(version) .. "' (valid: algol60)")
    end
    return version
end

--- Return the (cached) ParserGrammar for the given ALGOL version.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `parser_grammar()`.  On subsequent calls, returns the cached
-- ParserGrammar object immediately.
-- @return ParserGrammar  The ALGOL 60 parser grammar.
local function get_grammar(version)
    version = normalize_version(version)
    if _grammar_cache[version] then
        return _grammar_cache[version]
    end

    local compiled = require("coding_adventures.algol_parser._grammar")
    _grammar_cache[version] = compiled.parser_grammar()
    return _grammar_cache[version]
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse an ALGOL 60 source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `algol_lexer.tokenize`.
--   2. Loads the ALGOL 60 parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- ALGOL 60 grammar).
--
-- @param source string  The ALGOL 60 text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local algol_parser = require("coding_adventures.algol_parser")
--   local ast = algol_parser.parse("begin integer x; x := 42 end")
--   -- ast.rule_name  → "program"
--   -- find a block node inside
function M.parse(source, version)
    local tokens = algol_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("algol_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for an ALGOL 60 source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The ALGOL 60 text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = algol_parser.create_parser("begin integer x; x := 0 end")
--   local ast, err = p:parse()
function M.create_parser(source, version)
    local tokens = algol_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for ALGOL 60.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or to check how many rules the grammar has.
--
-- @return ParserGrammar  The parsed ALGOL 60 parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
