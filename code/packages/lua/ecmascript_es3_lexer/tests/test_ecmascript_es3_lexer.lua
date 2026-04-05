-- Tests for ecmascript_es3_lexer
-- ==============================
--
-- Comprehensive busted test suite for the ECMAScript 3 (1999) lexer package.
--
-- ES3 adds over ES1: strict equality (===, !==), try/catch/finally/throw,
-- instanceof, and regex literals. This test suite validates all ES3 features
-- including the new tokens not present in ES1.

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

local es3_lexer = require("coding_adventures.ecmascript_es3_lexer")

-- =========================================================================
-- Helper utilities
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

local function values(tokens)
    local out = {}
    for _, tok in ipairs(tokens) do
        if tok.type ~= "EOF" then
            out[#out + 1] = tok.value
        end
    end
    return out
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("ecmascript_es3_lexer module", function()
    it("loads successfully", function()
        assert.is_not_nil(es3_lexer)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(es3_lexer.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", es3_lexer.VERSION)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(es3_lexer.tokenize)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(es3_lexer.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = es3_lexer.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.definitions)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty and trivial inputs", function()
    it("empty string produces only EOF", function()
        local tokens = es3_lexer.tokenize("")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("whitespace-only input produces only EOF", function()
        local tokens = es3_lexer.tokenize("   \t\r\n  ")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("line comment is consumed silently", function()
        local tokens = es3_lexer.tokenize("// comment")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)

    it("block comment is consumed silently", function()
        local tokens = es3_lexer.tokenize("/* block */")
        assert.are.equal(1, #tokens)
        assert.are.equal("EOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- ES1 keywords (retained in ES3)
-- =========================================================================

describe("ES1 keywords retained in ES3", function()
    it("tokenizes var", function()
        local tokens = es3_lexer.tokenize("var")
        assert.are.equal("VAR", tokens[1].type)
    end)

    it("tokenizes function", function()
        local tokens = es3_lexer.tokenize("function")
        assert.are.equal("FUNCTION", tokens[1].type)
    end)

    it("tokenizes return", function()
        local tokens = es3_lexer.tokenize("return")
        assert.are.equal("RETURN", tokens[1].type)
    end)

    it("tokenizes if and else", function()
        assert.are.same({"IF", "ELSE"}, types(es3_lexer.tokenize("if else")))
    end)

    it("tokenizes for and while", function()
        assert.are.same({"FOR", "WHILE"}, types(es3_lexer.tokenize("for while")))
    end)

    it("tokenizes true, false, null", function()
        assert.are.same({"TRUE", "FALSE", "NULL"}, types(es3_lexer.tokenize("true false null")))
    end)

    it("tokenizes this", function()
        local tokens = es3_lexer.tokenize("this")
        assert.are.equal("THIS", tokens[1].type)
    end)

    it("tokenizes typeof", function()
        local tokens = es3_lexer.tokenize("typeof")
        assert.are.equal("TYPEOF", tokens[1].type)
    end)

    it("tokenizes new", function()
        local tokens = es3_lexer.tokenize("new")
        assert.are.equal("NEW", tokens[1].type)
    end)

    it("tokenizes delete", function()
        local tokens = es3_lexer.tokenize("delete")
        assert.are.equal("DELETE", tokens[1].type)
    end)

    it("tokenizes void", function()
        local tokens = es3_lexer.tokenize("void")
        assert.are.equal("VOID", tokens[1].type)
    end)

    it("tokenizes in", function()
        local tokens = es3_lexer.tokenize("in")
        assert.are.equal("IN", tokens[1].type)
    end)

    it("tokenizes with", function()
        local tokens = es3_lexer.tokenize("with")
        assert.are.equal("WITH", tokens[1].type)
    end)

    it("tokenizes switch, case, default, break, continue, do", function()
        assert.are.same(
            {"SWITCH", "CASE", "DEFAULT", "BREAK", "CONTINUE", "DO"},
            types(es3_lexer.tokenize("switch case default break continue do"))
        )
    end)
end)

-- =========================================================================
-- New ES3 keywords
-- =========================================================================

describe("new ES3 keywords", function()
    it("tokenizes try", function()
        local tokens = es3_lexer.tokenize("try")
        assert.are.equal("TRY", tokens[1].type)
    end)

    it("tokenizes catch", function()
        local tokens = es3_lexer.tokenize("catch")
        assert.are.equal("CATCH", tokens[1].type)
    end)

    it("tokenizes finally", function()
        local tokens = es3_lexer.tokenize("finally")
        assert.are.equal("FINALLY", tokens[1].type)
    end)

    it("tokenizes throw", function()
        local tokens = es3_lexer.tokenize("throw")
        assert.are.equal("THROW", tokens[1].type)
    end)

    it("tokenizes instanceof", function()
        local tokens = es3_lexer.tokenize("instanceof")
        assert.are.equal("INSTANCEOF", tokens[1].type)
    end)
end)

-- =========================================================================
-- Identifiers and literals
-- =========================================================================

describe("identifiers", function()
    it("simple identifier", function()
        local tokens = es3_lexer.tokenize("myVar")
        assert.are.equal("NAME", tokens[1].type)
        assert.are.equal("myVar", tokens[1].value)
    end)

    it("dollar-prefixed identifier", function()
        local tokens = es3_lexer.tokenize("$el")
        assert.are.equal("NAME", tokens[1].type)
    end)
end)

describe("number tokens", function()
    it("integer", function()
        local tokens = es3_lexer.tokenize("42")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("hex", function()
        local tokens = es3_lexer.tokenize("0xFF")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("float", function()
        local tokens = es3_lexer.tokenize("3.14")
        assert.are.equal("NUMBER", tokens[1].type)
    end)

    it("scientific notation", function()
        local tokens = es3_lexer.tokenize("1e10")
        assert.are.equal("NUMBER", tokens[1].type)
    end)
end)

describe("string tokens", function()
    it("double-quoted string", function()
        local tokens = es3_lexer.tokenize('"hello"')
        assert.are.equal("STRING", tokens[1].type)
    end)

    it("single-quoted string", function()
        local tokens = es3_lexer.tokenize("'world'")
        assert.are.equal("STRING", tokens[1].type)
    end)
end)

-- =========================================================================
-- ES3-specific: strict equality operators
-- =========================================================================

describe("strict equality operators (new in ES3)", function()
    it("strict equals ===", function()
        local tokens = es3_lexer.tokenize("===")
        assert.are.equal("STRICT_EQUALS", tokens[1].type)
        assert.are.equal("===", tokens[1].value)
    end)

    it("strict not equals !==", function()
        local tokens = es3_lexer.tokenize("!==")
        assert.are.equal("STRICT_NOT_EQUALS", tokens[1].type)
        assert.are.equal("!==", tokens[1].value)
    end)

    it("loose equals == still works", function()
        local tokens = es3_lexer.tokenize("==")
        assert.are.equal("EQUALS_EQUALS", tokens[1].type)
    end)

    it("not equals != still works", function()
        local tokens = es3_lexer.tokenize("!=")
        assert.are.equal("NOT_EQUALS", tokens[1].type)
    end)
end)

-- =========================================================================
-- Other operators (inherited from ES1)
-- =========================================================================

describe("operators", function()
    it("logical AND &&", function()
        local tokens = es3_lexer.tokenize("&&")
        assert.are.equal("AND_AND", tokens[1].type)
    end)

    it("logical OR ||", function()
        local tokens = es3_lexer.tokenize("||")
        assert.are.equal("OR_OR", tokens[1].type)
    end)

    it("increment ++", function()
        local tokens = es3_lexer.tokenize("++")
        assert.are.equal("PLUS_PLUS", tokens[1].type)
    end)

    it("decrement --", function()
        local tokens = es3_lexer.tokenize("--")
        assert.are.equal("MINUS_MINUS", tokens[1].type)
    end)

    it("unsigned right shift >>>", function()
        local tokens = es3_lexer.tokenize(">>>")
        assert.are.equal("UNSIGNED_RIGHT_SHIFT", tokens[1].type)
    end)

    it("assignment =", function()
        local tokens = es3_lexer.tokenize("=")
        assert.are.equal("EQUALS", tokens[1].type)
    end)

    it("plus +", function()
        local tokens = es3_lexer.tokenize("+")
        assert.are.equal("PLUS", tokens[1].type)
    end)

    it("minus -", function()
        local tokens = es3_lexer.tokenize("-")
        assert.are.equal("MINUS", tokens[1].type)
    end)

    it("star *", function()
        local tokens = es3_lexer.tokenize("*")
        assert.are.equal("STAR", tokens[1].type)
    end)

    it("slash /", function()
        local tokens = es3_lexer.tokenize("/")
        assert.are.equal("SLASH", tokens[1].type)
    end)

    it("bang !", function()
        local tokens = es3_lexer.tokenize("!")
        assert.are.equal("BANG", tokens[1].type)
    end)

    it("question ?", function()
        local tokens = es3_lexer.tokenize("?")
        assert.are.equal("QUESTION", tokens[1].type)
    end)
end)

-- =========================================================================
-- Delimiters
-- =========================================================================

describe("delimiter tokens", function()
    it("parentheses", function()
        assert.are.same({"LPAREN", "RPAREN"}, types(es3_lexer.tokenize("()")))
    end)

    it("braces", function()
        assert.are.same({"LBRACE", "RBRACE"}, types(es3_lexer.tokenize("{}")))
    end)

    it("brackets", function()
        assert.are.same({"LBRACKET", "RBRACKET"}, types(es3_lexer.tokenize("[]")))
    end)

    it("semicolon", function()
        local tokens = es3_lexer.tokenize(";")
        assert.are.equal("SEMICOLON", tokens[1].type)
    end)

    it("comma", function()
        local tokens = es3_lexer.tokenize(",")
        assert.are.equal("COMMA", tokens[1].type)
    end)

    it("colon", function()
        local tokens = es3_lexer.tokenize(":")
        assert.are.equal("COLON", tokens[1].type)
    end)

    it("dot", function()
        local tokens = es3_lexer.tokenize(".")
        assert.are.equal("DOT", tokens[1].type)
    end)
end)

-- =========================================================================
-- Composite expressions
-- =========================================================================

describe("composite expressions", function()
    it("var declaration: var x = 1;", function()
        assert.are.same(
            {"VAR", "NAME", "EQUALS", "NUMBER", "SEMICOLON"},
            types(es3_lexer.tokenize("var x = 1;"))
        )
    end)

    it("try/catch/finally", function()
        local t = types(es3_lexer.tokenize("try { } catch (e) { } finally { }"))
        assert.are.equal("TRY", t[1])
        assert.is_true(#t > 5)
    end)

    it("throw expression", function()
        assert.are.same(
            {"THROW", "NEW", "NAME", "LPAREN", "STRING", "RPAREN"},
            types(es3_lexer.tokenize('throw new Error("msg")'))
        )
    end)

    it("instanceof expression", function()
        assert.are.same(
            {"NAME", "INSTANCEOF", "NAME"},
            types(es3_lexer.tokenize("x instanceof Foo"))
        )
    end)

    it("strict equality: a === b", function()
        assert.are.same(
            {"NAME", "STRICT_EQUALS", "NAME"},
            types(es3_lexer.tokenize("a === b"))
        )
    end)

    it("function declaration", function()
        assert.are.same(
            {"FUNCTION", "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN",
             "LBRACE", "RETURN", "NAME", "PLUS", "NAME", "SEMICOLON", "RBRACE"},
            types(es3_lexer.tokenize("function add(a, b) { return a + b; }"))
        )
    end)
end)

-- =========================================================================
-- Position tracking
-- =========================================================================

describe("position tracking", function()
    it("column tracking: var x = 1;", function()
        local tokens = es3_lexer.tokenize("var x = 1;")
        assert.are.equal(1, tokens[1].col)
        assert.are.equal(5, tokens[2].col)
        assert.are.equal(7, tokens[3].col)
        assert.are.equal(9, tokens[4].col)
        assert.are.equal(10, tokens[5].col)
    end)

    it("all tokens on line 1 for single-line input", function()
        local tokens = es3_lexer.tokenize("var x = 1;")
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
        local tokens = es3_lexer.tokenize("1")
        assert.are.equal("EOF", tokens[#tokens].type)
    end)
end)

describe("error handling", function()
    it("unexpected character # raises an error", function()
        assert.has_error(function()
            es3_lexer.tokenize("#")
        end)
    end)

    it("backtick raises an error", function()
        assert.has_error(function()
            es3_lexer.tokenize("`hello`")
        end)
    end)
end)
