-- Tests for ruby_lexer
-- =====================
--
-- Comprehensive busted test suite for the Ruby lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Keywords: def, end, class, module, if, elsif, else, unless, while,
--               until, for, do, return, begin, rescue, ensure, require,
--               puts, true, false, nil, and, or, not, then, yield
--   - Identifiers (NAME tokens for non-keywords)
--   - Numbers: integer literals
--   - Strings: double-quoted literals
--   - Multi-char operators: ==, .., =>, !=, <=, >=
--   - Single-char operators: =, +, -, *, /, <, >
--   - Punctuation: (, ), ,, :
--   - Whitespace is consumed silently
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

local rb_lexer = require("coding_adventures.ruby_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by rb_lexer.tokenize.
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
-- @param tokens  table  The token list returned by rb_lexer.tokenize.
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

describe("ruby_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(rb_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(rb_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", rb_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(rb_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(rb_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = rb_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = rb_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = rb_lexer.tokenize("   \t  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords
-- =========================================================================

describe("keyword tokens", function()
    -- Ruby function definition keywords
    it("tokenizes def", function()
        local tokens = rb_lexer.tokenize("def")
        assert.are.equal("DEF", tokens[1].type)
        assert.are.equal("def", tokens[1].value)
    end)

    it("tokenizes end", function()
        local tokens = rb_lexer.tokenize("end")
        assert.are.equal("END", tokens[1].type)
        assert.are.equal("end", tokens[1].value)
    end)

    -- Ruby class and module keywords
    it("tokenizes class", function()
        local tokens = rb_lexer.tokenize("class")
        assert.are.equal("CLASS", tokens[1].type)
        assert.are.equal("class", tokens[1].value)
    end)

    it("tokenizes module", function()
        local tokens = rb_lexer.tokenize("module")
        assert.are.equal("MODULE", tokens[1].type)
        assert.are.equal("module", tokens[1].value)
    end)

    -- Ruby control flow keywords
    it("tokenizes if", function()
        local tokens = rb_lexer.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
        assert.are.equal("if", tokens[1].value)
    end)

    it("tokenizes elsif", function()
        -- Note: Ruby uses `elsif`, not `elif`
        local tokens = rb_lexer.tokenize("elsif")
        assert.are.equal("ELSIF", tokens[1].type)
        assert.are.equal("elsif", tokens[1].value)
    end)

    it("tokenizes else", function()
        local tokens = rb_lexer.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
        assert.are.equal("else", tokens[1].value)
    end)

    it("tokenizes unless", function()
        -- Ruby's `unless` is the negated `if`
        local tokens = rb_lexer.tokenize("unless")
        assert.are.equal("UNLESS", tokens[1].type)
        assert.are.equal("unless", tokens[1].value)
    end)

    it("tokenizes while", function()
        local tokens = rb_lexer.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
        assert.are.equal("while", tokens[1].value)
    end)

    it("tokenizes until", function()
        -- Ruby's `until` is the negated `while`
        local tokens = rb_lexer.tokenize("until")
        assert.are.equal("UNTIL", tokens[1].type)
        assert.are.equal("until", tokens[1].value)
    end)

    it("tokenizes for", function()
        local tokens = rb_lexer.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
        assert.are.equal("for", tokens[1].value)
    end)

    it("tokenizes do", function()
        local tokens = rb_lexer.tokenize("do")
        assert.are.equal("DO", tokens[1].type)
        assert.are.equal("do", tokens[1].value)
    end)

    it("tokenizes return", function()
        local tokens = rb_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
        assert.are.equal("return", tokens[1].value)
    end)

    -- Ruby exception keywords
    it("tokenizes begin", function()
        local tokens = rb_lexer.tokenize("begin")
        assert.are.equal("BEGIN", tokens[1].type)
        assert.are.equal("begin", tokens[1].value)
    end)

    it("tokenizes rescue", function()
        local tokens = rb_lexer.tokenize("rescue")
        assert.are.equal("RESCUE", tokens[1].type)
        assert.are.equal("rescue", tokens[1].value)
    end)

    it("tokenizes ensure", function()
        local tokens = rb_lexer.tokenize("ensure")
        assert.are.equal("ENSURE", tokens[1].type)
        assert.are.equal("ensure", tokens[1].value)
    end)

    -- Ruby utility keywords
    it("tokenizes require", function()
        local tokens = rb_lexer.tokenize("require")
        assert.are.equal("REQUIRE", tokens[1].type)
        assert.are.equal("require", tokens[1].value)
    end)

    it("tokenizes puts", function()
        local tokens = rb_lexer.tokenize("puts")
        assert.are.equal("PUTS", tokens[1].type)
        assert.are.equal("puts", tokens[1].value)
    end)

    it("tokenizes yield", function()
        local tokens = rb_lexer.tokenize("yield")
        assert.are.equal("YIELD", tokens[1].type)
        assert.are.equal("yield", tokens[1].value)
    end)

    it("tokenizes then", function()
        local tokens = rb_lexer.tokenize("then")
        assert.are.equal("THEN", tokens[1].type)
        assert.are.equal("then", tokens[1].value)
    end)

    -- Ruby boolean/nil literals (all lowercase, unlike Python)
    it("tokenizes true", function()
        local tokens = rb_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = rb_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes nil", function()
        -- Ruby uses `nil`, not `null` or `None`
        local tokens = rb_lexer.tokenize("nil")
        assert.are.equal("NIL", tokens[1].type)
        assert.are.equal("nil", tokens[1].value)
    end)

    -- Ruby logical operators as keywords
    it("tokenizes and", function()
        local tokens = rb_lexer.tokenize("and")
        assert.are.equal("AND", tokens[1].type)
        assert.are.equal("and", tokens[1].value)
    end)

    it("tokenizes or", function()
        local tokens = rb_lexer.tokenize("or")
        assert.are.equal("OR", tokens[1].type)
        assert.are.equal("or", tokens[1].value)
    end)

    it("tokenizes not", function()
        local tokens = rb_lexer.tokenize("not")
        assert.are.equal("NOT", tokens[1].type)
        assert.are.equal("not", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = rb_lexer.tokenize("my_var")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("my_var", tokens[1].value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = rb_lexer.tokenize("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes an identifier with digits in the middle", function()
        local tokens = rb_lexer.tokenize("abc123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("abc123", tokens[1].value)
    end)

    it("tokenizes a CamelCase class name", function()
        local tokens = rb_lexer.tokenize("MyClass")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("MyClass", tokens[1].value)
    end)
end)

-- =========================================================================
-- Number tokens
-- =========================================================================

describe("number tokens", function()
    it("tokenizes an integer", function()
        local tokens = rb_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = rb_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by operators", function()
        local tokens = rb_lexer.tokenize("1+2")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = rb_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = rb_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('', tokens[1].value)
    end)

    it("tokenizes a string with escape sequence", function()
        local tokens = rb_lexer.tokenize('"a\\nb"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operator tokens
-- =========================================================================

describe("operator tokens", function()
    -- Multi-char operators must be matched before their single-char prefixes.
    -- The grammar file lists them before single-char variants.

    it("tokenizes == (equals equals)", function()
        local tokens = rb_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("tokenizes .. (range)", function()
        -- Ruby's range operator, absent from Python
        local tokens = rb_lexer.tokenize("..")
        assert.are.equal("DOT_DOT", tokens[1].type)
        assert.are.equal("..", tokens[1].value)
    end)

    it("tokenizes => (hash rocket)", function()
        -- Ruby's hash rocket, used in hash literals and case/when
        local tokens = rb_lexer.tokenize("=>")
        assert.are.equal("HASH_ROCKET", tokens[1].type)
        assert.are.equal("=>", tokens[1].value)
    end)

    it("tokenizes != (not equals)", function()
        local tokens = rb_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("!=", tokens[1].value)
    end)

    it("tokenizes <= (less than or equal)", function()
        local tokens = rb_lexer.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
        assert.are.equal("<=", tokens[1].value)
    end)

    it("tokenizes >= (greater than or equal)", function()
        local tokens = rb_lexer.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
        assert.are.equal(">=", tokens[1].value)
    end)

    it("tokenizes = (assignment)", function()
        local tokens = rb_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)

    it("tokenizes + (plus)", function()
        local tokens = rb_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes - (minus)", function()
        local tokens = rb_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
        assert.are.equal("-", tokens[1].value)
    end)

    it("tokenizes * (star)", function()
        local tokens = rb_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
        assert.are.equal("*", tokens[1].value)
    end)

    it("tokenizes / (slash)", function()
        local tokens = rb_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
        assert.are.equal("/", tokens[1].value)
    end)

    it("tokenizes < (less than)", function()
        local tokens = rb_lexer.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
        assert.are.equal("<", tokens[1].value)
    end)

    it("tokenizes > (greater than)", function()
        local tokens = rb_lexer.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
        assert.are.equal(">", tokens[1].value)
    end)
end)

-- =========================================================================
-- Punctuation tokens
-- =========================================================================

describe("punctuation tokens", function()
    it("tokenizes ( and )", function()
        local tokens = rb_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes comma", function()
        local tokens = rb_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
        assert.are.equal(",", tokens[1].value)
    end)

    it("tokenizes colon", function()
        local tokens = rb_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
        assert.are.equal(":", tokens[1].value)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a simple assignment: x = 1", function()
        local tokens = rb_lexer.tokenize("x = 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
        assert.are.equal("x", tokens[1].value)
    end)

    it("tokenizes a function definition header: def greet(name)", function()
        local tokens = rb_lexer.tokenize("def greet(name)")
        local t = types(tokens)
        assert.are.same({"DEF", "NAME", "LPAREN", "NAME", "RPAREN"}, t)
        assert.are.equal("greet", tokens[2].value)
    end)

    it("tokenizes a class definition: class Animal", function()
        local tokens = rb_lexer.tokenize("class Animal")
        local t = types(tokens)
        assert.are.same({"CLASS", "NAME"}, t)
        assert.are.equal("Animal", tokens[2].value)
    end)

    it("tokenizes a module definition: module Greetable", function()
        local tokens = rb_lexer.tokenize("module Greetable")
        local t = types(tokens)
        assert.are.same({"MODULE", "NAME"}, t)
    end)

    it("tokenizes if/elsif/else structure keywords", function()
        local src = "if x == 1"
        local tokens = rb_lexer.tokenize(src)
        local first_if = first_of(tokens, "IF")
        assert.is_not_nil(first_if)
        local first_eq = first_of(tokens, "EQUALS_EQUALS")
        assert.is_not_nil(first_eq)
    end)

    it("tokenizes a return statement: return true", function()
        local tokens = rb_lexer.tokenize("return true")
        local t = types(tokens)
        assert.are.same({"RETURN", "TRUE"}, t)
    end)

    it("tokenizes nil literal: x = nil", function()
        local tokens = rb_lexer.tokenize("x = nil")
        local first_nil = first_of(tokens, "NIL")
        assert.is_not_nil(first_nil)
        assert.are.equal("nil", first_nil.value)
    end)

    it("tokenizes arithmetic: a + b * c", function()
        local tokens = rb_lexer.tokenize("a + b * c")
        local t = types(tokens)
        assert.are.same({"NAME", "PLUS", "NAME", "STAR", "NAME"}, t)
    end)

    it("tokenizes hash rocket in hash literal", function()
        -- { :key => value }
        local tokens = rb_lexer.tokenize("x => y")
        local hr = first_of(tokens, "HASH_ROCKET")
        assert.is_not_nil(hr)
        assert.are.equal("=>", hr.value)
    end)

    it("tokenizes range operator: 1..10", function()
        local tokens = rb_lexer.tokenize("1..10")
        local t = types(tokens)
        assert.are.same({"NUMBER", "DOT_DOT", "NUMBER"}, t)
    end)

    it("tokenizes rescue keyword in rescue clause", function()
        local tokens = rb_lexer.tokenize("rescue RuntimeError")
        local first_rescue = first_of(tokens, "RESCUE")
        assert.is_not_nil(first_rescue)
    end)

    it("tokenizes ensure keyword", function()
        local tokens = rb_lexer.tokenize("ensure")
        assert.are.equal("ENSURE", tokens[1].type)
    end)

    it("tokenizes require statement: require path", function()
        local tokens = rb_lexer.tokenize("require path")
        local t = types(tokens)
        assert.are.same({"REQUIRE", "NAME"}, t)
    end)

    it("tokenizes puts call: puts x", function()
        local tokens = rb_lexer.tokenize("puts x")
        local t = types(tokens)
        assert.are.same({"PUTS", "NAME"}, t)
    end)

    it("tokenizes unless condition: unless x == 0", function()
        local tokens = rb_lexer.tokenize("unless x == 0")
        local first_unless = first_of(tokens, "UNLESS")
        assert.is_not_nil(first_unless)
    end)

    it("tokenizes comparison: a != b", function()
        local tokens = rb_lexer.tokenize("a != b")
        local t = types(tokens)
        assert.are.same({"NAME", "NOT_EQUALS", "NAME"}, t)
    end)

    it("tokenizes comparison: a <= b", function()
        local tokens = rb_lexer.tokenize("a <= b")
        local t = types(tokens)
        assert.are.same({"NAME", "LESS_EQUALS", "NAME"}, t)
    end)

    it("tokenizes comparison: a >= b", function()
        local tokens = rb_lexer.tokenize("a >= b")
        local t = types(tokens)
        assert.are.same({"NAME", "GREATER_EQUALS", "NAME"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = rb_lexer.tokenize("x = 1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = rb_lexer.tokenize("x\t=\t1")
        local t = types(tokens)
        assert.are.same({"NAME", "EQUALS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: x = 42", function()
        -- x _ = _ 4 2
        -- 1 2 3 4 5 6
        local tokens = rb_lexer.tokenize("x = 42")
        assert.are.equal(1, tokens[1].col)  -- x
        assert.are.equal(3, tokens[2].col)  -- =
        assert.are.equal(5, tokens[3].col)  -- 42
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = rb_lexer.tokenize("x = 1")
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
        local tokens = rb_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = rb_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character @", function()
        -- Note: @ is used for Ruby instance variables but is not in the grammar
        assert.has_error(function()
            rb_lexer.tokenize("@instance")
        end)
    end)

    it("raises an error on unexpected character $", function()
        assert.has_error(function()
            rb_lexer.tokenize("$global")
        end)
    end)
end)
