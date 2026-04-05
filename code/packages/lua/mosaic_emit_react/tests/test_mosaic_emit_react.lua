-- Tests for mosaic_emit_react
-- ============================
--
-- Comprehensive busted test suite for the React TSX emitter.
--
-- Coverage:
--   - Module loads and exposes public API
--   - emit returns { files } with a .tsx filename
--   - Generated file starts with AUTO-GENERATED header
--   - Props interface is generated with correct types
--   - Optional slots have ? and default values
--   - Primitive nodes map to correct JSX tags
--   - Column → <div style={{ display: "flex", flexDirection: "column" }}>
--   - Text  → <span>{slotRef}</span>
--   - Image → <img src=... /> (self-closing)
--   - Dimension property → px
--   - Color property → rgba()
--   - String property → literal in JSX
--   - Slot reference as child → {slotName}
--   - when block → conditional expression
--   - each block → .map() expression
--   - Typography style: → className with CSS import
--   - Component-type slot → React.ReactElement<T>Props> in interface
--   - list<text> slot → string[] in interface
--   - error on bad source

package.path = (
    "../src/?.lua;"                                  ..
    "../src/?/init.lua;"                             ..
    "../../mosaic_vm/src/?.lua;"                     ..
    "../../mosaic_vm/src/?/init.lua;"                ..
    "../../mosaic_analyzer/src/?.lua;"               ..
    "../../mosaic_analyzer/src/?/init.lua;"          ..
    "../../mosaic_parser/src/?.lua;"                 ..
    "../../mosaic_parser/src/?/init.lua;"            ..
    "../../mosaic_lexer/src/?.lua;"                  ..
    "../../mosaic_lexer/src/?/init.lua;"             ..
    package.path
)

local emit_react = require("coding_adventures.mosaic_emit_react")

-- ============================================================================
-- Helpers
-- ============================================================================

local function emit(src)
    local result, err = emit_react.emit(src)
    assert(result, "emit failed: " .. tostring(err))
    return result
end

local function content(src)
    return emit(src).files[1].content
end

local function has(text, pattern)
    return text:find(pattern, 1, true) ~= nil
end

-- ============================================================================
-- Module surface
-- ============================================================================

describe("mosaic_emit_react module", function()
    it("loads successfully", function()
        assert.is_not_nil(emit_react)
    end)

    it("exposes VERSION", function()
        assert.is_string(emit_react.VERSION)
    end)

    it("exposes emit as a function", function()
        assert.is_function(emit_react.emit)
    end)

    it("exposes new_renderer as a function", function()
        assert.is_function(emit_react.new_renderer)
    end)
end)

-- ============================================================================
-- Output structure
-- ============================================================================

describe("output structure", function()
    it("returns { files } table", function()
        local result = emit("component C { Text { } }")
        assert.is_table(result)
        assert.is_table(result.files)
        assert.equals(1, #result.files)
    end)

    it("filename matches component name with .tsx extension", function()
        local result = emit("component ProfileCard { Text { } }")
        assert.equals("ProfileCard.tsx", result.files[1].filename)
    end)

    it("content is a non-empty string", function()
        local c = content("component C { Text { } }")
        assert.is_string(c)
        assert.truthy(#c > 50)
    end)

    it("file starts with AUTO-GENERATED comment", function()
        local c = content("component C { Text { } }")
        assert.truthy(c:sub(1,2) == "//")
        assert.truthy(has(c, "AUTO-GENERATED"))
    end)

    it("file imports React", function()
        local c = content("component C { Text { } }")
        assert.truthy(has(c, 'import React from "react"'))
    end)

    it("file exports the function component", function()
        local c = content("component MyButton { Text { } }")
        assert.truthy(has(c, "export function MyButton("))
    end)

    it("file contains props interface", function()
        local c = content("component Label { Text { } }")
        assert.truthy(has(c, "interface LabelProps {"))
    end)
end)

-- ============================================================================
-- Slot types in props interface
-- ============================================================================

describe("slot types in interface", function()
    it("text slot → string", function()
        local c = content("component C { slot title: text; Text { } }")
        assert.truthy(has(c, "title: string;"))
    end)

    it("number slot → number", function()
        local c = content("component C { slot count: number; Text { } }")
        assert.truthy(has(c, "count: number;"))
    end)

    it("bool slot → boolean", function()
        local c = content("component C { slot visible: bool; Text { } }")
        assert.truthy(has(c, "visible: boolean;"))
    end)

    it("image slot → string", function()
        local c = content("component C { slot src: image; Text { } }")
        assert.truthy(has(c, "src: string;"))
    end)

    it("node slot → React.ReactNode", function()
        local c = content("component C { slot child: node; Text { } }")
        assert.truthy(has(c, "child: React.ReactNode;"))
    end)

    it("list<text> slot → string[]", function()
        local c = content("component C { slot items: list<text>; Text { } }")
        assert.truthy(has(c, "items: string[];"))
    end)

    it("optional slot has ? marker", function()
        local c = content([[component C { slot title: text = "Hi"; Text { } }]])
        assert.truthy(has(c, "title?: string;"))
    end)

    it("optional slot has default in function signature", function()
        local c = content([[component C { slot count: number = 0; Text { } }]])
        assert.truthy(has(c, "count = 0,"))
    end)
end)

-- ============================================================================
-- Primitive node JSX mapping
-- ============================================================================

describe("primitive node mapping", function()
    it("Column → div with flex column styles", function()
        local c = content("component C { Column { } }")
        assert.truthy(has(c, "flexDirection"))
        assert.truthy(has(c, "column"))
    end)

    it("Row → div with flex row styles", function()
        local c = content("component C { Row { } }")
        assert.truthy(has(c, "flexDirection"))
        assert.truthy(has(c, "row"))
    end)

    it("Text → span tag", function()
        local c = content("component C { Text { } }")
        assert.truthy(has(c, "<span"))
    end)

    it("Image → self-closing img tag", function()
        local c = content("component C { Image { } }")
        assert.truthy(has(c, "<img"))
        assert.truthy(has(c, "/>"))
    end)

    it("Spacer → div with flex: 1", function()
        local c = content("component C { Spacer { } }")
        assert.truthy(has(c, "flex: 1"))
    end)

    it("Divider → self-closing hr", function()
        local c = content("component C { Divider { } }")
        assert.truthy(has(c, "<hr"))
    end)
end)

-- ============================================================================
-- Properties
-- ============================================================================

describe("property emission", function()
    it("dimension property → px value", function()
        local c = content("component C { Column { padding: 16dp; } }")
        assert.truthy(has(c, "16px"))
    end)

    it("color property → rgba()", function()
        local c = content("component C { Column { background: #2563eb; } }")
        assert.truthy(has(c, "rgba("))
        assert.truthy(has(c, "backgroundColor"))
    end)

    it("string content → inline text", function()
        local c = content([[component C { Text { content: "Hello"; } }]])
        assert.truthy(has(c, "Hello"))
    end)

    it("slot_ref content → {slotRef}", function()
        local c = content("component C { slot title: text; Text { content: @title; } }")
        assert.truthy(has(c, "{title}"))
    end)

    it("Image source property → src=", function()
        local c = content("component C { slot img: image; Image { source: @img; } }")
        assert.truthy(has(c, "src="))
    end)
end)

-- ============================================================================
-- Children
-- ============================================================================

describe("slot child rendering", function()
    it("slot reference as child → {slotName} in JSX", function()
        local c = content("component C { slot hdr: node; Column { @hdr; } }")
        assert.truthy(has(c, "{hdr}"))
    end)
end)

describe("when block", function()
    it("generates a conditional expression", function()
        local c = content([[
            component C {
              slot show: bool;
              Column { when @show { Text { content: "Yes"; } } }
            }
        ]])
        assert.truthy(has(c, "{show &&"))
    end)
end)

describe("each block", function()
    it("generates a .map() expression", function()
        local c = content([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        assert.truthy(has(c, ".map("))
        assert.truthy(has(c, "React.Fragment"))
    end)
end)

-- ============================================================================
-- Typography scale
-- ============================================================================

describe("typography style", function()
    it("style: heading.large → className and CSS import", function()
        local c = content("component C { Text { style: heading.large; } }")
        assert.truthy(has(c, "mosaic-heading-large"))
        assert.truthy(has(c, "mosaic-type-scale.css"))
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("returns nil, errmsg on bad source", function()
        local result, err = emit_react.emit("not valid")
        assert.is_nil(result)
        assert.is_string(err)
    end)
end)
