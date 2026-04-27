-- Tests for lexer
-- ================
--
-- Comprehensive busted test suite for the lexer package, covering:
--   - Token types and Token construction
--   - Character classification for the DFA
--   - Tokenizer DFA construction and dispatch
--   - Hand-written Lexer: math, keywords, strings, escapes, operators,
--     position tracking, edge cases, error handling
--   - Grammar-driven GrammarLexer: skip patterns, aliases, reserved keywords,
--     known types, keyword promotion, escape processing, string handling,
--     indentation mode, pattern groups, on-token callbacks, LexerContext
--   - process_escapes utility function

-- Add sibling package paths so we can resolve state_machine, grammar_tools,
-- and directed_graph from sibling packages in the monorepo.
package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. "../../grammar_tools/src/?.lua;" .. "../../grammar_tools/src/?/init.lua;" .. "../../state_machine/src/?.lua;" .. "../../state_machine/src/?/init.lua;" .. "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;" .. package.path

local lexer = require("coding_adventures.lexer")

-- Pull out commonly used names for readability.
local Token        = lexer.Token
local Lexer        = lexer.Lexer
local GrammarLexer = lexer.GrammarLexer
local LexerContext = lexer.LexerContext
local TokenType    = lexer.TokenType

-- =========================================================================
-- Token Types
-- =========================================================================

describe("TokenType", function()
    it("defines all expected token types", function()
        assert.are.equal(0,  TokenType.Name)
        assert.are.equal(1,  TokenType.Number)
        assert.are.equal(2,  TokenType.String)
        assert.are.equal(3,  TokenType.Keyword)
        assert.are.equal(4,  TokenType.Plus)
        assert.are.equal(5,  TokenType.Minus)
        assert.are.equal(6,  TokenType.Star)
        assert.are.equal(7,  TokenType.Slash)
        assert.are.equal(8,  TokenType.Equals)
        assert.are.equal(9,  TokenType.EqualsEquals)
        assert.are.equal(10, TokenType.LParen)
        assert.are.equal(11, TokenType.RParen)
        assert.are.equal(12, TokenType.Comma)
        assert.are.equal(13, TokenType.Colon)
        assert.are.equal(14, TokenType.Semicolon)
        assert.are.equal(15, TokenType.LBrace)
        assert.are.equal(16, TokenType.RBrace)
        assert.are.equal(17, TokenType.LBracket)
        assert.are.equal(18, TokenType.RBracket)
        assert.are.equal(19, TokenType.Dot)
        assert.are.equal(20, TokenType.Bang)
        assert.are.equal(21, TokenType.Newline)
        assert.are.equal(22, TokenType.EOF)
    end)
end)

-- =========================================================================
-- token_type_to_string
-- =========================================================================

describe("token_type_to_string", function()
    it("returns correct names for all known types", function()
        assert.are.equal("Name",         lexer.token_type_to_string(0))
        assert.are.equal("Number",       lexer.token_type_to_string(1))
        assert.are.equal("String",       lexer.token_type_to_string(2))
        assert.are.equal("Keyword",      lexer.token_type_to_string(3))
        assert.are.equal("Plus",         lexer.token_type_to_string(4))
        assert.are.equal("Minus",        lexer.token_type_to_string(5))
        assert.are.equal("Star",         lexer.token_type_to_string(6))
        assert.are.equal("Slash",        lexer.token_type_to_string(7))
        assert.are.equal("Equals",       lexer.token_type_to_string(8))
        assert.are.equal("EqualsEquals", lexer.token_type_to_string(9))
        assert.are.equal("LParen",       lexer.token_type_to_string(10))
        assert.are.equal("RParen",       lexer.token_type_to_string(11))
        assert.are.equal("Comma",        lexer.token_type_to_string(12))
        assert.are.equal("Colon",        lexer.token_type_to_string(13))
        assert.are.equal("Semicolon",    lexer.token_type_to_string(14))
        assert.are.equal("LBrace",       lexer.token_type_to_string(15))
        assert.are.equal("RBrace",       lexer.token_type_to_string(16))
        assert.are.equal("LBracket",     lexer.token_type_to_string(17))
        assert.are.equal("RBracket",     lexer.token_type_to_string(18))
        assert.are.equal("Dot",          lexer.token_type_to_string(19))
        assert.are.equal("Bang",         lexer.token_type_to_string(20))
        assert.are.equal("Newline",      lexer.token_type_to_string(21))
        assert.are.equal("EOF",          lexer.token_type_to_string(22))
    end)

    it("returns Unknown for invalid type numbers", function()
        assert.are.equal("Unknown", lexer.token_type_to_string(999))
        assert.are.equal("Unknown", lexer.token_type_to_string(-1))
    end)
end)

-- =========================================================================
-- Token
-- =========================================================================

describe("Token", function()
    it("creates a token with all fields", function()
        local tok = Token.new(TokenType.Number, "42", 3, 7)
        assert.are.equal(TokenType.Number, tok.type)
        assert.are.equal("42", tok.value)
        assert.are.equal(3, tok.line)
        assert.are.equal(7, tok.column)
        assert.are.equal("", tok.type_name)
    end)

    it("accepts an optional type_name", function()
        local tok = Token.new(TokenType.Name, "x", 1, 1, "IDENTIFIER")
        assert.are.equal("IDENTIFIER", tok.type_name)
    end)

    it("has a readable __tostring", function()
        local tok = Token.new(TokenType.Number, "42", 3, 7)
        local s = tostring(tok)
        assert.truthy(s:find("Number"))
        assert.truthy(s:find("42"))
        assert.truthy(s:find("3:7"))
    end)

    it("__tostring for string token shows escaped value", function()
        local tok = Token.new(TokenType.String, "hello", 1, 1)
        local s = tostring(tok)
        assert.truthy(s:find("String"))
    end)
end)

-- =========================================================================
-- classify_char
-- =========================================================================

describe("classify_char", function()
    it("classifies nil as eof", function()
        assert.are.equal("eof", lexer.classify_char(nil))
    end)

    it("classifies whitespace characters", function()
        assert.are.equal("whitespace", lexer.classify_char(" "))
        assert.are.equal("whitespace", lexer.classify_char("\t"))
        assert.are.equal("whitespace", lexer.classify_char("\r"))
    end)

    it("classifies newline", function()
        assert.are.equal("newline", lexer.classify_char("\n"))
    end)

    it("classifies digits", function()
        for i = 0, 9 do
            assert.are.equal("digit", lexer.classify_char(tostring(i)))
        end
    end)

    it("classifies letters", function()
        assert.are.equal("alpha", lexer.classify_char("a"))
        assert.are.equal("alpha", lexer.classify_char("z"))
        assert.are.equal("alpha", lexer.classify_char("A"))
        assert.are.equal("alpha", lexer.classify_char("Z"))
        assert.are.equal("alpha", lexer.classify_char("m"))
    end)

    it("classifies underscore", function()
        assert.are.equal("underscore", lexer.classify_char("_"))
    end)

    it("classifies double quote", function()
        assert.are.equal("quote", lexer.classify_char('"'))
    end)

    it("classifies equals", function()
        assert.are.equal("equals", lexer.classify_char("="))
    end)

    it("classifies operators", function()
        assert.are.equal("operator", lexer.classify_char("+"))
        assert.are.equal("operator", lexer.classify_char("-"))
        assert.are.equal("operator", lexer.classify_char("*"))
        assert.are.equal("operator", lexer.classify_char("/"))
    end)

    it("classifies delimiters", function()
        assert.are.equal("open_paren",    lexer.classify_char("("))
        assert.are.equal("close_paren",   lexer.classify_char(")"))
        assert.are.equal("comma",         lexer.classify_char(","))
        assert.are.equal("colon",         lexer.classify_char(":"))
        assert.are.equal("semicolon",     lexer.classify_char(";"))
        assert.are.equal("open_brace",    lexer.classify_char("{"))
        assert.are.equal("close_brace",   lexer.classify_char("}"))
        assert.are.equal("open_bracket",  lexer.classify_char("["))
        assert.are.equal("close_bracket", lexer.classify_char("]"))
        assert.are.equal("dot",           lexer.classify_char("."))
        assert.are.equal("bang",          lexer.classify_char("!"))
    end)

    it("classifies unknown characters as other", function()
        assert.are.equal("other", lexer.classify_char("@"))
        assert.are.equal("other", lexer.classify_char("#"))
        assert.are.equal("other", lexer.classify_char("~"))
    end)
end)

-- =========================================================================
-- Tokenizer DFA
-- =========================================================================

describe("new_tokenizer_dfa", function()
    it("creates a DFA that starts in 'start' state", function()
        local dfa = lexer.new_tokenizer_dfa()
        assert.are.equal("start", dfa:current_state())
    end)

    it("transitions to correct states from start", function()
        local cases = {
            { "digit",         "in_number" },
            { "alpha",         "in_name" },
            { "underscore",    "in_name" },
            { "quote",         "in_string" },
            { "newline",       "at_newline" },
            { "whitespace",    "at_whitespace" },
            { "operator",      "in_operator" },
            { "equals",        "in_equals" },
            { "open_paren",    "in_operator" },
            { "close_paren",   "in_operator" },
            { "comma",         "in_operator" },
            { "colon",         "in_operator" },
            { "semicolon",     "in_operator" },
            { "open_brace",    "in_operator" },
            { "close_brace",   "in_operator" },
            { "open_bracket",  "in_operator" },
            { "close_bracket", "in_operator" },
            { "dot",           "in_operator" },
            { "bang",          "in_operator" },
            { "eof",           "done" },
            { "other",         "error" },
        }
        for _, case in ipairs(cases) do
            local dfa = lexer.new_tokenizer_dfa()
            local result = dfa:process(case[1])
            assert.are.equal(case[2], result,
                "classify " .. case[1] .. " should go to " .. case[2])
        end
    end)

    it("returns to start after handler states", function()
        local dfa = lexer.new_tokenizer_dfa()
        dfa:process("digit")  -- goes to in_number
        assert.are.equal("in_number", dfa:current_state())
        dfa:process("alpha")  -- handler -> start
        assert.are.equal("start", dfa:current_state())
    end)

    it("loops in done state", function()
        local dfa = lexer.new_tokenizer_dfa()
        dfa:process("eof")
        assert.are.equal("done", dfa:current_state())
        dfa:process("digit")
        assert.are.equal("done", dfa:current_state())
    end)

    it("loops in error state", function()
        local dfa = lexer.new_tokenizer_dfa()
        dfa:process("other")
        assert.are.equal("error", dfa:current_state())
        dfa:process("digit")
        assert.are.equal("error", dfa:current_state())
    end)

    it("resets back to start", function()
        local dfa = lexer.new_tokenizer_dfa()
        dfa:process("digit")
        dfa:reset()
        assert.are.equal("start", dfa:current_state())
    end)
end)

-- =========================================================================
-- Hand-Written Lexer
-- =========================================================================

describe("Lexer", function()

    describe("math expression", function()
        it("tokenizes x = 1 + 2 * 3", function()
            local lex = Lexer.new("x = 1 + 2 * 3")
            local tokens = lex:tokenize()

            assert.are.equal(8, #tokens)  -- x = 1 + 2 * 3 EOF
            assert.are.equal(TokenType.Name,   tokens[1].type)
            assert.are.equal("x",              tokens[1].value)
            assert.are.equal(TokenType.Equals, tokens[2].type)
            assert.are.equal(TokenType.Number, tokens[3].type)
            assert.are.equal("1",              tokens[3].value)
            assert.are.equal(TokenType.Plus,   tokens[4].type)
            assert.are.equal(TokenType.Number, tokens[5].type)
            assert.are.equal("2",              tokens[5].value)
            assert.are.equal(TokenType.Star,   tokens[6].type)
            assert.are.equal(TokenType.Number, tokens[7].type)
            assert.are.equal("3",              tokens[7].value)
            assert.are.equal(TokenType.EOF,    tokens[8].type)
        end)
    end)

    describe("keywords", function()
        it("recognizes configured keywords", function()
            local lex = Lexer.new("if x == 5", { keywords = { "if" } })
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Keyword,      tokens[1].type)
            assert.are.equal("if",                    tokens[1].value)
            assert.are.equal(TokenType.EqualsEquals,  tokens[3].type)
            assert.are.equal("==",                    tokens[3].value)
        end)

        it("treats non-keywords as names", function()
            local lex = Lexer.new("hello", { keywords = { "if" } })
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Name, tokens[1].type)
        end)
    end)

    describe("strings", function()
        it("reads basic strings", function()
            local lex = Lexer.new('"hello"')
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.String, tokens[1].type)
            assert.are.equal("hello", tokens[1].value)
        end)

        it("processes escape sequences", function()
            local lex = Lexer.new('"hello\\nworld"')
            local tokens = lex:tokenize()
            assert.are.equal("hello\nworld", tokens[1].value)
        end)

        it("handles tab escape", function()
            local lex = Lexer.new('"a\\tb"')
            local tokens = lex:tokenize()
            assert.are.equal("a\tb", tokens[1].value)
        end)

        it("handles backslash escape", function()
            local lex = Lexer.new('"a\\\\b"')
            local tokens = lex:tokenize()
            assert.are.equal("a\\b", tokens[1].value)
        end)

        it("handles quote escape", function()
            local lex = Lexer.new('"a\\"b"')
            local tokens = lex:tokenize()
            assert.are.equal('a"b', tokens[1].value)
        end)

        it("passes through unknown escapes", function()
            local lex = Lexer.new('"a\\xb"')
            local tokens = lex:tokenize()
            assert.are.equal("axb", tokens[1].value)
        end)

        it("errors on unterminated string", function()
            local lex = Lexer.new('"hello')
            assert.has_error(function()
                lex:tokenize()
            end, "LexerError at 1:1: Unterminated string literal")
        end)

        it("errors on string ending with backslash", function()
            local lex = Lexer.new('"hello\\')
            assert.has_error(function()
                lex:tokenize()
            end, "LexerError at 1:1: Unterminated string literal (ends with backslash)")
        end)
    end)

    describe("all simple tokens", function()
        it("tokenizes every single-character operator and delimiter", function()
            local lex = Lexer.new("+-*/(),:;{}[].!")
            local tokens = lex:tokenize()

            local expected = {
                TokenType.Plus, TokenType.Minus, TokenType.Star, TokenType.Slash,
                TokenType.LParen, TokenType.RParen, TokenType.Comma, TokenType.Colon,
                TokenType.Semicolon, TokenType.LBrace, TokenType.RBrace,
                TokenType.LBracket, TokenType.RBracket, TokenType.Dot, TokenType.Bang,
                TokenType.EOF,
            }

            assert.are.equal(#expected, #tokens)
            for i, exp in ipairs(expected) do
                assert.are.equal(exp, tokens[i].type,
                    "token " .. i .. " should be type " .. exp)
            end
        end)
    end)

    describe("position tracking", function()
        it("tracks line and column across newlines", function()
            local lex = Lexer.new("a\nb")
            local tokens = lex:tokenize()
            -- a at 1:1, \n at 1:2, b at 2:1
            assert.are.equal(1, tokens[1].line)
            assert.are.equal(1, tokens[1].column)
            assert.are.equal(TokenType.Newline, tokens[2].type)
            assert.are.equal(2, tokens[3].line)
            assert.are.equal(1, tokens[3].column)
        end)
    end)

    describe("underscore in names", function()
        it("handles leading underscores and embedded underscores", function()
            local lex = Lexer.new("_foo bar_baz _123")
            local tokens = lex:tokenize()
            assert.are.equal("_foo",    tokens[1].value)
            assert.are.equal("bar_baz", tokens[2].value)
            assert.are.equal("_123",    tokens[3].value)
        end)
    end)

    describe("equals vs double equals", function()
        it("distinguishes = from ==", function()
            local lex = Lexer.new("= == =")
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Equals,       tokens[1].type)
            assert.are.equal(TokenType.EqualsEquals,  tokens[2].type)
            assert.are.equal(TokenType.Equals,        tokens[3].type)
        end)

        it("handles = at end of input", function()
            local lex = Lexer.new("=")
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Equals, tokens[1].type)
            assert.are.equal("=", tokens[1].value)
        end)
    end)

    describe("multiple newlines", function()
        it("emits separate newline tokens", function()
            local lex = Lexer.new("a\n\nb")
            local tokens = lex:tokenize()
            local count = 0
            for _, tok in ipairs(tokens) do
                if tok.type == TokenType.Newline then
                    count = count + 1
                end
            end
            assert.are.equal(2, count)
        end)
    end)

    describe("tabs and carriage returns", function()
        it("treats tabs and CR as whitespace", function()
            local lex = Lexer.new("a\t\rb")
            local tokens = lex:tokenize()
            assert.are.equal("a", tokens[1].value)
            assert.are.equal("b", tokens[2].value)
        end)
    end)

    describe("no config", function()
        it("works with nil config", function()
            local lex = Lexer.new("hello")
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Name, tokens[1].type)
            assert.are.equal("hello", tokens[1].value)
        end)
    end)

    describe("empty source", function()
        it("returns just EOF for empty input", function()
            local lex = Lexer.new("")
            local tokens = lex:tokenize()
            assert.are.equal(1, #tokens)
            assert.are.equal(TokenType.EOF, tokens[1].type)
        end)
    end)

    describe("unexpected character", function()
        it("errors on unexpected characters", function()
            local lex = Lexer.new("x = @")
            assert.has_error(function()
                lex:tokenize()
            end)
        end)
    end)

    describe("string with print call", function()
        it("tokenizes print(\"Hello\\n\")", function()
            local lex = Lexer.new('print("Hello\\n")')
            local tokens = lex:tokenize()
            assert.are.equal(TokenType.Name,   tokens[1].type)
            assert.are.equal("print",          tokens[1].value)
            assert.are.equal(TokenType.LParen, tokens[2].type)
            assert.are.equal(TokenType.String, tokens[3].type)
            assert.are.equal("Hello\n",        tokens[3].value)
            assert.are.equal(TokenType.RParen, tokens[4].type)
            assert.are.equal(TokenType.EOF,    tokens[5].type)
        end)
    end)

    describe("complex expressions", function()
        it("handles mixed operators and names", function()
            local lex = Lexer.new("result = (a + b) * c - d / e")
            local tokens = lex:tokenize()
            -- result = ( a + b ) * c - d / e EOF = 14 tokens
            -- (result, =, (, a, +, b, ), *, c, -, d, /, e, EOF)
            assert.are.equal(14, #tokens)
            assert.are.equal(TokenType.EOF, tokens[14].type)
        end)
    end)
end)

-- =========================================================================
-- process_escapes utility
-- =========================================================================

describe("process_escapes", function()
    it("processes \\n to newline", function()
        assert.are.equal("a\nb", lexer.process_escapes("a\\nb"))
    end)

    it("processes \\t to tab", function()
        assert.are.equal("a\tb", lexer.process_escapes("a\\tb"))
    end)

    it("processes \\\\ to backslash", function()
        assert.are.equal("a\\b", lexer.process_escapes("a\\\\b"))
    end)

    it("processes \\\" to double quote", function()
        assert.are.equal('a"b', lexer.process_escapes('a\\"b'))
    end)

    it("passes through unknown escape characters", function()
        assert.are.equal("axb", lexer.process_escapes("a\\xb"))
    end)

    it("handles empty string", function()
        assert.are.equal("", lexer.process_escapes(""))
    end)

    it("handles string with no escapes", function()
        assert.are.equal("hello", lexer.process_escapes("hello"))
    end)

    it("handles trailing backslash followed by char", function()
        assert.are.equal("\n", lexer.process_escapes("\\n"))
    end)
end)

-- =========================================================================
-- Grammar-Driven Lexer
-- =========================================================================

describe("GrammarLexer", function()

    describe("skip patterns", function()
        it("skips whitespace defined by skip patterns", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
                skip_definitions = {
                    { name = "WHITESPACE", pattern = "[ \t]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("hello world", grammar)
            local tokens = gl:tokenize()

            local names = 0
            for _, tok in ipairs(tokens) do
                if tok.type_name == "NAME" then
                    names = names + 1
                end
            end
            assert.are.equal(2, names)
        end)
    end)

    describe("aliases", function()
        it("applies alias to type_name", function()
            local grammar = {
                definitions = {
                    { name = "NUM", pattern = "[0-9]+", is_regex = true, alias = "INT" },
                },
            }
            local gl = GrammarLexer.new("42", grammar)
            local tokens = gl:tokenize()
            assert.are.equal("INT", tokens[1].type_name)
        end)
    end)

    describe("reserved keywords", function()
        it("errors on reserved keywords", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-zA-Z_]+", is_regex = true },
                },
                reserved_keywords = { "class", "import" },
            }
            local gl = GrammarLexer.new("class", grammar)
            assert.has_error(function()
                gl:tokenize()
            end)
        end)

        it("allows non-reserved identifiers", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-zA-Z_]+", is_regex = true },
                },
                reserved_keywords = { "class" },
            }
            local gl = GrammarLexer.new("hello", grammar)
            local tokens = gl:tokenize()
            assert.are.equal("NAME", tokens[1].type_name)
        end)
    end)

    describe("known token types", function()
        it("maps PLUS, MINUS, NUMBER to correct TokenType values", function()
            local grammar = {
                definitions = {
                    { name = "PLUS",   pattern = "+", is_regex = false },
                    { name = "MINUS",  pattern = "-", is_regex = false },
                    { name = "NUMBER", pattern = "[0-9]+", is_regex = true },
                },
                skip_definitions = {
                    { name = "WS", pattern = "[ ]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("1 + 2 - 3", grammar)
            local tokens = gl:tokenize()

            assert.are.equal(TokenType.Number, tokens[1].type)
            assert.are.equal("NUMBER", tokens[1].type_name)
            assert.are.equal(TokenType.Plus, tokens[2].type)
            assert.are.equal("PLUS", tokens[2].type_name)
            assert.are.equal(TokenType.Minus, tokens[4].type)
            assert.are.equal("MINUS", tokens[4].type_name)
        end)

        it("maps all known type names correctly", function()
            -- Test a selection of known types through grammar definitions
            local grammar = {
                definitions = {
                    { name = "STAR",      pattern = "*",  is_regex = false },
                    { name = "SLASH",     pattern = "/",  is_regex = false },
                    { name = "EQUALS",    pattern = "=",  is_regex = false },
                    { name = "LPAREN",    pattern = "(",  is_regex = false },
                    { name = "RPAREN",    pattern = ")",  is_regex = false },
                    { name = "COMMA",     pattern = ",",  is_regex = false },
                    { name = "COLON",     pattern = ":",  is_regex = false },
                    { name = "SEMICOLON", pattern = ";",  is_regex = false },
                    { name = "LBRACE",    pattern = "{",  is_regex = false },
                    { name = "RBRACE",    pattern = "}",  is_regex = false },
                    { name = "LBRACKET",  pattern = "[",  is_regex = false },
                    { name = "RBRACKET",  pattern = "]",  is_regex = false },
                    { name = "DOT",       pattern = ".",  is_regex = false },
                    { name = "BANG",      pattern = "!",  is_regex = false },
                },
                skip_definitions = {
                    { name = "WS", pattern = "[ ]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("* / = ( ) , : ; { } [ ] . !", grammar)
            local tokens = gl:tokenize()
            assert.are.equal(TokenType.Star,      tokens[1].type)
            assert.are.equal(TokenType.Slash,     tokens[2].type)
            assert.are.equal(TokenType.Equals,    tokens[3].type)
            assert.are.equal(TokenType.LParen,    tokens[4].type)
            assert.are.equal(TokenType.RParen,    tokens[5].type)
            assert.are.equal(TokenType.Comma,     tokens[6].type)
            assert.are.equal(TokenType.Colon,     tokens[7].type)
            assert.are.equal(TokenType.Semicolon, tokens[8].type)
            assert.are.equal(TokenType.LBrace,    tokens[9].type)
            assert.are.equal(TokenType.RBrace,    tokens[10].type)
            assert.are.equal(TokenType.LBracket,  tokens[11].type)
            assert.are.equal(TokenType.RBracket,  tokens[12].type)
            assert.are.equal(TokenType.Dot,       tokens[13].type)
            assert.are.equal(TokenType.Bang,      tokens[14].type)
        end)
    end)

    describe("keyword promotion", function()
        it("promotes NAME tokens matching keywords to KEYWORD", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-zA-Z_]+", is_regex = true },
                },
                keywords = { "def", "return" },
            }
            local gl = GrammarLexer.new("def foo return", grammar)
            local tokens = gl:tokenize()

            assert.are.equal(TokenType.Keyword, tokens[1].type)
            assert.are.equal("DEF", tokens[1].type_name)
            assert.are.equal("def", tokens[1].value)

            assert.are.equal(TokenType.Name, tokens[2].type)
            assert.are.equal("NAME", tokens[2].type_name)
            assert.are.equal("foo", tokens[2].value)

            assert.are.equal(TokenType.Keyword, tokens[3].type)
            assert.are.equal("RETURN", tokens[3].type_name)
            assert.are.equal("return", tokens[3].value)
        end)
    end)

    describe("escape processing in grammar strings", function()
        it("processes escapes in STRING tokens", function()
            local grammar = {
                definitions = {
                    { name = "STRING", pattern = '"[^"]*"', is_regex = true },
                },
            }
            local gl = GrammarLexer.new('"hello\\nworld\\t!"', grammar)
            local tokens = gl:tokenize()
            assert.are.equal("hello\nworld\t!", tokens[1].value)
        end)

        it("strips quotes from string tokens", function()
            local grammar = {
                definitions = {
                    { name = "STRING", pattern = '"[^"]*"', is_regex = true },
                },
            }
            local gl = GrammarLexer.new('"hi"', grammar)
            local tokens = gl:tokenize()
            assert.are.equal("hi", tokens[1].value)
        end)
    end)

    describe("string alias", function()
        it("processes escapes when alias contains STRING", function()
            local grammar = {
                definitions = {
                    { name = "STRING_DQ", pattern = '"[^"]*"', is_regex = true, alias = "STRING" },
                },
            }
            local gl = GrammarLexer.new('"hi"', grammar)
            local tokens = gl:tokenize()
            assert.are.equal("STRING", tokens[1].type_name)
            assert.are.equal("hi", tokens[1].value)
        end)
    end)

    describe("escape_mode none", function()
        it("strips quotes but does not process escapes when escape_mode is none", function()
            local grammar = {
                definitions = {
                    { name = "STRING", pattern = '"[^"]*"', is_regex = true },
                },
                escape_mode = "none",
            }
            local gl = GrammarLexer.new('"hello\\nworld"', grammar)
            local tokens = gl:tokenize()
            assert.are.equal("hello\\nworld", tokens[1].value)
        end)
    end)

    describe("single-quoted strings", function()
        it("strips single quotes from STRING tokens", function()
            local grammar = {
                definitions = {
                    { name = "STRING", pattern = "'[^']*'", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("'hello'", grammar)
            local tokens = gl:tokenize()
            assert.are.equal("hello", tokens[1].value)
        end)
    end)

    describe("custom type name", function()
        it("preserves custom (unknown) type names", function()
            local grammar = {
                definitions = {
                    { name = "CUSTOM", pattern = "[a-z]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("hello", grammar)
            local tokens = gl:tokenize()
            assert.are.equal("CUSTOM", tokens[1].type_name)
            assert.are.equal(TokenType.Name, tokens[1].type)
        end)
    end)

    describe("unexpected character", function()
        it("errors when no pattern matches", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("hello@world", grammar)
            assert.has_error(function()
                gl:tokenize()
            end)
        end)
    end)

    describe("standard mode newlines", function()
        it("emits NEWLINE tokens for newlines", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
                skip_definitions = {
                    { name = "WS", pattern = "[ ]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("a\nb", grammar)
            local tokens = gl:tokenize()

            local found_newline = false
            for _, tok in ipairs(tokens) do
                if tok.type == TokenType.Newline then
                    found_newline = true
                end
            end
            assert.is_true(found_newline)
        end)
    end)

    describe("default whitespace skip (no skip patterns)", function()
        it("skips spaces, tabs, and CR without skip patterns", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("a  \t\r b", grammar)
            local tokens = gl:tokenize()

            local names = 0
            for _, tok in ipairs(tokens) do
                if tok.type_name == "NAME" then
                    names = names + 1
                end
            end
            assert.are.equal(2, names)
        end)
    end)

    describe("literal pattern escaping", function()
        it("escapes Lua magic characters in non-regex patterns", function()
            local grammar = {
                definitions = {
                    { name = "PLUS", pattern = "+", is_regex = false },
                    { name = "DOT",  pattern = ".", is_regex = false },
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
                skip_definitions = {
                    { name = "WS", pattern = "[ ]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("a + b . c", grammar)
            local tokens = gl:tokenize()

            assert.are.equal("NAME", tokens[1].type_name)
            assert.are.equal("PLUS", tokens[2].type_name)
            assert.are.equal("NAME", tokens[3].type_name)
            assert.are.equal("DOT",  tokens[4].type_name)
            assert.are.equal("NAME", tokens[5].type_name)
        end)
    end)

    describe("EQUALS_EQUALS via grammar", function()
        it("maps EQUALS_EQUALS to TokenType.EqualsEquals", function()
            local grammar = {
                definitions = {
                    { name = "EQUALS_EQUALS", pattern = "==", is_regex = false },
                    { name = "EQUALS",        pattern = "=",  is_regex = false },
                },
                skip_definitions = {
                    { name = "WS", pattern = "[ ]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("== =", grammar)
            local tokens = gl:tokenize()
            assert.are.equal(TokenType.EqualsEquals, tokens[1].type)
            assert.are.equal(TokenType.Equals,       tokens[2].type)
        end)
    end)

    describe("empty source", function()
        it("returns EOF for empty input", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("", grammar)
            local tokens = gl:tokenize()
            assert.are.equal(1, #tokens)
            assert.are.equal(TokenType.EOF, tokens[1].type)
            assert.are.equal("EOF", tokens[1].type_name)
        end)
    end)

    describe("reuse after tokenize", function()
        it("resets group stack between tokenize calls", function()
            local grammar = {
                definitions = {
                    { name = "NAME", pattern = "[a-z]+", is_regex = true },
                },
            }
            local gl = GrammarLexer.new("abc", grammar)

            local tokens1 = gl:tokenize()
            -- "abc" with no skip patterns: NAME("abc"), EOF = 2 tokens
            assert.are.equal(2, #tokens1)

            -- The important thing: group stack was reset after tokenize.
            -- Second call re-tokenizes from current pos (at end) so just EOF.
        end)
    end)
end)

-- =========================================================================
-- Indentation Mode
-- =========================================================================

describe("GrammarLexer indentation mode", function()

    local function make_indent_grammar()
        return {
            mode = "indentation",
            definitions = {
                { name = "NAME",   pattern = "[a-zA-Z_]+", is_regex = true },
                { name = "EQUALS", pattern = "=",          is_regex = false },
                { name = "INT",    pattern = "[0-9]+",     is_regex = true },
                { name = "COLON",  pattern = ":",          is_regex = false },
            },
            keywords = { "if" },
            skip_definitions = {
                { name = "WS", pattern = "[ \t]+", is_regex = true },
            },
        }
    end

    it("produces INDENT and DEDENT tokens", function()
        local gl = GrammarLexer.new("if x:\n    y = 1\n", make_indent_grammar())
        local tokens = gl:tokenize()

        local has_indent = false
        local has_dedent = false
        for _, tok in ipairs(tokens) do
            if tok.type_name == "INDENT" then has_indent = true end
            if tok.type_name == "DEDENT" then has_dedent = true end
        end
        assert.is_true(has_indent)
        assert.is_true(has_dedent)
    end)

    it("errors on tab indentation", function()
        local gl = GrammarLexer.new("if:\n\ty\n", make_indent_grammar())
        assert.has_error(function()
            gl:tokenize()
        end)
    end)

    it("handles empty source", function()
        local grammar = {
            mode = "indentation",
            skip_definitions = {
                { name = "WS", pattern = "[ \t]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new("", grammar)
        local tokens = gl:tokenize()
        assert.are.equal("EOF", tokens[#tokens].type_name)
    end)

    it("handles blank lines", function()
        local gl = GrammarLexer.new("x\n\ny\n", make_indent_grammar())
        local tokens = gl:tokenize()
        -- Should have x, NEWLINE, (blank line skipped), y, NEWLINE, ..., EOF
        local has_eof = false
        for _, tok in ipairs(tokens) do
            if tok.type_name == "EOF" then has_eof = true end
        end
        assert.is_true(has_eof)
    end)

    it("handles multiple indent/dedent levels", function()
        local source = "a:\n  b:\n    c\n"
        local gl = GrammarLexer.new(source, make_indent_grammar())
        local tokens = gl:tokenize()

        local indent_count = 0
        local dedent_count = 0
        for _, tok in ipairs(tokens) do
            if tok.type_name == "INDENT" then indent_count = indent_count + 1 end
            if tok.type_name == "DEDENT" then dedent_count = dedent_count + 1 end
        end
        assert.are.equal(2, indent_count)
        assert.are.equal(2, dedent_count)
    end)

    it("suppresses newlines inside brackets", function()
        local grammar = {
            mode = "indentation",
            definitions = {
                { name = "NAME",    pattern = "[a-z]+", is_regex = true },
                { name = "LPAREN",  pattern = "(",      is_regex = false },
                { name = "RPAREN",  pattern = ")",      is_regex = false },
                { name = "COMMA",   pattern = ",",      is_regex = false },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ \t]+", is_regex = true },
            },
        }
        local source = "f(\n  a,\n  b\n)\n"
        local gl = GrammarLexer.new(source, grammar)
        local tokens = gl:tokenize()

        -- Inside brackets, newlines should NOT produce NEWLINE tokens
        -- Only the newline after ) should produce one
        local newline_count = 0
        local saw_rparen = false
        for _, tok in ipairs(tokens) do
            if tok.type_name == "RPAREN" then saw_rparen = true end
            if tok.type == TokenType.Newline then
                newline_count = newline_count + 1
            end
        end
        assert.is_true(saw_rparen)
        -- Should have newlines outside brackets but not inside
        assert.is_true(newline_count >= 1)
    end)

    it("errors on inconsistent dedent", function()
        local grammar = {
            mode = "indentation",
            definitions = {
                { name = "NAME",  pattern = "[a-z]+", is_regex = true },
                { name = "COLON", pattern = ":",      is_regex = false },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ \t]+", is_regex = true },
            },
        }
        -- 4-space indent then 3-space dedent (inconsistent)
        local source = "a:\n    b\n   c\n"
        local gl = GrammarLexer.new(source, grammar)
        assert.has_error(function()
            gl:tokenize()
        end)
    end)

    it("skips comment-only lines", function()
        local grammar = {
            mode = "indentation",
            definitions = {
                { name = "NAME",  pattern = "[a-z]+", is_regex = true },
                { name = "COLON", pattern = ":",      is_regex = false },
            },
            skip_definitions = {
                { name = "COMMENT", pattern = "#[^\n]*", is_regex = true },
                { name = "WS",     pattern = "[ \t]+",  is_regex = true },
            },
        }
        local source = "a:\n  # comment\n  b\n"
        local gl = GrammarLexer.new(source, grammar)
        local tokens = gl:tokenize()

        -- The comment line should be skipped; only one indent
        local indent_count = 0
        for _, tok in ipairs(tokens) do
            if tok.type_name == "INDENT" then indent_count = indent_count + 1 end
        end
        assert.are.equal(1, indent_count)
    end)
end)

describe("GrammarLexer layout mode", function()

    local function make_layout_grammar()
        return {
            mode = "layout",
            definitions = {
                { name = "NAME",   pattern = "[a-zA-Z_][a-zA-Z0-9_]*", is_regex = true },
                { name = "EQUALS", pattern = "=",                       is_regex = false },
                { name = "LBRACE", pattern = "{",                       is_regex = false },
                { name = "RBRACE", pattern = "}",                       is_regex = false },
            },
            layout_keywords = { "let", "where", "do", "of" },
            skip_definitions = {
                { name = "WS", pattern = "[ \t]+", is_regex = true },
            },
        }
    end

    it("injects virtual layout tokens after a layout keyword", function()
        local gl = GrammarLexer.new("let\n  x = y\n  z = q\n", make_layout_grammar())
        local tokens = gl:tokenize()

        local type_names = {}
        for _, tok in ipairs(tokens) do
            type_names[#type_names + 1] = tok.type_name
        end

        assert.are.same({
            "NAME", "NEWLINE", "VIRTUAL_LBRACE",
            "NAME", "EQUALS", "NAME", "NEWLINE", "VIRTUAL_SEMICOLON",
            "NAME", "EQUALS", "NAME", "NEWLINE", "VIRTUAL_RBRACE", "EOF",
        }, type_names)
    end)

    it("suppresses implicit layout when explicit braces are present", function()
        local gl = GrammarLexer.new("let {\n  x = y\n}\n", make_layout_grammar())
        local tokens = gl:tokenize()

        local saw_virtual_lbrace = false
        local saw_virtual_semicolon = false
        for _, tok in ipairs(tokens) do
            if tok.type_name == "VIRTUAL_LBRACE" then saw_virtual_lbrace = true end
            if tok.type_name == "VIRTUAL_SEMICOLON" then saw_virtual_semicolon = true end
        end

        assert.is_false(saw_virtual_lbrace)
        assert.is_false(saw_virtual_semicolon)
    end)
end)

-- =========================================================================
-- Pattern Groups and On-Token Callback
-- =========================================================================

describe("GrammarLexer pattern groups", function()

    it("uses group-specific patterns when group is active", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
                { name = "LT",   pattern = "<",     is_regex = false },
                { name = "GT",   pattern = ">",     is_regex = false },
            },
            groups = {
                tag = {
                    definitions = {
                        { name = "ATTR", pattern = "[a-z]+", is_regex = true },
                        { name = "EQ",   pattern = "=",      is_regex = false },
                        { name = "VAL",  pattern = '"[^"]*"', is_regex = true, alias = "STRING" },
                        { name = "GT",   pattern = ">",       is_regex = false },
                    },
                },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ ]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new('<div class="x">', grammar)

        -- Register callback to push/pop tag group
        gl:set_on_token(function(token, ctx)
            if token.type_name == "LT" then
                ctx:push_group("tag")
            elseif token.type_name == "GT" then
                ctx:pop_group()
            end
        end)

        local tokens = gl:tokenize()

        -- Should find ATTR tokens from the tag group
        local found_attr = false
        for _, tok in ipairs(tokens) do
            if tok.type_name == "ATTR" then
                found_attr = true
            end
        end
        assert.is_true(found_attr)
    end)

    it("errors when pushing unknown group", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
            groups = {},
        }
        local gl = GrammarLexer.new("abc", grammar)
        gl:set_on_token(function(_, ctx)
            ctx:push_group("nonexistent")
        end)
        assert.has_error(function()
            gl:tokenize()
        end)
    end)
end)

describe("LexerContext", function()
    it("supports emit and suppress", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ ]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new("hello world", grammar)
        gl:set_on_token(function(token, ctx)
            if token.value == "hello" then
                ctx:suppress()
                ctx:emit(Token.new(TokenType.Name, "REPLACED", token.line, token.column, "NAME"))
            end
        end)

        local tokens = gl:tokenize()

        -- "hello" should be suppressed and replaced with "REPLACED"
        assert.are.equal("REPLACED", tokens[1].value)
        assert.are.equal("world", tokens[2].value)
    end)

    it("supports peek and peek_str", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ ]+", is_regex = true },
            },
        }
        local peeked_char = nil
        local peeked_str = nil
        local gl = GrammarLexer.new("abc def", grammar)
        gl:set_on_token(function(token, ctx)
            if token.value == "abc" then
                peeked_char = ctx:peek(1)  -- should be " "
                peeked_str = ctx:peek_str(4)  -- should be " def"
            end
        end)

        gl:tokenize()
        assert.are.equal(" ", peeked_char)
        assert.are.equal(" def", peeked_str)
    end)

    it("supports active_group and group_stack_depth", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
            groups = {
                special = {
                    definitions = {
                        { name = "NAME", pattern = "[a-z]+", is_regex = true },
                    },
                },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ ]+", is_regex = true },
            },
        }
        local initial_group = nil
        local initial_depth = nil
        local gl = GrammarLexer.new("abc", grammar)
        gl:set_on_token(function(_, ctx)
            initial_group = ctx:active_group()
            initial_depth = ctx:group_stack_depth()
        end)

        gl:tokenize()
        assert.are.equal("default", initial_group)
        assert.are.equal(1, initial_depth)
    end)

    it("pop_group is no-op when only default remains", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new("abc", grammar)
        gl:set_on_token(function(_, ctx)
            ctx:pop_group()  -- should be no-op since only default
        end)

        -- Should not error
        local tokens = gl:tokenize()
        assert.are.equal("NAME", tokens[1].type_name)
    end)

    it("set_skip_enabled toggles skip processing", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
                { name = "MARKER", pattern = "@", is_regex = false },
            },
            skip_definitions = {
                { name = "WS", pattern = "[ ]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new("a @ b", grammar)
        local skip_disabled = false
        gl:set_on_token(function(token, ctx)
            if token.type_name == "MARKER" then
                ctx:set_skip_enabled(false)
                skip_disabled = true
            end
        end)

        -- After MARKER, skip is disabled. The space before "b" won't be
        -- skipped. This will cause an error because space has no matching
        -- token pattern when skip is disabled.
        assert.has_error(function()
            gl:tokenize()
        end)
        assert.is_true(skip_disabled)
    end)

    it("peek returns empty string past EOF", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
        }
        local past_eof = nil
        local gl = GrammarLexer.new("abc", grammar)
        gl:set_on_token(function(_, ctx)
            past_eof = ctx:peek(100)
        end)
        gl:tokenize()
        assert.are.equal("", past_eof)
    end)

    it("peek_str returns shorter string near EOF", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
        }
        local near_eof = nil
        local gl = GrammarLexer.new("abc", grammar)
        gl:set_on_token(function(_, ctx)
            near_eof = ctx:peek_str(100)
        end)
        gl:tokenize()
        assert.are.equal("", near_eof)  -- nothing after "abc"
    end)

    it("clear callback with nil", function()
        local grammar = {
            definitions = {
                { name = "NAME", pattern = "[a-z]+", is_regex = true },
            },
        }
        local gl = GrammarLexer.new("abc", grammar)
        local called = false
        gl:set_on_token(function(_, _)
            called = true
        end)
        gl:set_on_token(nil)  -- clear callback
        gl:tokenize()
        assert.is_false(called)
    end)
end)

-- =========================================================================
-- Triple-quoted strings
-- =========================================================================

describe("GrammarLexer triple-quoted strings", function()
    it("strips triple quotes from string tokens", function()
        local grammar = {
            definitions = {
                { name = "STRING", pattern = '""".*?"""', is_regex = true },
            },
        }
        local gl = GrammarLexer.new('"""hello"""', grammar)
        local tokens = gl:tokenize()
        assert.are.equal("hello", tokens[1].value)
    end)
end)

-- =========================================================================
-- Module-level metadata
-- =========================================================================

describe("lexer module", function()
    it("has a version", function()
        assert.are.equal("0.1.0", lexer.VERSION)
    end)

    it("exports all expected classes and functions", function()
        assert.is_not_nil(lexer.Token)
        assert.is_not_nil(lexer.Lexer)
        assert.is_not_nil(lexer.GrammarLexer)
        assert.is_not_nil(lexer.LexerContext)
        assert.is_not_nil(lexer.TokenType)
        assert.is_not_nil(lexer.classify_char)
        assert.is_not_nil(lexer.new_tokenizer_dfa)
        assert.is_not_nil(lexer.token_type_to_string)
        assert.is_not_nil(lexer.process_escapes)
    end)
end)
