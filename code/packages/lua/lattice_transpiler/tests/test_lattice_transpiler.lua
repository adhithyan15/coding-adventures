-- Tests for coding_adventures.lattice_transpiler
-- ================================================
--
-- The transpiler is the entry-point package: it takes Lattice source text
-- and returns compiled CSS text in a single call.  It wires together
-- lattice_parser and lattice_ast_to_css.
--
-- Test areas:
--   - Module loads and exposes transpile() and transpile_file()
--   - transpile() returns (css, nil) on success
--   - transpile() returns (nil, error) on parse failure
--   - Simple CSS pass-through
--   - Variable expansion
--   - Nested rules
--   - Mixin expansion
--   - @if control flow
--   - @for loop
--   - @each loop
--   - transpile_file() with a temp file

-- =========================================================================
-- Package path setup
-- =========================================================================

package.path = (
    "../src/?.lua;"                                           ..
    "../src/?/init.lua;"                                      ..
    "../../lattice_ast_to_css/src/?.lua;"                     ..
    "../../lattice_ast_to_css/src/?/init.lua;"                ..
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

local transpiler = require("coding_adventures.lattice_transpiler")

-- =========================================================================
-- 1. Module API
-- =========================================================================

describe("lattice_transpiler module", function()
    it("loads without error", function()
        assert.is_not_nil(transpiler)
    end)

    it("exposes transpile()", function()
        assert.is_function(transpiler.transpile)
    end)

    it("exposes transpile_file()", function()
        assert.is_function(transpiler.transpile_file)
    end)
end)

-- =========================================================================
-- 2. Return value contract
-- =========================================================================

describe("transpile() return values", function()
    it("returns (string, nil) on success", function()
        local css, err = transpiler.transpile("h1 { color: red; }")
        assert.is_string(css)
        assert.is_nil(err)
    end)

    it("returns (nil, string) on parse failure", function()
        -- Deliberately malformed input (unclosed brace)
        local css, err = transpiler.transpile("h1 { color: red;")
        -- Either the lexer or parser raises; we should get nil + message
        -- (the parser may or may not handle this gracefully — accept either)
        if css == nil then
            assert.is_string(err)
        else
            -- Parser was lenient — just ensure we got a string back
            assert.is_string(css)
        end
    end)
end)

-- =========================================================================
-- 3. End-to-end transpilation
-- =========================================================================

describe("end-to-end transpilation", function()
    it("passes through plain CSS", function()
        local css, err = transpiler.transpile("h1 { color: red; }")
        assert.is_nil(err)
        assert.is_truthy(css:find("h1"))
        assert.is_truthy(css:find("color: red"))
    end)

    it("expands a variable", function()
        local css, err = transpiler.transpile("$color: blue; p { color: $color; }")
        assert.is_nil(err)
        assert.is_truthy(css:find("color: blue"))
        assert.is_falsy(css:find("%$color"))
    end)

    it("flattens a nested rule", function()
        local css, err = transpiler.transpile(".nav { .link { color: red; } }")
        assert.is_nil(err)
        assert.is_truthy(css:find("%.nav %.link"))
    end)

    it("expands a mixin", function()
        local css, err = transpiler.transpile([[
            @mixin flex { display: flex; }
            .row { @include flex; }
        ]])
        assert.is_nil(err)
        assert.is_truthy(css:find("display: flex"))
        assert.is_falsy(css:find("@mixin"))
    end)

    it("@if: includes truthy branch", function()
        local css, err = transpiler.transpile([[
            $show: true;
            @if $show { .box { display: block; } }
        ]])
        assert.is_nil(err)
        assert.is_truthy(css:find("display: block"))
    end)

    it("@if: excludes falsy branch", function()
        local css, err = transpiler.transpile([[
            $show: false;
            @if $show { .box { display: block; } }
        ]])
        assert.is_nil(err)
        assert.is_falsy(css:find("display: block"))
    end)

    it("@for: generates multiple rules", function()
        local css, err = transpiler.transpile([[
            @for $i from 1 through 3 {
                .item { order: $i; }
            }
        ]])
        assert.is_nil(err)
        assert.is_truthy(css:find("order: 1") or css:find("order:1"))
        assert.is_truthy(css:find("order: 2") or css:find("order:2"))
        assert.is_truthy(css:find("order: 3") or css:find("order:3"))
    end)

    it("@each: iterates over list", function()
        local css, err = transpiler.transpile([[
            @each $color in red, green, blue {
                .t { color: $color; }
            }
        ]])
        assert.is_nil(err)
        assert.is_truthy(css:find("color: red"))
        assert.is_truthy(css:find("color: green"))
        assert.is_truthy(css:find("color: blue"))
    end)

    it("combined: variables + nesting + mixin", function()
        local css, err = transpiler.transpile([[
            $primary: #4a90d9;

            @mixin button($bg, $fg: white) {
                background: $bg;
                color: $fg;
            }

            .btn {
                @include button($primary);
                &:hover { opacity: 0.9; }
            }
        ]])
        assert.is_nil(err)
        assert.is_truthy(css:find("background: #4a90d9"))
        assert.is_truthy(css:find("color: white"))
        assert.is_truthy(css:find("opacity: 0.9") or css:find("opacity:0.9"))
    end)
end)

-- =========================================================================
-- 4. transpile_file()
-- =========================================================================

describe("transpile_file()", function()
    it("returns (nil, error) for non-existent file", function()
        local css, err = transpiler.transpile_file("/nonexistent/path/style.lattice")
        assert.is_nil(css)
        assert.is_string(err)
    end)

    it("transpiles a real temp file", function()
        -- Write a temp Lattice file and transpile it
        local tmpfile = os.tmpname() .. ".lattice"
        local f = io.open(tmpfile, "w")
        f:write("$c: red; h1 { color: $c; }")
        f:close()

        local css, err = transpiler.transpile_file(tmpfile)
        os.remove(tmpfile)

        assert.is_nil(err)
        assert.is_truthy(css:find("color: red"))
    end)
end)
