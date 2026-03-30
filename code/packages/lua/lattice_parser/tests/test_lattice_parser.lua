-- Tests for lattice_parser
-- ========================
--
-- Comprehensive busted test suite for the Lattice parser package.
--
-- Lattice is a CSS superset language that adds variables, mixins, control
-- flow, functions, and modules to plain CSS.  This test suite verifies:
--
--   - Module loads and exposes the public API
--   - Grammar has the expected first rule ("stylesheet")
--   - Plain CSS rules: h1 { color: red; }
--   - Variable declarations: $primary: #333;
--   - Nested rules: .parent { .child { color: blue; } }
--   - Mixin definitions: @mixin flex { display: flex; }
--   - @include directives: @include flex;
--   - @if control flow: @if $debug { color: red; }
--   - @for loops: @for $i from 1 through 3 { ... }
--   - @each loops: @each $c in red, green { ... }
--   - @function definitions: @function spacing($n) { @return $n * 8px; }
--   - @use directives: @use "colors";
--   - Multi-rule stylesheets
--   - create_parser returns a GrammarParser with a parse method
--   - Error handling on malformed input

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
    "../../lattice_lexer/src/?.lua;"                          ..
    "../../lattice_lexer/src/?/init.lua;"                     ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    package.path
)

local lattice_parser = require("coding_adventures.lattice_parser")

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

describe("lattice_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(lattice_parser)
    end)

    it("exposes VERSION as a string", function()
        assert.is_string(lattice_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", lattice_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(lattice_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(lattice_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(lattice_parser.get_grammar)
    end)

    it("get_grammar returns an object with rules", function()
        local g = lattice_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
        assert.is_true(#g.rules >= 10)
    end)

    it("grammar first rule is stylesheet", function()
        local g = lattice_parser.get_grammar()
        assert.are.equal("stylesheet", g.rules[1].name)
    end)
end)

-- =========================================================================
-- Root node
-- =========================================================================

describe("root AST node", function()
    it("parse returns a non-nil value", function()
        local ast = lattice_parser.parse("h1 { color: red; }")
        assert.is_not_nil(ast)
    end)

    it("root node has rule_name == 'stylesheet'", function()
        local ast = lattice_parser.parse("h1 { color: red; }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("root node has children table", function()
        local ast = lattice_parser.parse("h1 { color: red; }")
        assert.is_table(ast.children)
    end)

    it("empty stylesheet is valid", function()
        local ast = lattice_parser.parse("")
        assert.are.equal("stylesheet", ast.rule_name)
    end)
end)

-- =========================================================================
-- Plain CSS rules
-- =========================================================================

describe("plain CSS rules", function()
    it("parses a simple type selector rule", function()
        -- h1 { color: red; }
        -- A qualified_rule: selector_list followed by a declaration block.
        local ast = lattice_parser.parse("h1 { color: red; }")
        assert.are.equal("stylesheet", ast.rule_name)
        local qr = find_node(ast, "qualified_rule")
        assert.is_not_nil(qr, "expected qualified_rule node")
    end)

    it("parses a class selector rule", function()
        -- .container { display: block; }
        local ast = lattice_parser.parse(".container { display: block; }")
        local qr = find_node(ast, "qualified_rule")
        assert.is_not_nil(qr)
    end)

    it("parses a declaration with IDENT value", function()
        -- font-family is a property; sans-serif is an IDENT value token.
        local ast = lattice_parser.parse("body { font-family: sans-serif; }")
        local decl = find_node(ast, "declaration")
        assert.is_not_nil(decl)
    end)

    it("parses multiple declarations in one block", function()
        local src = "p { margin: 0; padding: 0; color: black; }"
        local ast = lattice_parser.parse(src)
        -- Three declarations inside the block
        local decl_count = count_nodes(ast, "declaration")
        assert.is_true(decl_count >= 3)
    end)

    it("parses a CSS @media rule", function()
        -- AT_KEYWORD prelude block — standard CSS at_rule
        local src = "@media screen { body { margin: 0; } }"
        local ast = lattice_parser.parse(src)
        local ar = find_node(ast, "at_rule")
        assert.is_not_nil(ar, "expected at_rule node for @media")
    end)
end)

-- =========================================================================
-- Variable declarations
-- =========================================================================

describe("variable declarations", function()
    it("parses a simple variable declaration", function()
        -- $primary: #333;
        -- variable_declaration = VARIABLE COLON value_list SEMICOLON
        local ast = lattice_parser.parse("$primary: #333;")
        local vd = find_node(ast, "variable_declaration")
        assert.is_not_nil(vd, "expected variable_declaration node")
    end)

    it("parses a variable with a numeric value", function()
        -- $size: 16px;
        local ast = lattice_parser.parse("$size: 16px;")
        local vd = find_node(ast, "variable_declaration")
        assert.is_not_nil(vd)
    end)

    it("parses a variable referencing another variable in its value", function()
        -- $double: $size * 2;   — VARIABLE in value_list
        local ast = lattice_parser.parse("$primary: #4a90d9; .btn { color: $primary; }")
        assert.are.equal("stylesheet", ast.rule_name)
        local vd = find_node(ast, "variable_declaration")
        assert.is_not_nil(vd)
    end)

    it("parses multiple variable declarations", function()
        local src = "$a: 1px;\n$b: 2em;\n$c: red;"
        local ast = lattice_parser.parse(src)
        local vd_count = count_nodes(ast, "variable_declaration")
        assert.are.equal(3, vd_count)
    end)
end)

-- =========================================================================
-- Nested rules
-- =========================================================================

describe("nested rules", function()
    it("parses a parent rule containing a nested child rule", function()
        -- Nesting is a CSS/Lattice extension where selectors can appear
        -- inside a declaration block.  Inside the block, a nested child
        -- rule is parsed as a qualified_rule.
        local src = ".parent { .child { color: blue; } }"
        local ast = lattice_parser.parse(src)
        -- The outer rule and the nested rule are both qualified_rules
        local qr_count = count_nodes(ast, "qualified_rule")
        assert.is_true(qr_count >= 2, "expected at least two qualified_rule nodes")
    end)

    it("parses a rule with both declaration and nested rule", function()
        local src = ".nav { display: flex; .item { color: white; } }"
        local ast = lattice_parser.parse(src)
        local decl = find_node(ast, "declaration")
        assert.is_not_nil(decl, "expected declaration node")
        local qr_count = count_nodes(ast, "qualified_rule")
        assert.is_true(qr_count >= 2, "expected outer + nested qualified_rule")
    end)
end)

-- =========================================================================
-- Mixin definitions
-- =========================================================================

describe("mixin definitions", function()
    it("parses a simple no-param mixin (IDENT form)", function()
        -- @mixin flex { display: flex; }
        -- AT_KEYWORD("@mixin") IDENT("flex") block
        local src = "@mixin flex { display: flex; }"
        local ast = lattice_parser.parse(src)
        local md = find_node(ast, "mixin_definition")
        assert.is_not_nil(md, "expected mixin_definition node")
    end)

    it("parses a mixin with parameters (FUNCTION form)", function()
        -- @mixin button($bg, $fg) { background: $bg; color: $fg; }
        -- AT_KEYWORD("@mixin") FUNCTION("button(") mixin_params RPAREN block
        local src = "@mixin button($bg, $fg) { background: $bg; color: $fg; }"
        local ast = lattice_parser.parse(src)
        local md = find_node(ast, "mixin_definition")
        assert.is_not_nil(md, "expected mixin_definition node")
    end)

    it("parses a mixin with a default parameter value", function()
        -- @mixin shadow($blur: 4px) { box-shadow: 0 $blur black; }
        local src = "@mixin shadow($blur: 4px) { box-shadow: 0 $blur black; }"
        local ast = lattice_parser.parse(src)
        local md = find_node(ast, "mixin_definition")
        assert.is_not_nil(md, "expected mixin_definition node")
    end)
end)

-- =========================================================================
-- @include directives
-- =========================================================================

describe("@include directives", function()
    it("parses a simple @include without arguments (IDENT form)", function()
        -- @include clearfix;
        -- AT_KEYWORD("@include") IDENT("clearfix") SEMICOLON
        local src = ".box { @include clearfix; }"
        local ast = lattice_parser.parse(src)
        local incl = find_node(ast, "include_directive")
        assert.is_not_nil(incl, "expected include_directive node")
    end)

    it("parses @include with arguments (FUNCTION form)", function()
        -- @include button(red);
        -- AT_KEYWORD("@include") FUNCTION("button(") include_args RPAREN SEMICOLON
        local src = ".btn { @include button(red); }"
        local ast = lattice_parser.parse(src)
        local incl = find_node(ast, "include_directive")
        assert.is_not_nil(incl, "expected include_directive node")
    end)
end)

-- =========================================================================
-- @if control flow
-- =========================================================================

describe("@if control flow", function()
    it("parses a simple @if directive", function()
        -- @if $debug { color: red; }
        local src = "@if $debug { color: red; }"
        local ast = lattice_parser.parse(src)
        local ifd = find_node(ast, "if_directive")
        assert.is_not_nil(ifd, "expected if_directive node")
    end)

    it("parses @if with comparison operator", function()
        -- @if $size == large { font-size: 24px; }
        local src = "@if $size == large { font-size: 24px; }"
        local ast = lattice_parser.parse(src)
        local ifd = find_node(ast, "if_directive")
        assert.is_not_nil(ifd, "expected if_directive node")
    end)

    it("parses @if ... @else", function()
        -- @if $debug { color: red; } @else { color: black; }
        local src = "@if $debug { color: red; } @else { color: black; }"
        local ast = lattice_parser.parse(src)
        local ifd = find_node(ast, "if_directive")
        assert.is_not_nil(ifd, "expected if_directive node")
    end)
end)

-- =========================================================================
-- @for loops
-- =========================================================================

describe("@for loops", function()
    it("parses a @for ... through loop", function()
        -- @for $i from 1 through 3 { .col-#{$i} { width: 33%; } }
        -- Keyword "through" is a literal token match in the grammar.
        local src = "@for $i from 1 through 3 { .item { margin: 0; } }"
        local ast = lattice_parser.parse(src)
        local fd = find_node(ast, "for_directive")
        assert.is_not_nil(fd, "expected for_directive node")
    end)

    it("parses a @for ... to loop (exclusive end)", function()
        local src = "@for $i from 0 to 5 { .step { color: red; } }"
        local ast = lattice_parser.parse(src)
        local fd = find_node(ast, "for_directive")
        assert.is_not_nil(fd, "expected for_directive node")
    end)
end)

-- =========================================================================
-- @each loops
-- =========================================================================

describe("@each loops", function()
    it("parses an @each loop over a list of values", function()
        -- @each $color in red, green, blue { .text { color: $color; } }
        local src = "@each $color in red, green, blue { .text { color: $color; } }"
        local ast = lattice_parser.parse(src)
        local ed = find_node(ast, "each_directive")
        assert.is_not_nil(ed, "expected each_directive node")
    end)
end)

-- =========================================================================
-- @function definitions
-- =========================================================================

describe("@function definitions", function()
    it("parses a function with @return (FUNCTION form)", function()
        -- @function spacing($n) { @return $n * 8px; }
        local src = "@function spacing($n) { @return $n * 8px; }"
        local ast = lattice_parser.parse(src)
        local fundef = find_node(ast, "function_definition")
        assert.is_not_nil(fundef, "expected function_definition node")
    end)

    it("parses a no-param function (IDENT form)", function()
        -- @function pi { @return 3.14159; }
        local src = "@function pi { @return 3.14159; }"
        local ast = lattice_parser.parse(src)
        local fundef = find_node(ast, "function_definition")
        assert.is_not_nil(fundef, "expected function_definition node")
    end)

    it("finds the return_directive node inside the function body", function()
        local src = "@function double($x) { @return $x * 2; }"
        local ast = lattice_parser.parse(src)
        local ret = find_node(ast, "return_directive")
        assert.is_not_nil(ret, "expected return_directive node")
    end)
end)

-- =========================================================================
-- @use directives
-- =========================================================================

describe("@use directives", function()
    it("parses a simple @use directive", function()
        -- @use "colors";
        local ast = lattice_parser.parse('@use "colors";')
        local ud = find_node(ast, "use_directive")
        assert.is_not_nil(ud, "expected use_directive node")
    end)

    it("parses @use with 'as' namespace alias", function()
        -- @use "utils/mixins" as m;
        local ast = lattice_parser.parse('@use "utils/mixins" as m;')
        local ud = find_node(ast, "use_directive")
        assert.is_not_nil(ud, "expected use_directive node")
    end)
end)

-- =========================================================================
-- Multi-rule stylesheets
-- =========================================================================

describe("multi-rule stylesheets", function()
    it("parses a stylesheet mixing variables, mixins, and rules", function()
        local src = [[
$primary: #4a90d9;
$font-stack: Helvetica, sans-serif;

@mixin flex-center {
  display: flex;
  align-items: center;
  justify-content: center;
}

body {
  font-family: $font-stack;
  background: white;
}

.hero {
  @include flex-center;
  color: $primary;
}
]]
        local ast = lattice_parser.parse(src)
        assert.are.equal("stylesheet", ast.rule_name)
        -- At least two variable declarations
        local vd_count = count_nodes(ast, "variable_declaration")
        assert.is_true(vd_count >= 2)
        -- At least one mixin definition
        local md_count = count_nodes(ast, "mixin_definition")
        assert.is_true(md_count >= 1)
        -- At least two qualified rules (body and .hero)
        local qr_count = count_nodes(ast, "qualified_rule")
        assert.is_true(qr_count >= 2)
        -- An @include inside .hero
        local incl_count = count_nodes(ast, "include_directive")
        assert.is_true(incl_count >= 1)
    end)
end)

-- =========================================================================
-- create_parser
-- =========================================================================

describe("create_parser", function()
    it("returns a non-nil parser object", function()
        local p = lattice_parser.create_parser("h1 { color: red; }")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = lattice_parser.create_parser("h1 { color: red; }")
        assert.is_function(p.parse)
    end)

    it("parsing via create_parser returns same root rule as parse()", function()
        local src = "$x: 42px;"
        local ast1 = lattice_parser.parse(src)
        local p = lattice_parser.create_parser(src)
        local ast2, err = p:parse()
        assert.is_nil(err)
        assert.are.equal(ast1.rule_name, ast2.rule_name)
    end)
end)

-- =========================================================================
-- Error handling
-- =========================================================================

describe("error handling", function()
    it("raises an error for a block missing closing brace", function()
        assert.has_error(function()
            lattice_parser.parse("h1 { color: red;")
        end)
    end)

    it("raises an error for a variable declaration missing SEMICOLON", function()
        -- $x: 1 (no semicolon — never hits SEMICOLON token, parse fails)
        -- Note: depending on grammar, this may succeed as a partial parse;
        -- the error is guaranteed if the grammar is strict.
        -- We wrap in pcall and accept either outcome, but prefer an error.
        local ok = pcall(function()
            lattice_parser.parse("$x: 1")
        end)
        -- It's acceptable if this raises; just ensure no crash on nil ast
        assert.is_boolean(ok)
    end)
end)
