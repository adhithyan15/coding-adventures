-- dartmouth_basic_parser — Builds an AST from BASIC source using the grammar-driven engine
-- ==========================================================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in the language-tooling layer, above the dartmouth_basic_lexer and
-- grammar_tools packages.
--
-- # Historical context: 1964 Dartmouth BASIC
--
-- Dartmouth BASIC was designed by John Kemeny and Thomas Kurtz at Dartmouth
-- College in 1964. Their goal was radical for the time: give humanities
-- students — not just computer scientists — access to computing via the
-- college's time-sharing GE-225 mainframe. Every student could submit programs
-- from a teletype terminal and receive results within seconds.
--
-- The language was deliberately simple:
--
--   - All statements are LINE-NUMBERED (10, 20, 30, ...). You can type them
--     in any order; the system sorts them before running.
--   - Variables are single uppercase letters (A–Z) or a letter plus a digit
--     (A0–Z9). No declaration needed — all variables start at 0.
--   - Arithmetic expressions use standard precedence: ^ > unary − > * / > + −
--   - Control flow is GOTO and GOSUB/RETURN. No block structure.
--   - Input/output: PRINT to teletype, INPUT from teletype.
--   - The only data structure is a one-dimensional array (DIM).
--
-- Example program:
--
--   10 LET X = 1
--   20 PRINT X
--   30 LET X = X + 1
--   40 IF X <= 10 THEN 20
--   50 END
--
-- # What does a parser do?
--
-- A lexer breaks raw BASIC text into a flat stream of tokens:
--
--   "10 LET X = 5\n"  →  LINE_NUM KEYWORD(LET) NAME(X) EQ NUMBER(5) NEWLINE
--
-- A parser takes that flat stream and builds a tree (AST) that captures the
-- *grammatical structure* of the input:
--
--   program
--   └── line
--       ├── LINE_NUM "10"
--       ├── statement
--       │   └── let_stmt
--       │       ├── KEYWORD "LET"
--       │       ├── variable → NAME "X"
--       │       ├── EQ "="
--       │       └── expr → term → power → unary → primary → NUMBER "5"
--       └── NEWLINE
--
-- This tree is called an Abstract Syntax Tree (AST). A downstream compiler or
-- interpreter walks the AST rather than re-parsing the text.
--
-- # Grammar
--
-- The BASIC grammar is defined in `code/grammars/dartmouth_basic.grammar`:
--
--   program  = { line } ;
--   line     = LINE_NUM [ statement ] NEWLINE ;
--   statement = let_stmt | print_stmt | ... | def_stmt ;
--   ...
--   expr  = term { ( PLUS | MINUS ) term } ;
--   term  = power { ( STAR | SLASH ) power } ;
--   power = unary [ CARET power ] ;
--   unary = MINUS primary | primary ;
--   primary = NUMBER | BUILTIN_FN LPAREN expr RPAREN | ... ;
--
-- There are 29 rules total. The grammar is recursive: `primary` references
-- `variable` which may reference `expr`, allowing `A(I+1)` subscripts.
--
-- # Architecture
--
-- 1. **Tokenize** — call `dartmouth_basic_lexer.tokenize(source)` to get a
--    token list. The lexer handles line-number relabelling and REM suppression.
-- 2. **Load grammar** — call `grammar_tools.parse_parser_grammar(content)` to
--    get a `ParserGrammar` with `.rules`.
-- 3. **Parse** — construct a `GrammarParser` (from the `parser` package) and
--    call `:parse()`. The engine interprets the grammar rules against the
--    token stream, producing an AST.
--
-- # GrammarParser and ASTNode
--
-- `GrammarParser.new(tokens, grammar)` returns a parser instance.
-- Calling `:parse()` returns either:
--   (node, nil)    — success; `node` is the root ASTNode
--   (nil, errmsg)  — failure; `errmsg` is a human-readable error string
--
-- ASTNode fields:
--   node.rule_name   — which grammar rule produced this node
--   node.children    — array of child ASTNodes and/or Token tables
--   node:is_leaf()   — true when the node wraps exactly one token
--   node:token()     — the wrapped token (only valid when is_leaf() is true)
--
-- # Path navigation
--
-- This file lives at:
--   code/packages/lua/dartmouth_basic_parser/src/coding_adventures/dartmouth_basic_parser/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping the prefix and walking up 6 levels reaches `code/`, the repo root.
--
-- Directory structure from script_dir upward:
--   dartmouth_basic_parser/  (1)
--   coding_adventures/       (2)
--   src/                     (3)
--   dartmouth_basic_parser/  (4) — the package directory
--   lua/                     (5)
--   packages/                (6)
--   code/                    → then /grammars/dartmouth_basic.grammar

local grammar_tools = require("coding_adventures.grammar_tools")
local basic_lexer   = require("coding_adventures.dartmouth_basic_lexer")
local parser_pkg    = require("coding_adventures.parser")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================
--
-- These helpers mirror the pattern used by json_parser (which navigates to
-- json.grammar). We do the same to reach dartmouth_basic.grammar.

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
-- up() steps, so the grammar file cannot be found. We resolve to an
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
    -- If already absolute (Unix: starts with "/"; Windows: "X:") return as-is.
    if dir:sub(1, 1) == "/" or dir:sub(2, 2) == ":" then
        return dir
    end
    -- The path is relative. Resolve to absolute by fetching the current working
    -- directory with no arguments — we NEVER interpolate `dir` into the shell
    -- command string, which would risk command injection if the path contained
    -- shell metacharacters (single quotes, semicolons, backticks, etc.).
    -- Instead we obtain the cwd cleanly and join with the relative path in Lua.
    local is_win = package.config:sub(1, 1) == "\\"
    local f
    if is_win then
        f = io.popen("cd 2>nul")          -- "cd" with no args prints cwd on Windows
    else
        f = io.popen("pwd 2>/dev/null")   -- "pwd" with no args is always safe
    end
    local cwd = f and f:read("*l")
    if f then f:close() end
    if cwd and cwd ~= "" then
        cwd = cwd:gsub("\\", "/"):gsub("%c+$", "")
        -- Combine cwd + relative dir in Lua string space (no shell involvement).
        if dir == "." then
            return cwd
        end
        return cwd .. "/" .. dir
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
-- The parser grammar is loaded from disk once and cached. Repeated calls to
-- `parse()` or `create_parser()` reuse the cached grammar, avoiding repeated
-- file I/O and repeated rule compilation.
--
-- Caching matters here because the BASIC grammar has 29 rules, each of which
-- must be compiled into the grammar engine's internal representation. Doing
-- that work once and reusing it keeps parse() calls fast.

local _grammar_cache = nil

--- Load and parse `dartmouth_basic.grammar`, with caching.
-- On the first call, opens the file, parses it with
-- `grammar_tools.parse_parser_grammar`, and caches the result.
-- @return ParserGrammar  The parsed BASIC parser grammar.
-- @error                 Raises an error if the file cannot be opened or parsed.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate: 6 levels up from this file's directory → code/ root.
    -- Breakdown:
    --   init.lua lives in: .../dartmouth_basic_parser/     (1 up → coding_adventures/)
    --                       .../coding_adventures/          (2 up → src/)
    --                       .../src/                        (3 up → dartmouth_basic_parser/)
    --                       .../dartmouth_basic_parser/     (4 up → lua/)
    --                       .../lua/                        (5 up → packages/)
    --                       .../packages/                   (6 up → code/)
    --                       .../code/                       ← repo root
    local script_dir   = get_script_dir()
    local repo_root    = up(script_dir, 6)
    local grammar_path = repo_root .. "/grammars/dartmouth_basic.grammar"

    local f, open_err = io.open(grammar_path, "r")
    if not f then
        error(
            "dartmouth_basic_parser: cannot open grammar file: " .. grammar_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_parser_grammar(content)
    if not grammar then
        error(
            "dartmouth_basic_parser: failed to parse dartmouth_basic.grammar: " ..
            (parse_err or "unknown error")
        )
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Parse a Dartmouth BASIC source string and return the root ASTNode.
--
-- Internally:
--   1. Tokenizes `source` using `dartmouth_basic_lexer.tokenize`. The lexer
--      normalises to uppercase, relabels line numbers, and suppresses REM
--      comment tokens.
--   2. Loads the BASIC parser grammar (cached after the first call).
--   3. Runs the grammar-driven `GrammarParser` on the token stream.
--   4. Returns the AST root on success, or raises an error on failure.
--
-- The root node will have `rule_name == "program"` (the first rule in the
-- BASIC grammar).
--
-- A "program" is a sequence of "line" nodes, each starting with a LINE_NUM
-- and ending with a NEWLINE. An empty string produces a program node with
-- no line children.
--
-- @param source string  The Dartmouth BASIC source text to parse.
-- @return ASTNode       Root of the AST with rule_name "program".
-- @error                Raises an error on lexer or parser failure.
--
-- Example:
--
--   local bp = require("coding_adventures.dartmouth_basic_parser")
--   local ast = bp.parse("10 LET X = 5\n20 PRINT X\n30 END\n")
--   -- ast.rule_name  → "program"
--   -- #ast.children  → 3  (three "line" nodes)
function M.parse(source)
    local tokens = basic_lexer.tokenize(source)
    local grammar = get_grammar()
    local gp = parser_pkg.GrammarParser.new(tokens, grammar)
    local ast, err = gp:parse()
    if not ast then
        error("dartmouth_basic_parser: " .. (err or "parse failed"))
    end
    return ast
end

--- Create a GrammarParser for a BASIC source string without immediately parsing.
--
-- Use this when you want to control parsing yourself — for example, to use
-- trace mode (`GrammarParser.new_with_trace`) or to inspect the token stream
-- before parsing.
--
-- @param source string   The BASIC text to tokenize.
-- @return GrammarParser  An initialized parser, ready to call `:parse()`.
--
-- Example:
--
--   local p = dartmouth_basic_parser.create_parser("10 LET X = 1\n")
--   local ast, err = p:parse()
function M.create_parser(source)
    local tokens = basic_lexer.tokenize(source)
    local grammar = get_grammar()
    return parser_pkg.GrammarParser.new(tokens, grammar)
end

--- Return the cached (or freshly loaded) ParserGrammar for Dartmouth BASIC.
--
-- Exposed so callers can inspect the grammar rules directly — for example,
-- to enumerate rule names or to check how many rules the grammar has.
-- The BASIC grammar has 29 rules.
--
-- @return ParserGrammar  The parsed BASIC parser grammar.
function M.get_grammar()
    return get_grammar()
end

return M
