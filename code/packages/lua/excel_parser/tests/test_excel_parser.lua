-- Tests for excel_parser
-- =======================
--
-- Comprehensive busted test suite for the Excel formula parser.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Parses formulas with and without leading "="
--   - Simple literals: number, string, bool, error
--   - Cell references: A1, $B$2
--   - Range references: A1:B10
--   - Arithmetic: + - * / ^ (with correct precedence)
--   - Unary prefix: -A1, +42
--   - Postfix: 50%
--   - Comparison operators: = <> <= >= < >
--   - Concatenation: &
--   - Function calls: SUM(A1:B10), IF(A1>0,"pos","neg")
--   - Cross-sheet references: Sheet1!A1
--   - Array constants: {1,2;3,4}
--   - Nested function calls
--   - Empty argument in function call
--   - Error on trailing content
--   - Error on unexpected token

-- Resolve sibling packages from the monorepo
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../excel_lexer/src/?.lua;"                            ..
    "../../excel_lexer/src/?/init.lua;"                       ..
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

local excel_parser = require("coding_adventures.excel_parser")

-- =========================================================================
-- Helpers
-- =========================================================================

--- Assert that parsing `source` succeeds and return the AST.
local function parse_ok(source)
    local ok, result = pcall(excel_parser.parse, source)
    if not ok then
        error("parse_ok: parse failed for '" .. source .. "': " .. tostring(result), 2)
    end
    return result
end

--- Assert that parsing `source` raises an error.
local function parse_err(source)
    local ok = pcall(excel_parser.parse, source)
    assert.is_false(ok, "expected parse to fail for: " .. source)
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("excel_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(excel_parser)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(excel_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", excel_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(excel_parser.parse)
    end)

    it("exposes tokenize as a function", function()
        assert.is_function(excel_parser.tokenize)
    end)
end)

-- =========================================================================
-- Formula root node
-- =========================================================================

describe("formula root node", function()
    it("returns a node with kind='formula'", function()
        local ast = parse_ok("=1")
        assert.are.equal("formula", ast.kind)
    end)

    it("captures the leading EQUALS token", function()
        local ast = parse_ok("=1")
        assert.is_not_nil(ast.eq)
        assert.are.equal("EQUALS", ast.eq.type)
    end)

    it("parses formula without leading =", function()
        local ast = parse_ok("1+2")
        assert.are.equal("formula", ast.kind)
        assert.is_nil(ast.eq)
        assert.are.equal("binop", ast.body.kind)
    end)
end)

-- =========================================================================
-- Literal values
-- =========================================================================

describe("number literals", function()
    it("parses an integer", function()
        local ast = parse_ok("=42")
        assert.are.equal("number", ast.body.kind)
        assert.are.equal("42", ast.body.token.value)
    end)

    it("parses a float", function()
        local ast = parse_ok("=3.14")
        assert.are.equal("number", ast.body.kind)
    end)

    it("parses a decimal fraction .5", function()
        local ast = parse_ok("=.5")
        assert.are.equal("number", ast.body.kind)
    end)
end)

describe("string literals", function()
    it("parses a double-quoted string", function()
        local ast = parse_ok('="hello"')
        assert.are.equal("string", ast.body.kind)
        assert.are.equal('"hello"', ast.body.token.value)
    end)

    it("parses an empty string", function()
        local ast = parse_ok('=""')
        assert.are.equal("string", ast.body.kind)
    end)
end)

describe("boolean literals", function()
    it("parses TRUE", function()
        local ast = parse_ok("=TRUE")
        assert.are.equal("bool", ast.body.kind)
        assert.are.equal("true", ast.body.token.value)
    end)

    it("parses FALSE", function()
        local ast = parse_ok("=FALSE")
        assert.are.equal("bool", ast.body.kind)
    end)
end)

describe("error constant literals", function()
    it("parses #DIV/0!", function()
        local ast = parse_ok("=#DIV/0!")
        assert.are.equal("error", ast.body.kind)
    end)

    it("parses #VALUE!", function()
        local ast = parse_ok("=#VALUE!")
        assert.are.equal("error", ast.body.kind)
    end)
end)

-- =========================================================================
-- Cell and range references
-- =========================================================================

describe("cell references", function()
    it("parses A1", function()
        local ast = parse_ok("=A1")
        assert.are.equal("cell", ast.body.kind)
        assert.are.equal("a1", ast.body.token.value)
    end)

    it("parses absolute reference $B$2", function()
        local ast = parse_ok("=$B$2")
        assert.are.equal("cell", ast.body.kind)
        assert.are.equal("$b$2", ast.body.token.value)
    end)
end)

describe("range references", function()
    it("parses A1:B10", function()
        local ast = parse_ok("=A1:B10")
        assert.are.equal("range", ast.body.kind)
        assert.are.equal("cell", ast.body.start_ref.kind)
        assert.are.equal("cell", ast.body.end_ref.kind)
    end)

    it("parses absolute range $A$1:$Z$100", function()
        local ast = parse_ok("=$A$1:$Z$100")
        assert.are.equal("range", ast.body.kind)
    end)
end)

describe("cross-sheet references", function()
    it("parses Sheet1!A1", function()
        local ast = parse_ok("=Sheet1!A1")
        assert.are.equal("ref_prefix", ast.body.kind)
        assert.are.equal("REF_PREFIX", ast.body.prefix.type)
        assert.are.equal("cell", ast.body.ref.kind)
    end)
end)

-- =========================================================================
-- Arithmetic operators and precedence
-- =========================================================================

describe("arithmetic operators", function()
    it("parses addition A1+B2", function()
        local ast = parse_ok("=A1+B2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("PLUS", ast.body.op.type)
    end)

    it("parses subtraction A1-B2", function()
        local ast = parse_ok("=A1-B2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("MINUS", ast.body.op.type)
    end)

    it("parses multiplication A1*B2", function()
        local ast = parse_ok("=A1*B2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("STAR", ast.body.op.type)
    end)

    it("parses division A1/B2", function()
        local ast = parse_ok("=A1/B2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("SLASH", ast.body.op.type)
    end)

    it("parses exponentiation A1^2", function()
        local ast = parse_ok("=A1^2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("CARET", ast.body.op.type)
    end)

    it("* binds tighter than + (1+2*3 → PLUS(1, STAR(2,3)))", function()
        local ast = parse_ok("=1+2*3")
        -- Root is addition
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("PLUS", ast.body.op.type)
        -- Right side is multiplication
        assert.are.equal("binop", ast.body.right.kind)
        assert.are.equal("STAR", ast.body.right.op.type)
    end)

    it("1*2+3 → PLUS(STAR(1,2), 3)", function()
        local ast = parse_ok("=1*2+3")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("PLUS", ast.body.op.type)
        assert.are.equal("binop", ast.body.left.kind)
        assert.are.equal("STAR", ast.body.left.op.type)
    end)

    it("parentheses override precedence: (1+2)*3", function()
        local ast = parse_ok("=(1+2)*3")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("STAR", ast.body.op.type)
        assert.are.equal("group", ast.body.left.kind)
    end)
end)

-- =========================================================================
-- Unary and postfix operators
-- =========================================================================

describe("unary operators", function()
    it("parses unary minus -A1", function()
        local ast = parse_ok("=-A1")
        assert.are.equal("unop", ast.body.kind)
        assert.are.equal("MINUS", ast.body.op.type)
        assert.are.equal("cell", ast.body.operand.kind)
    end)

    it("parses double negation --A1", function()
        local ast = parse_ok("=--A1")
        assert.are.equal("unop", ast.body.kind)
        assert.are.equal("MINUS", ast.body.op.type)
        assert.are.equal("unop", ast.body.operand.kind)
    end)
end)

describe("postfix percent operator", function()
    it("parses 50%", function()
        local ast = parse_ok("=50%")
        assert.are.equal("postfix", ast.body.kind)
        assert.are.equal("PERCENT", ast.body.op.type)
        assert.are.equal("number", ast.body.operand.kind)
    end)

    it("parses A1*100% (percent binds tightest)", function()
        local ast = parse_ok("=A1*100%")
        -- Root should be *, with right = postfix(%)
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("STAR", ast.body.op.type)
        assert.are.equal("postfix", ast.body.right.kind)
    end)
end)

-- =========================================================================
-- Comparison and concatenation
-- =========================================================================

describe("comparison operators", function()
    it("parses A1=B1 (equality)", function()
        local ast = parse_ok("=A1=B1")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("EQUALS", ast.body.op.type)
    end)

    it("parses A1<>B1 (not equal)", function()
        local ast = parse_ok("=A1<>B1")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("NOT_EQUALS", ast.body.op.type)
    end)

    it("parses A1>=0", function()
        local ast = parse_ok("=A1>=0")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("GREATER_EQUALS", ast.body.op.type)
    end)
end)

describe("concatenation operator", function()
    it("parses A1&\" world\"", function()
        local ast = parse_ok('=A1&" world"')
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("AMP", ast.body.op.type)
    end)
end)

-- =========================================================================
-- Function calls
-- =========================================================================

describe("function calls", function()
    it("parses SUM(A1:B10)", function()
        local ast = parse_ok("=SUM(A1:B10)")
        assert.are.equal("call", ast.body.kind)
        assert.are.equal("sum", ast.body.name.value)
        assert.are.equal(1, #ast.body.args)
        assert.are.equal("range", ast.body.args[1].kind)
    end)

    it("parses IF(A1>0,\"pos\",\"neg\")", function()
        local ast = parse_ok('=IF(A1>0,"pos","neg")')
        assert.are.equal("call", ast.body.kind)
        assert.are.equal("if", ast.body.name.value)
        assert.are.equal(3, #ast.body.args)
    end)

    it("parses COUNT() — zero-argument function", function()
        local ast = parse_ok("=COUNT()")
        assert.are.equal("call", ast.body.kind)
        assert.are.equal(0, #ast.body.args)
    end)

    it("parses nested function call ABS(SUM(A1:A10))", function()
        local ast = parse_ok("=ABS(SUM(A1:A10))")
        assert.are.equal("call", ast.body.kind)
        assert.are.equal("abs", ast.body.name.value)
        assert.are.equal("call", ast.body.args[1].kind)
        assert.are.equal("sum", ast.body.args[1].name.value)
    end)

    it("parses IFERROR(A1/B1,#DIV/0!)", function()
        local ast = parse_ok("=IFERROR(A1/B1,#DIV/0!)")
        assert.are.equal("call", ast.body.kind)
        assert.are.equal(2, #ast.body.args)
        assert.are.equal("error", ast.body.args[2].kind)
    end)
end)

-- =========================================================================
-- Array constants
-- =========================================================================

describe("array constants", function()
    it("parses a 1D array {1,2,3}", function()
        local ast = parse_ok("={1,2,3}")
        assert.are.equal("array", ast.body.kind)
        assert.are.equal(1, #ast.body.rows)
        assert.are.equal(3, #ast.body.rows[1])
    end)

    it("parses a 2D array {1,2;3,4}", function()
        local ast = parse_ok("={1,2;3,4}")
        assert.are.equal("array", ast.body.kind)
        assert.are.equal(2, #ast.body.rows)
        assert.are.equal(2, #ast.body.rows[1])
        assert.are.equal(2, #ast.body.rows[2])
    end)

    it("parses array with strings ={\"a\",\"b\"}", function()
        local ast = parse_ok('={"a","b"}')
        assert.are.equal("array", ast.body.kind)
        assert.are.equal("string", ast.body.rows[1][1].kind)
    end)

    it("parses array with negative number ={-1,2}", function()
        local ast = parse_ok("={-1,2}")
        assert.are.equal("array", ast.body.kind)
        assert.are.equal("unop", ast.body.rows[1][1].kind)
    end)
end)

-- =========================================================================
-- Complex / real-world formulas
-- =========================================================================

describe("complex formulas", function()
    it("parses =VLOOKUP(A2,B:C,2,FALSE)", function()
        local ast = parse_ok("=VLOOKUP(A2,B:C,2,FALSE)")
        assert.are.equal("call", ast.body.kind)
        assert.are.equal("vlookup", ast.body.name.value)
        assert.are.equal(4, #ast.body.args)
    end)

    it("parses =A1+Sheet1!B2*0.1", function()
        local ast = parse_ok("=A1+Sheet1!B2*0.1")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("PLUS", ast.body.op.type)
    end)

    it("parses =SUMIF(A1:A10,\">0\",B1:B10)", function()
        local ast = parse_ok('=SUMIF(A1:A10,">0",B1:B10)')
        assert.are.equal("call", ast.body.kind)
        assert.are.equal(3, #ast.body.args)
    end)

    it("parses =A1^2+B1^2 (Pythagorean)", function()
        local ast = parse_ok("=A1^2+B1^2")
        assert.are.equal("binop", ast.body.kind)
        assert.are.equal("PLUS", ast.body.op.type)
        assert.are.equal("binop", ast.body.left.kind)
        assert.are.equal("CARET", ast.body.left.op.type)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises on trailing content", function()
        parse_err("=1 2")
    end)

    it("raises on unclosed parenthesis", function()
        parse_err("=SUM(A1")
    end)

    it("raises on empty input (no tokens to parse)", function()
        -- An empty formula has no expression body after the optional =
        parse_err("=")
    end)
end)
