-- Tests for javascript_lexer
-- ==========================
--
-- Comprehensive busted test suite for the JavaScript lexer package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty input produces only EOF
--   - Keywords: var/let/const/function/return/if/else/for/while
--               class/new/this/typeof/instanceof/true/false/null/undefined
--   - Identifiers (NAME tokens for non-keywords)
--   - Numbers: integer, hex literals
--   - Strings: single-quoted and double-quoted
--   - Operators: +, -, *, /, %, ++, --, ===, !==, ==, !=, =, +=, -=, &&, ||, !
--   - Arrow function: =>
--   - Strict equality: === and !==
--   - Punctuation: (, ), {, }, [, ], ;, ,, ., :
--   - Whitespace is consumed silently
--   - Arrow function expression: (x) => x + 1
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

local js_lexer = require("coding_adventures.javascript_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by js_lexer.tokenize.
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
-- @param tokens  table  The token list returned by js_lexer.tokenize.
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

describe("javascript_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(js_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(js_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", js_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(js_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(js_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = js_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = js_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = js_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Keywords
-- =========================================================================

describe("keyword tokens", function()
    -- JavaScript variable declaration keywords
    it("tokenizes var", function()
        local tokens = js_lexer.tokenize("var")
        assert.are.equal("VAR", tokens[1].type)
        assert.are.equal("var", tokens[1].value)
    end)

    it("tokenizes let", function()
        local tokens = js_lexer.tokenize("let")
        assert.are.equal("LET", tokens[1].type)
        assert.are.equal("let", tokens[1].value)
    end)

    it("tokenizes const", function()
        local tokens = js_lexer.tokenize("const")
        assert.are.equal("CONST", tokens[1].type)
        assert.are.equal("const", tokens[1].value)
    end)

    it("tokenizes function", function()
        local tokens = js_lexer.tokenize("function")
        assert.are.equal("FUNCTION", tokens[1].type)
        assert.are.equal("function", tokens[1].value)
    end)

    it("tokenizes return", function()
        local tokens = js_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
        assert.are.equal("return", tokens[1].value)
    end)

    it("tokenizes if", function()
        local tokens = js_lexer.tokenize("if")
        assert.are.equal("IF", tokens[1].type)
        assert.are.equal("if", tokens[1].value)
    end)

    it("tokenizes else", function()
        local tokens = js_lexer.tokenize("else")
        assert.are.equal("ELSE", tokens[1].type)
        assert.are.equal("else", tokens[1].value)
    end)

    it("tokenizes for", function()
        local tokens = js_lexer.tokenize("for")
        assert.are.equal("FOR", tokens[1].type)
        assert.are.equal("for", tokens[1].value)
    end)

    it("tokenizes while", function()
        local tokens = js_lexer.tokenize("while")
        assert.are.equal("WHILE", tokens[1].type)
        assert.are.equal("while", tokens[1].value)
    end)

    it("tokenizes class", function()
        local tokens = js_lexer.tokenize("class")
        assert.are.equal("CLASS", tokens[1].type)
        assert.are.equal("class", tokens[1].value)
    end)

    it("tokenizes new", function()
        local tokens = js_lexer.tokenize("new")
        assert.are.equal("NEW", tokens[1].type)
        assert.are.equal("new", tokens[1].value)
    end)

    it("tokenizes this", function()
        local tokens = js_lexer.tokenize("this")
        assert.are.equal("THIS", tokens[1].type)
        assert.are.equal("this", tokens[1].value)
    end)

    it("tokenizes typeof", function()
        local tokens = js_lexer.tokenize("typeof")
        assert.are.equal("TYPEOF", tokens[1].type)
        assert.are.equal("typeof", tokens[1].value)
    end)

    it("tokenizes instanceof", function()
        local tokens = js_lexer.tokenize("instanceof")
        assert.are.equal("INSTANCEOF", tokens[1].type)
        assert.are.equal("instanceof", tokens[1].value)
    end)

    it("tokenizes true", function()
        local tokens = js_lexer.tokenize("true")
        assert.are.equal("TRUE", tokens[1].type)
        assert.are.equal("true", tokens[1].value)
    end)

    it("tokenizes false", function()
        local tokens = js_lexer.tokenize("false")
        assert.are.equal("FALSE", tokens[1].type)
        assert.are.equal("false", tokens[1].value)
    end)

    it("tokenizes null", function()
        local tokens = js_lexer.tokenize("null")
        assert.are.equal("NULL", tokens[1].type)
        assert.are.equal("null", tokens[1].value)
    end)

    it("tokenizes undefined", function()
        local tokens = js_lexer.tokenize("undefined")
        assert.are.equal("UNDEFINED", tokens[1].type)
        assert.are.equal("undefined", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifiers
-- =========================================================================

describe("identifier tokens", function()
    it("tokenizes a simple identifier", function()
        local tokens = js_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("tokenizes an identifier starting with underscore", function()
        local tokens = js_lexer.tokenize("_private")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("_private", tokens[1].value)
    end)

    it("tokenizes an identifier starting with dollar sign", function()
        local tokens = js_lexer.tokenize("$element")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("$element", tokens[1].value)
    end)

    it("tokenizes identifier with digits in the middle", function()
        local tokens = js_lexer.tokenize("abc123")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("abc123", tokens[1].value)
    end)
end)

-- =========================================================================
-- Number tokens
-- =========================================================================

describe("number tokens", function()
    it("tokenizes an integer", function()
        local tokens = js_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes zero", function()
        local tokens = js_lexer.tokenize("0")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("0", tokens[1].value)
    end)

    it("tokenizes multiple numbers separated by operators", function()
        local tokens = js_lexer.tokenize("1+2")
        local t = types(tokens)
        assert.are.same({"NUMBER", "PLUS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- String tokens
-- =========================================================================

describe("string tokens", function()
    it("tokenizes a double-quoted string", function()
        local tokens = js_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('"hello"', tokens[1].value)
    end)

    it("tokenizes an empty double-quoted string", function()
        local tokens = js_lexer.tokenize('""')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('""', tokens[1].value)
    end)

    it("tokenizes a string with escape sequence", function()
        local tokens = js_lexer.tokenize('"a\\nb"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operator tokens
-- =========================================================================

describe("operator tokens", function()
    -- Strict equality operators (must match before == and !=)
    it("tokenizes === (strict equals)", function()
        local tokens = js_lexer.tokenize("===")
        assert.are.equal("STRICT_EQUALS", tokens[1].type)
        assert.are.equal("===", tokens[1].value)
    end)

    it("tokenizes !== (strict not equals)", function()
        local tokens = js_lexer.tokenize("!==")
        assert.are.equal("STRICT_NOT_EQUALS", tokens[1].type)
        assert.are.equal("!==", tokens[1].value)
    end)

    it("tokenizes == (loose equals)", function()
        local tokens = js_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
        assert.are.equal("==", tokens[1].value)
    end)

    it("tokenizes != (not equals)", function()
        local tokens = js_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
        assert.are.equal("!=", tokens[1].value)
    end)

    it("tokenizes => (arrow)", function()
        local tokens = js_lexer.tokenize("=>")
        assert.are.equal("ARROW", tokens[1].type)
        assert.are.equal("=>", tokens[1].value)
    end)

    it("tokenizes <= (less than or equal)", function()
        local tokens = js_lexer.tokenize("<=")
        assert.are.equal("LESS_EQUALS", tokens[1].type)
        assert.are.equal("<=", tokens[1].value)
    end)

    it("tokenizes >= (greater than or equal)", function()
        local tokens = js_lexer.tokenize(">=")
        assert.are.equal("GREATER_EQUALS", tokens[1].type)
        assert.are.equal(">=", tokens[1].value)
    end)

    it("tokenizes = (assignment)", function()
        local tokens = js_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
        assert.are.equal("=", tokens[1].value)
    end)

    it("tokenizes + (plus)", function()
        local tokens = js_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
        assert.are.equal("+", tokens[1].value)
    end)

    it("tokenizes - (minus)", function()
        local tokens = js_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
        assert.are.equal("-", tokens[1].value)
    end)

    it("tokenizes * (star)", function()
        local tokens = js_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
        assert.are.equal("*", tokens[1].value)
    end)

    it("tokenizes / (slash)", function()
        local tokens = js_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
        assert.are.equal("/", tokens[1].value)
    end)

    it("tokenizes < (less than)", function()
        local tokens = js_lexer.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
        assert.are.equal("<", tokens[1].value)
    end)

    it("tokenizes > (greater than)", function()
        local tokens = js_lexer.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
        assert.are.equal(">", tokens[1].value)
    end)

    it("tokenizes ! (bang)", function()
        local tokens = js_lexer.tokenize("!")
        assert.are.equal("BANG", tokens[1].type)
        assert.are.equal("!", tokens[1].value)
    end)
end)

-- =========================================================================
-- Punctuation tokens
-- =========================================================================

describe("punctuation tokens", function()
    it("tokenizes ( and )", function()
        local tokens = js_lexer.tokenize("()")
        local t = types(tokens)
        assert.are.same({"LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes { and }", function()
        local tokens = js_lexer.tokenize("{}")
        local t = types(tokens)
        assert.are.same({"LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes [ and ]", function()
        local tokens = js_lexer.tokenize("[]")
        local t = types(tokens)
        assert.are.same({"LBRACKET", "RBRACKET"}, t)
    end)

    it("tokenizes semicolon", function()
        local tokens = js_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
        assert.are.equal(";", tokens[1].value)
    end)

    it("tokenizes comma", function()
        local tokens = js_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
        assert.are.equal(",", tokens[1].value)
    end)

    it("tokenizes dot", function()
        local tokens = js_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
        assert.are.equal(".", tokens[1].value)
    end)

    it("tokenizes colon", function()
        local tokens = js_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
        assert.are.equal(":", tokens[1].value)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("tokenizes a variable declaration: var x = 1;", function()
        local tokens = js_lexer.tokenize("var x = 1;")
        local t = types(tokens)
        assert.are.same({"VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"}, t)
        assert.are.equal("x", tokens[2].value)
    end)

    it("tokenizes a const declaration: const PI = 3;", function()
        local tokens = js_lexer.tokenize("const PI = 3;")
        local t = types(tokens)
        assert.are.same({"CONST", "NAME", "EQUALS", "NUMBER", "SEMICOLON"}, t)
    end)

    it("tokenizes an arrow function expression: (x) => x + 1", function()
        local tokens = js_lexer.tokenize("(x) => x + 1")
        local t = types(tokens)
        -- ( x ) => x + 1
        assert.are.same(
            {"LPAREN", "NAME", "RPAREN", "ARROW", "NAME", "PLUS", "NUMBER"},
            t
        )
    end)

    it("tokenizes strict equality comparison: a === b", function()
        local tokens = js_lexer.tokenize("a === b")
        local t = types(tokens)
        assert.are.same({"NAME", "STRICT_EQUALS", "NAME"}, t)
    end)

    it("tokenizes strict inequality comparison: a !== b", function()
        local tokens = js_lexer.tokenize("a !== b")
        local t = types(tokens)
        assert.are.same({"NAME", "STRICT_NOT_EQUALS", "NAME"}, t)
    end)

    it("tokenizes a function declaration", function()
        -- function add(a, b) { return a + b; }
        local src = "function add(a, b) { return a + b; }"
        local tokens = js_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({
            "FUNCTION", "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
            "LBRACE", "RETURN", "NAME", "PLUS", "NAME", "SEMICOLON", "RBRACE"
        }, t)
    end)

    it("tokenizes an if/else statement", function()
        local src = "if (x) { return true; } else { return false; }"
        local tokens = js_lexer.tokenize(src)
        local first_if = first_of(tokens, "IF")
        assert.is_not_nil(first_if)
        local first_else = first_of(tokens, "ELSE")
        assert.is_not_nil(first_else)
        local first_true = first_of(tokens, "TRUE")
        assert.is_not_nil(first_true)
        local first_false = first_of(tokens, "FALSE")
        assert.is_not_nil(first_false)
    end)

    it("tokenizes a class declaration", function()
        local src = "class Animal { }"
        local tokens = js_lexer.tokenize(src)
        local t = types(tokens)
        assert.are.same({"CLASS", "NAME", "LBRACE", "RBRACE"}, t)
    end)

    it("tokenizes typeof expression", function()
        local tokens = js_lexer.tokenize("typeof x")
        local t = types(tokens)
        assert.are.same({"TYPEOF", "NAME"}, t)
    end)

    it("tokenizes instanceof expression", function()
        local tokens = js_lexer.tokenize("x instanceof Foo")
        local t = types(tokens)
        assert.are.same({"NAME", "INSTANCEOF", "NAME"}, t)
    end)

    it("tokenizes new expression", function()
        local tokens = js_lexer.tokenize("new Foo()")
        local t = types(tokens)
        assert.are.same({"NEW", "NAME", "LPAREN", "RPAREN"}, t)
    end)

    it("tokenizes method call: obj.method(arg)", function()
        local tokens = js_lexer.tokenize("obj.method(arg)")
        local t = types(tokens)
        assert.are.same({"NAME", "DOT", "NAME", "LPAREN", "NAME", "RPAREN"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = js_lexer.tokenize("var x = 1")
        local t = types(tokens)
        assert.are.same({"VAR", "NAME", "EQUALS", "NUMBER"}, t)
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = js_lexer.tokenize("var\n\tx\n=\n1")
        local t = types(tokens)
        assert.are.same({"VAR", "NAME", "EQUALS", "NUMBER"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("tracks column for single-line input: var x = 1;", function()
        -- v a r _ x _ = _ 1 ;
        -- 1 . . . 5 . 7 . 9 10
        local tokens = js_lexer.tokenize("var x = 1;")
        assert.are.equal(1, tokens[1].col)  -- var
        assert.are.equal(5, tokens[2].col)  -- x
        assert.are.equal(7, tokens[3].col)  -- =
        assert.are.equal(9, tokens[4].col)  -- 1
        assert.are.equal(10, tokens[5].col) -- ;
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = js_lexer.tokenize("var x = 1;")
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
        local tokens = js_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = js_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character @", function()
        assert.has_error(function()
            js_lexer.tokenize("@")
        end)
    end)

    it("raises an error on backtick (template literal not in grammar)", function()
        assert.has_error(function()
            js_lexer.tokenize("`hello`")
        end)
    end)
end)
