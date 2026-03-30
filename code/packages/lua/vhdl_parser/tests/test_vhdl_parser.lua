-- Tests for vhdl_parser
-- =======================
--
-- Comprehensive busted test suite for the VHDL parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty entity: entity empty is end entity;
--   - Entity with port clause
--   - Architecture body
--   - Grammar first rule is "design_file"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Empty design file (no units)
--   - Invalid input raises an error

-- Resolve sibling packages from the monorepo so busted can find them.
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
    "../../vhdl_lexer/src/?.lua;"                                ..
    "../../vhdl_lexer/src/?/init.lua;"                           ..
    "../../parser/src/?.lua;"                                    ..
    "../../parser/src/?/init.lua;"                               ..
    package.path
)

local vhdl_parser = require("coding_adventures.vhdl_parser")

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

-- =========================================================================
-- Module surface
-- =========================================================================

describe("vhdl_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(vhdl_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(vhdl_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", vhdl_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(vhdl_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(vhdl_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(vhdl_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = vhdl_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- vhdl.grammar has many rules (design_file, design_unit, entity_declaration, ...)
        assert.is_true(#g.rules >= 10)
    end)

    it("grammar first rule is design_file", function()
        local g = vhdl_parser.get_grammar()
        assert.are.equal("design_file", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'design_file'", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        assert.are.equal("design_file", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        assert.is_table(ast.children)
    end)

    it("design_file contains a design_unit", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        local du = find_node(ast, "design_unit")
        assert.is_not_nil(du, "expected 'design_unit' node")
    end)

    it("design_unit contains entity_declaration", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        local ed = find_node(ast, "entity_declaration")
        assert.is_not_nil(ed, "expected 'entity_declaration' node")
    end)
end)

-- =========================================================================
-- Entity declarations
-- =========================================================================

describe("entity declarations", function()
    it("parses minimal entity", function()
        local ast = vhdl_parser.parse("entity empty is end entity;")
        assert.are.equal("design_file", ast.rule_name)
        local ed = find_node(ast, "entity_declaration")
        assert.is_not_nil(ed)
    end)

    it("parses multiple design units", function()
        local src = [[
            entity a is end entity;
            entity b is end entity;
        ]]
        local ast = vhdl_parser.parse(src)
        local count = count_nodes(ast, "entity_declaration")
        assert.are.equal(2, count)
    end)

    it("parses empty design file (no units)", function()
        -- { design_unit } allows zero units
        local ast = vhdl_parser.parse("")
        assert.are.equal("design_file", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = vhdl_parser.create_parser("entity empty is end entity;")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = vhdl_parser.create_parser("entity empty is end entity;")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src = "entity empty is end entity;"
        local ast1 = vhdl_parser.parse(src)
        local p    = vhdl_parser.create_parser(src)
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
            vhdl_parser.parse("@@@ NOT VHDL @@@")
        end)
    end)
end)
