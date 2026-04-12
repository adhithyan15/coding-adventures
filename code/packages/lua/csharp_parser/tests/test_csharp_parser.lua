-- Tests for csharp_parser
-- ========================
--
-- Comprehensive busted test suite for the C# parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - int x = 5 → program → statement → var_declaration
--   - Assignment: x = 10;
--   - Expression precedence: 1 + 2 * 3
--   - Parenthesized expressions: (a + b) * c
--   - Expression statements
--   - Grammar first rule is "program"
--   - create_csharp_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Version-aware parsing for all 12 C# versions
--   - Invalid input raises an error

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                              ..
    "../src/?/init.lua;"                                         ..
    "../../grammar_tools/src/?.lua;"                             ..
    "../../grammar_tools/src/?/init.lua;"                        ..
    "../../lexer/src/?.lua;"                                     ..
    "../../lexer/src/?/init.lua;"                                ..
    "../../state_machine/src/?.lua;"                             ..
    "../../state_machine/src/?/init.lua;"                        ..
    "../../directed_graph/src/?.lua;"                            ..
    "../../directed_graph/src/?/init.lua;"                       ..
    "../../csharp_lexer/src/?.lua;"                              ..
    "../../csharp_lexer/src/?/init.lua;"                         ..
    "../../parser/src/?.lua;"                                    ..
    "../../parser/src/?/init.lua;"                               ..
    package.path
)

local csharp_parser = require("coding_adventures.csharp_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Recursively collect all rule_name values present in the AST.
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

--- Return the first node in the AST with the given rule_name, or nil.
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

--- Count how many nodes in the AST have the given rule_name.
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

describe("csharp_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(csharp_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(csharp_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", csharp_parser.VERSION)
    end)

    it("exposes parse_csharp as a function", function()
        assert.is_function(csharp_parser.parse_csharp)
    end)

    it("exposes create_csharp_parser as a function", function()
        assert.is_function(csharp_parser.create_csharp_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(csharp_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = csharp_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules >= 4)
    end)

    it("grammar first rule is program", function()
        local g = csharp_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse_csharp returns a non-nil value", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        assert.is_table(ast.children)
    end)

    it("program contains a statement node", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)
end)

-- =========================================================================
-- Variable declarations
-- =========================================================================

describe("variable declarations", function()
    it("parses int x = 5", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        assert.are.equal("program", ast.rule_name)
        local vd = find_node(ast, "var_declaration")
        assert.is_not_nil(vd, "expected 'var_declaration' node")
    end)

    it("parses multiple declarations", function()
        local ast = csharp_parser.parse_csharp("int x = 1;\nint y = 2;")
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
        local ast = csharp_parser.parse_csharp("x = 10;")
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign, "expected 'assignment' node")
    end)
end)

-- =========================================================================
-- Expression statements
-- =========================================================================

describe("expression statements", function()
    it("parses expression_stmt", function()
        local ast = csharp_parser.parse_csharp("42;")
        assert.are.equal("program", ast.rule_name)
        local es = find_node(ast, "expression_stmt")
        assert.is_not_nil(es, "expected 'expression_stmt' node")
    end)
end)

-- =========================================================================
-- Expression precedence
-- =========================================================================
--
-- C# (and most C-family languages) follows the standard arithmetic precedence
-- rules: multiplication and division bind more tightly than addition and
-- subtraction. The grammar encodes this through rule layering:
--
--   expression → term { +/- term }     (lower precedence)
--   term       → factor { */÷ factor } (higher precedence)
--
-- So `1 + 2 * 3` must parse as `1 + (2 * 3)`, not `(1 + 2) * 3`.

describe("expression precedence", function()
    it("1 + 2 * 3 has expression and term nodes", function()
        local ast = csharp_parser.parse_csharp("int r = 1 + 2 * 3;")
        assert.are.equal("program", ast.rule_name)
        local expr = find_node(ast, "expression")
        assert.is_not_nil(expr, "expected 'expression' node")
        local term = find_node(ast, "term")
        assert.is_not_nil(term, "expected 'term' node")
    end)

    it("arithmetic expression produces factor nodes", function()
        local ast = csharp_parser.parse_csharp("int r = 5 + 3;")
        assert.are.equal("program", ast.rule_name)
        local factor = find_node(ast, "factor")
        assert.is_not_nil(factor, "expected 'factor' node")
    end)

    it("parses parenthesized expression", function()
        local ast = csharp_parser.parse_csharp("int r = (2 + 3) * 4;")
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
        local ast = csharp_parser.parse_csharp("int a = 1;\nint b = 2;")
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "var_declaration")
        assert.are.equal(2, count)
    end)

    it("parses mixed statements", function()
        local ast = csharp_parser.parse_csharp(
            "int x = 5;\nx = 10;\n42;"
        )
        assert.are.equal("program", ast.rule_name)
        assert.are.equal(1, count_nodes(ast, "var_declaration"))
        assert.are.equal(1, count_nodes(ast, "assignment"))
        assert.are.equal(1, count_nodes(ast, "expression_stmt"))
    end)

    it("parses empty program (no statements)", function()
        local ast = csharp_parser.parse_csharp("")
        assert.are.equal("program", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- create_csharp_parser
-- =========================================================================

describe("create_csharp_parser", function()
    it("returns a non-nil parser object", function()
        local p = csharp_parser.create_csharp_parser("int x = 1;")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = csharp_parser.create_csharp_parser("int x = 1;")
        assert.is_function(p.parse)
    end)

    it("parsing via create_csharp_parser returns same root rule_name as parse_csharp()", function()
        local src  = "int x = 42;"
        local ast1 = csharp_parser.parse_csharp(src)
        local p    = csharp_parser.create_csharp_parser(src)
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
            csharp_parser.parse_csharp("@@@ GARBAGE @@@")
        end)
    end)

    it("raises an error for unknown version string", function()
        assert.has_error(function()
            csharp_parser.parse_csharp("int x = 1;", "99.0")
        end)
    end)

    it("raises an error for invalid version string", function()
        assert.has_error(function()
            csharp_parser.parse_csharp("int x = 1;", "csharp12")
        end)
    end)
end)

-- =========================================================================
-- Version-aware parsing
-- =========================================================================
--
-- All 12 C# versions share the same core grammar subset tested here:
-- variable declarations, assignments, and arithmetic expressions.
-- The grammar files differ in which *keywords* they accept (e.g., "async"
-- was added in 5.0, "record" in 9.0), but plain integer arithmetic is valid
-- in every version.

describe("version-aware parsing", function()

    it("parse with no version (defaults to 12.0)", function()
        local ast = csharp_parser.parse_csharp("int x = 5;")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with empty string version (defaults to 12.0)", function()
        local ast = csharp_parser.parse_csharp("int x = 5;", "")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 1.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "1.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 2.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "2.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 3.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "3.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 4.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "4.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 5.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "5.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 6.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "6.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 7.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "7.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 8.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "8.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 9.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "9.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 10.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "10.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 11.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "11.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    it("parse with version 12.0", function()
        local ast = csharp_parser.parse_csharp("int x = 1;", "12.0")
        assert.is_not_nil(ast)
        assert.are.equal("program", ast.rule_name)
    end)

    -- create_csharp_parser with version

    it("create_csharp_parser with version 8.0 returns a usable GrammarParser", function()
        local p = csharp_parser.create_csharp_parser("int x = 1;", "8.0")
        assert.is_not_nil(p)
        local ast, err = p:parse()
        assert.is_nil(err)
        assert.is_not_nil(ast)
    end)

    it("create_csharp_parser with no version works", function()
        local p = csharp_parser.create_csharp_parser("int x = 1;")
        assert.is_not_nil(p)
        local ast, err = p:parse()
        assert.is_nil(err)
        assert.is_not_nil(ast)
    end)

    -- get_grammar with version

    it("get_grammar with version 1.0 returns a grammar object", function()
        local g = csharp_parser.get_grammar("1.0")
        assert.is_not_nil(g)
    end)

    it("get_grammar with version 12.0 returns a grammar object", function()
        local g = csharp_parser.get_grammar("12.0")
        assert.is_not_nil(g)
    end)

    it("get_grammar with no version returns a grammar object", function()
        local g = csharp_parser.get_grammar()
        assert.is_not_nil(g)
    end)

    it("get_grammar caches results (same object returned for same version)", function()
        local g1 = csharp_parser.get_grammar("8.0")
        local g2 = csharp_parser.get_grammar("8.0")
        assert.are.equal(g1, g2)
    end)
end)
