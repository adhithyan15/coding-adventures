-- Tests for mosaic_parser
-- =======================
--
-- Comprehensive busted test suite for the hand-written Mosaic recursive descent parser.
--
-- Coverage:
--   - Module loads and exposes the public API
--   - Simple component with no slots
--   - Component with primitive slots (text, number, bool, image, color, node)
--   - Component with list slot type: list<text>
--   - Component with list<Button> (component type)
--   - Component with named-component slot type
--   - Default values: string, number, bool, color
--   - Properties: string, number, dimension, hex_color, slot_ref, ident, enum
--   - Nested child node elements
--   - Slot reference as child (@slotName;)
--   - when block
--   - each block
--   - Property with keyword as name (color:, text:)
--   - Import declarations (simple and with alias)
--   - Error cases: missing component, unclosed brace, missing tree, trailing content

package.path = (
    "../src/?.lua;"                          ..
    "../src/?/init.lua;"                     ..
    "../../mosaic_lexer/src/?.lua;"          ..
    "../../mosaic_lexer/src/?/init.lua;"     ..
    package.path
)

local parser = require("coding_adventures.mosaic_parser")

-- ============================================================================
-- Helpers
-- ============================================================================

local function parse(src)
    local ast, err = parser.parse(src)
    assert(ast, "unexpected parse error: " .. tostring(err))
    return ast
end

local function component(src)
    return parse(src).component
end

-- ============================================================================
-- Module surface
-- ============================================================================

describe("mosaic_parser module", function()
    it("loads successfully", function()
        assert.is_not_nil(parser)
    end)

    it("exposes VERSION", function()
        assert.is_string(parser.VERSION)
    end)

    it("exposes parse as a function", function()
        assert.is_function(parser.parse)
    end)

    it("returns ast, nil on success", function()
        local ast, err = parser.parse("component Foo { Text { } }")
        assert.is_table(ast)
        assert.is_nil(err)
    end)
end)

-- ============================================================================
-- Basic structure
-- ============================================================================

describe("basic component structure", function()
    it("parses minimal component", function()
        local ast = parse("component Foo { Text { } }")
        assert.equals("file",           ast.rule)
        assert.equals("component_decl", ast.component.rule)
        assert.equals("Foo",            ast.component.name)
    end)

    it("component has empty slots when no slots declared", function()
        local c = component("component X { Text { } }")
        assert.equals(0, #c.slots)
    end)

    it("component tree is the root node_element", function()
        local c = component("component X { Column { } }")
        assert.equals("node_element", c.tree.rule)
        assert.equals("Column",       c.tree.tag)
    end)

    it("file has empty imports when no imports", function()
        local ast = parse("component X { Text { } }")
        assert.equals(0, #ast.imports)
    end)
end)

-- ============================================================================
-- Slot declarations
-- ============================================================================

describe("slot declarations", function()
    it("parses text slot", function()
        local c = component("component C { slot title: text; Text { } }")
        assert.equals(1,           #c.slots)
        assert.equals("title",     c.slots[1].name)
        assert.equals("primitive", c.slots[1].slot_type.kind)
        assert.equals("text",      c.slots[1].slot_type.name)
        assert.is_true(c.slots[1].required)
    end)

    it("parses all primitive slot types", function()
        for _, t in ipairs({"text","number","bool","image","color","node"}) do
            local src = ("component C { slot x: %s; Text { } }"):format(t)
            local c = component(src)
            assert.equals(t, c.slots[1].slot_type.name)
        end
    end)

    it("parses list<text> slot", function()
        local c = component("component C { slot items: list<text>; Text { } }")
        local s = c.slots[1]
        assert.equals("list",      s.slot_type.kind)
        assert.equals("primitive", s.slot_type.element_type.kind)
        assert.equals("text",      s.slot_type.element_type.name)
    end)

    it("parses list<Button> slot (component type)", function()
        local c = component("component C { slot actions: list<Button>; Text { } }")
        local s = c.slots[1]
        assert.equals("list",      s.slot_type.kind)
        assert.equals("component", s.slot_type.element_type.kind)
        assert.equals("Button",    s.slot_type.element_type.name)
    end)

    it("parses named-component slot type", function()
        local c = component("component C { slot action: Button; Text { } }")
        local s = c.slots[1]
        assert.equals("component", s.slot_type.kind)
        assert.equals("Button",    s.slot_type.name)
    end)

    it("parses slot with string default", function()
        local c = component([[component C { slot title: text = "Hello"; Text { } }]])
        local s = c.slots[1]
        assert.is_false(s.required)
        assert.equals("string", s.default_value.kind)
        assert.equals("Hello",  s.default_value.value)
    end)

    it("parses slot with number default", function()
        local c = component("component C { slot count: number = 0; Text { } }")
        local s = c.slots[1]
        assert.is_false(s.required)
        assert.equals("number", s.default_value.kind)
        assert.equals("0",      s.default_value.value)
    end)

    it("parses slot with bool true default", function()
        local c = component("component C { slot v: bool = true; Text { } }")
        local s = c.slots[1]
        assert.equals("bool", s.default_value.kind)
        assert.is_true(s.default_value.value)
    end)

    it("parses slot with bool false default", function()
        local c = component("component C { slot v: bool = false; Text { } }")
        local s = c.slots[1]
        assert.equals("bool",  s.default_value.kind)
        assert.is_false(s.default_value.value)
    end)

    it("parses slot with hex color default", function()
        local c = component("component C { slot bg: color = #fff; Text { } }")
        local s = c.slots[1]
        assert.equals("color_hex", s.default_value.kind)
        assert.equals("#fff",      s.default_value.value)
    end)

    it("parses multiple slots", function()
        local c = component([[
            component C {
              slot a: text;
              slot b: number;
              slot c: bool;
              Text { }
            }
        ]])
        assert.equals(3, #c.slots)
        assert.equals("a", c.slots[1].name)
        assert.equals("b", c.slots[2].name)
        assert.equals("c", c.slots[3].name)
    end)
end)

-- ============================================================================
-- Property assignments
-- ============================================================================

describe("property assignments", function()
    it("parses string property", function()
        local c = component([[component C { Text { content: "Hello"; } }]])
        local prop = c.tree.properties[1]
        assert.equals("content", prop.name)
        assert.equals("string",  prop.value.kind)
        assert.equals("Hello",   prop.value.value)
    end)

    it("parses dimension property", function()
        local c = component("component C { Column { padding: 16dp; } }")
        local prop = c.tree.properties[1]
        assert.equals("padding",   prop.name)
        assert.equals("dimension", prop.value.kind)
        assert.equals("16dp",      prop.value.value)
    end)

    it("parses number property", function()
        local c = component("component C { Column { opacity: 0.5; } }")
        local prop = c.tree.properties[1]
        assert.equals("number", prop.value.kind)
    end)

    it("parses color property", function()
        local c = component("component C { Column { background: #2563eb; } }")
        local prop = c.tree.properties[1]
        assert.equals("color_hex", prop.value.kind)
        assert.equals("#2563eb",   prop.value.value)
    end)

    it("parses slot_ref property", function()
        local c = component("component C { slot t: text; Text { content: @t; } }")
        local prop = c.tree.properties[1]
        assert.equals("content",  prop.name)
        assert.equals("slot_ref", prop.value.kind)
        assert.equals("t",        prop.value.slot_name)
    end)

    it("parses ident property", function()
        local c = component("component C { Column { align: center; } }")
        local prop = c.tree.properties[1]
        assert.equals("ident",  prop.value.kind)
        assert.equals("center", prop.value.value)
    end)

    it("parses enum property (Namespace.member)", function()
        local c = component("component C { Column { align: Alignment.center; } }")
        local prop = c.tree.properties[1]
        assert.equals("enum",      prop.value.kind)
        assert.equals("Alignment", prop.value.namespace)
        assert.equals("center",    prop.value.member)
    end)

    it("parses keyword as property name (color:)", function()
        local c = component("component C { Text { color: #ff0000; } }")
        local prop = c.tree.properties[1]
        assert.equals("color", prop.name)
    end)

    it("parses bool true property", function()
        local c = component("component C { Text { visible: true; } }")
        local prop = c.tree.properties[1]
        assert.equals("bool",  prop.value.kind)
        assert.is_true(prop.value.value)
    end)
end)

-- ============================================================================
-- Children
-- ============================================================================

describe("node children", function()
    it("parses nested child node element", function()
        local c = component("component C { Column { Text { } } }")
        assert.equals(1, #c.tree.children)
        local child = c.tree.children[1]
        assert.equals("node_element", child.rule)
        assert.equals("Text",         child.tag)
    end)

    it("parses slot reference as child", function()
        local c = component("component C { slot hdr: node; Column { @hdr; } }")
        local child = c.tree.children[1]
        assert.equals("slot_reference", child.rule)
        assert.equals("hdr",            child.slot_name)
    end)

    it("parses when block", function()
        local c = component([[
            component C {
              slot show: bool;
              Column { when @show { Text { } } }
            }
        ]])
        local child = c.tree.children[1]
        assert.equals("when_block", child.rule)
        assert.equals("show",       child.slot_name)
        assert.equals(1,            #child.children)
    end)

    it("parses each block", function()
        local c = component([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        local child = c.tree.children[1]
        assert.equals("each_block", child.rule)
        assert.equals("items",      child.slot_name)
        assert.equals("item",       child.item_name)
        assert.equals(1,            #child.children)
    end)

    it("parses multiple children in order", function()
        local c = component([[
            component C {
              Column {
                Text { content: "A"; }
                Text { content: "B"; }
                Text { content: "C"; }
              }
            }
        ]])
        assert.equals(3, #c.tree.children)
    end)
end)

-- ============================================================================
-- Import declarations
-- ============================================================================

describe("import declarations", function()
    it("parses a simple import", function()
        local ast = parse([[
            import Button from "./button.mosaic";
            component C { Text { } }
        ]])
        assert.equals(1, #ast.imports)
        local imp = ast.imports[1]
        assert.equals("import_decl",      imp.rule)
        assert.equals("Button",           imp.component_name)
        assert.is_nil(imp.alias)
        assert.equals("./button.mosaic",  imp.path)
    end)

    it("parses import with alias", function()
        local ast = parse([[
            import Card as InfoCard from "./card.mosaic";
            component C { Text { } }
        ]])
        local imp = ast.imports[1]
        assert.equals("Card",     imp.component_name)
        assert.equals("InfoCard", imp.alias)
    end)

    it("parses multiple imports", function()
        local ast = parse([[
            import Button from "./button.mosaic";
            import Icon from "./icon.mosaic";
            component C { Text { } }
        ]])
        assert.equals(2, #ast.imports)
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("returns nil, errmsg when input is empty", function()
        local ast, err = parser.parse("")
        assert.is_nil(ast)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on missing component keyword", function()
        local ast, err = parser.parse("slot x: text;")
        assert.is_nil(ast)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on unclosed brace", function()
        local ast, err = parser.parse("component Foo { Text {")
        assert.is_nil(ast)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on missing node tree", function()
        local ast, err = parser.parse("component Foo { slot x: text; }")
        assert.is_nil(ast)
        assert.is_string(err)
    end)

    it("returns nil, errmsg on trailing content after component", function()
        local ast, err = parser.parse(
            "component Foo { Text { } } component Bar { Text { } }"
        )
        assert.is_nil(ast)
        assert.is_string(err)
    end)
end)
