-- Tests for parser -- comprehensive busted test suite
-- ====================================================
--
-- These tests cover:
--   1. Hand-written Parser: expressions, assignments, precedence, errors
--   2. AST node types: NumberLiteral, StringLiteral, NameNode, BinaryOp, etc.
--   3. GrammarParser: packrat memoization, alternation, optional, repetition,
--      groups, literals, newline significance, trace mode, error handling
--   4. ParseError and GrammarParseError formatting
--   5. ASTNode: is_leaf, token methods
--   6. Token type utilities: token_type_name, all token type constants
--
-- Running: cd tests && busted test_parser.lua

-- Add all transitive dependency paths so require() can find every package.
package.path = "../src/?.lua;" .. "../src/?/init.lua;"
    .. "../../grammar_tools/src/?.lua;" .. "../../grammar_tools/src/?/init.lua;"
    .. "../../lexer/src/?.lua;" .. "../../lexer/src/?/init.lua;"
    .. "../../state_machine/src/?.lua;" .. "../../state_machine/src/?/init.lua;"
    .. "../../directed_graph/src/?.lua;" .. "../../directed_graph/src/?/init.lua;"
    .. package.path

local parser = require("coding_adventures.parser")

-- Shortcuts for token type constants
local T = {
    NAME          = parser.TOKEN_NAME,
    NUMBER        = parser.TOKEN_NUMBER,
    STRING        = parser.TOKEN_STRING,
    KEYWORD       = parser.TOKEN_KEYWORD,
    PLUS          = parser.TOKEN_PLUS,
    MINUS         = parser.TOKEN_MINUS,
    STAR          = parser.TOKEN_STAR,
    SLASH         = parser.TOKEN_SLASH,
    EQUALS        = parser.TOKEN_EQUALS,
    EQUALS_EQUALS = parser.TOKEN_EQUALS_EQUALS,
    LPAREN        = parser.TOKEN_LPAREN,
    RPAREN        = parser.TOKEN_RPAREN,
    COMMA         = parser.TOKEN_COMMA,
    COLON         = parser.TOKEN_COLON,
    SEMICOLON     = parser.TOKEN_SEMICOLON,
    LBRACE        = parser.TOKEN_LBRACE,
    RBRACE        = parser.TOKEN_RBRACE,
    LBRACKET      = parser.TOKEN_LBRACKET,
    RBRACKET      = parser.TOKEN_RBRACKET,
    DOT           = parser.TOKEN_DOT,
    BANG          = parser.TOKEN_BANG,
    NEWLINE       = parser.TOKEN_NEWLINE,
    EOF           = parser.TOKEN_EOF,
}

--- Helper: create a token table.
local function tok(type_val, value, line, column, type_name)
    return {
        type = type_val,
        value = value,
        line = line or 1,
        column = column or 1,
        type_name = type_name or "",
    }
end

--- Helper: create an EOF token.
local function eof(line, column)
    return tok(T.EOF, "", line or 1, column or 1)
end

--- Helper: build a grammar rule element (mimics grammar-tools output).
-- Grammar-tools produces tables with a `type` field and type-specific data.
-- Note: grammar-tools uses `type` (not `kind`) for element discrimination.
-- The `kind` field is only used by ASTNode objects (parsed output).
local grammar = {}

function grammar.rule_ref(name, is_token)
    return { type = "rule_reference", name = name, is_token = is_token }
end

function grammar.literal(value)
    return { type = "literal", value = value }
end

function grammar.sequence(elements)
    return { type = "sequence", elements = elements }
end

function grammar.alternation(choices)
    return { type = "alternation", choices = choices }
end

function grammar.repetition(element)
    return { type = "repetition", element = element }
end

function grammar.optional(element)
    return { type = "optional", element = element }
end

function grammar.group(element)
    return { type = "group", element = element }
end

--- Helper: build a ParserGrammar table from rules.
function grammar.make(rules_list)
    return { rules = rules_list }
end

--- Helper: build a grammar rule.
function grammar.rule(name, body)
    return { name = name, body = body }
end


-- =========================================================================
-- Version
-- =========================================================================

describe("parser", function()
    it("has a version", function()
        assert.are.equal("0.1.0", parser.VERSION)
    end)
end)

-- =========================================================================
-- Token type constants
-- =========================================================================

describe("token type constants", function()
    it("defines all 23 token type constants", function()
        -- Verify they are integers 0..22
        assert.are.equal(0,  parser.TOKEN_NAME)
        assert.are.equal(1,  parser.TOKEN_NUMBER)
        assert.are.equal(2,  parser.TOKEN_STRING)
        assert.are.equal(3,  parser.TOKEN_KEYWORD)
        assert.are.equal(4,  parser.TOKEN_PLUS)
        assert.are.equal(5,  parser.TOKEN_MINUS)
        assert.are.equal(6,  parser.TOKEN_STAR)
        assert.are.equal(7,  parser.TOKEN_SLASH)
        assert.are.equal(8,  parser.TOKEN_EQUALS)
        assert.are.equal(9,  parser.TOKEN_EQUALS_EQUALS)
        assert.are.equal(10, parser.TOKEN_LPAREN)
        assert.are.equal(11, parser.TOKEN_RPAREN)
        assert.are.equal(12, parser.TOKEN_COMMA)
        assert.are.equal(13, parser.TOKEN_COLON)
        assert.are.equal(14, parser.TOKEN_SEMICOLON)
        assert.are.equal(15, parser.TOKEN_LBRACE)
        assert.are.equal(16, parser.TOKEN_RBRACE)
        assert.are.equal(17, parser.TOKEN_LBRACKET)
        assert.are.equal(18, parser.TOKEN_RBRACKET)
        assert.are.equal(19, parser.TOKEN_DOT)
        assert.are.equal(20, parser.TOKEN_BANG)
        assert.are.equal(21, parser.TOKEN_NEWLINE)
        assert.are.equal(22, parser.TOKEN_EOF)
    end)
end)

-- =========================================================================
-- token_type_name
-- =========================================================================

describe("token_type_name", function()
    it("returns type_name when set", function()
        local t = tok(T.NAME, "x", 1, 1, "CUSTOM_TYPE")
        assert.are.equal("CUSTOM_TYPE", parser.token_type_name(t))
    end)

    it("falls back to numeric type mapping", function()
        local t = tok(T.NUMBER, "42", 1, 1)
        assert.are.equal("NUMBER", parser.token_type_name(t))
    end)

    it("returns UNKNOWN for unrecognised types", function()
        local t = { type = 999, value = "?", line = 1, column = 1 }
        assert.are.equal("UNKNOWN", parser.token_type_name(t))
    end)

    it("maps all known token types", function()
        local expected = {
            [T.NAME] = "NAME", [T.NUMBER] = "NUMBER", [T.STRING] = "STRING",
            [T.KEYWORD] = "KEYWORD", [T.PLUS] = "PLUS", [T.MINUS] = "MINUS",
            [T.STAR] = "STAR", [T.SLASH] = "SLASH", [T.EQUALS] = "EQUALS",
            [T.EQUALS_EQUALS] = "EQUALS_EQUALS",
            [T.LPAREN] = "LPAREN", [T.RPAREN] = "RPAREN",
            [T.COMMA] = "COMMA", [T.COLON] = "COLON",
            [T.SEMICOLON] = "SEMICOLON",
            [T.LBRACE] = "LBRACE", [T.RBRACE] = "RBRACE",
            [T.LBRACKET] = "LBRACKET", [T.RBRACKET] = "RBRACKET",
            [T.DOT] = "DOT", [T.BANG] = "BANG",
            [T.NEWLINE] = "NEWLINE", [T.EOF] = "EOF",
        }
        for type_val, name in pairs(expected) do
            local t = tok(type_val, "x", 1, 1)
            assert.are.equal(name, parser.token_type_name(t),
                "Failed for type " .. type_val)
        end
    end)

    it("prefers type_name over numeric type", function()
        -- Even if numeric type is NUMBER, type_name takes priority
        local t = tok(T.NUMBER, "42", 1, 1, "INT")
        assert.are.equal("INT", parser.token_type_name(t))
    end)

    it("treats empty string type_name as unset", function()
        local t = tok(T.PLUS, "+", 1, 1, "")
        assert.are.equal("PLUS", parser.token_type_name(t))
    end)
end)

-- =========================================================================
-- AST node types
-- =========================================================================

describe("AST node types", function()
    describe("NumberLiteral", function()
        it("stores value and kind", function()
            local n = parser.NumberLiteral.new(42)
            assert.are.equal("NumberLiteral", n.kind)
            assert.are.equal(42, n.value)
        end)
    end)

    describe("StringLiteral", function()
        it("stores value and kind", function()
            local s = parser.StringLiteral.new("hello")
            assert.are.equal("StringLiteral", s.kind)
            assert.are.equal("hello", s.value)
        end)
    end)

    describe("NameNode", function()
        it("stores name and kind", function()
            local n = parser.NameNode.new("foo")
            assert.are.equal("NameNode", n.kind)
            assert.are.equal("foo", n.name)
        end)
    end)

    describe("BinaryOp", function()
        it("stores left, op, right and kind", function()
            local left = parser.NumberLiteral.new(1)
            local right = parser.NumberLiteral.new(2)
            local b = parser.BinaryOp.new(left, "+", right)
            assert.are.equal("BinaryOp", b.kind)
            assert.are.equal("+", b.op)
            assert.are.equal(1, b.left.value)
            assert.are.equal(2, b.right.value)
        end)
    end)

    describe("Assignment", function()
        it("stores target and value", function()
            local target = parser.NameNode.new("x")
            local value = parser.NumberLiteral.new(42)
            local a = parser.Assignment.new(target, value)
            assert.are.equal("Assignment", a.kind)
            assert.are.equal("x", a.target.name)
            assert.are.equal(42, a.value.value)
        end)
    end)

    describe("ExpressionStmt", function()
        it("stores expression", function()
            local expr = parser.NumberLiteral.new(99)
            local es = parser.ExpressionStmt.new(expr)
            assert.are.equal("ExpressionStmt", es.kind)
            assert.are.equal(99, es.expression.value)
        end)
    end)

    describe("Program", function()
        it("stores statements", function()
            local stmts = {
                parser.ExpressionStmt.new(parser.NumberLiteral.new(1)),
                parser.ExpressionStmt.new(parser.NumberLiteral.new(2)),
            }
            local p = parser.Program.new(stmts)
            assert.are.equal("Program", p.kind)
            assert.are.equal(2, #p.statements)
        end)
    end)
end)

-- =========================================================================
-- ParseError
-- =========================================================================

describe("ParseError", function()
    it("formats error with line and column", function()
        local err = parser.ParseError.new("test error", tok(T.NAME, "x", 5, 10))
        assert.are.equal("test error at line 5, column 10", err:error_string())
    end)

    it("works with tostring", function()
        local err = parser.ParseError.new("bad token", tok(T.PLUS, "+", 3, 7))
        assert.are.equal("bad token at line 3, column 7", tostring(err))
    end)
end)

-- =========================================================================
-- Hand-written Parser
-- =========================================================================

describe("Parser (hand-written)", function()
    describe("expressions", function()
        it("parses addition: 1 + 2", function()
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            assert.are.equal(1, #prog.statements)
            local stmt = prog.statements[1]
            assert.are.equal("ExpressionStmt", stmt.kind)
            local expr = stmt.expression
            assert.are.equal("BinaryOp", expr.kind)
            assert.are.equal("+", expr.op)
            assert.are.equal(1, expr.left.value)
            assert.are.equal(2, expr.right.value)
        end)

        it("parses subtraction: 5 - 3", function()
            local tokens = {
                tok(T.NUMBER, "5", 1, 1),
                tok(T.MINUS,  "-", 1, 3),
                tok(T.NUMBER, "3", 1, 5),
                eof(1, 6),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local binop = prog.statements[1].expression
            assert.are.equal("-", binop.op)
        end)

        it("parses multiplication: 2 * 3", function()
            local tokens = {
                tok(T.NUMBER, "2", 1, 1),
                tok(T.STAR,   "*", 1, 3),
                tok(T.NUMBER, "3", 1, 5),
                eof(1, 6),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local binop = prog.statements[1].expression
            assert.are.equal("*", binop.op)
        end)

        it("parses division: 6 / 2", function()
            local tokens = {
                tok(T.NUMBER, "6", 1, 1),
                tok(T.SLASH,  "/", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local binop = prog.statements[1].expression
            assert.are.equal("/", binop.op)
        end)

        it("respects precedence: 1 + 2 * 3 = 1 + (2*3)", function()
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                tok(T.STAR,   "*", 1, 7),
                tok(T.NUMBER, "3", 1, 9),
                eof(1, 10),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local add = prog.statements[1].expression
            assert.are.equal("+", add.op)
            assert.are.equal("NumberLiteral", add.left.kind)
            assert.are.equal(1, add.left.value)
            assert.are.equal("BinaryOp", add.right.kind)
            assert.are.equal("*", add.right.op)
        end)

        it("parses parenthesized expression: (1 + 2)", function()
            local tokens = {
                tok(T.LPAREN, "(", 1, 1),
                tok(T.NUMBER, "1", 1, 2),
                tok(T.PLUS,   "+", 1, 4),
                tok(T.NUMBER, "2", 1, 6),
                tok(T.RPAREN, ")", 1, 7),
                eof(1, 8),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local binop = prog.statements[1].expression
            assert.are.equal("+", binop.op)
        end)

        it("parses string literal", function()
            local tokens = {
                tok(T.STRING, "hello", 1, 1),
                eof(1, 8),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local expr = prog.statements[1].expression
            assert.are.equal("StringLiteral", expr.kind)
            assert.are.equal("hello", expr.value)
        end)

        it("parses name as expression", function()
            local tokens = {
                tok(T.NAME, "foo", 1, 1),
                eof(1, 4),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local expr = prog.statements[1].expression
            assert.are.equal("NameNode", expr.kind)
            assert.are.equal("foo", expr.name)
        end)
    end)

    describe("assignments", function()
        it("parses simple assignment: x = 42", function()
            local tokens = {
                tok(T.NAME,   "x",  1, 1),
                tok(T.EQUALS, "=",  1, 3),
                tok(T.NUMBER, "42", 1, 5),
                eof(1, 7),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            assert.are.equal(1, #prog.statements)
            local stmt = prog.statements[1]
            assert.are.equal("Assignment", stmt.kind)
            assert.are.equal("x", stmt.target.name)
            assert.are.equal(42, stmt.value.value)
        end)

        it("parses assignment with expression: x = 1 + 2", function()
            local tokens = {
                tok(T.NAME,   "x", 1, 1),
                tok(T.EQUALS, "=", 1, 3),
                tok(T.NUMBER, "1", 1, 5),
                tok(T.PLUS,   "+", 1, 7),
                tok(T.NUMBER, "2", 1, 9),
                eof(1, 10),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local stmt = prog.statements[1]
            assert.are.equal("Assignment", stmt.kind)
            assert.are.equal("x", stmt.target.name)
            assert.are.equal("BinaryOp", stmt.value.kind)
            assert.are.equal("+", stmt.value.op)
        end)
    end)

    describe("multiple statements", function()
        it("parses three statements separated by newlines", function()
            local tokens = {
                tok(T.NUMBER,  "1",  1, 1),
                tok(T.NEWLINE, "\n", 1, 2),
                tok(T.NUMBER,  "2",  2, 1),
                tok(T.NEWLINE, "\n", 2, 2),
                tok(T.NUMBER,  "3",  3, 1),
                eof(3, 2),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            assert.are.equal(3, #prog.statements)
        end)
    end)

    describe("newline handling", function()
        it("skips leading newlines", function()
            local tokens = {
                tok(T.NEWLINE, "\n", 1, 1),
                tok(T.NEWLINE, "\n", 2, 1),
                tok(T.NUMBER,  "42", 3, 1),
                eof(3, 3),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            assert.are.equal(1, #prog.statements)
        end)
    end)

    describe("error handling", function()
        it("raises ParseError on unexpected token", function()
            local tokens = {
                tok(T.PLUS, "+", 1, 1),
                eof(1, 2),
            }
            local p = parser.Parser.new(tokens)
            local ok, err = pcall(function() p:parse() end)
            assert.is_false(ok)
            -- The error should be a ParseError or contain its message
            assert.truthy(tostring(err):find("Unexpected token"))
        end)

        it("ParseError includes line and column", function()
            local tokens = {
                tok(T.PLUS, "+", 3, 7),
                eof(3, 8),
            }
            local p = parser.Parser.new(tokens)
            local ok, err = pcall(function() p:parse() end)
            assert.is_false(ok)
            assert.truthy(tostring(err):find("line 3"))
        end)

        it("raises error on missing expected token", function()
            -- Assignment without EQUALS: x 42 (missing =)
            -- Actually, this should parse as expression stmt "x" then "42"
            -- Let's test missing closing paren instead
            local tokens = {
                tok(T.LPAREN, "(", 1, 1),
                tok(T.NUMBER, "1", 1, 2),
                eof(1, 3),
            }
            local p = parser.Parser.new(tokens)
            local ok, err = pcall(function() p:parse() end)
            assert.is_false(ok)
            assert.truthy(tostring(err):find("Expected"))
        end)
    end)

    describe("empty program", function()
        it("parses empty input", function()
            local tokens = { eof(1, 1) }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            assert.are.equal(0, #prog.statements)
        end)
    end)

    describe("left-associativity", function()
        it("left-associates addition: 1 + 2 + 3 = (1+2) + 3", function()
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                tok(T.PLUS,   "+", 1, 7),
                tok(T.NUMBER, "3", 1, 9),
                eof(1, 10),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local outer = prog.statements[1].expression
            assert.are.equal("+", outer.op)
            assert.are.equal("BinaryOp", outer.left.kind)
            assert.are.equal("+", outer.left.op)
            assert.are.equal("NumberLiteral", outer.right.kind)
            assert.are.equal(3, outer.right.value)
        end)

        it("left-associates multiplication: 2 * 3 * 4 = (2*3) * 4", function()
            local tokens = {
                tok(T.NUMBER, "2", 1, 1),
                tok(T.STAR,   "*", 1, 3),
                tok(T.NUMBER, "3", 1, 5),
                tok(T.STAR,   "*", 1, 7),
                tok(T.NUMBER, "4", 1, 9),
                eof(1, 10),
            }
            local p = parser.Parser.new(tokens)
            local prog = p:parse()
            local outer = prog.statements[1].expression
            assert.are.equal("*", outer.op)
            assert.are.equal("BinaryOp", outer.left.kind)
            assert.are.equal("NumberLiteral", outer.right.kind)
        end)
    end)
end)

-- =========================================================================
-- ASTNode (grammar-driven)
-- =========================================================================

describe("ASTNode", function()
    it("creates with rule_name and children", function()
        local node = parser.ASTNode.new("expr", { "a", "b" })
        assert.are.equal("expr", node.rule_name)
        assert.are.equal(2, #node.children)
    end)

    it("defaults children to empty table", function()
        local node = parser.ASTNode.new("empty")
        assert.are.equal(0, #node.children)
    end)

    describe("is_leaf", function()
        it("returns true for single token child", function()
            local t = tok(T.NUMBER, "42", 1, 1)
            local node = parser.ASTNode.new("num", { t })
            assert.is_true(node:is_leaf())
        end)

        it("returns false for multiple children", function()
            local t = tok(T.NUMBER, "42", 1, 1)
            local child = parser.ASTNode.new("inner", { t })
            local node = parser.ASTNode.new("expr", { child, t })
            assert.is_false(node:is_leaf())
        end)

        it("returns false for empty children", function()
            local node = parser.ASTNode.new("empty", {})
            assert.is_false(node:is_leaf())
        end)

        it("returns false when single child is an ASTNode", function()
            local child = parser.ASTNode.new("inner", {})
            local node = parser.ASTNode.new("outer", { child })
            assert.is_false(node:is_leaf())
        end)
    end)

    describe("token", function()
        it("returns the token for a leaf node", function()
            local t = tok(T.NUMBER, "42", 1, 1)
            local node = parser.ASTNode.new("num", { t })
            local result = node:token()
            assert.is_not_nil(result)
            assert.are.equal("42", result.value)
        end)

        it("returns nil for a non-leaf node", function()
            local child = parser.ASTNode.new("inner", {})
            local node = parser.ASTNode.new("outer", { child })
            assert.is_nil(node:token())
        end)
    end)
end)

-- =========================================================================
-- GrammarParseError
-- =========================================================================

describe("GrammarParseError", function()
    it("formats error with line:column", function()
        local err = parser.GrammarParseError.new("Expected NUMBER",
            tok(T.NAME, "x", 3, 5))
        assert.are.equal("Parse error at 3:5: Expected NUMBER", err:error_string())
    end)

    it("works with tostring", function()
        local err = parser.GrammarParseError.new("bad", tok(T.EOF, "", 1, 1))
        assert.are.equal("Parse error at 1:1: bad", tostring(err))
    end)
end)

-- =========================================================================
-- GrammarParser
-- =========================================================================

describe("GrammarParser", function()
    describe("basic parsing", function()
        it("parses a single NUMBER token", function()
            -- Grammar: expr = NUMBER ;
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.is_not_nil(ast)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("parses via rule reference", function()
            -- Grammar: program = expr ; expr = NUMBER ;
            local g = grammar.make({
                grammar.rule("program", grammar.rule_ref("expr", false)),
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("program", ast.rule_name)
        end)
    end)

    describe("memoization", function()
        it("produces same result on second parse", function()
            -- Grammar: program = { statement } ; statement = assignment | expression_stmt ;
            -- assignment = NAME EQUALS expression ; expression_stmt = expression ;
            -- expression = NUMBER ;
            local g = grammar.make({
                grammar.rule("program",
                    grammar.repetition(grammar.rule_ref("statement", false))),
                grammar.rule("statement",
                    grammar.alternation({
                        grammar.rule_ref("assignment", false),
                        grammar.rule_ref("expression_stmt", false),
                    })),
                grammar.rule("assignment",
                    grammar.sequence({
                        grammar.rule_ref("NAME", true),
                        grammar.rule_ref("EQUALS", true),
                        grammar.rule_ref("expression", false),
                    })),
                grammar.rule("expression_stmt",
                    grammar.rule_ref("expression", false)),
                grammar.rule("expression",
                    grammar.rule_ref("NUMBER", true)),
            })

            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }

            local p1 = parser.GrammarParser.new(tokens, g)
            local ast1, err1 = p1:parse()
            assert.is_nil(err1)

            local p2 = parser.GrammarParser.new(tokens, g)
            local ast2, err2 = p2:parse()
            assert.is_nil(err2)

            assert.are.equal(ast1.rule_name, ast2.rule_name)
        end)

        it("exercises memo hit on same rule at same position", function()
            -- Grammar: expr = add_expr | NUMBER ; add_expr = NUMBER PLUS NUMBER ;
            -- When parsing "1 + 2", expr tries add_expr first (which tries NUMBER).
            -- Then if add_expr fails... but actually it succeeds here.
            -- The memo is exercised internally.
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.rule_ref("add_expr", false),
                        grammar.rule_ref("NUMBER", true),
                    })),
                grammar.rule("add_expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("PLUS", true),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })

            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }

            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("handles direct left recursion by growing the memoized match", function()
            -- Grammar: expr = expr PLUS NUMBER | NUMBER ;
            -- This would recurse forever without the seed-and-grow guard.
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.sequence({
                            grammar.rule_ref("expr", false),
                            grammar.rule_ref("PLUS", true),
                            grammar.rule_ref("NUMBER", true),
                        }),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })

            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                tok(T.PLUS,   "+", 1, 7),
                tok(T.NUMBER, "3", 1, 9),
                eof(1, 10),
            }

            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
            assert.are.equal(3, #ast.children)
            assert.are.equal("expr", ast.children[1].rule_name)
        end)
    end)

    describe("string-based token types", function()
        it("matches tokens by type_name field", function()
            -- Grammar: expr = INT ;
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("INT", true)),
            })
            local tokens = {
                tok(T.NAME, "42", 1, 1, "INT"),
                tok(T.EOF, "", 1, 3, "EOF"),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)
    end)

    describe("alternation", function()
        it("matches first alternative (NUMBER)", function()
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("NAME", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("matches second alternative (NAME)", function()
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("NAME", true),
                    })),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1),
                eof(1, 2),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)
    end)

    describe("optional", function()
        it("matches without optional part", function()
            -- Grammar: expr = NUMBER [PLUS NUMBER] ;
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.optional(
                            grammar.sequence({
                                grammar.rule_ref("PLUS", true),
                                grammar.rule_ref("NUMBER", true),
                            })
                        ),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)

        it("matches with optional part", function()
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.optional(
                            grammar.sequence({
                                grammar.rule_ref("PLUS", true),
                                grammar.rule_ref("NUMBER", true),
                            })
                        ),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)
    end)

    describe("repetition", function()
        it("matches zero or more", function()
            -- Grammar: list = { NUMBER } ;
            local g = grammar.make({
                grammar.rule("list",
                    grammar.repetition(grammar.rule_ref("NUMBER", true))),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.NUMBER, "2", 1, 3),
                tok(T.NUMBER, "3", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal(3, #ast.children)
        end)

        it("matches zero times", function()
            local g = grammar.make({
                grammar.rule("list",
                    grammar.repetition(grammar.rule_ref("NUMBER", true))),
            })
            local tokens = { eof(1, 1) }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal(0, #ast.children)
        end)
    end)

    describe("group", function()
        it("matches grouped alternation", function()
            -- Grammar: expr = NUMBER (PLUS | MINUS) NUMBER ;
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.group(
                            grammar.alternation({
                                grammar.rule_ref("PLUS", true),
                                grammar.rule_ref("MINUS", true),
                            })
                        ),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.MINUS,  "-", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)
    end)

    describe("literal", function()
        it("matches literal value", function()
            -- Grammar: expr = NUMBER "+" NUMBER ;
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.literal("+"),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)

        it("fails on literal mismatch", function()
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.literal("+"),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.MINUS, "-", 1, 1),
                tok(T.NUMBER, "1", 1, 3),
                eof(1, 4),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)
    end)

    describe("newline significance", function()
        it("detects significant newlines", function()
            -- Grammar: file = { NAME NEWLINE } ;
            local g = grammar.make({
                grammar.rule("file",
                    grammar.repetition(
                        grammar.sequence({
                            grammar.rule_ref("NAME", true),
                            grammar.rule_ref("NEWLINE", true),
                        })
                    )),
            })
            local tokens = {
                tok(T.NAME,    "x",  1, 1),
                tok(T.NEWLINE, "\n", 1, 2),
                eof(2, 1),
            }
            local p = parser.GrammarParser.new(tokens, g)
            assert.is_true(p:newlines_are_significant())
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("file", ast.rule_name)
        end)

        it("detects insignificant newlines", function()
            -- Grammar: expr = NUMBER ;
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NEWLINE, "\n", 1, 1),
                tok(T.NUMBER,  "42", 2, 1),
                eof(2, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            assert.is_false(p:newlines_are_significant())
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("detects NEWLINE in alternation", function()
            local g = grammar.make({
                grammar.rule("line",
                    grammar.alternation({
                        grammar.sequence({
                            grammar.rule_ref("NAME", true),
                            grammar.rule_ref("NEWLINE", true),
                        }),
                        grammar.sequence({
                            grammar.rule_ref("NUMBER", true),
                            grammar.rule_ref("NEWLINE", true),
                        }),
                    })),
            })
            local p = parser.GrammarParser.new({ eof(1, 1) }, g)
            assert.is_true(p:newlines_are_significant())
        end)

        it("detects NEWLINE in repetition", function()
            local g = grammar.make({
                grammar.rule("lines",
                    grammar.repetition(
                        grammar.sequence({
                            grammar.rule_ref("NAME", true),
                            grammar.rule_ref("NEWLINE", true),
                        })
                    )),
            })
            local p = parser.GrammarParser.new({ eof(1, 1) }, g)
            assert.is_true(p:newlines_are_significant())
        end)

        it("detects NEWLINE in optional", function()
            local g = grammar.make({
                grammar.rule("line",
                    grammar.sequence({
                        grammar.rule_ref("NAME", true),
                        grammar.optional(grammar.rule_ref("NEWLINE", true)),
                    })),
            })
            local p = parser.GrammarParser.new({ eof(1, 1) }, g)
            assert.is_true(p:newlines_are_significant())
        end)

        it("detects NEWLINE in group", function()
            local g = grammar.make({
                grammar.rule("line",
                    grammar.group(
                        grammar.sequence({
                            grammar.rule_ref("NAME", true),
                            grammar.rule_ref("NEWLINE", true),
                        })
                    )),
            })
            local p = parser.GrammarParser.new({ eof(1, 1) }, g)
            assert.is_true(p:newlines_are_significant())
        end)

        it("skips trailing newlines when insignificant", function()
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER,  "42", 1, 1),
                tok(T.NEWLINE, "\n", 1, 3),
                tok(T.NEWLINE, "\n", 2, 1),
                eof(3, 1),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)
    end)

    describe("error handling", function()
        it("returns error for empty grammar", function()
            local g = grammar.make({})
            local tokens = { eof(1, 1) }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
            assert.truthy(err:find("no rules"))
        end)

        it("returns error on parse failure", function()
            -- Grammar: expr = NUMBER PLUS NUMBER ;
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("PLUS", true),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1),
                eof(1, 2),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)

        it("returns error on unconsumed tokens", function()
            -- Grammar: expr = NUMBER ;
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)

        it("reports furthest failure position", function()
            -- Grammar where assignment gets further than expression_stmt
            local g = grammar.make({
                grammar.rule("program",
                    grammar.repetition(grammar.rule_ref("statement", false))),
                grammar.rule("statement",
                    grammar.alternation({
                        grammar.rule_ref("assignment", false),
                        grammar.rule_ref("expr_stmt", false),
                    })),
                grammar.rule("assignment",
                    grammar.sequence({
                        grammar.rule_ref("NAME", true),
                        grammar.rule_ref("EQUALS", true),
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("NEWLINE", true),
                    })),
                grammar.rule("expr_stmt",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("NEWLINE", true),
                    })),
            })

            local tokens = {
                tok(T.NAME,   "x", 1, 1),
                tok(T.EQUALS, "=", 1, 3),
                tok(T.PLUS,   "+", 1, 5),  -- Error: expected NUMBER
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)

        it("reports furthest failure with unconsumed tokens", function()
            -- Grammar: expr = term ; term = NUMBER PLUS NUMBER ;
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("term", false)),
                grammar.rule("term",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("PLUS", true),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                tok(T.STAR,   "*", 1, 7),
                tok(T.NUMBER, "3", 1, 9),
                eof(1, 10),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)

        it("reports error when no furthest expected", function()
            -- Grammar: expr = { NUMBER } ;
            -- Only NAME tokens, repetition matches 0 times, then unconsumed NAME
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.repetition(grammar.rule_ref("NUMBER", true))),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1),
                eof(1, 2),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
            assert.truthy(err:find("Unexpected token"))
        end)
    end)

    describe("Starlark-like pipeline", function()
        it("parses assignment with significant newlines", function()
            local g = grammar.make({
                grammar.rule("file",
                    grammar.repetition(
                        grammar.sequence({
                            grammar.rule_ref("statement", false),
                            grammar.rule_ref("NEWLINE", true),
                        })
                    )),
                grammar.rule("statement",
                    grammar.alternation({
                        grammar.rule_ref("assignment", false),
                        grammar.rule_ref("simple_expr", false),
                    })),
                grammar.rule("assignment",
                    grammar.sequence({
                        grammar.rule_ref("NAME", true),
                        grammar.rule_ref("EQUALS", true),
                        grammar.rule_ref("simple_expr", false),
                    })),
                grammar.rule("simple_expr",
                    grammar.alternation({
                        grammar.rule_ref("NAME", true),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NAME,    "x",  1, 1, "NAME"),
                tok(T.EQUALS,  "=",  1, 3, "EQUALS"),
                tok(T.NUMBER,  "42", 1, 5, "NUMBER"),
                tok(T.NEWLINE, "\n", 1, 7, "NEWLINE"),
                tok(T.EOF,     "",   2, 1, "EOF"),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("file", ast.rule_name)
        end)
    end)

    describe("all token types via grammar", function()
        it("exercises matching for each enum token type", function()
            local type_names = {
                { T.NAME,          "NAME" },
                { T.NUMBER,        "NUMBER" },
                { T.STRING,        "STRING" },
                { T.KEYWORD,       "KEYWORD" },
                { T.PLUS,          "PLUS" },
                { T.MINUS,         "MINUS" },
                { T.STAR,          "STAR" },
                { T.SLASH,         "SLASH" },
                { T.EQUALS,        "EQUALS" },
                { T.EQUALS_EQUALS, "EQUALS_EQUALS" },
                { T.LPAREN,        "LPAREN" },
                { T.RPAREN,        "RPAREN" },
                { T.COMMA,         "COMMA" },
                { T.COLON,         "COLON" },
                { T.SEMICOLON,     "SEMICOLON" },
                { T.LBRACE,        "LBRACE" },
                { T.RBRACE,        "RBRACE" },
                { T.LBRACKET,      "LBRACKET" },
                { T.RBRACKET,      "RBRACKET" },
                { T.DOT,           "DOT" },
                { T.BANG,          "BANG" },
                { T.NEWLINE,       "NEWLINE" },
                { T.EOF,           "EOF" },
            }

            for _, pair in ipairs(type_names) do
                local type_val, type_str = pair[1], pair[2]
                local g = grammar.make({
                    grammar.rule("expr", grammar.rule_ref(type_str, true)),
                })
                local tokens = {
                    tok(type_val, "x", 1, 1),
                    eof(1, 2),
                }
                local p = parser.GrammarParser.new(tokens, g)
                local ast, err = p:parse()
                -- EOF and NEWLINE may behave specially; we're exercising the code paths
                if type_str == "EOF" then
                    assert.is_nil(err, "Failed for " .. type_str)
                end
            end
        end)
    end)

    describe("custom token types", function()
        it("matches tokens with custom type_name", function()
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("CUSTOM_TYPE", true)),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1, "CUSTOM_TYPE"),
                tok(T.EOF,  "", 1, 2, "EOF"),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("matches KEYWORD rules against promoted keyword token names", function()
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("KEYWORD", true)),
            })
            local tokens = {
                tok(T.KEYWORD, "var", 1, 1, "VAR"),
                tok(T.EOF,     "",    1, 4, "EOF"),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)
    end)

    describe("trace mode", function()
        it("produces same result with trace=true", function()
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1, "NUMBER"),
                tok(T.EOF,    "",   1, 3, "EOF"),
            }
            local p = parser.GrammarParser.new_with_trace(tokens, g, true)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.is_not_nil(ast)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("trace matches no-trace results", function()
            local g = grammar.make({
                grammar.rule("program",
                    grammar.repetition(grammar.rule_ref("item", false))),
                grammar.rule("item", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1, "NUMBER"),
                tok(T.NUMBER, "2", 1, 3, "NUMBER"),
                tok(T.EOF,    "",  1, 5, "EOF"),
            }

            local p1 = parser.GrammarParser.new_with_trace(tokens, g, false)
            local ast1, err1 = p1:parse()
            assert.is_nil(err1)

            local p2 = parser.GrammarParser.new_with_trace(tokens, g, true)
            local ast2, err2 = p2:parse()
            assert.is_nil(err2)

            assert.are.equal(ast1.rule_name, ast2.rule_name)
            assert.are.equal(#ast1.children, #ast2.children)
        end)

        it("handles trace on failure path", function()
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("NUMBER", true)),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1, "NAME"),
                tok(T.EOF,  "", 1, 2, "EOF"),
            }
            local p = parser.GrammarParser.new_with_trace(tokens, g, true)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)
    end)

    describe("backward compatibility", function()
        it("matches tokens by numeric type when type_name is absent", function()
            -- Token has no type_name, grammar references PLUS
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.rule_ref("PLUS", true),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "1", 1, 1),
                tok(T.PLUS,   "+", 1, 3),
                tok(T.NUMBER, "2", 1, 5),
                eof(1, 6),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)
    end)

    describe("newline skipping in literals", function()
        it("skips insignificant newlines before literal match", function()
            -- Grammar without NEWLINE reference: expr = NUMBER "+" NUMBER ;
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.sequence({
                        grammar.rule_ref("NUMBER", true),
                        grammar.literal("+"),
                        grammar.rule_ref("NUMBER", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER,  "1",  1, 1),
                tok(T.NEWLINE, "\n", 1, 2),  -- insignificant
                tok(T.PLUS,    "+",  2, 1),
                tok(T.NUMBER,  "2",  2, 3),
                eof(2, 4),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)
    end)

    describe("edge cases", function()
        it("unknown rule reference returns nil gracefully", function()
            -- Rule body references a rule that doesn't exist
            local g = grammar.make({
                grammar.rule("expr", grammar.rule_ref("nonexistent", false)),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local _, err = p:parse()
            assert.is_not_nil(err)
        end)

        it("empty repetition produces well-formed ASTNode", function()
            local g = grammar.make({
                grammar.rule("list",
                    grammar.repetition(grammar.rule_ref("NUMBER", true))),
            })
            local tokens = { eof(1, 1) }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.is_not_nil(ast.children)
            assert.are.equal(0, #ast.children)
        end)

        it("sequence failure restores position", function()
            -- Grammar: expr = (NUMBER PLUS NUMBER) | NAME ;
            -- Provide NAME token: sequence fails, alternation tries NAME
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.sequence({
                            grammar.rule_ref("NUMBER", true),
                            grammar.rule_ref("PLUS", true),
                            grammar.rule_ref("NUMBER", true),
                        }),
                        grammar.rule_ref("NAME", true),
                    })),
            })
            local tokens = {
                tok(T.NAME, "x", 1, 1),
                eof(1, 2),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
            assert.are.equal("expr", ast.rule_name)
        end)

        it("rule reference failure restores position", function()
            -- Grammar: expr = inner | NUMBER ; inner = NAME PLUS NAME ;
            -- Provide NUMBER: inner fails, falls through to NUMBER
            local g = grammar.make({
                grammar.rule("expr",
                    grammar.alternation({
                        grammar.rule_ref("inner", false),
                        grammar.rule_ref("NUMBER", true),
                    })),
                grammar.rule("inner",
                    grammar.sequence({
                        grammar.rule_ref("NAME", true),
                        grammar.rule_ref("PLUS", true),
                        grammar.rule_ref("NAME", true),
                    })),
            })
            local tokens = {
                tok(T.NUMBER, "42", 1, 1),
                eof(1, 3),
            }
            local p = parser.GrammarParser.new(tokens, g)
            local ast, err = p:parse()
            assert.is_nil(err)
        end)
    end)
end)
