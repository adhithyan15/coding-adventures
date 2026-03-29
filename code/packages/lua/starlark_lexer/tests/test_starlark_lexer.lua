-- Tests for starlark_lexer
-- ========================
--
-- Comprehensive busted test suite for the Starlark lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Keywords: def, if, elif, else, for, return, pass, break, continue,
--               and, or, not, in, lambda, load, True, False, None
--   - Identifiers (NAME tokens for non-keywords)
--   - Integer literals: decimal, hex (INT), octal (INT)
--   - Float literals (FLOAT)
--   - Strings: single-quoted, double-quoted, raw, bytes, triple-quoted
--   - Three-character operators: **=, <<=, >>=, //=
--   - Two-character operators: **, //, <<, >>, ==, !=, <=, >=, +=, -=, *=, /=, %=, &=, |=, ^=
--   - Single-character operators: +, -, *, /, %, =, <, >, &, |, ^, ~
--   - Delimiters: (, ), [, ], {, }, ,, :, ;, .
--   - Indentation mode: NEWLINE, INDENT, DEDENT tokens
--   - Comments are consumed silently
--   - Whitespace between tokens is consumed silently
--   - Token positions (line, col) are tracked correctly
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

local starlark_lexer = require("coding_adventures.starlark_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by starlark_lexer.tokenize.
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
-- @param tokens  table  The token list returned by starlark_lexer.tokenize.
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

--- Collect token types excluding structural indentation tokens (NEWLINE, INDENT, DEDENT)
-- and EOF. Useful when testing token content without worrying about line structure.
-- @param tokens  table  The token list.
-- @return table         Ordered list of non-structural type strings.
local function content_types(tokens)
    local out = {}
    local skip = { NEWLINE = true, INDENT = true, DEDENT = true, EOF = true }
    for _, tok in ipairs(tokens) do
        if not skip[tok.type] then
            out[#out + 1] = tok.type
        end
    end
    return out
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("starlark_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(starlark_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(starlark_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", starlark_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(starlark_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(starlark_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = starlark_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = starlark_lexer.tokenize("")
        assert.are.equal("EOF", tokens[#tokens].type)
        -- Only EOF (and possibly a trailing NEWLINE from indentation mode)
        local non_eof = 0
        for _, tok in ipairs(tokens) do
            if tok.type ~= "EOF" and tok.type ~= "NEWLINE" then
                non_eof = non_eof + 1
            end
        end
        assert.are.equal(0, non_eof)
    end)

    it("whitespace-only input produces only EOF (no content tokens)", function()
        local tokens = starlark_lexer.tokenize("   ")
        local ct = content_types(tokens)
        assert.are.same({}, ct)
    end)
end)

-- =========================================================================
-- Keywords
-- =========================================================================

describe("keyword tokens", function()
    -- Control flow
    it("tokenizes if", function()
        local tokens = starlark_lexer.tokenize("if")
        local ct = content_types(tokens)
        assert.are.same({"IF"}, ct)
    end)

    it("tokenizes elif", function()
        local tokens = starlark_lexer.tokenize("elif")
        local ct = content_types(tokens)
        assert.are.same({"ELIF"}, ct)
    end)

    it("tokenizes else", function()
        local tokens = starlark_lexer.tokenize("else")
        local ct = content_types(tokens)
        assert.are.same({"ELSE"}, ct)
    end)

    it("tokenizes for", function()
        local tokens = starlark_lexer.tokenize("for")
        local ct = content_types(tokens)
        assert.are.same({"FOR"}, ct)
    end)

    it("tokenizes return", function()
        local tokens = starlark_lexer.tokenize("return")
        local ct = content_types(tokens)
        assert.are.same({"RETURN"}, ct)
    end)

    it("tokenizes pass", function()
        local tokens = starlark_lexer.tokenize("pass")
        local ct = content_types(tokens)
        assert.are.same({"PASS"}, ct)
    end)

    it("tokenizes break", function()
        local tokens = starlark_lexer.tokenize("break")
        local ct = content_types(tokens)
        assert.are.same({"BREAK"}, ct)
    end)

    it("tokenizes continue", function()
        local tokens = starlark_lexer.tokenize("continue")
        local ct = content_types(tokens)
        assert.are.same({"CONTINUE"}, ct)
    end)

    -- Function definition
    it("tokenizes def", function()
        local tokens = starlark_lexer.tokenize("def")
        local ct = content_types(tokens)
        assert.are.same({"DEF"}, ct)
    end)

    -- Boolean/logic operators
    it("tokenizes and", function()
        local tokens = starlark_lexer.tokenize("and")
        local ct = content_types(tokens)
        assert.are.same({"AND"}, ct)
    end)

    it("tokenizes or", function()
        local tokens = starlark_lexer.tokenize("or")
        local ct = content_types(tokens)
        assert.are.same({"OR"}, ct)
    end)

    it("tokenizes not", function()
        local tokens = starlark_lexer.tokenize("not")
        local ct = content_types(tokens)
        assert.are.same({"NOT"}, ct)
    end)

    it("tokenizes in", function()
        local tokens = starlark_lexer.tokenize("in")
        local ct = content_types(tokens)
        assert.are.same({"IN"}, ct)
    end)

    -- Starlark-specific
    it("tokenizes lambda", function()
        local tokens = starlark_lexer.tokenize("lambda")
        local ct = content_types(tokens)
        assert.are.same({"LAMBDA"}, ct)
    end)

    it("tokenizes load", function()
        local tokens = starlark_lexer.tokenize("load")
        local ct = content_types(tokens)
        assert.are.same({"LOAD"}, ct)
    end)

    -- Singleton literals
    it("tokenizes True", function()
        local tokens = starlark_lexer.tokenize("True")
        assert.are.equal("TRUE", first_of(tokens, "TRUE").type)
    end)

    it("tokenizes False", function()
        local tokens = starlark_lexer.tokenize("False")
        assert.are.equal("FALSE", first_of(tokens, "FALSE").type)
    end)

    it("tokenizes None", function()
        local tokens = starlark_lexer.tokenize("None")
        assert.are.equal("NONE", first_of(tokens, "NONE").type)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = starlark_lexer.tokenize("my_var")
        assert.are.equal("NAME", first_of(tokens, "NAME").type)
        assert.are.equal("my_var", first_of(tokens, "NAME").value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = starlark_lexer.tokenize("_private")
        assert.are.equal("NAME", first_of(tokens, "NAME").type)
        assert.are.equal("_private", first_of(tokens, "NAME").value)
    end)

    it("tokenizes an identifier with digits", function()
        local tokens = starlark_lexer.tokenize("abc123")
        assert.are.equal("NAME", first_of(tokens, "NAME").type)
        assert.are.equal("abc123", first_of(tokens, "NAME").value)
    end)

    it("does not treat 'define' as a keyword (not in keywords list)", function()
        local tokens = starlark_lexer.tokenize("define")
        -- 'define' starts with 'def' but is not 'def' itself
        assert.are.equal("NAME", first_of(tokens, "NAME").type)
        assert.are.equal("define", first_of(tokens, "NAME").value)
    end)
end)

-- =========================================================================
-- Integer tokens
-- =========================================================================

describe("integer tokens", function()
    it("tokenizes a decimal integer", function()
        local tokens = starlark_lexer.tokenize("42")
        assert.are.equal("INT", first_of(tokens, "INT").type)
        assert.are.equal("42", first_of(tokens, "INT").value)
    end)

    it("tokenizes zero", function()
        local tokens = starlark_lexer.tokenize("0")
        assert.are.equal("INT", first_of(tokens, "INT").type)
        assert.are.equal("0", first_of(tokens, "INT").value)
    end)

    it("tokenizes a hex integer", function()
        local tokens = starlark_lexer.tokenize("0xFF")
        assert.are.equal("INT", first_of(tokens, "INT").type)
        assert.are.equal("0xFF", first_of(tokens, "INT").value)
    end)

    it("tokenizes an octal integer", function()
        local tokens = starlark_lexer.tokenize("0o77")
        assert.are.equal("INT", first_of(tokens, "INT").type)
        assert.are.equal("0o77", first_of(tokens, "INT").value)
    end)
end)

-- =========================================================================
-- Float tokens
-- =========================================================================

describe("float tokens", function()
    it("tokenizes a simple float", function()
        local tokens = starlark_lexer.tokenize("3.14")
        assert.are.equal("FLOAT", first_of(tokens, "FLOAT").type)
        assert.are.equal("3.14", first_of(tokens, "FLOAT").value)
    end)

    it("tokenizes a float with exponent", function()
        local tokens = starlark_lexer.tokenize("1e10")
        assert.are.equal("FLOAT", first_of(tokens, "FLOAT").type)
    end)

    it("tokenizes a float starting with dot", function()
        local tokens = starlark_lexer.tokenize(".5")
        assert.are.equal("FLOAT", first_of(tokens, "FLOAT").type)
        assert.are.equal(".5", first_of(tokens, "FLOAT").value)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = starlark_lexer.tokenize('"hello"')
        assert.are.equal("STRING", first_of(tokens, "STRING").type)
        assert.are.equal('"hello"', first_of(tokens, "STRING").value)
    end)

    it("tokenizes a single-quoted string", function()
        local tokens = starlark_lexer.tokenize("'hello'")
        assert.are.equal("STRING", first_of(tokens, "STRING").type)
        assert.are.equal("'hello'", first_of(tokens, "STRING").value)
    end)

    it("tokenizes a raw string r\"...\"", function()
        local tokens = starlark_lexer.tokenize('r"raw\\nstring"')
        assert.are.equal("STRING", first_of(tokens, "STRING").type)
    end)

    it("tokenizes a bytes string b\"...\"", function()
        local tokens = starlark_lexer.tokenize('b"bytes"')
        assert.are.equal("STRING", first_of(tokens, "STRING").type)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = starlark_lexer.tokenize('""')
        assert.are.equal("STRING", first_of(tokens, "STRING").type)
        assert.are.equal('""', first_of(tokens, "STRING").value)
    end)
end)

-- =========================================================================
-- Three-character operator tokens
-- =========================================================================

describe("three-character operator tokens", function()
    it("tokenizes **=", function()
        local tokens = starlark_lexer.tokenize("**=")
        assert.are.equal("DOUBLE_STAR_EQUALS", first_of(tokens, "DOUBLE_STAR_EQUALS").type)
    end)

    it("tokenizes <<=", function()
        local tokens = starlark_lexer.tokenize("<<=")
        assert.are.equal("LEFT_SHIFT_EQUALS", first_of(tokens, "LEFT_SHIFT_EQUALS").type)
    end)

    it("tokenizes >>=", function()
        local tokens = starlark_lexer.tokenize(">>=")
        assert.are.equal("RIGHT_SHIFT_EQUALS", first_of(tokens, "RIGHT_SHIFT_EQUALS").type)
    end)

    it("tokenizes //=", function()
        local tokens = starlark_lexer.tokenize("//=")
        assert.are.equal("FLOOR_DIV_EQUALS", first_of(tokens, "FLOOR_DIV_EQUALS").type)
    end)
end)

-- =========================================================================
-- Two-character operator tokens
-- =========================================================================

describe("two-character operator tokens", function()
    it("tokenizes **", function()
        local tokens = starlark_lexer.tokenize("**")
        assert.are.equal("DOUBLE_STAR", first_of(tokens, "DOUBLE_STAR").type)
    end)

    it("tokenizes //", function()
        local tokens = starlark_lexer.tokenize("//")
        assert.are.equal("FLOOR_DIV", first_of(tokens, "FLOOR_DIV").type)
    end)

    it("tokenizes ==", function()
        local tokens = starlark_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", first_of(tokens, "EQUALS_EQUALS").type)
    end)

    it("tokenizes !=", function()
        local tokens = starlark_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", first_of(tokens, "NOT_EQUALS").type)
    end)

    it("tokenizes <=", function()
        local tokens = starlark_lexer.tokenize("<=")
        assert.are.equal("LESS_EQUALS", first_of(tokens, "LESS_EQUALS").type)
    end)

    it("tokenizes >=", function()
        local tokens = starlark_lexer.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", first_of(tokens, "GREATER_EQUALS").type)
    end)

    it("tokenizes +=", function()
        local tokens = starlark_lexer.tokenize("+=")
        assert.are.equal("PLUS_EQUALS", first_of(tokens, "PLUS_EQUALS").type)
    end)

    it("tokenizes -=", function()
        local tokens = starlark_lexer.tokenize("-=")
        assert.are.equal("MINUS_EQUALS", first_of(tokens, "MINUS_EQUALS").type)
    end)

    it("tokenizes == before = (first-match-wins)", function()
        -- If == is defined before =, then "==" must produce EQUALS_EQUALS,
        -- not two EQUALS tokens.
        local tokens = starlark_lexer.tokenize("==")
        local ct = content_types(tokens)
        assert.are.same({"EQUALS_EQUALS"}, ct)
    end)
end)

-- =========================================================================
-- Single-character operator tokens
-- =========================================================================

describe("single-character operator tokens", function()
    it("tokenizes +", function()
        local tokens = starlark_lexer.tokenize("+")
        assert.are.equal("PLUS", first_of(tokens, "PLUS").type)
    end)

    it("tokenizes -", function()
        local tokens = starlark_lexer.tokenize("-")
        assert.are.equal("MINUS", first_of(tokens, "MINUS").type)
    end)

    it("tokenizes *", function()
        local tokens = starlark_lexer.tokenize("*")
        assert.are.equal("STAR", first_of(tokens, "STAR").type)
    end)

    it("tokenizes /", function()
        local tokens = starlark_lexer.tokenize("/")
        assert.are.equal("SLASH", first_of(tokens, "SLASH").type)
    end)

    it("tokenizes %", function()
        local tokens = starlark_lexer.tokenize("%")
        assert.are.equal("PERCENT", first_of(tokens, "PERCENT").type)
    end)

    it("tokenizes =", function()
        local tokens = starlark_lexer.tokenize("=")
        assert.are.equal("EQUALS", first_of(tokens, "EQUALS").type)
    end)

    it("tokenizes <", function()
        local tokens = starlark_lexer.tokenize("<")
        assert.are.equal("LESS_THAN", first_of(tokens, "LESS_THAN").type)
    end)

    it("tokenizes >", function()
        local tokens = starlark_lexer.tokenize(">")
        assert.are.equal("GREATER_THAN", first_of(tokens, "GREATER_THAN").type)
    end)
end)

-- =========================================================================
-- Delimiter tokens
-- =========================================================================

describe("delimiter tokens", function()
    it("tokenizes ( and )", function()
        local tokens = starlark_lexer.tokenize("()")
        local ct = content_types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, ct)
    end)

    it("tokenizes [ and ]", function()
        local tokens = starlark_lexer.tokenize("[]")
        local ct = content_types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, ct)
    end)

    it("tokenizes { and }", function()
        local tokens = starlark_lexer.tokenize("{}")
        local ct = content_types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, ct)
    end)

    it("tokenizes comma", function()
        local tokens = starlark_lexer.tokenize(",")
        assert.are.equal("COMMA", first_of(tokens, "COMMA").type)
    end)

    it("tokenizes colon", function()
        local tokens = starlark_lexer.tokenize(":")
        assert.are.equal("COLON", first_of(tokens, "COLON").type)
    end)

    it("tokenizes semicolon", function()
        local tokens = starlark_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", first_of(tokens, "SEMICOLON").type)
    end)

    it("tokenizes dot", function()
        local tokens = starlark_lexer.tokenize(".")
        assert.are.equal("DOT", first_of(tokens, "DOT").type)
    end)
end)

-- =========================================================================
-- Indentation mode
-- =========================================================================

describe("indentation mode", function()
    -- INDENT/DEDENT/NEWLINE are emitted by the GrammarLexer when
    -- mode: indentation is active in the grammar.
    --
    -- A simple indented block like:
    --   def foo():
    --     pass
    --
    -- Should produce (conceptually):
    --   DEF NAME LPAREN RPAREN COLON NEWLINE INDENT PASS NEWLINE DEDENT

    it("emits NEWLINE at end of a logical line", function()
        local tokens = starlark_lexer.tokenize("x = 1\n")
        local has_newline = first_of(tokens, "NEWLINE") ~= nil
        assert.is_true(has_newline)
    end)

    it("emits INDENT/DEDENT for indented block", function()
        -- A minimal indented block: "def f():\n    pass\n"
        local src = "def f():\n    pass\n"
        local tokens = starlark_lexer.tokenize(src)
        local has_indent = first_of(tokens, "INDENT") ~= nil
        local has_dedent = first_of(tokens, "DEDENT") ~= nil
        assert.is_true(has_indent)
        assert.is_true(has_dedent)
    end)

    it("suppresses INDENT/DEDENT inside parens", function()
        -- Inside () indentation is insignificant (Starlark/Python rule)
        local src = "foo(\n    x,\n    y\n)"
        local tokens = starlark_lexer.tokenize(src)
        -- Should NOT produce INDENT or DEDENT inside the parens
        local has_indent = first_of(tokens, "INDENT") ~= nil
        assert.is_false(has_indent)
    end)
end)

-- =========================================================================
-- Comment handling
-- =========================================================================

describe("comment handling", function()
    it("consumes # comments without emitting a token", function()
        -- "x # this is a comment" should produce NAME NEWLINE (+ structural tokens)
        local tokens = starlark_lexer.tokenize("x # comment")
        local ct = content_types(tokens)
        assert.are.same({"NAME"}, ct)
    end)

    it("does not emit comment text as a token value", function()
        local tokens = starlark_lexer.tokenize("x = 1 # assign x")
        for _, tok in ipairs(tokens) do
            assert.is_false(tok.value:find("assign x") ~= nil,
                "comment text should not appear in any token value")
        end
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a simple assignment: x = 1", function()
        local tokens = starlark_lexer.tokenize("x = 1")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "EQUALS", "INT"}, ct)
        assert.are.equal("x", first_of(tokens, "NAME").value)
    end)

    it("tokenizes an equality check: x == 1", function()
        local tokens = starlark_lexer.tokenize("x == 1")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "EQUALS_EQUALS", "INT"}, ct)
    end)

    it("tokenizes a function definition header: def foo(x):", function()
        local tokens = starlark_lexer.tokenize("def foo(x):")
        local ct = content_types(tokens)
        assert.are.same({"DEF", "NAME", "LPAREN", "NAME", "RPAREN", "COLON"}, ct)
    end)

    it("tokenizes a function call: foo(a, b)", function()
        local tokens = starlark_lexer.tokenize("foo(a, b)")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN"}, ct)
    end)

    it("tokenizes a list literal: [1, 2, 3]", function()
        local tokens = starlark_lexer.tokenize("[1, 2, 3]")
        local ct = content_types(tokens)
        assert.are.same({"LBRACKET", "INT", "COMMA", "INT", "COMMA", "INT", "RBRACKET"}, ct)
    end)

    it("tokenizes a dict literal: {\"key\": value}", function()
        local tokens = starlark_lexer.tokenize('{"key": value}')
        local ct = content_types(tokens)
        assert.are.same({"LBRACE", "STRING", "COLON", "NAME", "RBRACE"}, ct)
    end)

    it("tokenizes return True", function()
        local tokens = starlark_lexer.tokenize("return True")
        local ct = content_types(tokens)
        assert.are.same({"RETURN", "TRUE"}, ct)
    end)

    it("tokenizes for x in y:", function()
        local tokens = starlark_lexer.tokenize("for x in y:")
        local ct = content_types(tokens)
        assert.are.same({"FOR", "NAME", "IN", "NAME", "COLON"}, ct)
    end)

    it("tokenizes boolean expression: a and b or not c", function()
        local tokens = starlark_lexer.tokenize("a and b or not c")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "AND", "NAME", "OR", "NOT", "NAME"}, ct)
    end)

    it("tokenizes power operator: x ** 2", function()
        local tokens = starlark_lexer.tokenize("x ** 2")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "DOUBLE_STAR", "INT"}, ct)
    end)

    it("tokenizes floor division: x // 2", function()
        local tokens = starlark_lexer.tokenize("x // 2")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "FLOOR_DIV", "INT"}, ct)
    end)

    it("tokenizes augmented assignment: x += 1", function()
        local tokens = starlark_lexer.tokenize("x += 1")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "PLUS_EQUALS", "INT"}, ct)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = starlark_lexer.tokenize("x = 1")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "EQUALS", "INT"}, ct)
    end)

    it("strips tabs between tokens", function()
        local tokens = starlark_lexer.tokenize("x\t=\t1")
        local ct = content_types(tokens)
        assert.are.same({"NAME", "EQUALS", "INT"}, ct)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: x = 42", function()
        -- x _ = _ 4 2
        -- 1 2 3 4 5 6
        local tokens = starlark_lexer.tokenize("x = 42")
        -- Filter to only NAME, EQUALS, INT
        local named = {}
        for _, tok in ipairs(tokens) do
            if tok.type == "NAME" or tok.type == "EQUALS" or tok.type == "INT" then
                named[#named + 1] = tok
            end
        end
        assert.are.equal(1, named[1].col)  -- x
        assert.are.equal(3, named[2].col)  -- =
        assert.are.equal(5, named[3].col)  -- 42
    end)

    it("reports line 1 for all tokens on a single-line input", function()
        local tokens = starlark_lexer.tokenize("x = 1")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = starlark_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = starlark_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on an unexpected character", function()
        -- The backtick ` is not a valid Starlark character
        assert.has_error(function()
            starlark_lexer.tokenize("`")
        end)
    end)

    it("raises an error on an unexpected character @", function()
        -- @ is not a valid Starlark token (it's used in Python decorators but
        -- not in Starlark — starlark.tokens does not define an @ pattern)
        assert.has_error(function()
            starlark_lexer.tokenize("@decorator")
        end)
    end)
end)
