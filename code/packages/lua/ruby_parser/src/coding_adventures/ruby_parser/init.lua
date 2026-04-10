-- ruby_parser -- Builds an AST from Ruby text using the grammar-driven engine
-- ================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer alongside javascript_parser, python_parser,
-- and json_parser, above the lexer, grammar_tools, and ruby_lexer packages.
--
-- # What does a Ruby parser do?
--
-- A lexer breaks raw Ruby source into a flat token stream:
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
-- # Ruby grammar
--
-- The Ruby grammar is defined in `code/grammars/ruby.grammar`.
-- The grammar covers a focused subset:
--
--   program      = { statement } ;
--   statement    = assignment | method_call | expression_stmt ;
--   assignment   = NAME EQUALS expression ;
--   method_call  = ( NAME | KEYWORD ) LPAREN [ expression { COMMA expression } ] RPAREN ;
--   expression_stmt = expression ;
--   expression   = term { ( PLUS | MINUS ) term } ;
--   term         = factor { ( STAR | SLASH ) factor } ;
--   factor       = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
--
-- This grammar handles:
--   - Assignments: x = 5
--   - Method calls: puts("hello"), Math.sqrt(4)
--   - Arithmetic expressions: 1 + 2 * 3 (respects precedence via term/factor)
--   - Parenthesized groups: (a + b) * c
--   - Expression statements: just an expression on its own line
--   - Keywords as expressions (e.g., true, false, nil)
--
-- # Architecture
--
-- 1. **Tokenize** — call `ruby_lexer.tokenize(source)` to get a token list.
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
-- The grammar encodes Ruby operator precedence through rule layering:
--
--   expression  → handles + and - (lowest precedence)
--   term        → handles * and / (higher precedence)
--   factor      → literals, names, keywords, parenthesized expressions (highest)
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
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/ruby_parser/src/coding_adventures/ruby_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   ruby_parser/    (1)
--   coding_adventures/  (2)
--   src/                (3)
--   ruby_parser/    (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/ruby.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local ruby_lexer    = require("coding_adventures.ruby_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by javascript_parser, python_parser,
-- and json_parser.  We navigate up 6 levels to reach `code/`, then descend
-- into `grammars/ruby.grammar`.

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
    -- Security: Do not pass the dir string to io.popen (shell injection risk).
    -- Instead, use os.getenv to resolve relative paths -- no subprocess or
    -- shell invocation is involved. The previously removed pattern
    --   io.popen("cd '" .. dir .. "' 2>/dev/null && pwd")
    -- was unsafe because dir could contain shell metacharacters.
    -- Fixed: 2026-04-10 security review.
    if dir:sub(1, 1) ~= "/" and dir:sub(2, 2) ~= ":" then
        local cwd = os.getenv("PWD") or os.getenv("CD") or ""
        if cwd ~= "" then
            dir = cwd:gsub("\\\\", "/"):gsub("%c+$", "") .. "/" .. dir
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

--- Load and parse `ruby.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed Ruby parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/ruby.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "ruby_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "ruby_parser: failed to parse ruby.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Ruby source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `ruby_lexer.tokenize`.
--   2. Loads the Ruby parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- Ruby grammar).
--
-- The grammar supports:
--   - Assignments: `x = 5`
--   - Method calls: `puts("hello")`
--   - Arithmetic: `1 + 2 * 3` (correct precedence via term/factor layering)
--   - Parenthesized expressions: `(a + b) * c`
--   - Keywords as expressions: `true`, `false`, `nil`
--   - Expression statements
--
-- @param source string  The Ruby text to parse.
-- @return ASTNode       Root of the AST.
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local ruby_parser = require("coding_adventures.ruby_parser")
--   local ast = ruby_parser.parse("x = 5")
--   -- ast.rule_name  → "program"
--   -- contains statement → assignment
function M.parse(source)
    local tokens = ruby_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("ruby_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Ruby source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Ruby text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = ruby_parser.create_parser("x = 1")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = ruby_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Ruby.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed Ruby parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
