-- starlark_parser -- Builds an AST from Starlark text using the grammar-driven engine
-- =======================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up implementation
-- of the computing stack from transistors to operating systems. It sits in the
-- language-tooling layer alongside json_parser, sql_parser, and javascript_parser,
-- above the lexer, grammar_tools, and starlark_lexer packages.
--
-- # What is Starlark?
--
-- Starlark is a deterministic subset of Python used for configuration files,
-- most famously in Bazel BUILD files. It looks like Python but with important
-- constraints that guarantee termination and deterministic evaluation:
--
--   - No while loops (all iteration is over finite collections)
--   - No classes or class definitions
--   - No try/except/raise
--   - No global/nonlocal
--   - No side effects at module level (besides assignments)
--   - Recursion is disabled (functions cannot call themselves)
--
-- These constraints make Starlark files safe to evaluate in a build system:
-- every file terminates, and repeated evaluation always produces the same result.
--
-- # What does a Starlark parser do?
--
-- The lexer breaks raw Starlark source into a flat token stream:
--
--   'x = 1 + 2'
--   →  NAME("x") EQUALS("=") INT("1") PLUS("+") INT("2") NEWLINE EOF
--
-- The parser takes that flat stream and builds a tree that captures the
-- *structure* of the program:
--
--   file
--   └── statement
--       └── simple_stmt
--           └── small_stmt
--               └── assign_stmt
--                   ├── expression_list
--                   │   └── expression → or_expr → … → atom → NAME "x"
--                   ├── assign_op → EQUALS "="
--                   └── expression_list
--                       └── expression → arith
--                           ├── term → factor → power → primary → atom → INT "1"
--                           ├── PLUS "+"
--                           └── term → factor → power → primary → atom → INT "2"
--
-- This tree is called an Abstract Syntax Tree (AST). Downstream tools
-- (evaluators, BUILD file interpreters, type checkers) walk the AST.
--
-- # Starlark grammar
--
-- The Starlark grammar is defined in `code/grammars/starlark.grammar`.
-- Key rules:
--
--   file         = { NEWLINE | statement } ;
--   statement    = compound_stmt | simple_stmt ;
--   simple_stmt  = small_stmt { SEMICOLON small_stmt } NEWLINE ;
--   small_stmt   = return_stmt | break_stmt | continue_stmt | pass_stmt
--                | load_stmt | assign_stmt ;
--   compound_stmt = if_stmt | for_stmt | def_stmt ;
--   def_stmt     = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;
--   if_stmt      = "if" expression COLON suite
--                  { "elif" expression COLON suite }
--                  [ "else" COLON suite ] ;
--   for_stmt     = "for" loop_vars "in" expression COLON suite ;
--   expression   = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;
--
-- # Architecture
--
-- 1. **Tokenize** — call `starlark_lexer.tokenize(source)` to get a token list.
--    The starlark_lexer handles indentation-mode, emitting INDENT, DEDENT, and
--    NEWLINE tokens that structure the token stream for block detection.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)`
--    to get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package)
--    and call `:parse()`. The engine interprets the grammar rules against
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
--   node.rule_name  — which grammar rule produced this node ("file", …)
--   node.children   — array of child ASTNodes and/or Token tables
--   node:is_leaf()  — true when the node wraps exactly one token
--   node:token()    — the wrapped token (only valid when is_leaf() is true)
--
-- # Starlark vs Python — key differences
--
-- | Feature           | Python | Starlark |
-- |-------------------|--------|----------|
-- | while loops       | yes    | no       |
-- | classes           | yes    | no       |
-- | try/except        | yes    | no       |
-- | recursion         | yes    | no       |
-- | global/nonlocal   | yes    | no       |
-- | lambdas           | yes    | yes      |
-- | comprehensions    | yes    | yes      |
-- | augmented assign  | yes    | yes      |
-- | load statement    | no     | yes      |
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/starlark_parser/src/coding_adventures/starlark_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   starlark_parser/  (1)
--   coding_adventures/  (2)
--   src/                (3)
--   starlark_parser/    (4) — the package directory
--   lua/                (5)
--   packages/           (6)
--   code/               → then /grammars/starlark.grammar

local grammar_tools   = require("coding_adventures.grammar_tools")
local starlark_lexer  = require("coding_adventures.starlark_lexer")
local parser_pkg      = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by json_parser, toml_parser,
-- sql_parser, and javascript_parser. We navigate up 6 levels to reach
-- `code/`, then descend into `grammars/starlark.grammar`.

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
-- The parser grammar is loaded from disk once and cached. Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar, avoiding
-- repeated file I/O and repeated rule compilation.
--
-- The grammar file is `code/grammars/starlark.grammar`. This is the same
-- format used by javascript.grammar, json.grammar, etc. — a custom BNF
-- dialect understood by `grammar_tools.parse_parser_grammar`.

local _grammar_cache = nil

--- Load and parse `starlark.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed Starlark parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/starlark.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "starlark_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "starlark_parser: failed to parse starlark.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Starlark source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `starlark_lexer.tokenize`.
--      The lexer is in indentation mode: it emits INDENT, DEDENT, and
--      NEWLINE tokens that encode block structure.
--   2. Loads the Starlark parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "file"` (the first rule in the
-- Starlark grammar).
--
-- The grammar supports:
--   - Simple assignments: `x = 1`
--   - Augmented assignments: `x += 1`
--   - Function calls: `print("hello")`
--   - Function definitions: `def foo(x): return x + 1`
--   - If/elif/else statements
--   - For loops: `for item in items:`
--   - Return/break/continue/pass
--   - Load statements: `load("//rules.star", "cc_library")`
--   - List literals: `[1, 2, 3]`
--   - Dict literals: `{"key": "value"}`
--   - Lambda expressions: `lambda x: x + 1`
--   - Comprehensions: `[x * 2 for x in items]`
--   - Tuple unpacking: `a, b = 1, 2`
--   - BUILD file patterns: `cc_library(name="foo", srcs=["foo.cc"])`
--
-- @param source string  The Starlark text to parse.
-- @return ASTNode       Root of the AST (rule_name == "file").
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local starlark_parser = require("coding_adventures.starlark_parser")
--   local ast = starlark_parser.parse('x = 1\n')
--   -- ast.rule_name  → "file"
--
--   -- Parse a BUILD file fragment
--   local build_ast = starlark_parser.parse('cc_library(name="foo", srcs=["foo.cc"])\n')
--   -- build_ast.rule_name  → "file"
function M.parse(source)
    local tokens = starlark_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("starlark_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a Starlark source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to
-- use trace mode or to inspect the token stream before parsing.
--
-- @param source string   The Starlark text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = starlark_parser.create_parser("x = 1\n")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = starlark_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Starlark.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or check the grammar structure.
--
-- @return ParserGrammar  The parsed Starlark parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
