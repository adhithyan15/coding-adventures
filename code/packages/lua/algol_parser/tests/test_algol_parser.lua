-- Tests for algol_parser
-- =======================
--
-- Comprehensive busted test suite for the ALGOL 60 parser package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - Minimal program: begin integer x; x := 42 end
--   - Assignment: simple, multi-target
--   - If/then and if/then/else
--   - For loop (step/until, while, simple forms)
--   - Nested blocks
--   - Boolean expressions
--   - Grammar inspection (rule count, first rule)
--   - create_parser API
--   - Error handling for invalid programs

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
    "../../algol_lexer/src/?.lua;"                            ..
    "../../algol_lexer/src/?/init.lua;"                       ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local algol_parser = require("coding_adventures.algol_parser")

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

describe("algol_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(algol_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(algol_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", algol_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(algol_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(algol_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(algol_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = algol_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        -- algol.grammar has many rules; at least 10 is a reasonable minimum check
        assert.is_true(#g.rules >= 10)
    end)

    it("grammar first rule is program", function()
        local g = algol_parser.get_grammar()
        assert.are.equal("program", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    local src = "begin integer x; x := 42 end"

    it("parse returns a non-nil value", function()
        local ast = algol_parser.parse(src)
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'program'", function()
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
    end)

    it("root node has children", function()
        local ast = algol_parser.parse(src)
        assert.is_table(ast.children)
        assert.is_true(#ast.children >= 1)
    end)

    it("program contains a block node", function()
        local ast = algol_parser.parse(src)
        local blk = find_node(ast, "block")
        assert.is_not_nil(blk, "expected a 'block' node inside 'program'")
    end)
end)

-- =========================================================================
-- Minimal program
-- =========================================================================

describe("minimal program", function()
    it("parses begin integer x; x := 42 end", function()
        local ast = algol_parser.parse("begin integer x; x := 42 end")
        assert.are.equal("program", ast.rule_name)
        assert.is_not_nil(find_node(ast, "block"))
    end)

    it("block contains a type_decl", function()
        local ast = algol_parser.parse("begin integer x; x := 42 end")
        assert.is_not_nil(find_node(ast, "type_decl"),
            "expected 'type_decl' node inside the block")
    end)

    it("block contains a declaration node", function()
        local ast = algol_parser.parse("begin integer x; x := 42 end")
        assert.is_not_nil(find_node(ast, "declaration"),
            "expected 'declaration' node inside the block")
    end)

    it("type_decl contains ident_list", function()
        local ast = algol_parser.parse("begin integer x; x := 42 end")
        assert.is_not_nil(find_node(ast, "ident_list"),
            "expected 'ident_list' inside 'type_decl'")
    end)
end)

-- =========================================================================
-- Assignment
-- =========================================================================

describe("assignment", function()
    it("parses simple assignment x := 0", function()
        local src = "begin integer x; x := 0 end"
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        local assign = find_node(ast, "assign_stmt")
        assert.is_not_nil(assign, "expected 'assign_stmt' node")
    end)

    it("parses assignment with arithmetic x := 1 + 2", function()
        local src = "begin integer x; x := 1 + 2 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "assign_stmt"))
    end)

    it("parses assignment with real literal x := 3.14", function()
        local src = "begin real x; x := 3.14 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "assign_stmt"))
    end)

    it("parses multiple declarations", function()
        local src = "begin integer x, y; x := 1; y := 2 end"
        local ast = algol_parser.parse(src)
        -- Should have one type_decl with two idents in ident_list
        assert.is_not_nil(find_node(ast, "type_decl"))
        -- Two assign statements
        local assign_count = count_nodes(ast, "assign_stmt")
        assert.is_true(assign_count >= 2)
    end)
end)

-- =========================================================================
-- If/then/else
-- =========================================================================

describe("if/then/else", function()
    it("parses if/then without else", function()
        local src = "begin integer x; if x > 0 then x := 1 end"
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        local cond = find_node(ast, "cond_stmt")
        assert.is_not_nil(cond, "expected 'cond_stmt' node")
    end)

    it("parses if/then/else", function()
        local src = "begin integer x; if x > 0 then x := 1 else x := 0 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "cond_stmt"))
    end)

    it("if/then/else has a relation node inside bool_expr", function()
        local src = "begin integer x; if x > 0 then x := 1 else x := 0 end"
        local ast = algol_parser.parse(src)
        -- relation is created when a relational operator appears in bool_expr
        assert.is_not_nil(find_node(ast, "relation"),
            "expected 'relation' node inside the condition")
    end)

    it("parses if/then with boolean literal condition", function()
        local src = "begin integer x; if true then x := 1 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "cond_stmt"))
    end)

    it("parses nested if with else inside else branch", function()
        -- else-branch is a 'statement' which allows another cond_stmt
        local src = "begin integer x; if x > 0 then x := 1 else if x < 0 then x := -1 else x := 0 end"
        local ast = algol_parser.parse(src)
        local cond_count = count_nodes(ast, "cond_stmt")
        assert.is_true(cond_count >= 2, "expected at least 2 cond_stmt nodes for chained if/else")
    end)
end)

-- =========================================================================
-- For loop
-- =========================================================================

describe("for loop", function()
    it("parses for loop step/until form", function()
        local src = "begin integer i; for i := 1 step 1 until 10 do i := i + 1 end"
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node, "expected 'for_stmt' node")
    end)

    it("for_stmt contains a for_list node", function()
        local src = "begin integer i; for i := 1 step 1 until 10 do i := i + 1 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "for_list"))
    end)

    it("parses for loop while form", function()
        local src = "begin integer i; for i := 1 while i <= 10 do i := i + 1 end"
        local ast = algol_parser.parse(src)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node, "expected 'for_stmt' for while-form")
    end)

    it("parses for loop simple form (single value)", function()
        local src = "begin integer i; for i := 5 do i := i + 1 end"
        local ast = algol_parser.parse(src)
        local for_node = find_node(ast, "for_stmt")
        assert.is_not_nil(for_node, "expected 'for_stmt' for simple form")
    end)

    it("for loop body is an assignment", function()
        local src = "begin integer i; for i := 1 step 1 until 5 do i := i + 1 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "assign_stmt"),
            "expected 'assign_stmt' inside for body")
    end)
end)

-- =========================================================================
-- Compound statement (begin...end with no declarations)
-- =========================================================================

describe("compound statement", function()
    it("parses compound_stmt inside if/then", function()
        local src = "begin integer x; if x > 0 then begin x := 1; x := x + 1 end end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "compound_stmt"),
            "expected 'compound_stmt' inside then-branch")
    end)
end)

-- =========================================================================
-- Boolean expressions
-- =========================================================================

describe("boolean expressions", function()
    it("parses 'and' conjunction", function()
        local src = "begin integer x, y; if x > 0 and y > 0 then x := 1 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "bool_factor"),
            "expected 'bool_factor' node for 'and'")
    end)

    it("parses 'or' disjunction", function()
        local src = "begin integer x, y; if x > 0 or y > 0 then x := 1 end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "bool_term"),
            "expected 'bool_term' node for 'or'")
    end)

    it("parses 'not' negation", function()
        local src = "begin boolean f; if not f then f := true end"
        local ast = algol_parser.parse(src)
        assert.is_not_nil(find_node(ast, "bool_secondary"),
            "expected 'bool_secondary' node for 'not'")
    end)
end)

-- =========================================================================
-- Nested blocks
-- =========================================================================

describe("nested blocks", function()
    it("parses a block nested inside a block", function()
        local src = "begin integer x; x := 1; begin integer y; y := 2 end end"
        local ast = algol_parser.parse(src)
        -- There should be at least 2 block nodes (outer and inner)
        local block_count = count_nodes(ast, "block")
        assert.is_true(block_count >= 2, "expected at least 2 block nodes")
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = algol_parser.create_parser("begin integer x; x := 0 end")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = algol_parser.create_parser("begin integer x; x := 0 end")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns the same root rule as parse()", function()
        local src = "begin integer x; x := 42 end"
        local ast1 = algol_parser.parse(src)
        local p    = algol_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for missing end", function()
        assert.has_error(function()
            algol_parser.parse("begin integer x; x := 1")
        end)
    end)

    it("raises an error for missing begin", function()
        assert.has_error(function()
            algol_parser.parse("integer x; x := 1 end")
        end)
    end)

    it("raises an error for missing semicolon after declaration", function()
        assert.has_error(function()
            algol_parser.parse("begin integer x x := 1 end")
        end)
    end)

    it("raises an error for incomplete assignment (missing expression)", function()
        assert.has_error(function()
            algol_parser.parse("begin integer x; x := end")
        end)
    end)

    it("raises an error for empty input", function()
        assert.has_error(function()
            algol_parser.parse("")
        end)
    end)

    it("raises an error for if without then", function()
        assert.has_error(function()
            algol_parser.parse("begin integer x; if x > 0 x := 1 end")
        end)
    end)
end)

-- =========================================================================
-- Realistic ALGOL 60 programs
-- =========================================================================

describe("realistic programs", function()
    it("parses a program with multiple declarations and statements", function()
        local src = [[
begin
    integer x, y, z;
    real sum;
    x := 10;
    y := 20;
    z := x + y;
    sum := z * 1.5
end]]
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.is_not_nil(find_node(ast, "block"))
        local assign_count = count_nodes(ast, "assign_stmt")
        assert.is_true(assign_count >= 4)
    end)

    it("parses a program with if/then/else and for loop", function()
        local src = [[
begin
    integer i, total;
    total := 0;
    for i := 1 step 1 until 10 do
        if i > 5 then
            total := total + i
        else
            total := total - 1
end]]
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.is_not_nil(find_node(ast, "for_stmt"))
        assert.is_not_nil(find_node(ast, "cond_stmt"))
    end)

    it("parses a program with boolean variables and not/and/or", function()
        local src = [[
begin
    boolean p, q;
    integer result;
    p := true;
    q := false;
    if p and not q then
        result := 1
    else
        result := 0
end]]
        local ast = algol_parser.parse(src)
        assert.are.equal("program", ast.rule_name)
        assert.is_not_nil(find_node(ast, "cond_stmt"))
    end)
end)
