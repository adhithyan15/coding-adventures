-- Tests for coding_adventures.lattice_ast_to_css
-- ================================================
--
-- This test suite exercises the Lattice AST → CSS compiler.
-- It parses Lattice source using the real lattice_parser, then compiles
-- the resulting AST with lattice_ast_to_css.compile(), and checks the
-- CSS output.
--
-- Test areas:
--   - Module loads and exposes compile()
--   - Plain CSS pass-through
--   - Variable declaration and expansion
--   - Nested rule flattening
--   - Mixin definition and @include expansion
--   - @if with truthy and falsy conditions
--   - @for loop (through and to)
--   - @each loop
--   - @function and function calls
--   - Multiple rules and blank-line separation

-- =========================================================================
-- Package path setup
-- =========================================================================
--
-- We resolve all sibling packages directly from the monorepo so tests run
-- without requiring a global luarocks install.

package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../lattice_parser/src/?.lua;"                         ..
    "../../lattice_parser/src/?/init.lua;"                    ..
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

local lattice_parser     = require("coding_adventures.lattice_parser")
local lattice_ast_to_css = require("coding_adventures.lattice_ast_to_css")

-- =========================================================================
-- Helper: parse + compile in one step
-- =========================================================================

--- Parse Lattice source and compile to CSS.
-- @param source string  Lattice source text
-- @return string        Compiled CSS text
local function compile(source)
    local ast = lattice_parser.parse(source)
    return lattice_ast_to_css.compile(ast)
end

--- Assert that compiling `source` produces CSS containing the `expected` substring.
-- @param source   string  Lattice source
-- @param expected string  Expected substring in output CSS
-- @param msg      string  Test description
local function assert_contains(source, expected, msg)
    local css = compile(source)
    assert.is_truthy(
        css:find(expected, 1, true),
        (msg or "compile output") .. "\n  Expected to contain: " .. expected .. "\n  Got: " .. css
    )
end

-- =========================================================================
-- 1. Module API
-- =========================================================================

describe("lattice_ast_to_css module", function()
    it("loads without error", function()
        assert.is_not_nil(lattice_ast_to_css)
    end)

    it("exposes compile()", function()
        assert.is_function(lattice_ast_to_css.compile)
    end)

    it("compile() returns a string", function()
        local ast = lattice_parser.parse("h1 { color: red; }")
        local css = lattice_ast_to_css.compile(ast)
        assert.is_string(css)
    end)
end)

-- =========================================================================
-- 2. Plain CSS pass-through
-- =========================================================================

describe("plain CSS pass-through", function()
    it("passes through a simple rule unchanged", function()
        local css = compile("h1 { color: red; }")
        assert.is_truthy(css:find("h1"))
        assert.is_truthy(css:find("color: red"))
    end)

    it("passes through multiple declarations", function()
        local css = compile("p { color: black; font-size: 16px; }")
        assert.is_truthy(css:find("color: black"))
        assert.is_truthy(css:find("font-size: 16px"))
    end)

    it("passes through a class selector", function()
        local css = compile(".card { background: white; }")
        assert.is_truthy(css:find("%.card"))
        assert.is_truthy(css:find("background: white"))
    end)

    it("returns empty string for empty input", function()
        local css = compile("")
        assert.equal("", css)
    end)

    it("output ends with a newline", function()
        local css = compile("h1 { color: red; }")
        assert.equal("\n", css:sub(-1))
    end)
end)

-- =========================================================================
-- 3. Variable declaration and expansion
-- =========================================================================

describe("variable expansion", function()
    it("expands a simple variable reference", function()
        local css = compile("$color: red; h1 { color: $color; }")
        assert.is_truthy(css:find("color: red"))
        -- The variable declaration itself should not appear in output
        assert.is_falsy(css:find("%$color"))
    end)

    it("expands a hex color variable", function()
        local css = compile("$primary: #4a90d9; a { color: $primary; }")
        assert.is_truthy(css:find("#4a90d9"))
    end)

    it("expands a dimension variable", function()
        local css = compile("$size: 16px; body { font-size: $size; }")
        assert.is_truthy(css:find("font-size: 16px"))
    end)

    it("expands variable inside a nested rule", function()
        local css = compile("$bg: white; .card { .inner { background: $bg; } }")
        assert.is_truthy(css:find("background: white"))
    end)

    it("scoped variable shadows outer variable", function()
        -- Inner variable declared inside block should override outer
        local css = compile([[
            $color: red;
            .a { $color: blue; color: $color; }
        ]])
        assert.is_truthy(css:find("color: blue"))
    end)
end)

-- =========================================================================
-- 4. Nested rule flattening
-- =========================================================================

describe("nested rule flattening", function()
    it("flattens one level of nesting", function()
        local css = compile(".parent { .child { color: blue; } }")
        assert.is_truthy(css:find("%.parent %.child"))
        assert.is_truthy(css:find("color: blue"))
        -- No nested braces in output
        assert.is_falsy(css:find("%.parent {[^}]-%.child"))
    end)

    it("flattens two levels of nesting", function()
        local css = compile(".a { .b { .c { color: green; } } }")
        assert.is_truthy(css:find("%.a %.b %.c"))
        assert.is_truthy(css:find("color: green"))
    end)

    it("handles & parent reference", function()
        local css = compile("a { &:hover { color: red; } }")
        assert.is_truthy(css:find("a:hover"))
        assert.is_truthy(css:find("color: red"))
    end)

    it("parent rule with own declarations and nested child", function()
        local css = compile(".parent { color: black; .child { color: blue; } }")
        assert.is_truthy(css:find("%.parent {"))
        assert.is_truthy(css:find("color: black"))
        assert.is_truthy(css:find("%.parent %.child"))
        assert.is_truthy(css:find("color: blue"))
    end)
end)

-- =========================================================================
-- 5. Mixin definition and @include expansion
-- =========================================================================

describe("mixin expansion", function()
    it("expands a no-parameter mixin", function()
        local css = compile([[
            @mixin flex-center {
                display: flex;
                align-items: center;
            }
            .box { @include flex-center; }
        ]])
        assert.is_truthy(css:find("display: flex"))
        assert.is_truthy(css:find("align%-items: center"))
    end)

    it("expands a mixin with a parameter", function()
        local css = compile([[
            @mixin color-box($bg) {
                background: $bg;
            }
            .red-box { @include color-box(red); }
        ]])
        assert.is_truthy(css:find("background: red"))
    end)

    it("expands a mixin with a default parameter", function()
        local css = compile([[
            @mixin button($bg, $fg: white) {
                background: $bg;
                color: $fg;
            }
            .btn { @include button(blue); }
        ]])
        assert.is_truthy(css:find("background: blue"))
        assert.is_truthy(css:find("color: white"))
    end)

    it("mixin definition does not appear in output", function()
        local css = compile([[
            @mixin flex { display: flex; }
            .box { @include flex; }
        ]])
        assert.is_falsy(css:find("@mixin"))
        assert.is_falsy(css:find("@include"))
    end)
end)

-- =========================================================================
-- 6. @if control flow
-- =========================================================================

describe("@if control flow", function()
    it("includes block when condition is true", function()
        local css = compile([[
            $debug: true;
            @if $debug {
                .debug { display: block; }
            }
        ]])
        assert.is_truthy(css:find("display: block"))
    end)

    it("excludes block when condition is false", function()
        local css = compile([[
            $debug: false;
            @if $debug {
                .debug { display: block; }
            }
        ]])
        assert.is_falsy(css:find("display: block"))
    end)

    it("@else block is used when @if is false", function()
        local css = compile([[
            $theme: light;
            h1 {
                @if $theme == dark {
                    color: white;
                } @else {
                    color: black;
                }
            }
        ]])
        assert.is_falsy(css:find("color: white"))
        assert.is_truthy(css:find("color: black"))
    end)

    it("numeric comparison: less than", function()
        local css = compile([[
            $n: 3;
            @if $n < 10 {
                .small { font-size: 12px; }
            }
        ]])
        assert.is_truthy(css:find("font-size: 12px"))
    end)

    it("numeric comparison: greater than or equal", function()
        local css = compile([[
            $n: 5;
            @if $n >= 5 {
                .big { font-size: 18px; }
            }
        ]])
        assert.is_truthy(css:find("font-size: 18px"))
    end)
end)

-- =========================================================================
-- 7. @for loop
-- =========================================================================

describe("@for loop", function()
    it("generates rules for each iteration (through)", function()
        local css = compile([[
            @for $i from 1 through 3 {
                .item-$i { order: $i; }
            }
        ]])
        -- The variable $i should be substituted in values
        -- (selector interpolation depends on parser support)
        -- At minimum verify the loop body ran 3 times with the right values
        assert.is_truthy(css:find("order: 1") or css:find("order:1"))
        assert.is_truthy(css:find("order: 2") or css:find("order:2"))
        assert.is_truthy(css:find("order: 3") or css:find("order:3"))
    end)

    it("generates rules for each iteration (to — exclusive)", function()
        local css = compile([[
            @for $i from 1 to 3 {
                .col { z-index: $i; }
            }
        ]])
        -- "to" is exclusive: 1 and 2 only, not 3
        assert.is_truthy(css:find("z%-index: 1") or css:find("z%-index:1"))
        assert.is_truthy(css:find("z%-index: 2") or css:find("z%-index:2"))
        assert.is_falsy(css:find("z%-index: 3") or css:find("z%-index:3"))
    end)
end)

-- =========================================================================
-- 8. @each loop
-- =========================================================================

describe("@each loop", function()
    it("iterates over a list of values", function()
        local css = compile([[
            @each $color in red, green, blue {
                .text { color: $color; }
            }
        ]])
        assert.is_truthy(css:find("color: red"))
        assert.is_truthy(css:find("color: green"))
        assert.is_truthy(css:find("color: blue"))
    end)
end)

-- =========================================================================
-- 9. @function and function calls
-- =========================================================================

describe("@function evaluation", function()
    it("calls a function and substitutes the return value", function()
        local css = compile([[
            @function double($n) {
                @return $n * 2;
            }
            .box { width: double(8); }
        ]])
        assert.is_truthy(css:find("width: 16") or css:find("width:16"))
    end)

    it("function with dimension argument", function()
        local css = compile([[
            @function spacing($n) {
                @return $n * 8;
            }
            .card { padding: spacing(2); }
        ]])
        -- 2 * 8 = 16
        assert.is_truthy(css:find("padding: 16") or css:find("padding:16"))
    end)
end)

-- =========================================================================
-- 10. Multi-rule stylesheets
-- =========================================================================

describe("multi-rule stylesheets", function()
    it("compiles multiple top-level rules", function()
        local css = compile("h1 { color: red; } h2 { color: blue; }")
        assert.is_truthy(css:find("h1"))
        assert.is_truthy(css:find("h2"))
        assert.is_truthy(css:find("color: red"))
        assert.is_truthy(css:find("color: blue"))
    end)

    it("variable defined before use across rules", function()
        local css = compile([[
            $brand: #ff6600;
            h1 { color: $brand; }
            a { border-color: $brand; }
        ]])
        local count = 0
        for _ in css:gmatch("#ff6600") do count = count + 1 end
        assert.is_truthy(count >= 2, "Expected $brand to be expanded twice")
    end)
end)
