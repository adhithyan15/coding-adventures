-- javascript_parser -- Builds an AST from JavaScript text using the grammar-driven engine
-- ========================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside sql_parser, json_parser, and
-- toml_parser, above the lexer, grammar_tools, and javascript_lexer packages.
--
-- # What does a JavaScript parser do?
--
-- A lexer breaks raw JavaScript source into a flat token stream:
--
--   'var x = 5;'
--   →  KEYWORD("var") NAME("x") EQUALS("=") NUMBER("5") SEMICOLON(";") EOF
--
-- A parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   program
--   └── statement
--       └── var_declaration
--           ├── KEYWORD  "var"
--           ├── NAME     "x"
--           ├── EQUALS   "="
--           ├── expression
--           │   └── term
--           │       └── factor
--           │           └── NUMBER  "5"
--           └── SEMICOLON  ";"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, transpilers, linters) walk the AST rather than re-parsing.
--
-- # JavaScript grammar
--
-- The JavaScript grammar is defined in `code/grammars/javascript.grammar`.
-- The grammar covers a focused subset:
--
--   program        = { statement } ;
--   statement      = var_declaration | assignment | expression_stmt ;
--   var_declaration = KEYWORD NAME EQUALS expression SEMICOLON ;
--   assignment     = NAME EQUALS expression SEMICOLON ;
--   expression_stmt = expression SEMICOLON ;
--   expression     = term { ( PLUS | MINUS ) term } ;
--   term           = factor { ( STAR | SLASH ) factor } ;
--   factor         = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
--
-- This grammar handles:
--   - Variable declarations:  var x = 5;  let y = "hello";  const z = true;
--   - Assignments:            x = 10;
--   - Arithmetic expressions: 1 + 2 * 3  (respects precedence via term/factor)
--   - Parenthesized groups:   (a + b) * c
--   - Expression statements:  f(x);   (as NAME LPAREN … RPAREN — lexed as NAME)
--
-- # Architecture
--
-- 1. **Tokenize** — call `javascript_lexer.tokenize(source)` to get a token list.
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
-- The grammar encodes JavaScript operator precedence through rule layering:
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
local javascript_lexer = require("coding_adventures.javascript_lexer")
local parser_pkg       = require("coding_adventures.parser")

-- =========================================================================
-- Compiled grammars
-- =========================================================================
--
-- Historically this module read `.grammar` files from `code/grammars/` at
-- runtime via `io.open`, walking outside this package's own directory into
-- the monorepo. That works when running inside the monorepo checkout, but
-- a published LuaRocks package does not ship `code/grammars/`, so
-- `luarocks install` + first use would raise a file-not-found error.
--
-- Instead, each grammar is now pre-compiled to a native Lua data structure
-- (via `code/programs/lua/grammar-tools`) and checked in as a sibling
-- `_grammar_<version>.lua` file, `require`d like any other module. No
-- runtime file I/O, no path traversal outside the package.

local compiled_grammars = {
    [""]       = require("coding_adventures.javascript_parser._grammar_default"),
    ["es1"]    = require("coding_adventures.javascript_parser._grammar_es1"),
    ["es3"]    = require("coding_adventures.javascript_parser._grammar_es3"),
    ["es5"]    = require("coding_adventures.javascript_parser._grammar_es5"),
    ["es2015"] = require("coding_adventures.javascript_parser._grammar_es2015"),
    ["es2016"] = require("coding_adventures.javascript_parser._grammar_es2016"),
    ["es2017"] = require("coding_adventures.javascript_parser._grammar_es2017"),
    ["es2018"] = require("coding_adventures.javascript_parser._grammar_es2018"),
    ["es2019"] = require("coding_adventures.javascript_parser._grammar_es2019"),
    ["es2020"] = require("coding_adventures.javascript_parser._grammar_es2020"),
    ["es2021"] = require("coding_adventures.javascript_parser._grammar_es2021"),
    ["es2022"] = require("coding_adventures.javascript_parser._grammar_es2022"),
    ["es2023"] = require("coding_adventures.javascript_parser._grammar_es2023"),
    ["es2024"] = require("coding_adventures.javascript_parser._grammar_es2024"),
    ["es2025"] = require("coding_adventures.javascript_parser._grammar_es2025"),
}

local M = {}
M.VERSION = "0.2.0"

-- =========================================================================
-- Valid ECMAScript / JavaScript versions
-- =========================================================================
--
-- Each version maps to grammar files under code/grammars/ecmascript/.
--
-- When version is nil or "" → loads code/grammars/javascript.grammar (generic)
-- When version is "es2015"  → loads code/grammars/ecmascript/es2015.grammar
--
-- Recognized versions: es1, es3, es5, es2015..es2025

local VALID_JS_VERSIONS = {
    ["es1"]    = true, ["es3"]    = true, ["es5"]    = true,
    ["es2015"] = true, ["es2016"] = true, ["es2017"] = true,
    ["es2018"] = true, ["es2019"] = true, ["es2020"] = true,
    ["es2021"] = true, ["es2022"] = true, ["es2023"] = true,
    ["es2024"] = true, ["es2025"] = true,
}

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The parser grammar is loaded once per version and cached.  Repeated
-- calls to `parse()` or `create_parser()` reuse the cached grammar,
-- avoiding repeated construction of the same rule tables.

-- Cache keyed by version string (or "" for generic).
local _grammar_cache = {}

--- Resolve a version string to its entry in `compiled_grammars`.
--
-- @param version string|nil  ECMAScript version tag, or nil/"" for generic.
-- @return string             The resolved lookup key into `compiled_grammars`.
local function resolve_version(version)
    if not version or version == "" then
        return ""
    end

    if not VALID_JS_VERSIONS[version] then
        error(
            "javascript_parser: unknown ECMAScript version '" .. version .. "'. " ..
            "Valid values are: es1, es3, es5, es2015..es2025, or nil/\"\" for generic."
        )
    end

    return version
end

--- Load and parse the grammar for a specific version, with per-version caching.
--
-- @param version string|nil  ECMAScript version tag (see resolve_version).
-- @return ParserGrammar      The parsed JavaScript parser grammar.
local function get_grammar(version)
    local key = resolve_version(version)
    if _grammar_cache[key] then
        return _grammar_cache[key]
    end

    local grammar = compiled_grammars[key].parser_grammar()
    _grammar_cache[key] = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a JavaScript source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `javascript_lexer.tokenize`.
--   2. Loads the JavaScript parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- JavaScript grammar).
--
-- The grammar supports:
--   - var/let/const declarations: `var x = 5;`
--   - Assignments: `x = 10;`
--   - Arithmetic: `1 + 2 * 3` (correct precedence via term/factor layering)
--   - Parenthesized expressions: `(a + b) * c`
--   - Expression statements
--
-- @param source  string       The JavaScript text to parse.
-- @param version string|nil   ECMAScript version: "es1", "es3", "es5",
--                             "es2015".."es2025", or nil/"" for generic.
-- @return ASTNode             Root of the AST.
-- @error                      Raises an error on lexer or parser failure.
--
-- Example (generic):
--
--   local javascript_parser = require("coding_adventures.javascript_parser")
--   local ast = javascript_parser.parse("var x = 5;")
--   -- ast.rule_name  → "program"
--
-- Example (versioned):
--
--   local ast = javascript_parser.parse("var x = 5;", "es2015")
function M.parse(source, version)
    local tokens = javascript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("javascript_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a JavaScript source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source  string       The JavaScript text to tokenize.
-- @param version string|nil   ECMAScript version tag (see parse for valid values).
-- @return GrammarParser       An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = javascript_parser.create_parser("var x = 1;", "es5")
--   local ast, err = p:parse()
function M.create_parser(source, version)
    local tokens = javascript_lexer.tokenize(source, version)
    local grammar = get_grammar(version)
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for JavaScript.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @param version string|nil  ECMAScript version tag (see parse for valid values).
-- @return ParserGrammar      The parsed JavaScript parser grammar.
function M.get_grammar(version)
    return get_grammar(version)
end

return M
