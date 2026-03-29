-- lisp_lexer -- Tokenizes Lisp source text using the grammar-driven infrastructure
-- ================================================================================
--
-- This package is part of the coding-adventures monorepo.  It tokenizes
-- Lisp/Scheme source text into a flat stream of typed tokens, powered by the
-- shared `lisp.tokens` grammar file and the `GrammarLexer` from the `lexer`
-- package.
--
-- # A Brief History of Lisp
--
-- Lisp (LISt Processing) was invented by John McCarthy at MIT in 1958, making
-- it the second-oldest high-level programming language still in widespread use
-- (after FORTRAN, 1957).  McCarthy's goal was to create a language based on
-- Alonzo Church's lambda calculus — a mathematical theory of computation
-- expressed entirely through function application.
--
-- Lisp introduced concepts that took decades to appear in mainstream languages:
--   • Garbage collection (1958)
--   • Higher-order functions (functions as values)
--   • Closures and lexical scope (Scheme, 1975)
--   • Macros and code-as-data (homoiconicity)
--   • Read-Eval-Print Loop (REPL) for interactive development
--   • Dynamic typing
--
-- # What is an S-expression?
--
-- The fundamental unit of Lisp syntax is the S-expression (symbolic expression).
-- An S-expression is either:
--
--   • An **atom**: a number, symbol, or string.
--       42       — integer
--       define   — symbol (identifier)
--       "hello"  — string literal
--
--   • A **list**: zero or more S-expressions enclosed in parentheses.
--       (+ 1 2)           — call the + function with 1 and 2
--       (define x 42)     — bind the name x to 42
--       (lambda (x) (* x x))  — anonymous function
--
-- Because both code and data are S-expressions, Lisp programs can manipulate
-- their own structure.  This property is called **homoiconicity** and is the
-- foundation of Lisp macros.
--
-- # Lisp token types
--
-- From `lisp.tokens`:
--
--   NUMBER  /-?[0-9]+/         — integer literals (e.g. 42, -7)
--   SYMBOL  /[a-zA-Z_+...]/    — identifiers and operators (e.g. define, +, ?)
--   STRING  /"([^"\\]|\\.)*"/  — string literals (e.g. "hello")
--   LPAREN  "("                — open list
--   RPAREN  ")"                — close list
--   QUOTE   "'"                — shorthand for (quote x): 'x ≡ (quote x)
--   DOT     "."                — cons cell separator: (a . b)
--
--   WHITESPACE  /[ \t\r\n]+/   — skipped (never emitted)
--   COMMENT     /;[^\n]*/      — line comments (skipped; never emitted)
--
-- # What is the DOT notation?
--
-- In Lisp, all lists are built from **cons cells** (pairs).  A cons cell is a
-- pair of two values: the **car** (head) and the **cdr** (tail).
--
-- A proper list like (1 2 3) is really:
--   (cons 1 (cons 2 (cons 3 nil)))
--
-- The DOT notation lets you write cons cells directly:
--   (1 . (2 . (3 . nil)))  ← same as (1 2 3)
--   (a . b)                ← an "improper" or dotted pair
--
-- DOT pairs are rare in everyday Lisp but important in the implementation of
-- association lists (alists) and in the internal representation of pairs.
--
-- # What is the QUOTE shorthand?
--
-- Normally, (+ 1 2) evaluates the symbol + and calls it as a function.
-- But what if we want the list (+ 1 2) as data, not code?  We use quote:
--
--   (quote (+ 1 2))   → the list (+ 1 2), unevaluated
--   '(+ 1 2)          → same thing — ' is syntactic sugar for (quote ...)
--
-- The QUOTE token (') is thus a reader macro: the lexer emits it as a single
-- token, and the parser expands `'x` into `(quote x)` during AST construction.
--
-- # Architecture
--
-- This module:
--   1. Locates the shared `lisp.tokens` grammar file in `code/grammars/`.
--   2. Reads and parses it once (cached) using `grammar_tools.parse_token_grammar`.
--   3. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   4. Returns the flat token list.
--
-- # Path navigation
--
-- The source file lives at:
--   code/packages/lua/lisp_lexer/src/coding_adventures/lisp_lexer/init.lua
--
-- `debug.getinfo(1, "S").source` gives the absolute path (prefixed with "@").
-- Stripping that prefix and walking up 6 directory levels reaches `code/`:
--
--   lisp_lexer/          (1)   — innermost: coding_adventures/lisp_lexer/
--   coding_adventures/   (2)
--   src/                 (3)
--   lisp_lexer/          (4)   — the package directory
--   lua/                 (5)
--   packages/            (6)
--   code/                → then /grammars/lisp.tokens

local grammar_tools = require("coding_adventures.grammar_tools")
local lexer_pkg     = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Path helpers
-- =========================================================================

--- Return the directory portion of a file path (without trailing slash).
-- For example:  "/a/b/c/init.lua"  →  "/a/b/c"
-- @param path string The full file path.
-- @return string     The directory portion.
local function dirname(path)
    return path:match("(.+)/[^/]+$") or "."
end

--- Return the absolute directory of this source file.
-- Lua embeds the source path in the chunk debug info with a leading "@".
-- We strip that prefix to get the real filesystem path.
-- @return string Absolute directory of this init.lua file.
local function get_script_dir()
    local info = debug.getinfo(1, "S")
    local src  = info.source
    if src:sub(1, 1) == "@" then
        src = src:sub(2)
    end
    return dirname(src)
end

--- Walk up `levels` directory levels from `path`.
-- Each call to this function strips one path component.
-- For example: up("/a/b/c", 2) → "/a"
-- @param path   string  Starting directory.
-- @param levels number  How many levels to climb.
-- @return string        Resulting directory.
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
-- The grammar is read from disk exactly once and cached in a module-level
-- variable.  Subsequent calls to `tokenize` reuse the cached grammar.
-- This avoids repeated file I/O and repeated regex compilation.

local _grammar_cache = nil

--- Load and parse the `lisp.tokens` grammar, with caching.
-- On the first call, opens and parses the file.  On subsequent calls,
-- returns the cached TokenGrammar object immediately.
-- @return TokenGrammar  The parsed Lisp token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    -- Navigate from this file's directory up to the repo root.
    -- init.lua is 3 dirs inside the package (src/coding_adventures/lisp_lexer/).
    -- The package itself is 3 more dirs inside the repo (packages/lua/lisp_lexer/).
    -- Total: 6 levels up lands us at `code/`, the repo root.
    local script_dir  = get_script_dir()
    local repo_root   = up(script_dir, 6)
    local tokens_path = repo_root .. "/grammars/lisp.tokens"

    local f, open_err = io.open(tokens_path, "r")
    if not f then
        error(
            "lisp_lexer: cannot open grammar file: " .. tokens_path ..
            " (" .. (open_err or "unknown error") .. ")"
        )
    end
    local content = f:read("*all")
    f:close()

    local grammar, parse_err = grammar_tools.parse_token_grammar(content)
    if not grammar then
        error("lisp_lexer: failed to parse lisp.tokens: " .. (parse_err or "unknown error"))
    end

    _grammar_cache = grammar
    return grammar
end

-- =========================================================================
-- Public API
-- =========================================================================

--- Tokenize a Lisp source string.
--
-- Loads the `lisp.tokens` grammar (cached after first call) and feeds the
-- source to a `GrammarLexer`.  Returns the complete flat token list,
-- including a terminal `EOF` token.
--
-- Whitespace and comments are consumed silently via the skip patterns in
-- `lisp.tokens`.  The caller receives only meaningful tokens:
--   NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT, EOF.
--
-- @param source string  The Lisp text to tokenize.
-- @return table         Array of Token objects (type, value, line, col).
-- @error                Raises an error on unexpected characters.
--
-- Example:
--
--   local lisp_lexer = require("coding_adventures.lisp_lexer")
--   local tokens = lisp_lexer.tokenize("(define x 42)")
--   -- tokens[1].type  → "LPAREN"
--   -- tokens[1].value → "("
--   -- tokens[2].type  → "SYMBOL"
--   -- tokens[2].value → "define"
--   -- tokens[3].type  → "SYMBOL"
--   -- tokens[3].value → "x"
--   -- tokens[4].type  → "NUMBER"
--   -- tokens[4].value → "42"
--   -- tokens[5].type  → "RPAREN"
--   -- tokens[5].value → ")"
function M.tokenize(source)
    local grammar = get_grammar()
    local gl      = lexer_pkg.GrammarLexer.new(source, grammar)
    return gl:tokenize()
end

--- Return the cached (or freshly loaded) TokenGrammar for Lisp.
--
-- Exposed for callers that want to inspect or reuse the grammar object
-- directly — for example, to build a custom GrammarLexer with callbacks.
--
-- @return TokenGrammar  The parsed Lisp token grammar.
function M.get_grammar()
    return get_grammar()
end

return M
