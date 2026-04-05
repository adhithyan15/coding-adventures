-- Tests for mosaic_analyzer
-- ==========================
--
-- Comprehensive busted test suite for the Mosaic analyzer.
--
-- Coverage:
--   - Module loads and exposes public API
--   - Analyzes minimal component (no slots)
--   - Slot type resolution: text, number, bool, image, color, node
--   - List slot types: list<text>, list<Button>
--   - Component-type slots
--   - Default value normalization: string, number, bool, color, dimension
--   - required vs optional slots
--   - Property value normalization: string, number, dimension, color_hex, slot_ref, ident, enum, bool
--   - Root node is_primitive flag (Row, Column, etc. = true; custom names = false)
--   - Nested children analysis
--   - when block in IR
--   - each block in IR
--   - Import analysis
--   - analyze_ast variant
--   - Error propagation (bad source)

package.path = (
    "../src/?.lua;"                          ..
    "../src/?/init.lua;"                     ..
    "../../mosaic_parser/src/?.lua;"         ..
    "../../mosaic_parser/src/?/init.lua;"    ..
    "../../mosaic_lexer/src/?.lua;"          ..
    "../../mosaic_lexer/src/?/init.lua;"     ..
    package.path
)

local analyzer = require("coding_adventures.mosaic_analyzer")

-- ============================================================================
-- Helpers
-- ============================================================================

local function analyze(src)
    local ir, err = analyzer.analyze(src)
    assert(ir, "unexpected analyzer error: " .. tostring(err))
    return ir
end

local function comp(src)
    return analyze(src).component
end

-- ============================================================================
-- Module surface
-- ============================================================================

describe("mosaic_analyzer module", function()
    it("loads successfully", function()
        assert.is_not_nil(analyzer)
    end)

    it("exposes VERSION", function()
        assert.is_string(analyzer.VERSION)
    end)

    it("exposes analyze as a function", function()
        assert.is_function(analyzer.analyze)
    end)

    it("exposes analyze_ast as a function", function()
        assert.is_function(analyzer.analyze_ast)
    end)

    it("returns ir, nil on success", function()
        local ir, err = analyzer.analyze("component Foo { Text { } }")
        assert.is_table(ir)
        assert.is_nil(err)
    end)
end)

-- ============================================================================
-- Component structure
-- ============================================================================

describe("component structure", function()
    it("component name is correct", function()
        local c = comp("component MyWidget { Text { } }")
        assert.equals("MyWidget", c.name)
    end)

    it("component has slots array", function()
        local c = comp("component X { Text { } }")
        assert.is_table(c.slots)
        assert.equals(0, #c.slots)
    end)

    it("component has tree node", function()
        local c = comp("component X { Column { } }")
        assert.is_table(c.tree)
        assert.equals("Column", c.tree.tag)
    end)

    it("imports are present in IR", function()
        local ir = analyze([[
            import Button from "./button.mosaic";
            component C { Text { } }
        ]])
        assert.equals(1, #ir.imports)
        assert.equals("Button",          ir.imports[1].component_name)
        assert.equals("./button.mosaic", ir.imports[1].path)
    end)
end)

-- ============================================================================
-- Slot type resolution
-- ============================================================================

describe("slot type resolution", function()
    it("text slot type", function()
        local c = comp("component C { slot t: text; Text { } }")
        assert.equals("text", c.slots[1].type.kind)
    end)

    it("number slot type", function()
        local c = comp("component C { slot n: number; Text { } }")
        assert.equals("number", c.slots[1].type.kind)
    end)

    it("bool slot type", function()
        local c = comp("component C { slot b: bool; Text { } }")
        assert.equals("bool", c.slots[1].type.kind)
    end)

    it("image slot type", function()
        local c = comp("component C { slot img: image; Text { } }")
        assert.equals("image", c.slots[1].type.kind)
    end)

    it("color slot type", function()
        local c = comp("component C { slot bg: color; Text { } }")
        assert.equals("color", c.slots[1].type.kind)
    end)

    it("node slot type", function()
        local c = comp("component C { slot content: node; Text { } }")
        assert.equals("node", c.slots[1].type.kind)
    end)

    it("list<text> slot type", function()
        local c = comp("component C { slot items: list<text>; Text { } }")
        local t = c.slots[1].type
        assert.equals("list", t.kind)
        assert.equals("text", t.element_type.kind)
    end)

    it("list<Button> slot type (component element)", function()
        local c = comp("component C { slot actions: list<Button>; Text { } }")
        local t = c.slots[1].type
        assert.equals("list",      t.kind)
        assert.equals("component", t.element_type.kind)
        assert.equals("Button",    t.element_type.name)
    end)

    it("named component slot type", function()
        local c = comp("component C { slot action: Button; Text { } }")
        local t = c.slots[1].type
        assert.equals("component", t.kind)
        assert.equals("Button",    t.name)
    end)
end)

-- ============================================================================
-- Default value normalization
-- ============================================================================

describe("default value normalization", function()
    it("string default", function()
        local c = comp([[component C { slot t: text = "Hi"; Text { } }]])
        local dv = c.slots[1].default_value
        assert.equals("string", dv.kind)
        assert.equals("Hi",     dv.value)
    end)

    it("number default (converted to Lua number)", function()
        local c = comp("component C { slot n: number = 42; Text { } }")
        local dv = c.slots[1].default_value
        assert.equals("number", dv.kind)
        assert.equals(42,       dv.value)  -- Lua number, not string
    end)

    it("bool true default", function()
        local c = comp("component C { slot v: bool = true; Text { } }")
        local dv = c.slots[1].default_value
        assert.equals("bool", dv.kind)
        assert.is_true(dv.value)
    end)

    it("bool false default", function()
        local c = comp("component C { slot v: bool = false; Text { } }")
        local dv = c.slots[1].default_value
        assert.equals("bool",  dv.kind)
        assert.is_false(dv.value)
    end)

    it("hex color default", function()
        local c = comp("component C { slot bg: color = #2563eb; Text { } }")
        local dv = c.slots[1].default_value
        assert.equals("color_hex", dv.kind)
        assert.equals("#2563eb",   dv.value)
    end)
end)

-- ============================================================================
-- Required vs optional slots
-- ============================================================================

describe("required vs optional slots", function()
    it("slot without default is required", function()
        local c = comp("component C { slot t: text; Text { } }")
        assert.is_true(c.slots[1].required)
    end)

    it("slot with default is optional (required=false)", function()
        local c = comp([[component C { slot t: text = "x"; Text { } }]])
        assert.is_false(c.slots[1].required)
        assert.is_not_nil(c.slots[1].default_value)
    end)
end)

-- ============================================================================
-- Property value normalization
-- ============================================================================

describe("property value normalization", function()
    it("dimension property has numeric value and unit", function()
        local c = comp("component C { Column { padding: 16dp; } }")
        local v = c.tree.properties[1].value
        assert.equals("dimension", v.kind)
        assert.equals(16,          v.value)
        assert.equals("dp",        v.unit)
    end)

    it("dimension 1.5sp", function()
        local c = comp("component C { Column { gap: 1.5sp; } }")
        local v = c.tree.properties[1].value
        assert.equals("dimension", v.kind)
        assert.equals(1.5,         v.value)
        assert.equals("sp",        v.unit)
    end)

    it("dimension 100%", function()
        local c = comp("component C { Column { width: 100%; } }")
        local v = c.tree.properties[1].value
        assert.equals("dimension", v.kind)
        assert.equals(100,         v.value)
        assert.equals("%",         v.unit)
    end)

    it("string property preserved", function()
        local c = comp([[component C { Text { content: "hello"; } }]])
        local v = c.tree.properties[1].value
        assert.equals("string", v.kind)
        assert.equals("hello",  v.value)
    end)

    it("number property converted to Lua number", function()
        local c = comp("component C { Text { opacity: 0.5; } }")
        local v = c.tree.properties[1].value
        assert.equals("number", v.kind)
        assert.equals(0.5,      v.value)
    end)

    it("color_hex property preserved", function()
        local c = comp("component C { Column { background: #2563eb; } }")
        local v = c.tree.properties[1].value
        assert.equals("color_hex", v.kind)
        assert.equals("#2563eb",   v.value)
    end)

    it("slot_ref property", function()
        local c = comp("component C { slot t: text; Text { content: @t; } }")
        local v = c.tree.properties[1].value
        assert.equals("slot_ref", v.kind)
        assert.equals("t",        v.slot_name)
    end)

    it("ident property", function()
        local c = comp("component C { Column { align: center; } }")
        local v = c.tree.properties[1].value
        assert.equals("ident",  v.kind)
        assert.equals("center", v.value)
    end)

    it("enum property", function()
        local c = comp("component C { Column { align: Alignment.center; } }")
        local v = c.tree.properties[1].value
        assert.equals("enum",      v.kind)
        assert.equals("Alignment", v.namespace)
        assert.equals("center",    v.member)
    end)

    it("bool true property", function()
        local c = comp("component C { Text { visible: true; } }")
        local v = c.tree.properties[1].value
        assert.equals("bool", v.kind)
        assert.is_true(v.value)
    end)
end)

-- ============================================================================
-- is_primitive flag
-- ============================================================================

describe("is_primitive node flag", function()
    local primitives = {"Row","Column","Box","Stack","Text","Image","Icon","Spacer","Divider","Scroll"}
    for _, tag in ipairs(primitives) do
        it(tag .. " is primitive", function()
            local src = ("component C { %s { } }"):format(tag)
            local c = comp(src)
            assert.is_true(c.tree.is_primitive)
        end)
    end

    it("custom component name is NOT primitive", function()
        local c = comp("component C { ProfileCard { } }")
        assert.is_false(c.tree.is_primitive)
    end)
end)

-- ============================================================================
-- Children in the IR
-- ============================================================================

describe("children analysis", function()
    it("nested node child becomes kind=node", function()
        local c = comp("component C { Column { Text { } } }")
        local child = c.tree.children[1]
        assert.equals("node", child.kind)
        assert.equals("Text", child.node.tag)
    end)

    it("slot_reference child becomes kind=slot_ref", function()
        local c = comp("component C { slot hdr: node; Column { @hdr; } }")
        local child = c.tree.children[1]
        assert.equals("slot_ref", child.kind)
        assert.equals("hdr",      child.slot_name)
    end)

    it("when block becomes kind=when", function()
        local c = comp([[
            component C {
              slot show: bool;
              Column { when @show { Text { } } }
            }
        ]])
        local child = c.tree.children[1]
        assert.equals("when", child.kind)
        assert.equals("show", child.slot_name)
        assert.equals(1,      #child.children)
        assert.equals("node", child.children[1].kind)
    end)

    it("each block becomes kind=each", function()
        local c = comp([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        local child = c.tree.children[1]
        assert.equals("each",  child.kind)
        assert.equals("items", child.slot_name)
        assert.equals("item",  child.item_name)
        assert.equals(1,       #child.children)
    end)
end)

-- ============================================================================
-- analyze_ast variant
-- ============================================================================

describe("analyze_ast", function()
    it("accepts a pre-parsed AST", function()
        local parser = require("coding_adventures.mosaic_parser")
        local ast = parser.parse("component X { Text { } }")
        local ir, err = analyzer.analyze_ast(ast)
        assert.is_nil(err)
        assert.is_table(ir)
        assert.equals("X", ir.component.name)
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("returns nil, errmsg on bad source", function()
        local ir, err = analyzer.analyze("not valid mosaic")
        assert.is_nil(ir)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on empty source", function()
        local ir, err = analyzer.analyze("")
        assert.is_nil(ir)
        assert.is_string(err)
    end)
end)
