-- Tests for java_parser
-- ======================
--
-- Comprehensive busted test suite for the Java parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - int x = 5 → program → statement → var_declaration
--   - Assignment: x = 10;
--   - Expression precedence: 1 + 2 * 3
--   - Parenthesized expressions: (a + b) * c
--   - Expression statements
--   - Grammar first rule is "program"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Version-aware parsing
--   - Invalid input raises an error

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
    "../../java_lexer/src/?.lua;"                               ..
    "../../java_lexer/src/?/init.lua;"                          ..
    "../../parser/src/?.lua;"                                   ..
    "../../parser/src/?/init.lua;"                              ..
    package.path
)

local java_parser = require("coding_adventures.java_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

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

describe("java_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(java_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(java_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", java_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(java_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(java_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(java_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = java_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules >= 4)
    end)

    it("grammar first rule is program", function()
        local g = java_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = java_parser.parse("int x = 5;")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = java_parser.parse("int x = 5;")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = java_parser.parse("int x = 5;")
        assert.is_table(ast.children)
    end)

    it("program contains a statement node", function()
        local ast = java_parser.parse("int x = 5;")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)
end)

-- =========================================================================
-- Variable declarations
-- =========================================================================

describe("variable declarations", function()
    it("parses int x = 5", function()
        local ast = java_parser.parse("int x = 5;")
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd, "expected 'var_declaration' node")
    end)

    it("parses multiple declarations", function()
        local ast = java_parser.parse("int x = 1;\nint y = 2;")
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
        local ast = java_parser.parse("x = 10;")
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment_expression")
        assert.is_not_nil(assign, "expected 'assignment_expression' node")
    end)
end)

-- =========================================================================
-- Expression statements
-- =========================================================================

describe("expression statements", function()
    it("parses expression_stmt", function()
        local ast = java_parser.parse("42;")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_statement")
        assert.is_not_nil(es, "expected 'expression_statement' node")
    end)
end)

-- =========================================================================
-- Expression precedence
-- =========================================================================

describe("expression precedence", function()
    it("1 + 2 * 3 has expression and multiplicative_expression nodes", function()
        local ast = java_parser.parse("int r = 1 + 2 * 3;")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node")
        local term = find_node(ast, "multiplicative_expression")
        assert.is_not_nil(term, "expected 'multiplicative_expression' node")
    end)

    it("arithmetic expression produces primary nodes", function()
        local ast = java_parser.parse("int r = 5 + 3;")
        assert.are.equal("program", ast.rule_name)
        local factor = find_node(ast, "primary")
        assert.is_not_nil(factor, "expected 'primary' node")
    end)

    it("parses parenthesized expression", function()
        local ast = java_parser.parse("int r = (2 + 3) * 4;")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node in (2+3)*4")
    end)
end)

-- =========================================================================
-- Multiple statements
-- =========================================================================

describe("multiple statements", function()
    it("parses two var declarations", function()
        local ast = java_parser.parse("int a = 1;\nint b = 2;")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "var_declaration")
        assert.are.equal(2, count)
    end)

    it("parses mixed statements", function()
        local ast = java_parser.parse(
            "int x = 5;\nx = 10;\n42;"
        )
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(1, count_nodes(ast, "var_declaration"))
        assert.are.equal(2, count_nodes(ast, "expression_statement"))
    end)

    it("parses empty program (no statements)", function()
        local ast = java_parser.parse("")
        assert.are.equal("program", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = java_parser.create_parser("int x = 1;")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = java_parser.create_parser("int x = 1;")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src = "int x = 42;"
        local ast1 = java_parser.parse(src)
        local p    = java_parser.create_parser(src)
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
            java_parser.parse("@@@ GARBAGE @@@")
        end)
    end)
end)

-- =========================================================================
-- Version-aware parsing
-- =========================================================================

describe("version-aware parsing", function()

    it("parse with no version (default)", function()
        local ast = java_parser.parse("int x = 5;")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with empty string version (default)", function()
        local ast = java_parser.parse("int x = 5;", "")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 1.0", function()
        local ast = java_parser.parse("int x = 1;", "1.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 1.1", function()
        local ast = java_parser.parse("int x = 1;", "1.1")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 8", function()
        local ast = java_parser.parse("int x = 1;", "8")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 21", function()
        local ast = java_parser.parse("int x = 1;", "21")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("create_parser with version 8 returns a usable GrammarParser", function()
        local p = java_parser.create_parser("int x = 1;", "8")
        assert.is_not_nil(p)
        local ast, err = p:parse()
        assert.is_nil(err)
        assert.is_not_nil(ast)
    end)

    it("create_parser with no version works", function()
        local p = java_parser.create_parser("int x = 1;")
        assert.is_not_nil(p)
        local ast, err = p:parse()
        assert.is_nil(err)
        assert.is_not_nil(ast)
    end)

    it("get_grammar with version 8 returns a grammar object", function()
        local g = java_parser.get_grammar("8")
        assert.is_not_nil(g)
    end)

    it("get_grammar with no version returns a grammar object", function()
        local g = java_parser.get_grammar()
        assert.is_not_nil(g)
    end)

    it("raises an error for unknown version string", function()
        assert.has_error(function()
            java_parser.parse("int x = 1;", "99")
        end)
    end)

    it("raises an error for invalid version string", function()
        assert.has_error(function()
            java_parser.parse("int x = 1;", "java21")
        end)
    end)
end)
