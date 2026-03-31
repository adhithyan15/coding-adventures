-- Tests for json_parser
-- =====================
--
-- Comprehensive busted test suite for the JSON parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Simple key-value object: {"key": 42}
--   - Empty object {}
--   - Empty array []
--   - Nested: {"a": [1, 2, {"b": true}]}
--   - All value types: string, number, true, false, null
--   - Multiple pairs in object
--   - Multiple values in array
--   - Deeply nested structures
--   - Root AST node has rule_name "value"
--   - ASTNode is_leaf / token access
--   - Invalid JSON raises an error
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    "../../json_lexer/src/?.lua;"                             ..
    "../../json_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local json_parser = require("coding_adventures.json_parser")

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

--- Find the first ASTNode with the given rule_name (breadth-first).
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

describe("json_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(json_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(json_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", json_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(json_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(json_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(json_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = json_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules >= 4)
    end)

    it("grammar first rule is value", function()
        local g = json_parser.get_grammar()
        assert.are.equal("value", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = json_parser.parse('42')
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'value'", function()
        local ast = json_parser.parse('42')
        assert.are.equal("value", ast.rule_name)
    end)

    it("root node has children", function()
        local ast = json_parser.parse('42')
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- Scalar values
-- =========================================================================

describe("scalar value types", function()
    it("parses a bare number", function()
        local ast = json_parser.parse("42")
        assert.are.equal("value", ast.rule_name)
        -- The value node wraps a NUMBER token leaf
        assert.is_true(#ast.children >= 1)
    end)

    it("parses a negative number", function()
        local ast = json_parser.parse("-3.14")
        assert.are.equal("value", ast.rule_name)
    end)

    it("parses a string", function()
        local ast = json_parser.parse('"hello"')
        assert.are.equal("value", ast.rule_name)
    end)

    it("parses true", function()
        local ast = json_parser.parse("true")
        assert.are.equal("value", ast.rule_name)
    end)

    it("parses false", function()
        local ast = json_parser.parse("false")
        assert.are.equal("value", ast.rule_name)
    end)

    it("parses null", function()
        local ast = json_parser.parse("null")
        assert.are.equal("value", ast.rule_name)
    end)
end)

-- =========================================================================
-- Empty containers
-- =========================================================================

describe("empty containers", function()
    it("parses empty object {}", function()
        local ast = json_parser.parse("{}")
        assert.are.equal("value", ast.rule_name)
        -- Should contain an "object" node inside
        local obj = find_node(ast, "object")
        assert.is_not_nil(obj, "expected an 'object' node")
    end)

    it("parses empty array []", function()
        local ast = json_parser.parse("[]")
        assert.are.equal("value", ast.rule_name)
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr, "expected an 'array' node")
    end)
end)

-- =========================================================================
-- Simple object
-- =========================================================================

describe("simple object", function()
    it("parses {\"key\": 42}", function()
        local ast = json_parser.parse('{"key": 42}')
        assert.are.equal("value", ast.rule_name)
        local obj = find_node(ast, "object")
        assert.is_not_nil(obj, "expected 'object' node")
        local pair = find_node(ast, "pair")
        assert.is_not_nil(pair, "expected 'pair' node")
    end)

    it("object node contains a pair node", function()
        local ast = json_parser.parse('{"name": "Alice"}')
        local pair = find_node(ast, "pair")
        assert.is_not_nil(pair)
    end)

    it("parses object with multiple pairs", function()
        local ast = json_parser.parse('{"a": 1, "b": 2, "c": 3}')
        -- Should have 3 pair nodes
        local pair_count = count_nodes(ast, "pair")
        assert.are.equal(3, pair_count)
    end)

    it("parses object with boolean values", function()
        local ast = json_parser.parse('{"ok": true, "fail": false}')
        assert.are.equal("value", ast.rule_name)
        assert.are.equal(2, count_nodes(ast, "pair"))
    end)

    it("parses object with null value", function()
        local ast = json_parser.parse('{"data": null}')
        assert.are.equal("value", ast.rule_name)
        assert.are.equal(1, count_nodes(ast, "pair"))
    end)
end)

-- =========================================================================
-- Simple array
-- =========================================================================

describe("simple array", function()
    it("parses array of numbers", function()
        local ast = json_parser.parse("[1, 2, 3]")
        assert.are.equal("value", ast.rule_name)
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr)
    end)

    it("parses array of strings", function()
        local ast = json_parser.parse('["a", "b", "c"]')
        assert.are.equal("value", ast.rule_name)
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr)
    end)

    it("parses array of mixed value types", function()
        local ast = json_parser.parse('[1, "two", true, false, null]')
        assert.are.equal("value", ast.rule_name)
        -- value nodes inside the array: 5 for elements + 1 root
        local val_count = count_nodes(ast, "value")
        assert.is_true(val_count >= 5)
    end)

    it("parses single-element array", function()
        local ast = json_parser.parse("[42]")
        assert.are.equal("value", ast.rule_name)
        assert.is_not_nil(find_node(ast, "array"))
    end)
end)

-- =========================================================================
-- Nested structures
-- =========================================================================

describe("nested structures", function()
    it("parses object nested inside object", function()
        local ast = json_parser.parse('{"a": {"b": 2}}')
        assert.are.equal("value", ast.rule_name)
        local pairs = count_nodes(ast, "pair")
        assert.are.equal(2, pairs)
    end)

    it("parses array inside object", function()
        local ast = json_parser.parse('{"tags": ["lua", "parser"]}')
        assert.are.equal("value", ast.rule_name)
        assert.is_not_nil(find_node(ast, "array"))
    end)

    it("parses object inside array", function()
        local ast = json_parser.parse('[{"id": 1}, {"id": 2}]')
        assert.are.equal("value", ast.rule_name)
        local pair_count = count_nodes(ast, "pair")
        assert.are.equal(2, pair_count)
    end)

    it("parses deeply nested: {\"a\": [1, 2, {\"b\": true}]}", function()
        local ast = json_parser.parse('{"a": [1, 2, {"b": true}]}')
        assert.are.equal("value", ast.rule_name)
        -- Should have "value", "object" (outer), "pair" (a:…), "array", "value"s,
        -- "object" (inner), "pair" (b:true)
        assert.is_not_nil(find_node(ast, "array"))
        assert.are.equal(2, count_nodes(ast, "pair"))
    end)

    it("parses array of arrays", function()
        local ast = json_parser.parse("[[1, 2], [3, 4]]")
        assert.are.equal("value", ast.rule_name)
        local arr_count = count_nodes(ast, "array")
        assert.is_true(arr_count >= 2)
    end)

    it("parses a realistic JSON document", function()
        local src = [[
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5,
  "tags": ["lua", "parser"],
  "address": {
    "city": "Metropolis",
    "zip": null
  }
}]]
        local ast = json_parser.parse(src)
        assert.are.equal("value", ast.rule_name)
        -- Should have the outer object
        local obj = find_node(ast, "object")
        assert.is_not_nil(obj)
        -- Should have "tags" array and "address" object
        assert.is_not_nil(find_node(ast, "array"))
        -- Multiple pairs
        local pair_count = count_nodes(ast, "pair")
        assert.is_true(pair_count >= 6)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = json_parser.create_parser('{}')
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = json_parser.create_parser('{}')
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns the same AST as parse()", function()
        local src = '{"x": 1}'
        local ast1 = json_parser.parse(src)
        local p    = json_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for trailing garbage", function()
        assert.has_error(function()
            json_parser.parse('42 garbage')
        end)
    end)

    it("raises an error for unterminated object", function()
        assert.has_error(function()
            json_parser.parse('{"key": 1')
        end)
    end)

    it("raises an error for unterminated array", function()
        assert.has_error(function()
            json_parser.parse('[1, 2')
        end)
    end)

    it("raises an error for missing colon in pair", function()
        assert.has_error(function()
            json_parser.parse('{"key" 1}')
        end)
    end)

    it("raises an error for bare identifier", function()
        assert.has_error(function()
            json_parser.parse('undefined')
        end)
    end)

    it("raises an error for empty input", function()
        assert.has_error(function()
            json_parser.parse('')
        end)
    end)
end)
