-- Tests for javascript_parser
-- ============================
--
-- Comprehensive busted test suite for the JavaScript parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - var x = 5 → program → statement → var_declaration
--   - let y = "hello" → var_declaration with KEYWORD "let"
--   - const z = true → var_declaration with KEYWORD "const"
--   - Assignment: x = 10;
--   - Expression precedence: 1 + 2 * 3 → addition wraps multiplication
--   - Parenthesized expressions: (a + b) * c
--   - Expression statements
--   - Grammar first rule is "program"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Invalid JavaScript raises an error

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                             ..
    "../src/?/init.lua;"                                        ..
    "../../grammar_tools/src/?.lua;"                            ..
    "../../grammar_tools/src/?/init.lua;"                       ..
    "../../lexer/src/?.lua;"                                    ..
    "../../lexer/src/?/init.lua;"                               ..
    "../../state_machine/src/?.lua;"                            ..
    "../../state_machine/src/?/init.lua;"                       ..
    "../../directed_graph/src/?.lua;"                           ..
    "../../directed_graph/src/?/init.lua;"                      ..
    "../../javascript_lexer/src/?.lua;"                         ..
    "../../javascript_lexer/src/?/init.lua;"                    ..
    "../../parser/src/?.lua;"                                   ..
    "../../parser/src/?/init.lua;"                              ..
    package.path
)

local javascript_parser = require("coding_adventures.javascript_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Recursively collect all rule_names from an AST, in pre-order.
-- @param node  ASTNode  The root node to walk.
-- @return table         Ordered list of rule_name strings encountered.
local function collect_rule_names(node)
    local out = {}
    local function walk(n)
        if type(n) ~= "table" then return end
        if n.rule_name then
            out[#out + 1] = n.rule_name
            if n.children then
                for _, child in ipairs(n.children) do
                    walk(child)
                end
            end
        end
    end
    walk(node)
    return out
end

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

-- =========================================================================
-- Module surface
-- =========================================================================

describe("javascript_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(javascript_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(javascript_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", javascript_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(javascript_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(javascript_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(javascript_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = javascript_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- javascript.grammar has: program, statement, var_declaration,
        -- assignment, expression_stmt, expression, term, factor
        assert.is_true(#g.rules >= 6)
    end)

    it("grammar first rule is program", function()
        local g = javascript_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = javascript_parser.parse("var x = 5;")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = javascript_parser.parse("var x = 5;")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = javascript_parser.parse("var x = 5;")
        assert.is_table(ast.children)
    end)

    it("program contains a statement node", function()
        local ast = javascript_parser.parse("var x = 5;")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)
end)

-- =========================================================================
-- Variable declarations
-- =========================================================================

describe("variable declarations", function()
    it("parses var x = 5", function()
        local ast = javascript_parser.parse("var x = 5;")
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd, "expected 'var_declaration' node")
    end)

    it("parses let y = \"hello\"", function()
        local ast = javascript_parser.parse('let y = "hello";')
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd, "expected 'var_declaration' node for let")
    end)

    it("parses const z = true", function()
        local ast = javascript_parser.parse("const z = true;")
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd, "expected 'var_declaration' node for const")
    end)

    it("parses var with numeric expression", function()
        local ast = javascript_parser.parse("var total = 100;")
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd)
    end)

    it("parses multiple declarations", function()
        local ast = javascript_parser.parse("var x = 1;\nlet y = 2;")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "var_declaration")
        assert.are.equal(2, count)
    end)
end)

-- =========================================================================
-- Assignments
-- =========================================================================

describe("assignments", function()
    it("parses simple assignment x = 10", function()
        local ast = javascript_parser.parse("x = 10;")
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign, "expected 'assignment' node")
    end)

    it("parses string assignment", function()
        local ast = javascript_parser.parse('name = "Alice";')
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign)
    end)
end)

-- =========================================================================
-- Expression statements
-- =========================================================================

describe("expression statements", function()
    it("parses expression_stmt", function()
        local ast = javascript_parser.parse("42;")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_stmt")
        assert.is_not_nil(es, "expected 'expression_stmt' node")
    end)

    it("parses expression_stmt with name", function()
        local ast = javascript_parser.parse("x;")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_stmt")
        assert.is_not_nil(es)
    end)
end)

-- =========================================================================
-- Expression precedence
-- =========================================================================

describe("expression precedence", function()
    it("1 + 2 * 3 → addition at expression level, multiplication at term", function()
        -- The grammar encodes precedence: expression handles +/-, term handles */.
        -- "1 + 2 * 3" should parse as: expression containing term (1), PLUS, term (2*3).
        -- This means 2*3 appears as a single `term` node.
        local ast = javascript_parser.parse("var r = 1 + 2 * 3;")
        assert.are.equal("program", ast.rule_name)
        -- There should be both expression and term nodes in the tree
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node")
        local term = find_node(ast, "term")
        assert.is_not_nil(term, "expected 'term' node")
    end)

    it("1 * 2 + 3 → still has expression and term nodes", function()
        local ast = javascript_parser.parse("var r = 1 * 2 + 3;")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr)
    end)

    it("arithmetic expression produces factor nodes", function()
        local ast = javascript_parser.parse("var r = 5 + 3;")
        assert.are.equal("program", ast.rule_name)
        local factor = find_node(ast, "factor")
        assert.is_not_nil(factor, "expected 'factor' node")
    end)

    it("parses parenthesized expression", function()
        -- (a + b) * c — parenthesized groups are handled by the factor rule
        local ast = javascript_parser.parse("var r = (2 + 3) * 4;")
        assert.are.equal("program", ast.rule_name)
        -- Should have expression, term, factor nodes
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node in (2+3)*4")
    end)

    it("deeply nested arithmetic", function()
        local ast = javascript_parser.parse("var r = 1 + 2 + 3 + 4;")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "term")
        assert.is_true(count >= 4, "expected at least 4 term nodes")
    end)
end)

-- =========================================================================
-- Multiple statements
-- =========================================================================

describe("multiple statements", function()
    it("parses two var declarations", function()
        local ast = javascript_parser.parse("var a = 1;\nvar b = 2;")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "var_declaration")
        assert.are.equal(2, count)
    end)

    it("parses mixed statements", function()
        local ast = javascript_parser.parse(
            "var x = 5;\nx = 10;\n42;"
        )
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(1, count_nodes(ast, "var_declaration"))
        assert.are.equal(1, count_nodes(ast, "assignment"))
        assert.are.equal(1, count_nodes(ast, "expression_stmt"))
    end)

    it("parses empty program (no statements)", function()
        -- An empty program is valid: { statement } means zero or more
        local ast = javascript_parser.parse("")
        assert.are.equal("program", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = javascript_parser.create_parser("var x = 1;")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = javascript_parser.create_parser("var x = 1;")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src = "var x = 42;"
        local ast1 = javascript_parser.parse(src)
        local p    = javascript_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for completely invalid input", function()
        assert.has_error(function()
            javascript_parser.parse("@@@ GARBAGE @@@")
        end)
    end)
end)
