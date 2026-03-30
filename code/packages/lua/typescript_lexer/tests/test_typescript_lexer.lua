-- Tests for typescript_lexer
-- ==========================
--
-- Comprehensive busted test suite for the TypeScript lexer package.
--
-- TypeScript is a strict superset of JavaScript. This test suite verifies:
--
-- Part 1 — All JavaScript tokens still work:
--   - Keywords: var/let/const/function/return/if/else/for/while
--               class/new/this/typeof/instanceof/true/false/null/undefined
--   - Identifiers, numbers, strings
--   - Operators: +, -, *, /, ===, !==, ==, !=, =, <=, >=, =>, <, >, !
--   - Delimiters: (, ), {, }, [, ], ;, ,, ., :
--   - Whitespace silently consumed
--
-- Part 2 — TypeScript-specific keywords:
--   - interface, type, enum, namespace, declare, abstract
--   - implements, extends, readonly
--   - as, is (not in grammar — AS is shared with JS)
--   - keyof, typeof, infer
--   - Access modifiers: public, private, protected
--   - Type keywords: never, unknown, any, void, number (kw), string (kw),
--                    boolean, object, symbol, bigint
--
-- Part 3 — TypeScript constructs:
--   - Type annotation: x: number
--   - Generic syntax: Array<string>
--   - Interface body: interface Foo { bar: number; }
--   - Enum declaration: enum Color { Red, Green }
--   - Access modifiers in class

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

local ts_lexer = require("coding_adventures.typescript_lexer")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Collect token types from a list of tokens (ignoring the trailing EOF).
-- @param tokens  table  The token list returned by ts_lexer.tokenize.
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
-- @param tokens  table  The token list returned by ts_lexer.tokenize.
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

describe("typescript_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(ts_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(ts_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", ts_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(ts_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(ts_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = ts_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = ts_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = ts_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- JavaScript keywords (inherited by TypeScript)
-- =========================================================================

describe("JavaScript keyword tokens (TypeScript inherits all of these)", function()
    it("tokenizes var", function()
        local tokens = ts_lexer.tokenize("var")
        assert.are.equal("VAR", tokens[1].type)
    end)

    it("tokenizes let", function()
        local tokens = ts_lexer.tokenize("let")
        assert.are.equal("LET", tokens[1].type)
    end)

    it("tokenizes const", function()
        local tokens = ts_lexer.tokenize("const")
        assert.are.equal("CONST", tokens[1].type)
    end)

    it("tokenizes function", function()
        local tokens = ts_lexer.tokenize("function")
        assert.are.equal("FUNCTION", tokens[1].type)
    end)

    it("tokenizes return", function()
        local tokens = ts_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
    end)

    it("tokenizes if / else", function()
        local tokens = ts_lexer.tokenize("if else")
        local t = types(tokens)
        assert.are.same({"IF", "ELSE"}, t)
    end)

    it("tokenizes for / while", function()
        local tokens = ts_lexer.tokenize("for while")
        local t = types(tokens)
        assert.are.same({"FOR", "WHILE"}, t)
    end)

    it("tokenizes class", function()
        local tokens = ts_lexer.tokenize("class")
        assert.are.equal("CLASS", tokens[1].type)
    end)

    it("tokenizes new / this", function()
        local tokens = ts_lexer.tokenize("new this")
        local t = types(tokens)
        assert.are.same({"NEW", "THIS"}, t)
    end)

    it("tokenizes typeof / instanceof", function()
        local tokens = ts_lexer.tokenize("typeof instanceof")
        local t = types(tokens)
        assert.are.same({"TYPEOF", "INSTANCEOF"}, t)
    end)

    it("tokenizes true / false / null / undefined", function()
        local tokens = ts_lexer.tokenize("true false null undefined")
        local t = types(tokens)
        assert.are.same({"TRUE", "FALSE", "NULL", "UNDEFINED"}, t)
    end)
end)

-- =========================================================================
-- TypeScript-specific keywords
-- =========================================================================

describe("TypeScript-specific keyword tokens", function()
    it("tokenizes interface", function()
        local tokens = ts_lexer.tokenize("interface")
        assert.are.equal("INTERFACE", tokens[1].type)
        assert.are.equal("interface", tokens[1].value)
    end)

    it("tokenizes type", function()
        local tokens = ts_lexer.tokenize("type")
        assert.are.equal("TYPE", tokens[1].type)
        assert.are.equal("type", tokens[1].value)
    end)

    it("tokenizes enum", function()
        local tokens = ts_lexer.tokenize("enum")
        assert.are.equal("ENUM", tokens[1].type)
        assert.are.equal("enum", tokens[1].value)
    end)

    it("tokenizes namespace", function()
        local tokens = ts_lexer.tokenize("namespace")
        assert.are.equal("NAMESPACE", tokens[1].type)
        assert.are.equal("namespace", tokens[1].value)
    end)

    it("tokenizes declare", function()
        local tokens = ts_lexer.tokenize("declare")
        assert.are.equal("DECLARE", tokens[1].type)
        assert.are.equal("declare", tokens[1].value)
    end)

    it("tokenizes readonly", function()
        local tokens = ts_lexer.tokenize("readonly")
        assert.are.equal("READONLY", tokens[1].type)
        assert.are.equal("readonly", tokens[1].value)
    end)

    it("tokenizes abstract", function()
        local tokens = ts_lexer.tokenize("abstract")
        assert.are.equal("ABSTRACT", tokens[1].type)
        assert.are.equal("abstract", tokens[1].value)
    end)

    it("tokenizes implements", function()
        local tokens = ts_lexer.tokenize("implements")
        assert.are.equal("IMPLEMENTS", tokens[1].type)
        assert.are.equal("implements", tokens[1].value)
    end)

    it("tokenizes extends", function()
        local tokens = ts_lexer.tokenize("extends")
        assert.are.equal("EXTENDS", tokens[1].type)
        assert.are.equal("extends", tokens[1].value)
    end)

    it("tokenizes keyof", function()
        local tokens = ts_lexer.tokenize("keyof")
        assert.are.equal("KEYOF", tokens[1].type)
        assert.are.equal("keyof", tokens[1].value)
    end)

    it("tokenizes infer", function()
        local tokens = ts_lexer.tokenize("infer")
        assert.are.equal("INFER", tokens[1].type)
        assert.are.equal("infer", tokens[1].value)
    end)

    it("tokenizes never", function()
        local tokens = ts_lexer.tokenize("never")
        assert.are.equal("NEVER", tokens[1].type)
        assert.are.equal("never", tokens[1].value)
    end)

    it("tokenizes unknown", function()
        local tokens = ts_lexer.tokenize("unknown")
        assert.are.equal("UNKNOWN", tokens[1].type)
        assert.are.equal("unknown", tokens[1].value)
    end)

    it("tokenizes any", function()
        local tokens = ts_lexer.tokenize("any")
        assert.are.equal("ANY", tokens[1].type)
        assert.are.equal("any", tokens[1].value)
    end)

    it("tokenizes void", function()
        local tokens = ts_lexer.tokenize("void")
        assert.are.equal("VOID", tokens[1].type)
        assert.are.equal("void", tokens[1].value)
    end)

    it("tokenizes boolean", function()
        local tokens = ts_lexer.tokenize("boolean")
        assert.are.equal("BOOLEAN", tokens[1].type)
        assert.are.equal("boolean", tokens[1].value)
    end)

    it("tokenizes object", function()
        local tokens = ts_lexer.tokenize("object")
        assert.are.equal("OBJECT", tokens[1].type)
        assert.are.equal("object", tokens[1].value)
    end)

    it("tokenizes symbol", function()
        local tokens = ts_lexer.tokenize("symbol")
        assert.are.equal("SYMBOL", tokens[1].type)
        assert.are.equal("symbol", tokens[1].value)
    end)

    it("tokenizes bigint", function()
        local tokens = ts_lexer.tokenize("bigint")
        assert.are.equal("BIGINT", tokens[1].type)
        assert.are.equal("bigint", tokens[1].value)
    end)
end)

-- =========================================================================
-- Access modifiers
-- =========================================================================

describe("access modifier tokens", function()
    it("tokenizes public", function()
        local tokens = ts_lexer.tokenize("public")
        assert.are.equal("PUBLIC", tokens[1].type)
        assert.are.equal("public", tokens[1].value)
    end)

    it("tokenizes private", function()
        local tokens = ts_lexer.tokenize("private")
        assert.are.equal("PRIVATE", tokens[1].type)
        assert.are.equal("private", tokens[1].value)
    end)

    it("tokenizes protected", function()
        local tokens = ts_lexer.tokenize("protected")
        assert.are.equal("PROTECTED", tokens[1].type)
        assert.are.equal("protected", tokens[1].value)
    end)
end)

-- =========================================================================
-- Identifiers and basic literals
-- =========================================================================

describe("identifiers and basic literals", function()
    it("tokenizes a simple identifier", function()
        local tokens = ts_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("tokenizes an integer number", function()
        local tokens = ts_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
        assert.are.equal("42", tokens[1].value)
    end)

    it("tokenizes a double-quoted string", function()
        local tokens = ts_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
        assert.are.equal('hello', tokens[1].value)
    end)
end)

-- =========================================================================
-- Operators
-- =========================================================================

describe("operator tokens", function()
    it("tokenizes === (strict equals)", function()
        local tokens = ts_lexer.tokenize("===")
        assert.are.equal("STRICT_EQUALS", tokens[1].type)
    end)

    it("tokenizes !== (strict not equals)", function()
        local tokens = ts_lexer.tokenize("!==")
        assert.are.equal("STRICT_NOT_EQUALS", tokens[1].type)
    end)

    it("tokenizes => (arrow)", function()
        local tokens = ts_lexer.tokenize("=>")
        assert.are.equal("ARROW", tokens[1].type)
    end)

    it("tokenizes = (equals)", function()
        local tokens = ts_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
    end)

    it("tokenizes + (plus)", function()
        local tokens = ts_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
    end)

    it("tokenizes < (less than) for generics", function()
        local tokens = ts_lexer.tokenize("<")
        assert.are.equal("LESS_THAN", tokens[1].type)
    end)

    it("tokenizes > (greater than) for generics", function()
        local tokens = ts_lexer.tokenize(">")
        assert.are.equal("GREATER_THAN", tokens[1].type)
    end)
end)

-- =========================================================================
-- TypeScript constructs
-- =========================================================================

describe("TypeScript language constructs", function()
    -- Type annotations use the COLON token that already exists in JS
    it("tokenizes a type annotation: x: number", function()
        local tokens = ts_lexer.tokenize("x: number")
        local t = types(tokens)
        -- x → NAME, : → COLON, number → NUMBER keyword
        assert.are.same({"NAME", "COLON", "NUMBER"}, t)
        assert.are.equal("x", tokens[1].value)
        assert.are.equal("number", tokens[3].value)
    end)

    -- Generic type syntax uses < and > tokens
    it("tokenizes generic type: Array<string>", function()
        local tokens = ts_lexer.tokenize("Array<string>")
        local t = types(tokens)
        -- Array → NAME, < → LESS_THAN, string → STRING keyword, > → GREATER_THAN
        assert.are.same({"NAME", "LESS_THAN", "STRING", "GREATER_THAN"}, t)
    end)

    -- Interface declaration
    it("tokenizes interface declaration: interface Foo { }", function()
        local tokens = ts_lexer.tokenize("interface Foo { }")
        local t = types(tokens)
        assert.are.same({"INTERFACE", "NAME", "LBRACE", "RBRACE"}, t)
        assert.are.equal("Foo", tokens[2].value)
    end)

    -- Interface with body
    it("tokenizes interface body: interface Point { x: number; y: number; }", function()
        local src = "interface Point { x: number; y: number; }"
        local tokens = ts_lexer.tokenize(src)
        local first_iface = first_of(tokens, "INTERFACE")
        assert.is_not_nil(first_iface)
        -- Spot check the first field name
        local t = types(tokens)
        -- interface Point { x : number ; y : number ; }
        assert.are.same({
            "INTERFACE", "NAME", "LBRACE",
            "NAME", "COLON", "NUMBER", "SEMICOLON",
            "NAME", "COLON", "NUMBER", "SEMICOLON",
            "RBRACE"
        }, t)
    end)

    -- Enum declaration
    it("tokenizes enum declaration: enum Color { Red, Green, Blue }", function()
        local tokens = ts_lexer.tokenize("enum Color { Red, Green, Blue }")
        local t = types(tokens)
        assert.are.same({
            "ENUM", "NAME", "LBRACE",
            "NAME", "COMMA",
            "NAME", "COMMA",
            "NAME",
            "RBRACE"
        }, t)
    end)

    -- Access modifier in class
    it("tokenizes access modifiers in a class method", function()
        local src = "public getName(): string { }"
        local tokens = ts_lexer.tokenize(src)
        local t = types(tokens)
        -- public getName ( ) : string { }
        assert.are.same({
            "PUBLIC", "NAME", "LPAREN", "RPAREN",
            "COLON", "STRING", "LBRACE", "RBRACE"
        }, t)
    end)

    -- Generic function with extends constraint
    it("tokenizes generic constraint: <T extends K>", function()
        local tokens = ts_lexer.tokenize("<T extends K>")
        local t = types(tokens)
        -- < T extends K >
        assert.are.same({"LESS_THAN", "NAME", "EXTENDS", "NAME", "GREATER_THAN"}, t)
    end)

    -- declare keyword
    it("tokenizes declare module statement", function()
        local tokens = ts_lexer.tokenize("declare module foo { }")
        local t = types(tokens)
        assert.are.same({"DECLARE", "NAME", "NAME", "LBRACE", "RBRACE"}, t)
    end)

    -- readonly keyword
    it("tokenizes readonly property", function()
        local tokens = ts_lexer.tokenize("readonly id: number")
        local t = types(tokens)
        assert.are.same({"READONLY", "NAME", "COLON", "NUMBER"}, t)
    end)

    -- abstract class
    it("tokenizes abstract class declaration", function()
        local tokens = ts_lexer.tokenize("abstract class Shape { }")
        local t = types(tokens)
        assert.are.same({"ABSTRACT", "CLASS", "NAME", "LBRACE", "RBRACE"}, t)
    end)

    -- implements keyword
    it("tokenizes implements clause", function()
        local tokens = ts_lexer.tokenize("class Dog implements Animal { }")
        local t = types(tokens)
        assert.are.same({
            "CLASS", "NAME", "IMPLEMENTS", "NAME", "LBRACE", "RBRACE"
        }, t)
    end)

    -- extends keyword
    it("tokenizes extends clause", function()
        local tokens = ts_lexer.tokenize("class Cat extends Animal { }")
        local t = types(tokens)
        assert.are.same({
            "CLASS", "NAME", "EXTENDS", "NAME", "LBRACE", "RBRACE"
        }, t)
    end)

    -- keyof operator
    it("tokenizes keyof type operator", function()
        local tokens = ts_lexer.tokenize("keyof T")
        local t = types(tokens)
        assert.are.same({"KEYOF", "NAME"}, t)
    end)

    -- as expression
    it("tokenizes type assertion with as", function()
        local tokens = ts_lexer.tokenize("x as string")
        local t = types(tokens)
        -- x → NAME, as → AS (JavaScript keyword, shared), string → STRING keyword
        assert.are.same({"NAME", "AS", "STRING"}, t)
    end)
end)

-- =========================================================================
-- Whitespace handling
-- =========================================================================

describe("whitespace handling", function()
    it("strips spaces between tokens", function()
        local tokens = ts_lexer.tokenize("const x: number = 1;")
        local t = types(tokens)
        assert.are.same({"CONST", "NAME", "COLON", "NUMBER", "EQUALS", "NUMBER", "SEMICOLON"}, t)
    end)

    it("strips tabs and newlines between tokens", function()
        local tokens = ts_lexer.tokenize("interface\nFoo\n{\n}")
        local t = types(tokens)
        assert.are.same({"INTERFACE", "NAME", "LBRACE", "RBRACE"}, t)
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("all tokens start on line 1 for single-line input", function()
        local tokens = ts_lexer.tokenize("interface Foo { }")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)

    it("tracks column for: interface Foo", function()
        -- i n t e r f a c e   F o o
        -- 1 . . . . . . . . 10 11
        local tokens = ts_lexer.tokenize("interface Foo")
        assert.are.equal(1, tokens[1].col)   -- interface
        assert.are.equal(11, tokens[2].col)  -- Foo
    end)
end)

-- =========================================================================
-- EOF token
-- =========================================================================

describe("EOF token", function()
    it("is always the last token", function()
        local tokens = ts_lexer.tokenize("interface Foo { }")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)

    it("has an empty value", function()
        local tokens = ts_lexer.tokenize("1")
        assert.are.equal("", tokens[#tokens].value)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error on unexpected character @", function()
        assert.has_error(function()
            ts_lexer.tokenize("@")
        end)
    end)

    it("raises an error on backtick (template literals not in grammar)", function()
        assert.has_error(function()
            ts_lexer.tokenize("`hello`")
        end)
    end)
end)
