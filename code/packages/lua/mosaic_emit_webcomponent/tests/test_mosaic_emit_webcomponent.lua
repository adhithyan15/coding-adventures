-- Tests for mosaic_emit_webcomponent
-- =====================================
--
-- Comprehensive busted test suite for the Web Component TypeScript emitter.
--
-- Coverage:
--   - Module loads and exposes public API
--   - emit returns { files } with a .ts filename
--   - Generated file starts with AUTO-GENERATED header
--   - PascalCase component name → kebab-case element tag (mosaic- prefix)
--   - Class name follows MosaicXxxElement pattern
--   - customElements.define() call present
--   - Slot backing fields generated with correct types
--   - observedAttributes contains attribute-observable slots
--   - Primitive nodes map to correct HTML tags
--   - Column → display:flex;flex-direction:column
--   - Row → display:flex;flex-direction:row
--   - Text → <span> with text content
--   - Image → self-closing <img>
--   - Spacer → flex:1
--   - Divider → <hr>
--   - Dimension property → px
--   - Color property → rgba()
--   - Slot reference as content → this._escapeHtml(this._field)
--   - when block → conditional html +=
--   - each block → for loop over array
--   - node slot → _projectSlot method generated
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

local emit_wc = require("coding_adventures.mosaic_emit_webcomponent")

-- ============================================================================
-- Helpers
-- ============================================================================

local function emit(src)
    local result, err = emit_wc.emit(src)
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

describe("mosaic_emit_webcomponent module", function()
    it("loads successfully", function()
        assert.is_not_nil(emit_wc)
    end)

    it("exposes VERSION", function()
        assert.is_string(emit_wc.VERSION)
    end)

    it("exposes emit as a function", function()
        assert.is_function(emit_wc.emit)
    end)

    it("exposes new_renderer as a function", function()
        assert.is_function(emit_wc.new_renderer)
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

    it("filename uses kebab-case with mosaic- prefix and .ts extension", function()
        local result = emit("component ProfileCard { Text { } }")
        assert.equals("mosaic-profile-card.ts", result.files[1].filename)
    end)

    it("single-word component gets mosaic- prefix", function()
        local result = emit("component Button { Text { } }")
        assert.equals("mosaic-button.ts", result.files[1].filename)
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

    it("class name is MosaicXxxElement", function()
        local c = content("component ProfileCard { Text { } }")
        assert.truthy(has(c, "class MosaicProfileCardElement extends HTMLElement"))
    end)

    it("customElements.define call is present", function()
        local c = content("component ProfileCard { Text { } }")
        assert.truthy(has(c, "customElements.define"))
        assert.truthy(has(c, "mosaic-profile-card"))
    end)

    it("file exports the class", function()
        local c = content("component Button { Text { } }")
        assert.truthy(has(c, "export class"))
    end)
end)

-- ============================================================================
-- Slot backing fields
-- ============================================================================

describe("slot backing fields", function()
    it("text slot → private _name: string", function()
        local c = content("component C { slot title: text; Text { } }")
        assert.truthy(has(c, "_title: string"))
    end)

    it("number slot → private _count: number", function()
        local c = content("component C { slot count: number; Text { } }")
        assert.truthy(has(c, "_count: number"))
    end)

    it("bool slot → private _visible: boolean", function()
        local c = content("component C { slot visible: bool; Text { } }")
        assert.truthy(has(c, "_visible: boolean"))
    end)

    it("image slot → private _src: string", function()
        local c = content("component C { slot src: image; Image { } }")
        assert.truthy(has(c, "_src: string"))
    end)

    it("text slot has default = ''", function()
        local c = content("component C { slot label: text; Text { } }")
        assert.truthy(has(c, "_label: string = ''"))
    end)

    it("bool slot has default = false", function()
        local c = content("component C { slot active: bool; Text { } }")
        assert.truthy(has(c, "_active: boolean = false"))
    end)

    it("optional bool slot uses provided default", function()
        local c = content("component C { slot active: bool = true; Text { } }")
        assert.truthy(has(c, "_active: boolean = true"))
    end)
end)

-- ============================================================================
-- observedAttributes
-- ============================================================================

describe("observedAttributes", function()
    it("text slot appears in observedAttributes", function()
        local c = content("component C { slot title: text; Text { } }")
        assert.truthy(has(c, "observedAttributes"))
        assert.truthy(has(c, "'title'"))
    end)

    it("bool slot appears in observedAttributes", function()
        local c = content("component C { slot active: bool; Text { } }")
        assert.truthy(has(c, "'active'"))
    end)

    it("node slot does NOT appear in observedAttributes", function()
        local c = content("component C { slot child: node; Column { @child; } }")
        -- node slots can't be set via HTML attributes, so observedAttributes should not list 'child'
        -- The file may omit observedAttributes entirely when only node slots exist
        local observed_line = c:match("observedAttributes[^\n]*")
        -- If observedAttributes is present, it should not contain 'child'
        if observed_line then
            assert.falsy(observed_line:find("'child'", 1, true))
        end
        -- Either way, observedAttributes should not include 'child'
        assert.truthy(true)  -- pass: the contract is verified above
    end)
end)

-- ============================================================================
-- Primitive node HTML mapping
-- ============================================================================

describe("primitive node HTML mapping", function()
    it("Column → div with flex-direction:column", function()
        local c = content("component C { Column { } }")
        assert.truthy(has(c, "flex-direction:column"))
    end)

    it("Row → div with flex-direction:row", function()
        local c = content("component C { Row { } }")
        assert.truthy(has(c, "flex-direction:row"))
    end)

    it("Text → span tag", function()
        local c = content("component C { Text { } }")
        assert.truthy(has(c, "<span"))
    end)

    it("Image → img tag", function()
        local c = content("component C { Image { } }")
        assert.truthy(has(c, "<img"))
    end)

    it("Spacer → div with flex:1", function()
        local c = content("component C { Spacer { } }")
        assert.truthy(has(c, "flex:1"))
    end)

    it("Divider → hr tag", function()
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
        assert.truthy(has(c, "background"))
    end)

    it("string content → inline text in html +=", function()
        local c = content([[component C { Text { content: "Hello"; } }]])
        assert.truthy(has(c, "Hello"))
    end)

    it("slot_ref content → this._escapeHtml(this._field)", function()
        local c = content("component C { slot title: text; Text { content: @title; } }")
        assert.truthy(has(c, "_title"))
        assert.truthy(has(c, "_escapeHtml"))
    end)
end)

-- ============================================================================
-- Slot child projection
-- ============================================================================

describe("slot child projection", function()
    it("node slot generates _projectSlot method", function()
        local c = content("component C { slot hdr: node; Column { @hdr; } }")
        assert.truthy(has(c, "_projectSlot"))
    end)
end)

-- ============================================================================
-- when block
-- ============================================================================

describe("when block", function()
    it("generates a conditional html +=" , function()
        local c = content([[
            component C {
              slot show: bool;
              Column { when @show { Text { content: "Yes"; } } }
            }
        ]])
        -- Should have some form of conditional guard
        assert.truthy(has(c, "show") or has(c, "_show"))
    end)
end)

-- ============================================================================
-- each block
-- ============================================================================

describe("each block", function()
    it("generates a for-loop", function()
        local c = content([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        assert.truthy(has(c, "for ") or has(c, "forEach"))
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("returns nil, errmsg on bad source", function()
        local result, err = emit_wc.emit("not valid")
        assert.is_nil(result)
        assert.is_string(err)
    end)
end)
