-- Tests for starlark_parser
-- ==========================
--
-- Comprehensive busted test suite for the Starlark parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - get_grammar returns grammar with "file" as first rule
--   - create_parser returns a GrammarParser with a parse method
--   - Simple assignments: x = 1
--   - Function calls: print("hello")
--   - Function definitions: def foo(x): return x + 1
--   - List literals: [1, 2, 3]
--   - Dict literals: {"key": "value"}
--   - If/else statements
--   - For loops
--   - BUILD file patterns: cc_library(name="foo", srcs=["foo.cc"])
--   - Pass statements
--   - Augmented assignments: x += 1
--   - Lambda expressions
--   - Ternary (if-else) expressions

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
    "../../starlark_lexer/src/?.lua;"                            ..
    "../../starlark_lexer/src/?/init.lua;"                       ..
    "../../parser/src/?.lua;"                                    ..
    "../../parser/src/?/init.lua;"                               ..
    package.path
)

local starlark_parser = require("coding_adventures.starlark_parser")

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

describe("starlark_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(starlark_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(starlark_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", starlark_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(starlark_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(starlark_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(starlark_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = starlark_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- starlark.grammar has many rules: file, statement, simple_stmt, …
        assert.is_true(#g.rules >= 10)
    end)

    it("grammar first rule is file", function()
        local g = starlark_parser.get_grammar()
        assert.are.equal("file", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = starlark_parser.parse("x = 1\n")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'file'", function()
        local ast = starlark_parser.parse("x = 1\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = starlark_parser.parse("x = 1\n")
        assert.is_table(ast.children)
    end)

    it("file contains a statement node", function()
        local ast = starlark_parser.parse("x = 1\n")
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)
end)

-- =========================================================================
-- Simple assignments
-- =========================================================================
--
-- Starlark assignments look like Python: target = expression
-- The LHS can be a name, attribute, or subscript. The grammar rule is
-- assign_stmt = expression_list [ ( assign_op | augmented_assign_op ) expression_list ]

describe("simple assignments", function()
    it("parses x = 1", function()
        local ast = starlark_parser.parse("x = 1\n")
        assert.are.equal("file", ast.rule_name)
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign, "expected 'assign_stmt' node")
    end)

    it("parses x = 'hello'", function()
        local ast = starlark_parser.parse("x = 'hello'\n")
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign)
    end)

    it("parses x = True", function()
        local ast = starlark_parser.parse("x = True\n")
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign)
    end)

    it("parses multiple assignments", function()
        local ast = starlark_parser.parse("x = 1\ny = 2\n")
        local count = count_nodes(ast, "assign_stmt")
        assert.is_true(count >= 2, "expected at least 2 assign_stmt nodes")
    end)

    it("parses augmented assignment x += 1", function()
        local ast = starlark_parser.parse("x += 1\n")
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign, "expected 'assign_stmt' for augmented assignment")
    end)

    it("parses tuple unpacking a, b = 1, 2", function()
        local ast = starlark_parser.parse("a, b = 1, 2\n")
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign, "expected 'assign_stmt' for tuple unpacking")
    end)
end)

-- =========================================================================
-- Function calls
-- =========================================================================
--
-- In Starlark, function calls are expression statements.
-- BUILD file rules like cc_library(name="foo") are just function calls.
-- The grammar parses these as assign_stmt with expression_list.

describe("function calls", function()
    it("parses print('hello')", function()
        local ast = starlark_parser.parse("print('hello')\n")
        assert.are.equal("file", ast.rule_name)
        local stmt = find_node(ast, "statement")
        assert.is_not_nil(stmt, "expected 'statement' node")
    end)

    it("parses f(x, y)", function()
        local ast = starlark_parser.parse("f(x, y)\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses BUILD-style cc_library call", function()
        -- This is the primary use-case for Starlark: BUILD files.
        -- cc_library(name="foo", srcs=["foo.cc"]) is a top-level function call.
        local ast = starlark_parser.parse('cc_library(name="foo", srcs=["foo.cc"])\n')
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses load statement", function()
        -- load() is a special built-in for importing symbols from other .star files.
        -- load("//rules/python.bzl", "py_library")
        local ast = starlark_parser.parse('load("//rules/python.bzl", "py_library")\n')
        assert.are.equal("file", ast.rule_name)
        local load_stmt = find_node(ast, "load_stmt")
        assert.is_not_nil(load_stmt, "expected 'load_stmt' node")
    end)
end)

-- =========================================================================
-- Function definitions
-- =========================================================================
--
-- Starlark functions look like Python functions.
-- Key constraints:
--   - Recursion is prohibited (checked at runtime, not parse time)
--   - No nested def that closes over mutable variables

describe("function definitions", function()
    it("parses def foo(x): return x + 1", function()
        local src = "def foo(x):\n    return x + 1\n"
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
        local def_node = find_node(ast, "def_stmt")
        assert.is_not_nil(def_node, "expected 'def_stmt' node")
    end)

    it("parses def with no parameters", function()
        local src = "def noop():\n    pass\n"
        local ast = starlark_parser.parse(src)
        local def_node = find_node(ast, "def_stmt")
        assert.is_not_nil(def_node)
    end)

    it("parses def with default parameters", function()
        -- Default values are a Python/Starlark staple:
        --   def greet(name, greeting="Hello"): ...
        local src = 'def greet(name, greeting="Hello"):\n    return greeting + name\n'
        local ast = starlark_parser.parse(src)
        local def_node = find_node(ast, "def_stmt")
        assert.is_not_nil(def_node)
    end)

    it("parses def with multiple statements in body", function()
        local src = "def add(a, b):\n    c = a + b\n    return c\n"
        local ast = starlark_parser.parse(src)
        local def_node = find_node(ast, "def_stmt")
        assert.is_not_nil(def_node)
    end)

    it("parses return statement", function()
        local src = "def f(x):\n    return x\n"
        local ast = starlark_parser.parse(src)
        local ret = find_node(ast, "return_stmt")
        assert.is_not_nil(ret, "expected 'return_stmt' node")
    end)

    it("parses pass in function body", function()
        local src = "def todo():\n    pass\n"
        local ast = starlark_parser.parse(src)
        local pass_node = find_node(ast, "pass_stmt")
        assert.is_not_nil(pass_node, "expected 'pass_stmt' node")
    end)
end)

-- =========================================================================
-- List literals
-- =========================================================================
--
-- Lists in Starlark look exactly like Python lists:
--   [1, 2, 3]           — literal list
--   [x * 2 for x in r]  — list comprehension

describe("list literals", function()
    it("parses [1, 2, 3]", function()
        local ast = starlark_parser.parse("x = [1, 2, 3]\n")
        assert.are.equal("file", ast.rule_name)
        local list = find_node(ast, "list_expr")
        assert.is_not_nil(list, "expected 'list_expr' node")
    end)

    it("parses empty list []", function()
        local ast = starlark_parser.parse("x = []\n")
        local list = find_node(ast, "list_expr")
        assert.is_not_nil(list)
    end)

    it("parses list of strings", function()
        local ast = starlark_parser.parse('srcs = ["a.cc", "b.cc"]\n')
        local list = find_node(ast, "list_expr")
        assert.is_not_nil(list)
    end)

    it("parses nested list access", function()
        local ast = starlark_parser.parse("y = x[0]\n")
        assert.are.equal("file", ast.rule_name)
    end)
end)

-- =========================================================================
-- Dict literals
-- =========================================================================
--
-- Dicts in Starlark:
--   {"key": "value"}     — literal dict
--   {k: v for k, v in …} — dict comprehension

describe("dict literals", function()
    it("parses {'key': 'value'}", function()
        local ast = starlark_parser.parse("d = {'key': 'value'}\n")
        assert.are.equal("file", ast.rule_name)
        local dict = find_node(ast, "dict_expr")
        assert.is_not_nil(dict, "expected 'dict_expr' node")
    end)

    it("parses empty dict {}", function()
        local ast = starlark_parser.parse("d = {}\n")
        local dict = find_node(ast, "dict_expr")
        assert.is_not_nil(dict)
    end)

    it("parses multi-entry dict", function()
        local ast = starlark_parser.parse("d = {'a': 1, 'b': 2, 'c': 3}\n")
        local dict = find_node(ast, "dict_expr")
        assert.is_not_nil(dict)
    end)
end)

-- =========================================================================
-- If/else statements
-- =========================================================================
--
-- Starlark if statements mirror Python:
--   if condition:
--       ...
--   elif other:
--       ...
--   else:
--       ...

describe("if/else statements", function()
    it("parses basic if", function()
        local src = "if x > 0:\n    y = 1\n"
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
        local if_node = find_node(ast, "if_stmt")
        assert.is_not_nil(if_node, "expected 'if_stmt' node")
    end)

    it("parses if/else", function()
        local src = "if x > 0:\n    y = 1\nelse:\n    y = 0\n"
        local ast = starlark_parser.parse(src)
        local if_node = find_node(ast, "if_stmt")
        assert.is_not_nil(if_node)
    end)

    it("parses if/elif/else", function()
        local src = "if score >= 90:\n    grade = 'A'\nelif score >= 80:\n    grade = 'B'\nelse:\n    grade = 'F'\n"
        local ast = starlark_parser.parse(src)
        local if_node = find_node(ast, "if_stmt")
        assert.is_not_nil(if_node, "expected 'if_stmt' node with elif chain")
    end)

    it("parses if with pass body", function()
        local src = "if True:\n    pass\n"
        local ast = starlark_parser.parse(src)
        local if_node = find_node(ast, "if_stmt")
        assert.is_not_nil(if_node)
    end)
end)

-- =========================================================================
-- For loops
-- =========================================================================
--
-- Starlark has for loops but NOT while loops. This is by design:
-- iterating over finite collections guarantees termination, making
-- BUILD file evaluation provably decidable. The grammar rule is:
--   for_stmt = "for" loop_vars "in" expression COLON suite ;

describe("for loops", function()
    it("parses for item in items:", function()
        local src = "for item in items:\n    process(item)\n"
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node, "expected 'for_stmt' node")
    end)

    it("parses for with tuple unpacking", function()
        -- Starlark supports tuple unpacking in for loops:
        --   for key, value in d.items():
        local src = "for k, v in pairs:\n    print(k, v)\n"
        local ast = starlark_parser.parse(src)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node)
    end)

    it("parses for with break and continue", function()
        local src = "for x in lst:\n    if x == 0:\n        continue\n    process(x)\n"
        local ast = starlark_parser.parse(src)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node)
    end)

    it("parses for over a range call", function()
        local src = "for i in range(10):\n    total += i\n"
        local ast = starlark_parser.parse(src)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node)
    end)
end)

-- =========================================================================
-- BUILD file patterns
-- =========================================================================
--
-- The primary real-world use of Starlark is Bazel BUILD files. These files
-- consist almost entirely of top-level function calls (BUILD rules):
--
--   cc_library(
--       name = "foo",
--       srcs = ["foo.cc", "foo.h"],
--       deps = [":bar"],
--   )
--
-- Each call is just a function call expression treated as a statement.

describe("BUILD file patterns", function()
    it("parses cc_library(name='foo', srcs=['foo.cc'])", function()
        local src = 'cc_library(name = "foo", srcs = ["foo.cc"])\n'
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses multi-line BUILD rule", function()
        local src = [[cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
    deps = [":my_lib"],
)
]]
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses multiple BUILD rules", function()
        local src = [[cc_library(name = "foo", srcs = ["foo.cc"])
cc_binary(name = "bar", deps = [":foo"])
]]
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses load + BUILD rule", function()
        local src = [[load("//rules:defs.bzl", "my_rule")
my_rule(name = "target")
]]
        local ast = starlark_parser.parse(src)
        assert.are.equal("file", ast.rule_name)
        local load_node = find_node(ast, "load_stmt")
        assert.is_not_nil(load_node, "expected 'load_stmt' node")
    end)
end)

-- =========================================================================
-- Expression features
-- =========================================================================
--
-- Starlark has a rich expression language inherited from Python.
-- Operator precedence (lowest to highest):
--   lambda > if-else > or > and > not > comparisons >
--   | > ^ > & > shifts > +/- > *//// > unary+/- > ** > primary

describe("expressions", function()
    it("parses arithmetic: a + b * c", function()
        local ast = starlark_parser.parse("x = a + b * c\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses comparison: a == b", function()
        local ast = starlark_parser.parse("x = a == b\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses boolean or: a or b", function()
        local ast = starlark_parser.parse("x = a or b\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses boolean and: a and b", function()
        local ast = starlark_parser.parse("x = a and b\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses ternary: a if cond else b", function()
        -- Starlark supports Python-style conditional expressions:
        --   value = x if x > 0 else -x
        local ast = starlark_parser.parse("x = a if cond else b\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses lambda: lambda x: x + 1", function()
        -- Lambda functions are supported in Starlark.
        -- They are single-expression anonymous functions.
        local ast = starlark_parser.parse("f = lambda x: x + 1\n")
        assert.are.equal("file", ast.rule_name)
        local lambda = find_node(ast, "lambda_expr")
        assert.is_not_nil(lambda, "expected 'lambda_expr' node")
    end)

    it("parses attribute access: obj.attr", function()
        local ast = starlark_parser.parse("x = obj.attr\n")
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses string concatenation", function()
        local ast = starlark_parser.parse('greeting = "Hello, " + name\n')
        assert.are.equal("file", ast.rule_name)
    end)

    it("parses modulo / string formatting: fmt % value", function()
        -- Starlark (like Python) uses % for string formatting and modulo.
        local ast = starlark_parser.parse('s = "Hello, %s" % name\n')
        assert.are.equal("file", ast.rule_name)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = starlark_parser.create_parser("x = 1\n")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = starlark_parser.create_parser("x = 1\n")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule_name as parse()", function()
        local src  = "x = 42\n"
        local ast1 = starlark_parser.parse(src)
        local p    = starlark_parser.create_parser(src)
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
            starlark_parser.parse("@@@ GARBAGE @@@\n")
        end)
    end)
end)
