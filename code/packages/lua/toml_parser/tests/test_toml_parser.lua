-- Tests for toml_parser
-- ======================
--
-- Comprehensive busted test suite for the TOML parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Root node has rule_name "document"
--   - Simple key-value: key = "value"
--   - Integer values
--   - Float values
--   - Boolean values (true/false)
--   - Null is not a TOML value (TOML has no null)
--   - Table headers: [section]
--   - Array-of-tables: [[products]]
--   - Dotted keys: a.b.c = 1
--   - Inline arrays: ports = [8001, 8002]
--   - Inline tables: point = {x = 1, y = 2}
--   - Multiple key-value pairs
--   - Multi-section document
--   - get_grammar returns grammar with rules
--   - create_parser returns a GrammarParser
--   - Invalid TOML raises an error

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
    "../../toml_lexer/src/?.lua;"                             ..
    "../../toml_lexer/src/?/init.lua;"                        ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local toml_parser = require("coding_adventures.toml_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Find the first ASTNode with the given rule_name (depth-first).
-- @param node      ASTNode|token
-- @param rule_name string
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

--- Count all ASTNodes with the given rule_name.
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

describe("toml_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(toml_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(toml_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", toml_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(toml_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(toml_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(toml_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = toml_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules >= 8)
    end)

    it("grammar first rule is document", function()
        local g = toml_parser.get_grammar()
        assert.are.equal("document", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = toml_parser.parse("key = 42\n")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'document'", function()
        local ast = toml_parser.parse("key = 42\n")
        assert.are.equal("document", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = toml_parser.parse("key = 42\n")
        assert.is_table(ast.children)
    end)
end)

-- =========================================================================
-- Simple key-value pairs
-- =========================================================================

describe("key-value pairs", function()
    it("parses a string value", function()
        local ast = toml_parser.parse('name = "Alice"\n')
        assert.are.equal("document", ast.rule_name)
        local kv = find_node(ast, "keyval")
        assert.is_not_nil(kv, "expected 'keyval' node")
    end)

    it("parses an integer value", function()
        local ast = toml_parser.parse("port = 8080\n")
        local kv = find_node(ast, "keyval")
        assert.is_not_nil(kv)
    end)

    it("parses a float value", function()
        local ast = toml_parser.parse("pi = 3.14\n")
        local kv = find_node(ast, "keyval")
        assert.is_not_nil(kv)
    end)

    it("parses boolean true", function()
        local ast = toml_parser.parse("debug = true\n")
        local kv = find_node(ast, "keyval")
        assert.is_not_nil(kv)
    end)

    it("parses boolean false", function()
        local ast = toml_parser.parse("enabled = false\n")
        local kv = find_node(ast, "keyval")
        assert.is_not_nil(kv)
    end)

    it("parses multiple key-value pairs", function()
        local src = 'host = "localhost"\nport = 9000\ndebug = true\n'
        local ast = toml_parser.parse(src)
        assert.are.equal("document", ast.rule_name)
        local kv_count = count_nodes(ast, "keyval")
        assert.are.equal(3, kv_count)
    end)
end)

-- =========================================================================
-- Keys
-- =========================================================================

describe("keys", function()
    it("parses bare key", function()
        local ast = toml_parser.parse("my_key = 1\n")
        local key = find_node(ast, "key")
        assert.is_not_nil(key)
    end)

    it("parses quoted key", function()
        local ast = toml_parser.parse('"my key" = 1\n')
        local key = find_node(ast, "key")
        assert.is_not_nil(key)
    end)

    it("parses dotted key (a.b = 1)", function()
        local ast = toml_parser.parse("a.b = 1\n")
        local key = find_node(ast, "key")
        assert.is_not_nil(key)
        -- A dotted key has multiple simple_key nodes
        local sk_count = count_nodes(key, "simple_key")
        assert.is_true(sk_count >= 2)
    end)

    it("parses three-level dotted key (a.b.c = 1)", function()
        local ast = toml_parser.parse("a.b.c = 1\n")
        local key = find_node(ast, "key")
        assert.is_not_nil(key)
        local sk_count = count_nodes(key, "simple_key")
        assert.is_true(sk_count >= 3)
    end)
end)

-- =========================================================================
-- Table headers
-- =========================================================================

describe("table headers", function()
    it("parses a simple table header [section]", function()
        local ast = toml_parser.parse("[server]\n")
        local th = find_node(ast, "table_header")
        assert.is_not_nil(th, "expected 'table_header' node")
    end)

    it("parses table header with key-value pair", function()
        local src = "[server]\nhost = \"localhost\"\n"
        local ast = toml_parser.parse(src)
        assert.is_not_nil(find_node(ast, "table_header"))
        assert.is_not_nil(find_node(ast, "keyval"))
    end)

    it("parses dotted table header [a.b]", function()
        local ast = toml_parser.parse("[a.b]\n")
        local th = find_node(ast, "table_header")
        assert.is_not_nil(th)
    end)

    it("parses multiple table sections", function()
        local src = "[alpha]\nx = 1\n[beta]\ny = 2\n"
        local ast = toml_parser.parse(src)
        local th_count = count_nodes(ast, "table_header")
        assert.are.equal(2, th_count)
    end)
end)

-- =========================================================================
-- Array-of-tables headers
-- =========================================================================

describe("array-of-tables headers", function()
    it("parses [[products]] header", function()
        local ast = toml_parser.parse("[[products]]\n")
        local ath = find_node(ast, "array_table_header")
        assert.is_not_nil(ath, "expected 'array_table_header' node")
    end)

    it("parses multiple [[array]] headers", function()
        local src = "[[fruits]]\nname = \"apple\"\n[[fruits]]\nname = \"banana\"\n"
        local ast = toml_parser.parse(src)
        local ath_count = count_nodes(ast, "array_table_header")
        assert.are.equal(2, ath_count)
    end)
end)

-- =========================================================================
-- Inline arrays
-- =========================================================================

describe("inline arrays", function()
    it("parses an empty inline array", function()
        local ast = toml_parser.parse("ports = []\n")
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr)
    end)

    it("parses an inline array of integers", function()
        local ast = toml_parser.parse("ports = [8001, 8002, 8003]\n")
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr)
    end)

    it("parses an inline array of strings", function()
        local ast = toml_parser.parse('colors = ["red", "green", "blue"]\n')
        local arr = find_node(ast, "array")
        assert.is_not_nil(arr)
    end)

    it("parses a nested inline array", function()
        local ast = toml_parser.parse("matrix = [[1, 2], [3, 4]]\n")
        -- Should find at least two array nodes
        local arr_count = count_nodes(ast, "array")
        assert.is_true(arr_count >= 2)
    end)
end)

-- =========================================================================
-- Inline tables
-- =========================================================================

describe("inline tables", function()
    it("parses an empty inline table", function()
        local ast = toml_parser.parse("empty = {}\n")
        local it_node = find_node(ast, "inline_table")
        assert.is_not_nil(it_node)
    end)

    it("parses an inline table with one pair", function()
        local ast = toml_parser.parse("point = {x = 1}\n")
        local it_node = find_node(ast, "inline_table")
        assert.is_not_nil(it_node)
    end)

    it("parses an inline table with multiple pairs", function()
        local ast = toml_parser.parse("point = {x = 1, y = 2}\n")
        local it_node = find_node(ast, "inline_table")
        assert.is_not_nil(it_node)
        local kv_count = count_nodes(ast, "keyval")
        -- outer keyval for "point = {...}" plus inner keyvals x=1 and y=2
        assert.is_true(kv_count >= 3)
    end)
end)

-- =========================================================================
-- Multi-section document
-- =========================================================================

describe("multi-section document", function()
    it("parses a realistic TOML config", function()
        local src = [[
[database]
server = "192.168.1.1"
ports = [5432, 5433]
enabled = true

[servers.alpha]
ip = "10.0.0.1"
dc = "eqdc10"

[[products]]
name = "Widget"
sku = 738594937

[[products]]
name = "Gadget"
sku = 284758393
]]
        local ast = toml_parser.parse(src)
        assert.are.equal("document", ast.rule_name)
        -- Should find table_header nodes
        local th_count = count_nodes(ast, "table_header")
        assert.is_true(th_count >= 2)
        -- Should find array_table_header nodes
        local ath_count = count_nodes(ast, "array_table_header")
        assert.are.equal(2, ath_count)
        -- Should have keyval nodes
        local kv_count = count_nodes(ast, "keyval")
        assert.is_true(kv_count >= 5)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = toml_parser.create_parser("key = 1\n")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = toml_parser.create_parser("key = 1\n")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule as parse()", function()
        local src = "x = 42\n"
        local ast1 = toml_parser.parse(src)
        local p = toml_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for missing equals sign", function()
        assert.has_error(function()
            toml_parser.parse("key value\n")
        end)
    end)

    it("raises an error for unterminated inline array", function()
        assert.has_error(function()
            toml_parser.parse("ports = [1, 2\n")
        end)
    end)

    it("raises an error for unterminated table header", function()
        assert.has_error(function()
            toml_parser.parse("[server\n")
        end)
    end)
end)
