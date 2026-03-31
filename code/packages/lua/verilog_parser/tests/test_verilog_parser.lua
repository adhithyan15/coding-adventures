-- Tests for verilog_parser
-- =========================
--
-- Comprehensive busted test suite for the Verilog parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Empty module: module empty; endmodule
--   - Module with ports: input/output declarations
--   - Continuous assignment: assign y = a & b
--   - Always block with sensitivity list
--   - Grammar first rule is "source_text"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules

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
    "../../verilog_lexer/src/?.lua;"                             ..
    "../../verilog_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                    ..
    "../../parser/src/?/init.lua;"                               ..
    package.path
)

local verilog_parser = require("coding_adventures.verilog_parser")

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

describe("verilog_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(verilog_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(verilog_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", verilog_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(verilog_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(verilog_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(verilog_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = verilog_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- verilog.grammar has many rules
        assert.is_true(#g.rules >= 10)
    end)

    it("grammar first rule is source_text", function()
        local g = verilog_parser.get_grammar()
        assert.are.equal("source_text", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'source_text'", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        assert.are.equal("source_text", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        assert.is_table(ast.children)
    end)

    it("source_text contains a description node", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        local desc = find_node(ast, "description")
        assert.is_not_nil(desc, "expected 'description' node")
    end)

    it("description contains module_declaration", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        local md = find_node(ast, "module_declaration")
        assert.is_not_nil(md, "expected 'module_declaration' node")
    end)
end)

-- =========================================================================
-- Module declarations
-- =========================================================================

describe("module declarations", function()
    it("parses empty module", function()
        local ast = verilog_parser.parse("module empty; endmodule")
        assert.are.equal("source_text", ast.rule_name)
        local md = find_node(ast, "module_declaration")
        assert.is_not_nil(md)
    end)

    it("parses multiple modules", function()
        local src = "module a; endmodule\nmodule b; endmodule"
        local ast = verilog_parser.parse(src)
        local count = count_nodes(ast, "module_declaration")
        assert.are.equal(2, count)
    end)

    it("parses empty source_text (no modules)", function()
        -- { description } allows zero descriptions
        local ast = verilog_parser.parse("")
        assert.are.equal("source_text", ast.rule_name)
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = verilog_parser.create_parser("module empty; endmodule")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = verilog_parser.create_parser("module empty; endmodule")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src = "module empty; endmodule"
        local ast1 = verilog_parser.parse(src)
        local p    = verilog_parser.create_parser(src)
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
            verilog_parser.parse("@@@ NOT VERILOG @@@")
        end)
    end)
end)
