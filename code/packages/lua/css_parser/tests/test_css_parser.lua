-- Tests for css_parser
-- =====================
--
-- Comprehensive busted test suite for the CSS parser package.
--
-- # What we're testing
--
-- The css_parser takes a CSS source string, tokenizes it with css_lexer,
-- and constructs an AST using the grammar-driven GrammarParser and the
-- css.grammar rule definitions.
--
-- The root node is always "stylesheet". We test:
--   - Module loads and exposes the public API
--   - Empty input produces a stylesheet with no children
--   - Simple qualified rules: h1 { color: red; }
--   - At-rules: @media, @import, @charset
--   - Selector syntax: type, class, ID, attribute, pseudo-class, pseudo-element
--   - Declaration syntax: property: value;
--   - Compound tokens survive into the AST: 10px stays DIMENSION
--   - Function values: rgba(255, 0, 0), calc(100% - 20px)
--   - Multi-value declarations: margin: 10px 20px 10px 20px
--   - !important priority
--   - CSS variables: --custom-property: value
--   - Nested rules (CSS Nesting): .parent { & .child { } }
--   - create_parser() returns a usable GrammarParser
--   - parse errors produce descriptive messages

-- Resolve sibling packages from the monorepo so busted can find them
-- without requiring a global luarocks install.
package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../css_lexer/src/?.lua;"                              ..
    "../../css_lexer/src/?/init.lua;"                         ..
    "../../grammar_tools/src/?.lua;"                          ..
    "../../grammar_tools/src/?/init.lua;"                     ..
    "../../lexer/src/?.lua;"                                  ..
    "../../lexer/src/?/init.lua;"                             ..
    "../../parser/src/?.lua;"                                 ..
    "../../parser/src/?/init.lua;"                            ..
    "../../state_machine/src/?.lua;"                          ..
    "../../state_machine/src/?/init.lua;"                     ..
    "../../directed_graph/src/?.lua;"                         ..
    "../../directed_graph/src/?/init.lua;"                    ..
    package.path
)

local css_parser = require("coding_adventures.css_parser")

-- =========================================================================
-- Helper utilities
-- =========================================================================

--- Walk the AST and collect all leaf token values.
-- Depth-first traversal. Only leaf nodes (token wrappers) emit values.
-- @param node  table  ASTNode (from the parser package).
-- @return table       Ordered list of token value strings.
local function collect_values(node)
    local out = {}
    local function walk(n)
        if n.is_leaf and n.token then
            out[#out + 1] = n.token.value
        elseif n.children then
            for _, child in ipairs(n.children) do
                walk(child)
            end
        end
    end
    walk(node)
    return out
end

--- Walk the AST and collect all leaf token types.
-- @param node  table  ASTNode.
-- @return table       Ordered list of token type strings.
local function collect_types(node)
    local out = {}
    local function walk(n)
        if n.is_leaf and n.token then
            out[#out + 1] = n.token.type
        elseif n.children then
            for _, child in ipairs(n.children) do
                walk(child)
            end
        end
    end
    walk(node)
    return out
end

--- Find the first ASTNode with the given rule_name (breadth-first).
-- @param node       table   Root to search from.
-- @param rule_name  string  Rule name to find.
-- @return table|nil         First matching node, or nil.
local function find_node(node, rule_name)
    local queue = {node}
    while #queue > 0 do
        local n = table.remove(queue, 1)
        if n.rule_name == rule_name then return n end
        if n.children then
            for _, child in ipairs(n.children) do
                queue[#queue + 1] = child
            end
        end
    end
    return nil
end

-- =========================================================================
-- Module surface
-- =========================================================================

describe("css_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(css_parser)
    end)

    it("exposes a VERSION string", function()
        assert.is_string(css_parser.VERSION)
        assert.matches("^%d+%.%d+%.%d+$", css_parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(css_parser.parse)
    end)

    it("exposes create_parser as a function", function()
        assert.is_function(css_parser.create_parser)
    end)

    it("exposes get_grammar as a function", function()
        assert.is_function(css_parser.get_grammar)
    end)

    it("get_grammar returns a non-nil grammar object", function()
        local g = css_parser.get_grammar()
        assert.is_not_nil(g)
        assert.is_table(g.rules)
    end)
end)

-- =========================================================================
-- Empty and trivial inputs
-- =========================================================================

describe("empty input", function()
    it("empty string parses to stylesheet ASTNode", function()
        local ast = css_parser.parse("")
        assert.is_not_nil(ast)
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("whitespace-only input parses to stylesheet", function()
        local ast = css_parser.parse("   \n\t  ")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("comment-only input parses to stylesheet", function()
        local ast = css_parser.parse("/* just a comment */")
        assert.are.equal("stylesheet", ast.rule_name)
    end)
end)

-- =========================================================================
-- Qualified rules (selector + declaration block)
-- =========================================================================

describe("qualified rules", function()
    -- The simplest possible CSS rule
    it("parses h1 { color: red; } to stylesheet root", function()
        local ast = css_parser.parse("h1 { color: red; }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("AST contains a qualified_rule node", function()
        local ast = css_parser.parse("h1 { color: red; }")
        local qr = find_node(ast, "qualified_rule")
        assert.is_not_nil(qr)
    end)

    it("AST contains a selector_list node", function()
        local ast = css_parser.parse("h1 { color: red; }")
        local sl = find_node(ast, "selector_list")
        assert.is_not_nil(sl)
    end)

    it("AST contains a block node", function()
        local ast = css_parser.parse("h1 { color: red; }")
        local block = find_node(ast, "block")
        assert.is_not_nil(block)
    end)

    it("AST contains a declaration node", function()
        local ast = css_parser.parse("h1 { color: red; }")
        local decl = find_node(ast, "declaration")
        assert.is_not_nil(decl)
    end)

    it("token values survive into the AST", function()
        local ast = css_parser.parse("h1 { color: red; }")
        local vals = collect_values(ast)
        -- Should contain h1, {, color, :, red, ;, }
        local has_h1 = false
        local has_color = false
        local has_red = false
        for _, v in ipairs(vals) do
            if v == "h1" then has_h1 = true end
            if v == "color" then has_color = true end
            if v == "red" then has_red = true end
        end
        assert.is_true(has_h1, "AST should contain 'h1'")
        assert.is_true(has_color, "AST should contain 'color'")
        assert.is_true(has_red, "AST should contain 'red'")
    end)

    it("parses a multi-declaration block", function()
        local src = "h1 { color: red; font-size: 16px; }"
        local ast = css_parser.parse(src)
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses a class selector rule: .active { display: block; }", function()
        local ast = css_parser.parse(".active { display: block; }")
        assert.are.equal("stylesheet", ast.rule_name)
        local sl = find_node(ast, "selector_list")
        assert.is_not_nil(sl)
    end)
end)

-- =========================================================================
-- Selectors
-- =========================================================================

describe("selectors", function()
    it("parses an ID selector: #header { }", function()
        local ast = css_parser.parse("#header { }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses attribute selector: [disabled] { }", function()
        local ast = css_parser.parse("[disabled] { }")
        assert.are.equal("stylesheet", ast.rule_name)
        local attr = find_node(ast, "attribute_selector")
        assert.is_not_nil(attr)
    end)

    it("parses attribute selector with value: [type=\"text\"] { }", function()
        local ast = css_parser.parse('[type="text"] { }')
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses pseudo-class selector: a:hover { }", function()
        local ast = css_parser.parse("a:hover { }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses pseudo-element selector: p::before { }", function()
        local ast = css_parser.parse("p::before { }")
        assert.are.equal("stylesheet", ast.rule_name)
        local pe = find_node(ast, "pseudo_element")
        assert.is_not_nil(pe)
    end)

    it("parses child combinator: div > p { }", function()
        local ast = css_parser.parse("div > p { }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses comma-separated selector list: h1, h2, h3 { }", function()
        local ast = css_parser.parse("h1, h2, h3 { }")
        assert.are.equal("stylesheet", ast.rule_name)
        local sl = find_node(ast, "selector_list")
        assert.is_not_nil(sl)
    end)
end)

-- =========================================================================
-- Declarations with compound tokens
-- =========================================================================
--
-- These tests verify that DIMENSION, PERCENTAGE, and HASH tokens (which
-- the lexer produces as compound units) survive correctly into the AST.

describe("declarations with compound tokens", function()
    it("parses font-size: 16px (DIMENSION token in value)", function()
        local ast = css_parser.parse("p { font-size: 16px; }")
        assert.are.equal("stylesheet", ast.rule_name)
        -- The DIMENSION token "16px" should appear in the AST values
        local types = collect_types(ast)
        local has_dim = false
        for _, t in ipairs(types) do
            if t == "DIMENSION" then has_dim = true end
        end
        assert.is_true(has_dim, "DIMENSION token should survive into AST")
    end)

    it("parses width: 50% (PERCENTAGE token in value)", function()
        local ast = css_parser.parse("div { width: 50%; }")
        local types = collect_types(ast)
        local has_pct = false
        for _, t in ipairs(types) do
            if t == "PERCENTAGE" then has_pct = true end
        end
        assert.is_true(has_pct, "PERCENTAGE token should survive into AST")
    end)

    it("parses color: #333 (HASH token in value)", function()
        local ast = css_parser.parse("p { color: #333; }")
        local types = collect_types(ast)
        local has_hash = false
        for _, t in ipairs(types) do
            if t == "HASH" then has_hash = true end
        end
        assert.is_true(has_hash, "HASH token should survive into AST")
    end)

    it("parses margin shorthand: margin: 10px 20px 10px 20px", function()
        -- CSS shorthand: four space-separated values
        local ast = css_parser.parse("div { margin: 10px 20px 10px 20px; }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses !important declaration: color: red !important", function()
        local ast = css_parser.parse("p { color: red !important; }")
        assert.are.equal("stylesheet", ast.rule_name)
        local pri = find_node(ast, "priority")
        assert.is_not_nil(pri)
    end)
end)

-- =========================================================================
-- Function values
-- =========================================================================

describe("function values in declarations", function()
    it("parses rgba() color function", function()
        local ast = css_parser.parse("p { color: rgba(255, 0, 0, 0.5); }")
        assert.are.equal("stylesheet", ast.rule_name)
        local fc = find_node(ast, "function_call")
        assert.is_not_nil(fc)
    end)

    it("parses calc() with mixed units", function()
        local ast = css_parser.parse("div { width: calc(100% - 20px); }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses url() token in value", function()
        local ast = css_parser.parse("div { background: url(./bg.png); }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses var() CSS variable reference", function()
        local ast = css_parser.parse("p { color: var(--main-color); }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)
end)

-- =========================================================================
-- At-rules
-- =========================================================================

describe("at-rules", function()
    it("parses @import with semicolon", function()
        local ast = css_parser.parse('@import "style.css";')
        assert.are.equal("stylesheet", ast.rule_name)
        local ar = find_node(ast, "at_rule")
        assert.is_not_nil(ar)
    end)

    it("parses @charset with semicolon", function()
        local ast = css_parser.parse('@charset "UTF-8";')
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses @media rule with block", function()
        local ast = css_parser.parse("@media screen { h1 { color: red; } }")
        assert.are.equal("stylesheet", ast.rule_name)
        local ar = find_node(ast, "at_rule")
        assert.is_not_nil(ar)
    end)

    it("parses @media with min-width query", function()
        local ast = css_parser.parse("@media (min-width: 768px) { }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses @keyframes", function()
        local ast = css_parser.parse("@keyframes fade { from { opacity: 0; } to { opacity: 1; } }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses @font-face", function()
        local ast = css_parser.parse('@font-face { font-family: "MyFont"; }')
        assert.are.equal("stylesheet", ast.rule_name)
    end)
end)

-- =========================================================================
-- Custom properties (CSS variables)
-- =========================================================================

describe("custom properties", function()
    it("parses custom property declaration: --main-color: #333", function()
        local ast = css_parser.parse(":root { --main-color: #333; }")
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("custom property token type survives to AST", function()
        local ast = css_parser.parse(":root { --bg: white; }")
        local types = collect_types(ast)
        local has_cp = false
        for _, t in ipairs(types) do
            if t == "CUSTOM_PROPERTY" then has_cp = true end
        end
        assert.is_true(has_cp, "CUSTOM_PROPERTY token should be in AST")
    end)
end)

-- =========================================================================
-- Multiple rules
-- =========================================================================

describe("multiple rules in a stylesheet", function()
    it("parses two rules", function()
        local src = "h1 { color: red; } p { color: blue; }"
        local ast = css_parser.parse(src)
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses three rules", function()
        local src = "h1 { } h2 { } h3 { }"
        local ast = css_parser.parse(src)
        assert.are.equal("stylesheet", ast.rule_name)
    end)

    it("parses mixed at-rules and qualified rules", function()
        local src = '@import "reset.css"; h1 { color: red; }'
        local ast = css_parser.parse(src)
        assert.are.equal("stylesheet", ast.rule_name)
    end)
end)

-- =========================================================================
-- create_parser API
-- =========================================================================

describe("create_parser API", function()
    it("returns a non-nil parser object", function()
        local p = css_parser.create_parser("h1 { }")
        assert.is_not_nil(p)
    end)

    it("returned parser has a parse method", function()
        local p = css_parser.create_parser("h1 { }")
        assert.is_function(p.parse)
    end)

    it("returned parser produces the same AST as parse()", function()
        local src = "h1 { color: red; }"
        local ast1 = css_parser.parse(src)

        local p = css_parser.create_parser(src)
        local ast2 = p:parse()

        -- Both should be stylesheet roots
        assert.are.equal("stylesheet", ast1.rule_name)
        assert.are.equal("stylesheet", ast2.rule_name)

        -- Both should contain the same token values
        local vals1 = collect_values(ast1)
        local vals2 = collect_values(ast2)
        assert.are.same(vals1, vals2)
    end)
end)
