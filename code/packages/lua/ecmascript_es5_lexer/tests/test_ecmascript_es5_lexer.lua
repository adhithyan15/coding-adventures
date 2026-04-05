-- Tests for ecmascript_es5_lexer
-- ==============================
--
-- Comprehensive busted test suite for the ECMAScript 5 (2009) lexer package.
--
-- ES5 adds the `debugger` keyword over ES3 and keeps all ES3 features
-- including strict equality, try/catch/finally/throw, and instanceof.

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

local es5_lexer = require("coding_adventures.ecmascript_es5_lexer")

-- =========================================================================
-- Helpers
-- =========================================================================

local function types(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.type
        end
    end
    return out
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("ecmascript_es5_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(es5_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(es5_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(es5_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(es5_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = es5_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = es5_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only produces only EOF", function()
        local tokens = es5_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
    end)

    it("line comment consumed silently", function()
        local tokens = es5_lexer.tokenize("// comment")
        assert.are.equal(1, #tokens)
    end)

    it("block comment consumed silently", function()
        local tokens = es5_lexer.tokenize("/* block */")
        assert.are.equal(1, #tokens)
    end)
end)

-- =========================================================================
-- ES5-specific: debugger keyword
-- =========================================================================

describe("ES5-specific: debugger keyword", function()
    it("tokenizes debugger", function()
        local tokens = es5_lexer.tokenize("debugger")
        assert.are.equal("DEBUGGER", tokens[1].type)
        assert.are.equal("debugger", tokens[1].value)
    end)

    it("debugger in statement context", function()
        assert.are.same(
            {"DEBUGGER", "SEMICOLON"},
            types(es5_lexer.tokenize("debugger;"))
        )
    end)
end)

-- =========================================================================
-- All ES3 keywords still work
-- =========================================================================

describe("ES3 keywords retained in ES5", function()
    it("tokenizes var", function()
        local tokens = es5_lexer.tokenize("var")
        assert.are.equal("VAR", tokens[1].type)
    end)

    it("tokenizes function", function()
        local tokens = es5_lexer.tokenize("function")
        assert.are.equal("FUNCTION", tokens[1].type)
    end)

    it("tokenizes try", function()
        local tokens = es5_lexer.tokenize("try")
        assert.are.equal("TRY", tokens[1].type)
    end)

    it("tokenizes catch", function()
        local tokens = es5_lexer.tokenize("catch")
        assert.are.equal("CATCH", tokens[1].type)
    end)

    it("tokenizes finally", function()
        local tokens = es5_lexer.tokenize("finally")
        assert.are.equal("FINALLY", tokens[1].type)
    end)

    it("tokenizes throw", function()
        local tokens = es5_lexer.tokenize("throw")
        assert.are.equal("THROW", tokens[1].type)
    end)

    it("tokenizes instanceof", function()
        local tokens = es5_lexer.tokenize("instanceof")
        assert.are.equal("INSTANCEOF", tokens[1].type)
    end)

    it("tokenizes true, false, null", function()
        assert.are.same(
            {"TRUE", "FALSE", "NULL"},
            types(es5_lexer.tokenize("true false null"))
        )
    end)

    it("tokenizes if, else, for, while, do, break, continue, switch, case, default, return", function()
        local t = types(es5_lexer.tokenize("if else for while do break continue switch case default return"))
        assert.are.equal("IF", t[1])
        assert.are.equal("RETURN", t[#t])
    end)

    it("tokenizes new, delete, typeof, void, in, this, with", function()
        local t = types(es5_lexer.tokenize("new delete typeof void in this with"))
        assert.are.equal("NEW", t[1])
        assert.are.equal("WITH", t[#t])
    end)
end)

-- =========================================================================
-- Strict equality (from ES3)
-- =========================================================================

describe("strict equality operators", function()
    it("strict equals ===", function()
        local tokens = es5_lexer.tokenize("===")
        assert.are.equal("STRICT_EQUALS", tokens[1].type)
    end)

    it("strict not equals !==", function()
        local tokens = es5_lexer.tokenize("!==")
        assert.are.equal("STRICT_NOT_EQUALS", tokens[1].type)
    end)

    it("loose equals ==", function()
        local tokens = es5_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
    end)

    it("not equals !=", function()
        local tokens = es5_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
    end)
end)

-- =========================================================================
-- Identifiers and literals
-- =========================================================================

describe("identifiers and literals", function()
    it("simple identifier", function()
        local tokens = es5_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
    end)

    it("integer", function()
        local tokens = es5_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("float", function()
        local tokens = es5_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("hex", function()
        local tokens = es5_lexer.tokenize("0xFF")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("string", function()
        local tokens = es5_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- Operators and delimiters
-- =========================================================================

describe("operators", function()
    it("logical AND &&", function()
        local tokens = es5_lexer.tokenize("&&")
        assert.are.equal("AND_AND", tokens[1].type)
    end)

    it("unsigned right shift >>>", function()
        local tokens = es5_lexer.tokenize(">>>")
        assert.are.equal("UNSIGNED_RIGHT_SHIFT", tokens[1].type)
    end)

    it("plus +", function()
        local tokens = es5_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
    end)

    it("question ?", function()
        local tokens = es5_lexer.tokenize("?")
        assert.are.equal("QUESTION", tokens[1].type)
    end)
end)

describe("delimiters", function()
    it("parentheses", function()
        assert.are.same({"LPAREN", "RPAREN"}, types(es5_lexer.tokenize("()")))
    end)

    it("braces", function()
        assert.are.same({"LBRACE", "RBRACE"}, types(es5_lexer.tokenize("{}")))
    end)

    it("semicolon", function()
        local tokens = es5_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("var declaration: var x = 1;", function()
        assert.are.same(
            {"VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"},
            types(es5_lexer.tokenize("var x = 1;"))
        )
    end)

    it("function declaration", function()
        local t = types(es5_lexer.tokenize("function add(a, b) { return a + b; }"))
        assert.are.equal("FUNCTION", t[1])
        assert.are.equal("RBRACE", t[#t])
    end)

    it("try/catch", function()
        local t = types(es5_lexer.tokenize("try { } catch (e) { }"))
        assert.are.equal("TRY", t[1])
    end)

    it("strict equality: a === b", function()
        assert.are.same(
            {"NAME", "STRICT_EQUALS", "NAME"},
            types(es5_lexer.tokenize("a === b"))
        )
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("column tracking: var x = 1;", function()
        local tokens = es5_lexer.tokenize("var x = 1;")
        assert.are.equal(1, tokens[1].col)
        assert.are.equal(5, tokens[2].col)
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = es5_lexer.tokenize("var x = 1;")
        for _, tok in ipairs(tokens) do
            assert.are.equal(1, tok.line)
        end
    end)
end)

-- =========================================================================
-- EOF and errors
-- =========================================================================

describe("EOF token", function()
    it("EOF is always last", function()
        local tokens = es5_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)
end)

describe("error handling", function()
    it("unexpected character # raises an error", function()
        assert.has_error(function()
            es5_lexer.tokenize("#")
        end)
    end)
end)
