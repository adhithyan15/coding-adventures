-- Tests for sql_parser
-- =====================
--
-- Comprehensive busted test suite for the SQL parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - SELECT * FROM users → program → statement → select_stmt
--   - SELECT with column list and WHERE clause
--   - SELECT with DISTINCT
--   - INSERT INTO ... VALUES (...)
--   - UPDATE ... SET ... WHERE ...
--   - DELETE FROM ... WHERE ...
--   - Expressions: comparisons, AND/OR, arithmetic
--   - Grammar first rule is "program"
--   - create_parser returns a GrammarParser
--   - get_grammar returns a grammar with rules
--   - Invalid SQL raises an error

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
    "../../sql_lexer/src/?.lua;"                              ..
    "../../sql_lexer/src/?/init.lua;"                         ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local sql_parser = require("coding_adventures.sql_parser")

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

describe("sql_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(sql_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(sql_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", sql_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(sql_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(sql_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(sql_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = sql_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- sql.grammar has many rules (program, statement, select_stmt, etc.)
        assert.is_true(#g.rules >= 5)
    end)

    it("grammar first rule is program", function()
        local g = sql_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        assert.is_table(ast.children)
        assert.is_true(#ast.children >= 1)
    end)

    it("program contains a statement node", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)
end)

-- =========================================================================
-- SELECT statements
-- =========================================================================

describe("SELECT statements", function()
    it("parses SELECT * FROM users", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        assert.are.equal("program", ast.rule_name)
        local sel = find_node(ast, "select_stmt")
        assert.is_not_nil(sel, "expected 'select_stmt' node")
    end)

    it("select_list node present for SELECT *", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        local sl = find_node(ast, "select_list")
        assert.is_not_nil(sl, "expected 'select_list' node")
    end)

    it("table_ref node present", function()
        local ast = sql_parser.parse("SELECT * FROM users")
        local tr = find_node(ast, "table_ref")
        assert.is_not_nil(tr, "expected 'table_ref' node")
    end)

    it("parses SELECT with column list", function()
        local ast = sql_parser.parse("SELECT name, age FROM users")
        assert.are.equal("program", ast.rule_name)
        local sel = find_node(ast, "select_stmt")
        assert.is_not_nil(sel)
        -- Two select_item nodes expected
        local count = count_nodes(ast, "select_item")
        assert.is_true(count >= 2, "expected at least 2 select_item nodes")
    end)

    it("parses SELECT name, age FROM users WHERE age > 18", function()
        local ast = sql_parser.parse("SELECT name, age FROM users WHERE age > 18")
        assert.are.equal("program", ast.rule_name)
        local wc = find_node(ast, "where_clause")
        assert.is_not_nil(wc, "expected 'where_clause' node")
    end)

    it("parses SELECT DISTINCT", function()
        local ast = sql_parser.parse("SELECT DISTINCT name FROM users")
        assert.are.equal("program", ast.rule_name)
        local sel = find_node(ast, "select_stmt")
        assert.is_not_nil(sel)
    end)

    it("parses SELECT with AND condition", function()
        local ast = sql_parser.parse(
            "SELECT * FROM employees WHERE salary > 50000 AND active = TRUE"
        )
        assert.are.equal("program", ast.rule_name)
        local wc = find_node(ast, "where_clause")
        assert.is_not_nil(wc)
    end)

    it("parses SELECT with ORDER BY", function()
        local ast = sql_parser.parse(
            "SELECT * FROM users ORDER BY name"
        )
        assert.are.equal("program", ast.rule_name)
        local oc = find_node(ast, "order_clause")
        assert.is_not_nil(oc, "expected 'order_clause' node")
    end)

    it("parses SELECT with LIMIT", function()
        local ast = sql_parser.parse(
            "SELECT * FROM users LIMIT 10"
        )
        assert.are.equal("program", ast.rule_name)
        local lc = find_node(ast, "limit_clause")
        assert.is_not_nil(lc, "expected 'limit_clause' node")
    end)

    it("parses SELECT with JOIN", function()
        local ast = sql_parser.parse(
            "SELECT * FROM users INNER JOIN orders ON users.id = orders.user_id"
        )
        assert.are.equal("program", ast.rule_name)
        local jc = find_node(ast, "join_clause")
        assert.is_not_nil(jc, "expected 'join_clause' node")
    end)

    it("parses SELECT with GROUP BY", function()
        local ast = sql_parser.parse(
            "SELECT department FROM employees GROUP BY department"
        )
        assert.are.equal("program", ast.rule_name)
        local gc = find_node(ast, "group_clause")
        assert.is_not_nil(gc, "expected 'group_clause' node")
    end)
end)

-- =========================================================================
-- INSERT statements
-- =========================================================================

describe("INSERT statements", function()
    it("parses INSERT INTO orders VALUES (1, 'item', 9.99)", function()
        local ast = sql_parser.parse("INSERT INTO orders VALUES (1, 'item', 9.99)")
        assert.are.equal("program", ast.rule_name)
        local ins = find_node(ast, "insert_stmt")
        assert.is_not_nil(ins, "expected 'insert_stmt' node")
    end)

    it("insert_stmt has row_value node", function()
        local ast = sql_parser.parse("INSERT INTO t VALUES (42, 'hello')")
        local rv = find_node(ast, "row_value")
        assert.is_not_nil(rv, "expected 'row_value' node")
    end)

    it("parses INSERT with column list", function()
        local ast = sql_parser.parse(
            "INSERT INTO users (id, name) VALUES (1, 'Alice')"
        )
        assert.are.equal("program", ast.rule_name)
        local ins = find_node(ast, "insert_stmt")
        assert.is_not_nil(ins)
    end)

    it("parses INSERT with multiple values", function()
        local ast = sql_parser.parse(
            "INSERT INTO t VALUES (1, 2, 3, 4)"
        )
        assert.are.equal("program", ast.rule_name)
        local rv = find_node(ast, "row_value")
        assert.is_not_nil(rv)
    end)
end)

-- =========================================================================
-- UPDATE statements
-- =========================================================================

describe("UPDATE statements", function()
    it("parses UPDATE users SET name = 'Bob' WHERE id = 1", function()
        local ast = sql_parser.parse("UPDATE users SET name = 'Bob' WHERE id = 1")
        assert.are.equal("program", ast.rule_name)
        local upd = find_node(ast, "update_stmt")
        assert.is_not_nil(upd, "expected 'update_stmt' node")
    end)

    it("update_stmt has assignment node", function()
        local ast = sql_parser.parse("UPDATE t SET col = 42")
        local assign = find_node(ast, "assignment")
        assert.is_not_nil(assign, "expected 'assignment' node")
    end)

    it("parses UPDATE with WHERE clause", function()
        local ast = sql_parser.parse("UPDATE t SET x = 1 WHERE y = 2")
        local wc = find_node(ast, "where_clause")
        assert.is_not_nil(wc, "expected 'where_clause' node")
    end)

    it("parses UPDATE with multiple assignments", function()
        local ast = sql_parser.parse(
            "UPDATE users SET name = 'Alice', age = 30 WHERE id = 1"
        )
        assert.are.equal("program", ast.rule_name)
        local assign_count = count_nodes(ast, "assignment")
        assert.is_true(assign_count >= 2, "expected at least 2 assignment nodes")
    end)
end)

-- =========================================================================
-- DELETE statements
-- =========================================================================

describe("DELETE statements", function()
    it("parses DELETE FROM temp WHERE expired = TRUE", function()
        local ast = sql_parser.parse("DELETE FROM temp WHERE expired = TRUE")
        assert.are.equal("program", ast.rule_name)
        local del = find_node(ast, "delete_stmt")
        assert.is_not_nil(del, "expected 'delete_stmt' node")
    end)

    it("parses DELETE without WHERE clause", function()
        local ast = sql_parser.parse("DELETE FROM temp")
        assert.are.equal("program", ast.rule_name)
        local del = find_node(ast, "delete_stmt")
        assert.is_not_nil(del)
    end)

    it("delete_stmt has where_clause when present", function()
        local ast = sql_parser.parse("DELETE FROM t WHERE id = 5")
        local wc = find_node(ast, "where_clause")
        assert.is_not_nil(wc)
    end)
end)

-- =========================================================================
-- Expression nodes
-- =========================================================================

describe("expression nodes", function()
    it("comparison produces comparison node", function()
        local ast = sql_parser.parse("SELECT * FROM t WHERE a > 1")
        local cmp = find_node(ast, "comparison")
        assert.is_not_nil(cmp, "expected 'comparison' node")
    end)

    it("column_ref node present in column list", function()
        local ast = sql_parser.parse("SELECT id FROM users")
        local cr = find_node(ast, "column_ref")
        assert.is_not_nil(cr, "expected 'column_ref' node")
    end)

    it("or_expr present in nested WHERE condition", function()
        local ast = sql_parser.parse(
            "SELECT * FROM t WHERE a > 1 OR b < 2"
        )
        local oe = find_node(ast, "or_expr")
        assert.is_not_nil(oe, "expected 'or_expr' node")
    end)

    it("and_expr present in AND WHERE condition", function()
        local ast = sql_parser.parse(
            "SELECT * FROM t WHERE a > 1 AND b < 2"
        )
        local ae = find_node(ast, "and_expr")
        assert.is_not_nil(ae, "expected 'and_expr' node")
    end)

    it("additive node present in arithmetic expression", function()
        local ast = sql_parser.parse("SELECT a + b FROM t")
        local add = find_node(ast, "additive")
        assert.is_not_nil(add, "expected 'additive' node")
    end)

    it("function_call node for COUNT(*)", function()
        local ast = sql_parser.parse(
            "SELECT COUNT(*) FROM employees"
        )
        local fc = find_node(ast, "function_call")
        assert.is_not_nil(fc, "expected 'function_call' node")
    end)
end)

-- =========================================================================
-- Semicolon-separated multiple statements
-- =========================================================================

describe("multiple statements", function()
    it("parses two statements separated by semicolon", function()
        local ast = sql_parser.parse(
            "SELECT * FROM a; SELECT * FROM b"
        )
        assert.are.equal("program", ast.rule_name)
        local count = count_nodes(ast, "select_stmt")
        assert.are.equal(2, count)
    end)

    it("parses statement with trailing semicolon", function()
        local ast = sql_parser.parse("SELECT 1 FROM t;")
        assert.are.equal("program", ast.rule_name)
        local sel = find_node(ast, "select_stmt")
        assert.is_not_nil(sel)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = sql_parser.create_parser("SELECT * FROM t")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = sql_parser.create_parser("SELECT * FROM t")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src = "SELECT * FROM users"
        local ast1 = sql_parser.parse(src)
        local p    = sql_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for completely invalid SQL", function()
        assert.has_error(function()
            sql_parser.parse("@@@ GARBAGE @@@")
        end)
    end)

    it("raises an error for incomplete SELECT (missing FROM)", function()
        assert.has_error(function()
            sql_parser.parse("SELECT name")
        end)
    end)

    it("raises an error for empty input", function()
        assert.has_error(function()
            sql_parser.parse("")
        end)
    end)
end)
