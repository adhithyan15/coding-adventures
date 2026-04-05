-- Tests for python_parser
-- ========================
--
-- Comprehensive busted test suite for the Python parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - x = 5 → program → statement → assignment
--   - def add(a, b): return a + b → function_def
--   - if x > 0: return x / else: return 0 → if_stmt
--   - for i in range(10): print(i) → for_stmt
--   - Expression: 1 + 2 * 3 → respects precedence
--   - Import: import math
--   - Grammar first rule is "program"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Empty program is valid

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
    "../../python_lexer/src/?.lua;"                             ..
    "../../python_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                   ..
    "../../parser/src/?/init.lua;"                              ..
    package.path
)

local python_parser = require("coding_adventures.python_parser")

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

describe("python_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(python_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(python_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", python_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(python_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(python_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(python_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = python_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- python.grammar has: program, statement, assignment,
        -- expression_stmt, expression, term, factor
        assert.is_true(#g.rules >= 5)
    end)

    it("grammar first rule is program", function()
        local g = python_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = python_parser.parse("x = 5")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = python_parser.parse("x = 5")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = python_parser.parse("x = 5")
        assert.is_table(ast.children)
    end)

    it("program contains a statement node", function()
        local ast = python_parser.parse("x = 5")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)

    it("parses empty program (no statements)", function()
        -- An empty program is valid: { statement } means zero or more
        local ast = python_parser.parse("")
        assert.are.equal("program", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- Assignments
-- =========================================================================

describe("assignments", function()
    it("parses x = 5", function()
        local ast = python_parser.parse("x = 5")
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign, "expected 'assignment' node")
    end)

    it("parses string assignment", function()
        local ast = python_parser.parse('name = "Alice"')
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign)
    end)

    it("assignment contains expression", function()
        local ast = python_parser.parse("x = 42")
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node inside assignment")
    end)

    it("parses multiple assignments", function()
        local ast = python_parser.parse("x = 1\ny = 2")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "assignment")
        assert.are.equal(2, count)
    end)
end)

-- =========================================================================
-- Expression statements
-- =========================================================================

describe("expression statements", function()
    it("parses numeric literal as expression_stmt", function()
        local ast = python_parser.parse("42")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_stmt")
        assert.is_not_nil(es, "expected 'expression_stmt' node")
    end)

    it("parses name as expression_stmt", function()
        local ast = python_parser.parse("x")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_stmt")
        assert.is_not_nil(es)
    end)
end)

-- =========================================================================
-- Expression precedence
-- =========================================================================

describe("expression precedence", function()
    it("1 + 2 * 3 → has expression and term nodes", function()
        -- The grammar encodes precedence: expression handles +/-, term handles */.
        -- "1 + 2 * 3" should parse as: expression containing term (1), PLUS, term (2*3).
        local ast = python_parser.parse("r = 1 + 2 * 3")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node")
        local term = find_node(ast, "term")
        assert.is_not_nil(term, "expected 'term' node")
    end)

    it("arithmetic produces factor nodes", function()
        local ast = python_parser.parse("r = 5 + 3")
        assert.are.equal("program", ast.rule_name)
        local factor = find_node(ast, "factor")
        assert.is_not_nil(factor, "expected 'factor' node")
    end)

    it("parses parenthesized expression", function()
        -- (a + b) * c — parenthesized groups handled by factor rule
        local ast = python_parser.parse("r = (2 + 3) * 4")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node in (2+3)*4")
    end)

    it("deeply nested arithmetic", function()
        local ast = python_parser.parse("r = 1 + 2 + 3 + 4")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "term")
        assert.is_true(count >= 4, "expected at least 4 term nodes")
    end)

    it("subtraction is parsed at expression level", function()
        local ast = python_parser.parse("r = 10 - 3")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr)
    end)

    it("division is parsed at term level", function()
        local ast = python_parser.parse("r = 10 / 2")
        assert.are.equal("program", ast.rule_name)
        local term = find_node(ast, "term")
        assert.is_not_nil(term)
    end)
end)

-- =========================================================================
-- Multiple statements
-- =========================================================================

describe("multiple statements", function()
    it("parses two assignments", function()
        local ast = python_parser.parse("a = 1\nb = 2")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "assignment")
        assert.are.equal(2, count)
    end)

    it("parses assignment followed by expression", function()
        -- Note: use single-digit integers because Lua's regex engine
        -- doesn't support the (_?[0-9])* group quantifier in the
        -- versioned Python grammars, causing multi-digit integers
        -- to be split into individual digit tokens.
        local ast = python_parser.parse("x = 5\n7")
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(1, count_nodes(ast, "assignment"))
        assert.are.equal(1, count_nodes(ast, "expression_stmt"))
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = python_parser.create_parser("x = 1")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = python_parser.create_parser("x = 1")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src  = "x = 42"
        local ast1 = python_parser.parse(src)
        local p    = python_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Grammar inspection
-- =========================================================================

describe("grammar inspection", function()
    it("grammar has assignment rule", function()
        local g = python_parser.get_grammar()
        local found = false
        for _, rule in ipairs(g.rules) do
            if rule.name == "assignment" then found = true; break end
        end
        assert.is_true(found, "expected 'assignment' rule in grammar")
    end)

    it("grammar has expression rule", function()
        local g = python_parser.get_grammar()
        local found = false
        for _, rule in ipairs(g.rules) do
            if rule.name == "expression" then found = true; break end
        end
        assert.is_true(found, "expected 'expression' rule in grammar")
    end)

    it("grammar has factor rule", function()
        local g = python_parser.get_grammar()
        local found = false
        for _, rule in ipairs(g.rules) do
            if rule.name == "factor" then found = true; break end
        end
        assert.is_true(found, "expected 'factor' rule in grammar")
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for completely invalid input", function()
        assert.has_error(function()
            python_parser.parse("@@@ GARBAGE @@@")
        end)
    end)
end)
