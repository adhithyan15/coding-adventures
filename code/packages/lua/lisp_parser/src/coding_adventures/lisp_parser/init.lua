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
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`.  The engine interprets the grammar rules against
--    the token stream, producing an AST.
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/lisp_parser/src/coding_adventures/lisp_parser/init.lua
--
-- Walking up 6 directory levels reaches `code/` (the repo root):
--
--   lisp_parser/        (1)
--   coding_adventures/  (2)
--   src/                (3)
--   lisp_parser/        (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/lisp.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local lisp_lexer    = require("coding_adventures.lisp_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by lisp_lexer (which navigates
-- to lisp.tokens).  We do the same to reach lisp.grammar.

--- Return the directory portion of a file path (no trailing slash).
-- Example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua prepends "@" to the source path in debug info — we strip it.
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
    if dir:sub(2, 2) ~= ":" then
        local f = io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
        local resolved = f and f:read("*l")
        if f then f:close() end
        if resolved and resolved ~= "" then
            return resolved
        end
    end
    return dir
end

--- Walk up `levels` directory levels from `path`.
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string
local function up(path, levels)
    local result = path
    for _ = 1, levels do
        result = dirname(result)
    end
    return result
end

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The parser grammar is loaded from disk once and cached.  Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar, avoiding
-- repeated file I/O and repeated rule compilation.

local _grammar_cache = nil

--- Load and parse `lisp.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed Lisp parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/lisp.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "lisp_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "lisp_parser: failed to parse lisp.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
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
