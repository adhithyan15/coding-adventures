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
--   1. Requires the pre-compiled `_grammar` module (once, cached).
--   2. Constructs a `GrammarLexer` from the `lexer` package for each call.
--   3. Returns the flat token list.
--
-- # Grammar source
--
-- The token grammar is no longer read from `code/grammars/` at runtime.
-- A published LuaRocks package does not ship the monorepo's `code/grammars/`
-- directory, so walking out of the package's own directory to find it would
-- fail after installation. Instead, `lisp.tokens` is pre-compiled (via
-- `grammar-tools compile-tokens`) into `_grammar.lua`, a plain Lua module
-- that embeds the TokenGrammar as native Lua data structures. That module
-- ships as part of this package, so `require()` always finds it.

local lexer_pkg = require("coding_adventures.lexer")

local M = {}
M.VERSION = "0.1.0"

-- =========================================================================
-- Grammar loading
-- =========================================================================
--
-- The compiled grammar module is required exactly once and its
-- `token_grammar()` result cached in a module-level variable. Subsequent
-- calls to `tokenize` reuse the cached grammar.

local _grammar_cache = nil

--- Return the (cached) TokenGrammar for Lisp.
-- On the first call, requires the pre-compiled `_grammar` module and
-- invokes `token_grammar()`. On subsequent calls, returns the cached
-- TokenGrammar object immediately.
-- @return TokenGrammar  The Lisp token grammar.
local function get_grammar()
    if _grammar_cache then
        return _grammar_cache
    end

    local compiled = require("coding_adventures.lisp_lexer._grammar")
    _grammar_cache = compiled.token_grammar()
    return _grammar_cache
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
    local raw     = gl:tokenize()
    local tokens  = {}
    for _, tok in ipairs(raw) do
        tokens[#tokens + 1] = {
            type  = tok.type_name,
            value = tok.value,
            line  = tok.line,
            col   = tok.column,
        }
    end
    return tokens
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
