-- Tests for dartmouth_basic_parser
-- ==================================
--
-- Comprehensive busted test suite for the Dartmouth BASIC parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - All 17 statement types parse correctly
--   - Expression precedence rules
--   - Multi-line programs
--   - Empty lines (bare LINE_NUM)
--   - create_parser and get_grammar API
--   - Error cases (malformed input)
--
-- # Why test all 17 statement types?
--
-- Dartmouth BASIC has exactly 17 statement types. Each is a distinct
-- first-class construct, not a variant of a general "command". Testing
-- each one ensures the grammar handles the full 1964 specification.
--
-- The 17 types:
--   LET, PRINT, INPUT, IF, GOTO, GOSUB, RETURN, FOR, NEXT,
--   END, STOP, REM, READ, DATA, RESTORE, DIM, DEF

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                                  ..
    "../src/?/init.lua;"                                             ..
    "../../grammar_tools/src/?.lua;"                                 ..
    "../../grammar_tools/src/?/init.lua;"                            ..
    "../../lexer/src/?.lua;"                                         ..
    "../../lexer/src/?/init.lua;"                                    ..
    "../../state_machine/src/?.lua;"                                 ..
    "../../state_machine/src/?/init.lua;"                            ..
    "../../directed_graph/src/?.lua;"                                ..
    "../../directed_graph/src/?/init.lua;"                           ..
    "../../dartmouth_basic_lexer/src/?.lua;"                         ..
    "../../dartmouth_basic_lexer/src/?/init.lua;"                    ..
    "../../parser/src/?.lua;"                                        ..
    "../../parser/src/?/init.lua;"                                   ..
    package.path
)

local bp = require("coding_adventures.dartmouth_basic_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Find the first ASTNode with the given rule_name (depth-first).
-- @param node      ASTNode  Root to search.
-- @param rule_name string   Rule name to find.
-- @return ASTNode|nil
local function find_node(node, rule_name)
    if type(node) ~= "table" then return nil end
    if node.rule_name == rule_name then return node end
    if node.children then
        for _, child in ipairs(node.children) do
            local found = find_node(child, rule_name)
            if found then return found end
        end
    end
    return nil
end

--- Count all nodes with the given rule_name (full traversal).
-- @param node      ASTNode
-- @param rule_name string
-- @return number
local function count_nodes(node, rule_name)
    if type(node) ~= "table" then return 0 end
    local n = (node.rule_name == rule_name) and 1 or 0
    if node.children then
        for _, child in ipairs(node.children) do
            n = n + count_nodes(child, rule_name)
        end
    end
    return n
end

--- Find a leaf node whose token has the given type.
-- Depth-first search through the AST.
-- @param node       ASTNode
-- @param tok_type   string   Token type to find (e.g. "KEYWORD")
-- @param tok_value  string   Optional: also match token value
-- @return ASTNode|nil
local function find_token(node, tok_type, tok_value)
    if type(node) ~= "table" then return nil end
    if node.is_leaf and node.is_leaf() then
        local tok = node.token and node:token()
        if tok and tok.type == tok_type then
            if tok_value == nil or tok.value == tok_value then
                return node
            end
        end
    end
    if node.children then
        for _, child in ipairs(node.children) do
            local found = find_token(child, tok_type, tok_value)
            if found then return found end
        end
    end
    return nil
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("dartmouth_basic_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(bp)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(bp.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", bp.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(bp.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(bp.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(bp.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = bp.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- The BASIC grammar has at least 20 rules (program, line, statement,
        -- 17 statement types, variable, relop, expr, term, power, unary, primary...)
        assert.is_true(#g.rules >= 20)
    end)

    it("grammar first rule is program", function()
        local g = bp.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = bp.parse("10 END\n")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = bp.parse("10 END\n")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children array", function()
        local ast = bp.parse("10 END\n")
        assert.is_table(ast.children)
    end)

    it("empty input produces program with no line children", function()
        local ast = bp.parse("")
        assert.are.equal("program", ast.rule_name)
        -- An empty program has no line children (only tokens / empty)
        local line = find_node(ast, "line")
        assert.is_nil(line, "empty program should have no line nodes")
    end)
end)

-- =========================================================================
-- Empty line: bare LINE_NUM
-- =========================================================================

describe("empty line", function()
    it("bare line number is valid: '10\\n'", function()
        local ast = bp.parse("10\n")
        assert.are.equal("program", ast.rule_name)
        local line = find_node(ast, "line")
        assert.is_not_nil(line, "expected a 'line' node")
    end)
end)

-- =========================================================================
-- Statement type 1: LET
-- =========================================================================

describe("LET statement", function()
    it("parses scalar assignment: 10 LET X = 5", function()
        local ast = bp.parse("10 LET X = 5\n")
        local stmt = find_node(ast, "let_stmt")
        assert.is_not_nil(stmt, "expected 'let_stmt' node")
    end)

    it("parses array assignment: 10 LET A(3) = 7", function()
        local ast = bp.parse("10 LET A(3) = 7\n")
        local stmt = find_node(ast, "let_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses expression in RHS: 10 LET X = X + 1", function()
        local ast = bp.parse("10 LET X = X + 1\n")
        local stmt = find_node(ast, "let_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 2: PRINT
-- =========================================================================

describe("PRINT statement", function()
    it("parses bare PRINT: 10 PRINT", function()
        local ast = bp.parse("10 PRINT\n")
        local stmt = find_node(ast, "print_stmt")
        assert.is_not_nil(stmt, "expected 'print_stmt' node")
    end)

    it("parses PRINT with string: 10 PRINT \"HELLO, WORLD\"", function()
        local ast = bp.parse("10 PRINT \"HELLO, WORLD\"\n")
        local stmt = find_node(ast, "print_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses PRINT with expression: 10 PRINT X", function()
        local ast = bp.parse("10 PRINT X\n")
        local stmt = find_node(ast, "print_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses PRINT with comma-separated items: 10 PRINT X, Y", function()
        local ast = bp.parse("10 PRINT X, Y\n")
        local stmt = find_node(ast, "print_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses PRINT with semicolon separator: 10 PRINT X; Y", function()
        local ast = bp.parse("10 PRINT X; Y\n")
        local stmt = find_node(ast, "print_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 3: INPUT
-- =========================================================================

describe("INPUT statement", function()
    it("parses INPUT with single variable: 10 INPUT X", function()
        local ast = bp.parse("10 INPUT X\n")
        local stmt = find_node(ast, "input_stmt")
        assert.is_not_nil(stmt, "expected 'input_stmt' node")
    end)

    it("parses INPUT with multiple variables: 10 INPUT A, B, C", function()
        local ast = bp.parse("10 INPUT A, B, C\n")
        local stmt = find_node(ast, "input_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 4: IF ... THEN
-- =========================================================================

describe("IF statement", function()
    it("parses IF with EQ relop: 10 IF X = 0 THEN 100", function()
        local ast = bp.parse("10 IF X = 0 THEN 100\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt, "expected 'if_stmt' node")
    end)

    it("parses IF with LT relop: 10 IF X < 5 THEN 20", function()
        local ast = bp.parse("10 IF X < 5 THEN 20\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses IF with GT relop: 10 IF A > B THEN 50", function()
        local ast = bp.parse("10 IF A > B THEN 50\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses IF with LE relop: 10 IF X <= 10 THEN 20", function()
        local ast = bp.parse("10 IF X <= 10 THEN 20\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses IF with GE relop: 10 IF X >= 0 THEN 30", function()
        local ast = bp.parse("10 IF X >= 0 THEN 30\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt)
    end)

    it("parses IF with NE relop: 10 IF X <> Y THEN 70", function()
        local ast = bp.parse("10 IF X <> Y THEN 70\n")
        local stmt = find_node(ast, "if_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 5: GOTO
-- =========================================================================

describe("GOTO statement", function()
    it("parses GOTO with line number: 10 GOTO 50", function()
        local ast = bp.parse("10 GOTO 50\n")
        local stmt = find_node(ast, "goto_stmt")
        assert.is_not_nil(stmt, "expected 'goto_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 6: GOSUB
-- =========================================================================

describe("GOSUB statement", function()
    it("parses GOSUB with line number: 10 GOSUB 200", function()
        local ast = bp.parse("10 GOSUB 200\n")
        local stmt = find_node(ast, "gosub_stmt")
        assert.is_not_nil(stmt, "expected 'gosub_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 7: RETURN
-- =========================================================================

describe("RETURN statement", function()
    it("parses RETURN: 10 RETURN", function()
        local ast = bp.parse("10 RETURN\n")
        local stmt = find_node(ast, "return_stmt")
        assert.is_not_nil(stmt, "expected 'return_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 8: FOR
-- =========================================================================

describe("FOR statement", function()
    it("parses FOR without STEP: 10 FOR I = 1 TO 10", function()
        local ast = bp.parse("10 FOR I = 1 TO 10\n")
        local stmt = find_node(ast, "for_stmt")
        assert.is_not_nil(stmt, "expected 'for_stmt' node")
    end)

    it("parses FOR with STEP: 10 FOR I = 10 TO 1 STEP -1", function()
        local ast = bp.parse("10 FOR I = 10 TO 1 STEP -1\n")
        local stmt = find_node(ast, "for_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 9: NEXT
-- =========================================================================

describe("NEXT statement", function()
    it("parses NEXT: 30 NEXT I", function()
        local ast = bp.parse("30 NEXT I\n")
        local stmt = find_node(ast, "next_stmt")
        assert.is_not_nil(stmt, "expected 'next_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 10: END
-- =========================================================================

describe("END statement", function()
    it("parses END: 10 END", function()
        local ast = bp.parse("10 END\n")
        local stmt = find_node(ast, "end_stmt")
        assert.is_not_nil(stmt, "expected 'end_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 11: STOP
-- =========================================================================

describe("STOP statement", function()
    it("parses STOP: 10 STOP", function()
        local ast = bp.parse("10 STOP\n")
        local stmt = find_node(ast, "stop_stmt")
        assert.is_not_nil(stmt, "expected 'stop_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 12: REM
-- =========================================================================

describe("REM statement", function()
    it("parses REM comment: 10 REM THIS IS A COMMENT", function()
        -- The lexer suppresses all tokens after REM, so the parser sees:
        --   LINE_NUM(10) KEYWORD(REM) NEWLINE
        local ast = bp.parse("10 REM THIS IS A COMMENT\n")
        local stmt = find_node(ast, "rem_stmt")
        assert.is_not_nil(stmt, "expected 'rem_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 13: READ
-- =========================================================================

describe("READ statement", function()
    it("parses READ with single variable: 10 READ X", function()
        local ast = bp.parse("10 READ X\n")
        local stmt = find_node(ast, "read_stmt")
        assert.is_not_nil(stmt, "expected 'read_stmt' node")
    end)

    it("parses READ with multiple variables: 10 READ A, B, C", function()
        local ast = bp.parse("10 READ A, B, C\n")
        local stmt = find_node(ast, "read_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 14: DATA
-- =========================================================================

describe("DATA statement", function()
    it("parses DATA with single number: 10 DATA 42", function()
        local ast = bp.parse("10 DATA 42\n")
        local stmt = find_node(ast, "data_stmt")
        assert.is_not_nil(stmt, "expected 'data_stmt' node")
    end)

    it("parses DATA with multiple numbers: 10 DATA 1, 2, 3, 4, 5", function()
        local ast = bp.parse("10 DATA 1, 2, 3, 4, 5\n")
        local stmt = find_node(ast, "data_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 15: RESTORE
-- =========================================================================

describe("RESTORE statement", function()
    it("parses RESTORE: 10 RESTORE", function()
        local ast = bp.parse("10 RESTORE\n")
        local stmt = find_node(ast, "restore_stmt")
        assert.is_not_nil(stmt, "expected 'restore_stmt' node")
    end)
end)

-- =========================================================================
-- Statement type 16: DIM
-- =========================================================================

describe("DIM statement", function()
    it("parses DIM with single array: 10 DIM A(10)", function()
        local ast = bp.parse("10 DIM A(10)\n")
        local stmt = find_node(ast, "dim_stmt")
        assert.is_not_nil(stmt, "expected 'dim_stmt' node")
    end)

    it("parses DIM with multiple arrays: 10 DIM A(10), B(20)", function()
        local ast = bp.parse("10 DIM A(10), B(20)\n")
        local stmt = find_node(ast, "dim_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Statement type 17: DEF
-- =========================================================================

describe("DEF statement", function()
    it("parses DEF user function: 10 DEF FNA(X) = X * X", function()
        local ast = bp.parse("10 DEF FNA(X) = X * X\n")
        local stmt = find_node(ast, "def_stmt")
        assert.is_not_nil(stmt, "expected 'def_stmt' node")
    end)

    it("parses DEF with builtin: 10 DEF FNB(T) = SIN(T)", function()
        local ast = bp.parse("10 DEF FNB(T) = SIN(T)\n")
        local stmt = find_node(ast, "def_stmt")
        assert.is_not_nil(stmt)
    end)
end)

-- =========================================================================
-- Expression precedence
-- =========================================================================

describe("expression precedence", function()
    -- The BASIC expression hierarchy:
    --   primary > unary > power > term > expr
    -- Each level is a separate grammar rule. We check that the parser
    -- produces the correct rule nodes.

    it("parses simple addition: 10 LET X = A + B", function()
        local ast = bp.parse("10 LET X = A + B\n")
        local expr = find_node(ast, "expr")
        assert.is_not_nil(expr, "expected 'expr' node")
    end)

    it("parses multiplication: 10 LET X = A * B", function()
        local ast = bp.parse("10 LET X = A * B\n")
        local term = find_node(ast, "term")
        assert.is_not_nil(term, "expected 'term' node")
    end)

    it("parses exponentiation: 10 LET X = A ^ B", function()
        local ast = bp.parse("10 LET X = A ^ B\n")
        local pow = find_node(ast, "power")
        assert.is_not_nil(pow, "expected 'power' node")
    end)

    it("parses unary minus: 10 LET X = -5", function()
        local ast = bp.parse("10 LET X = -5\n")
        local unary = find_node(ast, "unary")
        assert.is_not_nil(unary, "expected 'unary' node")
    end)

    it("parses parenthesised expression: 10 LET X = (A + B) * C", function()
        local ast = bp.parse("10 LET X = (A + B) * C\n")
        assert.is_not_nil(find_node(ast, "expr"))
        assert.is_not_nil(find_node(ast, "term"))
    end)

    it("parses builtin function call: 10 LET X = SIN(A)", function()
        local ast = bp.parse("10 LET X = SIN(A)\n")
        local prim = find_node(ast, "primary")
        assert.is_not_nil(prim, "expected 'primary' node")
    end)

    it("parses user function call: 10 LET X = FNA(Y)", function()
        local ast = bp.parse("10 LET X = FNA(Y)\n")
        local prim = find_node(ast, "primary")
        assert.is_not_nil(prim)
    end)

    it("parses complex expression: 10 LET X = A + B * C ^ 2", function()
        local ast = bp.parse("10 LET X = A + B * C ^ 2\n")
        -- All four expression rules should appear
        assert.is_not_nil(find_node(ast, "expr"))
        assert.is_not_nil(find_node(ast, "term"))
        assert.is_not_nil(find_node(ast, "power"))
    end)
end)

-- =========================================================================
-- Multi-line programs
-- =========================================================================

describe("multi-line programs", function()
    it("parses 'HELLO, WORLD' program", function()
        local ast = bp.parse("10 PRINT \"HELLO, WORLD\"\n20 END\n")
        assert.are.equal("program", ast.rule_name)
        -- Should have 2 line nodes
        local count = count_nodes(ast, "line")
        assert.are.equal(2, count)
    end)

    it("parses FOR loop program", function()
        local src = "10 FOR I = 1 TO 5\n20 PRINT I\n30 NEXT I\n40 END\n"
        local ast = bp.parse(src)
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "line")
        assert.are.equal(4, count)
    end)

    it("parses counting program with GOTO", function()
        local src =
            "10 LET X = 1\n" ..
            "20 PRINT X\n" ..
            "30 LET X = X + 1\n" ..
            "40 IF X <= 10 THEN 20\n" ..
            "50 END\n"
        local ast = bp.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(5, count_nodes(ast, "line"))
    end)

    it("parses GOSUB / RETURN program", function()
        local src =
            "10 GOSUB 100\n" ..
            "20 END\n" ..
            "100 PRINT \"IN SUBROUTINE\"\n" ..
            "110 RETURN\n"
        local ast = bp.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(4, count_nodes(ast, "line"))
    end)

    it("parses READ/DATA program", function()
        local src =
            "10 READ X\n" ..
            "20 PRINT X\n" ..
            "30 DATA 42\n" ..
            "40 END\n"
        local ast = bp.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.is_not_nil(find_node(ast, "read_stmt"))
        assert.is_not_nil(find_node(ast, "data_stmt"))
    end)

    it("program with REM comment line", function()
        local src =
            "10 REM DARTMOUTH BASIC EXAMPLE\n" ..
            "20 LET X = 42\n" ..
            "30 END\n"
        local ast = bp.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(3, count_nodes(ast, "line"))
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = bp.create_parser("10 END\n")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = bp.create_parser("10 END\n")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns the same root as parse()", function()
        local src  = "10 LET X = 1\n20 END\n"
        local ast1 = bp.parse(src)
        local p    = bp.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for missing THEN target in IF", function()
        assert.has_error(function()
            -- IF without THEN is invalid BASIC
            bp.parse("10 IF X > 0\n")
        end)
    end)

    it("raises an error for incomplete LET (no expression)", function()
        assert.has_error(function()
            bp.parse("10 LET X =\n")
        end)
    end)

    it("raises an error for incomplete FOR (no TO clause)", function()
        assert.has_error(function()
            bp.parse("10 FOR I = 1\n")
        end)
    end)
end)
