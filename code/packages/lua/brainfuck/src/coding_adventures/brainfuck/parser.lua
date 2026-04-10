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
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/brainfuck/src/coding_adventures/brainfuck/parser.lua
--
-- We use `debug.getinfo(1, "S").source` (prefixed with "@") and walk up 6
-- levels to reach `code/`, then descend into `grammars/brainfuck.grammar`.
--
-- Directory structure from script_dir upward:
--   brainfuck/           (1)  — this file's directory
--   coding_adventures/   (2)
--   src/                 (3)
--   brainfuck/           (4)  — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/brainfuck.grammar

local grammar_tools    = require("coding_adventures.grammar_tools")
local brainfuck_lexer  = require("coding_adventures.brainfuck.lexer")
local parser_pkg       = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- Identical to the pattern used by json_parser and brainfuck.lexer.

--- Return the directory portion of a file path (no trailing slash).
-- Example:  "/a/b/c/parser.lua"  →  "/a/b/c"
-- @param path string
-- @return string
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua prepends "@" to the source path in debug info — we strip it.
-- Resolves relative paths (containing "..") to absolute via shell.
-- @return string Absolute directory of this parser.lua file.
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
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
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
--
-- The parser grammar is loaded from disk once and cached. Repeated calls
-- to `parse()` or `create_parser()` reuse the cached grammar, avoiding
-- repeated file I/O and repeated rule compilation.

local _grammar_cache = nil

--- Load and parse `brainfuck.grammar`, with caching.
-- On the first call, opens the file and parses it with
-- `grammar_tools.parse_parser_grammar`. Subsequent calls return the cache.
-- @return ParserGrammar  The parsed Brainfuck parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/brainfuck.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "brainfuck.parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "brainfuck.parser: failed to parse brainfuck.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
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
