-- Tests for lisp_lexer
-- ====================
--
-- Comprehensive busted test suite for the Lisp lexer package.
--
-- The Lisp grammar is beautifully minimal compared to most languages.
-- A complete Lisp tokenizer needs to handle only seven token types:
--
--   NUMBER  — integer literals: 42, -7, 0
--   SYMBOL  — identifiers and operators: define, lambda, +, car, cdr, ?, !
--   STRING  — quoted strings: "hello world"
--   LPAREN  — open parenthesis: (
--   RPAREN  — close parenthesis: )
--   QUOTE   — tick shorthand for (quote ...): '
--   DOT     — cons cell separator: .
--
-- Whitespace and comments (; to end-of-line) are silently skipped.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - All seven token types tokenized correctly
--   - Operator symbols: + - * / = < > ! ? &
--   - Comments are skipped (inline and standalone)
--   - Whitespace is consumed silently
--   - Multi-line input with correct position tracking
--   - Nested lists
--   - Quoted forms (tick notation)
--   - Dotted pairs (cons cell notation)
--   - Multi-expression programs
--   - Unexpected character raises an error

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local lisp_lexer = require("coding_adventures.lisp_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by lisp_lexer.tokenize.
-- @return table         Ordered list of type strings (no EOF entry).
local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

--- Collect token values from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by lisp_lexer.tokenize.
-- @return table         Ordered list of value strings (no EOF entry).
local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.value
        end
    end
    return out
end

--- Find the first token with the given type.
-- @param tokens  table   Token list.
-- @param typ     string  Token type to search for.
-- @return table|nil      The first matching token, or nil.
local function first_of(tokens, typ)
    for _, tok in ipairs(tokens) do
        if tok.type == typ then return tok end
    end
    return nil
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("lisp_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(lisp_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(lisp_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", lisp_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(lisp_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(lisp_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = lisp_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = lisp_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = lisp_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("comment-only line produces only EOF", function()
        -- Lisp line comments begin with ; and run to end-of-line.
        -- They are used pervasively in Lisp code:
        --   ;; single semicolon for inline notes
        --   ;;; double for section headers (Emacs convention)
        local tokens = lisp_lexer.tokenize("; this is a comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("multiple comment lines produce only EOF", function()
        local tokens = lisp_lexer.tokenize("; line 1\n; line 2\n; line 3")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- NUMBER tokens
-- =========================================================================
--
-- Lisp numbers are integers only in this grammar (matching /-?[0-9]+/).
-- Full Lisps support rationals (3/4), floats (1.5), complex numbers, etc.,
-- but for this implementation we keep it simple.

describe("NUMBER tokens", function()
    it("tokenizes a positive integer", function()
        local tokens = lisp_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = lisp_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes a negative integer", function()
        -- In Lisp, -7 is a number literal, not the unary minus operator
        -- applied to 7.  The sign is part of the token itself.
        local tokens = lisp_lexer.tokenize("-7")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("-7", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by spaces", function()
        local tokens = lisp_lexer.tokenize("1 2 3")
        local t = types(tokens)
        assert.are.same({"NUMBER", "NUMBER", "NUMBER"}, t)
        assert.are.same({"1", "2", "3"}, values(tokens))
    end)
end)

-- =========================================================================
-- SYMBOL tokens
-- =========================================================================
--
-- Symbols are the workhorses of Lisp.  They name variables, functions,
-- special forms, macros, and operators.  Unlike most languages, Lisp allows
-- many punctuation characters in symbol names:
--
--   +  -  *  /  =  <  >  !  ?  &
--
-- This means `cadr`, `set!`, `null?`, `<=`, `string->number`, and even
-- just `+` are all valid symbol names.  The grammar pattern is:
--
--   SYMBOL = /[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*/

describe("SYMBOL tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = lisp_lexer.tokenize("define")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("define", tokens[1].value)
    end)

    it("tokenizes lambda", function()
        local tokens = lisp_lexer.tokenize("lambda")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("lambda", tokens[1].value)
    end)

    it("tokenizes the + operator as a symbol", function()
        -- In Lisp, + is a regular function stored in a binding named "+".
        -- (+ 1 2) calls the function at symbol +, returning 3.
        local tokens = lisp_lexer.tokenize("+")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes arithmetic operator symbols: - * /", function()
        local tokens = lisp_lexer.tokenize("- * /")
        local t = types(tokens)
        assert.are.same({"SYMBOL", "SYMBOL", "SYMBOL"}, t)
        assert.are.same({"-", "*", "/"}, values(tokens))
    end)

    it("tokenizes comparison operators: = < >", function()
        local tokens = lisp_lexer.tokenize("= < >")
        local t = types(tokens)
        assert.are.same({"SYMBOL", "SYMBOL", "SYMBOL"}, t)
    end)

    it("tokenizes predicate symbol with ? suffix", function()
        -- Lisps conventionally end predicate (boolean-returning) function
        -- names with ?: null?, pair?, string?, number?
        local tokens = lisp_lexer.tokenize("null?")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("null?", tokens[1].value)
    end)

    it("tokenizes mutating symbol with ! suffix", function()
        -- Lisps conventionally end mutating (side-effecting) functions
        -- with !: set!, vector-set!, string-set!
        local tokens = lisp_lexer.tokenize("set!")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("set!", tokens[1].value)
    end)

    it("tokenizes symbol starting with underscore", function()
        local tokens = lisp_lexer.tokenize("_internal")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("_internal", tokens[1].value)
    end)

    it("tokenizes symbol with & (used in lambda list keywords)", function()
        local tokens = lisp_lexer.tokenize("&rest")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("&rest", tokens[1].value)
    end)
end)

-- =========================================================================
-- STRING tokens
-- =========================================================================
--
-- Lisp strings are double-quoted, with backslash escape sequences.
-- The raw source text (including quotes and escapes) is preserved in
-- the token value — the parser or evaluator decodes escapes.

describe("STRING tokens", function()
    it("tokenizes a simple string", function()
        local tokens = lisp_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty string", function()
        local tokens = lisp_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)

    it("preserves backslash escape sequences in value", function()
        -- The lexer returns the raw source text; it does NOT decode \n → newline.
        local tokens = lisp_lexer.tokenize('"hello\\nworld"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello\\nworld', tokens[1].value)
    end)

    it("tokenizes string with escaped quote", function()
        local tokens = lisp_lexer.tokenize('"say \\"hi\\""')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("tokenizes string with spaces inside", function()
        local tokens = lisp_lexer.tokenize('"hello world"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello world', tokens[1].value)
    end)
end)

-- =========================================================================
-- LPAREN and RPAREN tokens
-- =========================================================================
--
-- Parentheses are the syntactic skeleton of Lisp.  Every list is enclosed
-- in a matched pair of parentheses.  Counting parens is a rite of passage:
--
--   (car (cdr (cdr '(1 2 3))))  ; → 3

describe("LPAREN and RPAREN tokens", function()
    it("tokenizes a single left paren", function()
        local tokens = lisp_lexer.tokenize("(")
        assert.are.equal("LPAREN", tokens[1].type)
        assert.are.equal("(", tokens[1].value)
    end)

    it("tokenizes a single right paren", function()
        local tokens = lisp_lexer.tokenize(")")
        assert.are.equal("RPAREN", tokens[1].type)
        assert.are.equal(")", tokens[1].value)
    end)

    it("tokenizes an empty list ()", function()
        -- The empty list () is also written as nil in many Lisps.
        -- It is the terminator of every proper list.
        local tokens = lisp_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)
end)

-- =========================================================================
-- QUOTE token
-- =========================================================================
--
-- The apostrophe ' is Lisp's most common reader macro.
-- 'x is syntax for (quote x) — it prevents evaluation of x.
--
-- Examples:
--   '42         → 42         (numbers are self-evaluating, but still works)
--   'foo        → foo        (the symbol foo, not its value)
--   '(1 2 3)    → (1 2 3)   (a literal list, not a function call)

describe("QUOTE token", function()
    it("tokenizes tick as QUOTE", function()
        local tokens = lisp_lexer.tokenize("'")
        assert.are.equal("QUOTE", tokens[1].type)
        assert.are.equal("'", tokens[1].value)
    end)

    it("tokenizes 'x as QUOTE SYMBOL", function()
        local tokens = lisp_lexer.tokenize("'x")
        local t = types(tokens)
        assert.are.same({"QUOTE", "SYMBOL"}, t)
    end)

    it("tokenizes '(1 2) as QUOTE LPAREN NUMBER NUMBER RPAREN", function()
        local tokens = lisp_lexer.tokenize("'(1 2)")
        local t = types(tokens)
        assert.are.same({"QUOTE", "LPAREN", "NUMBER", "NUMBER", "RPAREN"}, t)
    end)
end)

-- =========================================================================
-- DOT token
-- =========================================================================
--
-- The dot notation for cons cells:
--
--   (a . b)    — a cons cell with car=a and cdr=b
--   (1 . nil)  — equivalent to (1)
--   (1 2 . 3)  — an improper list ending in 3 instead of nil
--
-- Dotted pairs appear in association lists (alists):
--   '((name . "Alice") (age . 30))
--
-- In this grammar, DOT is the literal character ".".

describe("DOT token", function()
    it("tokenizes a dot as DOT", function()
        local tokens = lisp_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
        assert.are.equal(".", tokens[1].value)
    end)

    it("tokenizes (a . b) as a dotted pair", function()
        local tokens = lisp_lexer.tokenize("(a . b)")
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN"}, t)
        assert.are.equal("a", tokens[2].value)
        assert.are.equal(".", tokens[3].value)
        assert.are.equal("b", tokens[4].value)
    end)
end)

-- =========================================================================
-- Comment handling
-- =========================================================================
--
-- Lisp uses the semicolon ; for line comments.
-- By convention:
--   ;   — inline comment (after code on the same line)
--   ;;  — standalone comment for a block of code
--   ;;; — file or section header
-- All are handled identically by the lexer: everything from ; to newline
-- is consumed and never emitted as a token.

describe("comment handling", function()
    it("skips inline comment after a symbol", function()
        local tokens = lisp_lexer.tokenize("define ; this is a comment")
        local t = types(tokens)
        assert.are.same({"SYMBOL"}, t)
    end)

    it("skips comment between tokens", function()
        local tokens = lisp_lexer.tokenize("( ; comment\nx)")
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "RPAREN"}, t)
    end)

    it("skips double-semicolon comment", function()
        local tokens = lisp_lexer.tokenize(";; section header\n42")
        local t = types(tokens)
        assert.are.same({"NUMBER"}, t)
    end)

    it("skips comment-only line in multi-line expression", function()
        local src = "(define x\n  ; the answer\n  42)"
        local tokens = lisp_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = lisp_lexer.tokenize("( define x 42 )")
        local t = types(tokens)
        assert.are.same(
            {"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"},
            t
        )
    end)

    it("strips tabs between tokens", function()
        local tokens = lisp_lexer.tokenize("(\tdefine\tx\t42\t)")
        local t = types(tokens)
        assert.are.same(
            {"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"},
            t
        )
    end)

    it("strips newlines between tokens", function()
        local tokens = lisp_lexer.tokenize("(\ndefine\nx\n42\n)")
        local t = types(tokens)
        assert.are.same(
            {"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"},
            t
        )
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================
--
-- Token positions are important for error messages ("unexpected token at
-- line 3 col 7").  The lexer tracks line and column as it scans.

describe("position tracking", function()
    it("tracks column for single-line input", function()
        -- Input: (+ 1 2)
        -- col:   1234567
        local tokens = lisp_lexer.tokenize("(+ 1 2)")
        assert.are.equal(1, tokens[1].col)  -- (
        assert.are.equal(2, tokens[2].col)  -- +
        assert.are.equal(4, tokens[3].col)  -- 1
        assert.are.equal(6, tokens[4].col)  -- 2
        assert.are.equal(7, tokens[5].col)  -- )
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = lisp_lexer.tokenize("(+ 1 2)")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)

    it("tracks line numbers across newlines", function()
        local src = "(define\nx\n42)"
        local tokens = lisp_lexer.tokenize(src)
        -- (    → line 1
        -- define → line 1
        -- x    → line 2
        -- 42   → line 3
        -- )    → line 3
        assert.are.equal(1, tokens[1].line)  -- (
        assert.are.equal(1, tokens[2].line)  -- define
        assert.are.equal(2, tokens[3].line)  -- x
        assert.are.equal(3, tokens[4].line)  -- 42
        assert.are.equal(3, tokens[5].line)  -- )
    end)
end)

-- =========================================================================
-- Composite Lisp expressions
-- =========================================================================

describe("composite Lisp expressions", function()
    it("tokenizes a simple function call (+ 1 2)", function()
        local tokens = lisp_lexer.tokenize("(+ 1 2)")
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "NUMBER", "NUMBER", "RPAREN"}, t)
    end)

    it("tokenizes (define x 42)", function()
        local tokens = lisp_lexer.tokenize("(define x 42)")
        local t = types(tokens)
        assert.are.same(
            {"LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN"},
            t
        )
        assert.are.equal("define", tokens[2].value)
        assert.are.equal("x", tokens[3].value)
        assert.are.equal("42", tokens[4].value)
    end)

    it("tokenizes a lambda expression", function()
        -- (lambda (x) (* x x)) — anonymous squaring function
        local tokens = lisp_lexer.tokenize("(lambda (x) (* x x))")
        local t = types(tokens)
        assert.are.same({
            "LPAREN", "SYMBOL",                       -- (lambda
            "LPAREN", "SYMBOL", "RPAREN",             -- (x)
            "LPAREN", "SYMBOL", "SYMBOL", "SYMBOL", "RPAREN",  -- (* x x)
            "RPAREN"                                   -- )
        }, t)
    end)

    it("tokenizes nested lists", function()
        local tokens = lisp_lexer.tokenize("(car (cdr '(1 2 3)))")
        local t = types(tokens)
        -- ( car ( cdr ' ( 1 2 3 ) ) )
        assert.are.same({
            "LPAREN", "SYMBOL",
            "LPAREN", "SYMBOL",
            "QUOTE", "LPAREN", "NUMBER", "NUMBER", "NUMBER", "RPAREN",
            "RPAREN",
            "RPAREN"
        }, t)
    end)

    it("tokenizes a string in a list", function()
        local tokens = lisp_lexer.tokenize('(display "hello world")')
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "STRING", "RPAREN"}, t)
        assert.are.equal('hello world', tokens[3].value)
    end)

    it("tokenizes a dotted pair (a . b)", function()
        local tokens = lisp_lexer.tokenize("(a . b)")
        local t = types(tokens)
        assert.are.same({"LPAREN", "SYMBOL", "DOT", "SYMBOL", "RPAREN"}, t)
    end)

    it("tokenizes an association list", function()
        -- alist: '((x . 1) (y . 2))
        -- Used for simple key-value lookups in classic Lisp.
        local tokens = lisp_lexer.tokenize("'((x . 1) (y . 2))")
        local t = types(tokens)
        assert.are.same({
            "QUOTE",
            "LPAREN",
              "LPAREN", "SYMBOL", "DOT", "NUMBER", "RPAREN",
              "LPAREN", "SYMBOL", "DOT", "NUMBER", "RPAREN",
            "RPAREN"
        }, t)
    end)
end)

-- =========================================================================
-- Multi-expression programs
-- =========================================================================

describe("multi-expression programs", function()
    it("tokenizes two top-level expressions", function()
        local src = "(define x 42) (display x)"
        local tokens = lisp_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({
            "LPAREN", "SYMBOL", "SYMBOL", "NUMBER", "RPAREN",
            "LPAREN", "SYMBOL", "SYMBOL", "RPAREN"
        }, t)
    end)

    it("tokenizes a multi-line program", function()
        local src = [[
;; Fibonacci
(define (fib n)
  (if (< n 2)
      n
      (+ (fib (- n 1))
         (fib (- n 2)))))
]]
        local tokens = lisp_lexer.tokenize(src)
        -- Spot-check: should contain define, fib, +, -, if symbols
        local sym_values = {}
        for _, tok in ipairs(tokens) do
            if tok.type == "SYMBOL" then
                sym_values[tok.value] = true
            end
        end
        assert.is_truthy(sym_values["define"])
        assert.is_truthy(sym_values["fib"])
        assert.is_truthy(sym_values["if"])
        assert.is_truthy(sym_values["+"])
        assert.is_truthy(sym_values["-"])
        assert.is_truthy(sym_values["<"])
        -- Should have no whitespace or comment tokens
        for _, tok in ipairs(tokens) do
            assert.is_not_equal("WHITESPACE", tok.type)
            assert.is_not_equal("COMMENT", tok.type)
        end
    end)

    it("tokenizes multiple quoted forms", function()
        local tokens = lisp_lexer.tokenize("'a 'b 'c")
        local t = types(tokens)
        assert.are.same({
            "QUOTE", "SYMBOL",
            "QUOTE", "SYMBOL",
            "QUOTE", "SYMBOL"
        }, t)
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = lisp_lexer.tokenize("42")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = lisp_lexer.tokenize("42")
        assert.are.equal("", tokens[#tokens].value)
    end)

    it("is the only token for empty input", function()
        local tokens = lisp_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on an unexpected character", function()
        -- @ is not a valid Lisp token
        assert.has_error(function()
            lisp_lexer.tokenize("@")
        end)
    end)

    it("raises an error on # (not in this Lisp grammar)", function()
        -- Full Scheme uses #t and #f for booleans, but our grammar doesn't
        -- include them, so # should raise a lex error.
        assert.has_error(function()
            lisp_lexer.tokenize("#t")
        end)
    end)
end)
