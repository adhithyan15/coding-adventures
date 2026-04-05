-- Tests for mosaic_vm
-- ====================
--
-- Comprehensive busted test suite for the Mosaic VM.
--
-- Coverage:
--   - Module loads and exposes public API
--   - Traversal order: beginComponent, beginNode, endNode, endComponent, emit
--   - Color parsing: #rgb, #rrggbb, #rrggbbaa
--   - Dimension passthrough (value + unit)
--   - String property
--   - Number property
--   - Bool property
--   - Ident folded into string
--   - Slot ref resolved from component slots
--   - Enum value passthrough
--   - Nested node traversal
--   - Slot child rendering (renderSlotChild)
--   - when block traversal (beginWhen/endWhen)
--   - each block traversal (beginEach/endEach + loop scope)
--   - run_source convenience wrapper
--   - Error: unresolved slot reference
--   - Error: each on non-list slot

package.path = (
    "../src/?.lua;"                              ..
    "../src/?/init.lua;"                         ..
    "../../mosaic_analyzer/src/?.lua;"           ..
    "../../mosaic_analyzer/src/?/init.lua;"      ..
    "../../mosaic_parser/src/?.lua;"             ..
    "../../mosaic_parser/src/?/init.lua;"        ..
    "../../mosaic_lexer/src/?.lua;"              ..
    "../../mosaic_lexer/src/?/init.lua;"         ..
    package.path
)

local vm       = require("coding_adventures.mosaic_vm")
local analyzer = require("coding_adventures.mosaic_analyzer")

-- ============================================================================
-- Spy renderer
-- ============================================================================
--
-- A simple recorder renderer that logs every method call. Tests inspect
-- the `calls` list to verify traversal order and resolved values.

local function new_spy()
    local spy = { calls = {}, emit_result = { files = {} } }

    function spy:beginComponent(name, slots)
        self.calls[#self.calls + 1] = { method = "beginComponent", name = name, slots = slots }
    end
    function spy:endComponent()
        self.calls[#self.calls + 1] = { method = "endComponent" }
    end
    function spy:beginNode(tag, is_primitive, props, ctx)
        self.calls[#self.calls + 1] = { method = "beginNode", tag = tag,
            is_primitive = is_primitive, props = props, ctx = ctx }
    end
    function spy:endNode(tag)
        self.calls[#self.calls + 1] = { method = "endNode", tag = tag }
    end
    function spy:renderSlotChild(slot_name, slot_type, ctx)
        self.calls[#self.calls + 1] = { method = "renderSlotChild",
            slot_name = slot_name, slot_type = slot_type }
    end
    function spy:beginWhen(slot_name, ctx)
        self.calls[#self.calls + 1] = { method = "beginWhen", slot_name = slot_name }
    end
    function spy:endWhen()
        self.calls[#self.calls + 1] = { method = "endWhen" }
    end
    function spy:beginEach(slot_name, item_name, element_type, ctx)
        self.calls[#self.calls + 1] = { method = "beginEach",
            slot_name = slot_name, item_name = item_name, element_type = element_type }
    end
    function spy:endEach()
        self.calls[#self.calls + 1] = { method = "endEach" }
    end
    function spy:emit()
        return self.emit_result
    end

    return spy
end

--- Run source through analyzer + VM with the spy renderer.
local function run(src)
    local ir, err = analyzer.analyze(src)
    assert(ir, "analyze failed: " .. tostring(err))
    local spy = new_spy()
    local result, vm_err = vm.run(ir, spy)
    assert(result, "vm.run failed: " .. tostring(vm_err))
    return spy
end

--- Find the first call with the given method name.
local function first(calls, method)
    for _, c in ipairs(calls) do
        if c.method == method then return c end
    end
    return nil
end

--- Find all calls with the given method name.
local function all(calls, method)
    local out = {}
    for _, c in ipairs(calls) do
        if c.method == method then out[#out + 1] = c end
    end
    return out
end

--- Extract call method names as a list.
local function methods(calls)
    local out = {}
    for _, c in ipairs(calls) do out[#out + 1] = c.method end
    return out
end

-- ============================================================================
-- Module surface
-- ============================================================================

describe("mosaic_vm module", function()
    it("loads successfully", function()
        assert.is_not_nil(vm)
    end)

    it("exposes VERSION", function()
        assert.is_string(vm.VERSION)
    end)

    it("exposes run as a function", function()
        assert.is_function(vm.run)
    end)

    it("exposes run_source as a function", function()
        assert.is_function(vm.run_source)
    end)
end)

-- ============================================================================
-- Traversal order
-- ============================================================================

describe("traversal order", function()
    it("calls beginComponent, beginNode, endNode, endComponent in order", function()
        local spy = run("component C { Text { } }")
        local m_list = methods(spy.calls)
        assert.equals("beginComponent", m_list[1])
        assert.equals("beginNode",      m_list[2])
        assert.equals("endNode",        m_list[3])
        assert.equals("endComponent",   m_list[4])
    end)

    it("emit result is returned from vm.run", function()
        local ir, _ = analyzer.analyze("component C { Text { } }")
        local spy = new_spy()
        spy.emit_result = { files = { { filename = "C.tsx", content = "x" } } }
        local result = vm.run(ir, spy)
        assert.equals("C.tsx", result.files[1].filename)
    end)

    it("nested nodes: parent beginNode comes before child beginNode", function()
        local spy = run("component C { Column { Text { } } }")
        local nodes = all(spy.calls, "beginNode")
        assert.equals("Column", nodes[1].tag)
        assert.equals("Text",   nodes[2].tag)
    end)

    it("nested nodes: child endNode comes before parent endNode", function()
        local spy = run("component C { Column { Text { } } }")
        local ends = all(spy.calls, "endNode")
        assert.equals("Text",   ends[1].tag)
        assert.equals("Column", ends[2].tag)
    end)
end)

-- ============================================================================
-- Property value resolution
-- ============================================================================

describe("color resolution", function()
    it("#rrggbb → r, g, b, a=255", function()
        local spy = run("component C { Column { background: #2563eb; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("color", v.kind)
        assert.equals(0x25,   v.r)
        assert.equals(0x63,   v.g)
        assert.equals(0xeb,   v.b)
        assert.equals(255,    v.a)
    end)

    it("#rgb → doubles each digit, a=255", function()
        local spy = run("component C { Column { background: #fff; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("color", v.kind)
        assert.equals(255, v.r)
        assert.equals(255, v.g)
        assert.equals(255, v.b)
        assert.equals(255, v.a)
    end)

    it("#rrggbbaa → explicit alpha", function()
        local spy = run("component C { Column { background: #ff000080; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("color", v.kind)
        assert.equals(255, v.r)
        assert.equals(0,   v.g)
        assert.equals(0,   v.b)
        assert.equals(0x80, v.a)
    end)
end)

describe("dimension resolution", function()
    it("16dp → kind=dimension, value=16, unit=dp", function()
        local spy = run("component C { Column { padding: 16dp; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("dimension", v.kind)
        assert.equals(16,          v.value)
        assert.equals("dp",        v.unit)
    end)

    it("100% → kind=dimension, value=100, unit=%", function()
        local spy = run("component C { Column { width: 100%; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("dimension", v.kind)
        assert.equals(100,         v.value)
        assert.equals("%",         v.unit)
    end)
end)

describe("other value types", function()
    it("string property", function()
        local spy = run([[component C { Text { content: "Hello"; } }]])
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("string", v.kind)
        assert.equals("Hello",  v.value)
    end)

    it("number property", function()
        local spy = run("component C { Text { opacity: 0.5; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("number", v.kind)
        assert.equals(0.5,      v.value)
    end)

    it("bool property", function()
        local spy = run("component C { Text { visible: true; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("bool",  v.kind)
        assert.is_true(v.value)
    end)

    it("ident property is folded into string", function()
        local spy = run("component C { Column { align: center; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("string", v.kind)
        assert.equals("center", v.value)
    end)

    it("enum property passthrough", function()
        local spy = run("component C { Column { align: Alignment.center; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("enum",      v.kind)
        assert.equals("Alignment", v.namespace)
        assert.equals("center",    v.member)
    end)

    it("slot_ref property resolves with slot type", function()
        local spy = run("component C { slot title: text; Text { content: @title; } }")
        local call = first(spy.calls, "beginNode")
        local v = call.props[1].value
        assert.equals("slot_ref", v.kind)
        assert.equals("title",    v.slot_name)
        assert.equals("text",     v.slot_type.kind)
        assert.is_false(v.is_loop_var)
    end)
end)

-- ============================================================================
-- Slot child rendering
-- ============================================================================

describe("slot child rendering", function()
    it("calls renderSlotChild with slot name and type", function()
        local spy = run("component C { slot hdr: node; Column { @hdr; } }")
        local call = first(spy.calls, "renderSlotChild")
        assert.is_not_nil(call)
        assert.equals("hdr",  call.slot_name)
        assert.equals("node", call.slot_type.kind)
    end)
end)

-- ============================================================================
-- when block
-- ============================================================================

describe("when block", function()
    it("calls beginWhen and endWhen around children", function()
        local spy = run([[
            component C {
              slot show: bool;
              Column { when @show { Text { } } }
            }
        ]])
        local m_list = methods(spy.calls)
        -- Find beginWhen and endWhen positions
        local bw_pos, ew_pos, bn_pos, en_pos
        for i, c in ipairs(spy.calls) do
            if c.method == "beginWhen" then bw_pos = i end
            if c.method == "endWhen"   then ew_pos = i end
            -- The Text node
            if c.method == "beginNode" and c.tag == "Text" then bn_pos = i end
            if c.method == "endNode"   and c.tag == "Text" then en_pos = i end
        end
        assert.truthy(bw_pos)
        assert.truthy(ew_pos)
        assert.truthy(bw_pos < bn_pos)
        assert.truthy(en_pos < ew_pos)
    end)

    it("beginWhen receives the correct slot name", function()
        local spy = run([[
            component C {
              slot show: bool;
              Column { when @show { Text { } } }
            }
        ]])
        local call = first(spy.calls, "beginWhen")
        assert.equals("show", call.slot_name)
    end)
end)

-- ============================================================================
-- each block
-- ============================================================================

describe("each block", function()
    it("calls beginEach and endEach around body", function()
        local spy = run([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        local be_pos, ee_pos, bn_pos
        for i, c in ipairs(spy.calls) do
            if c.method == "beginEach" then be_pos = i end
            if c.method == "endEach"   then ee_pos = i end
            if c.method == "beginNode" and c.tag == "Text" then bn_pos = i end
        end
        assert.truthy(be_pos)
        assert.truthy(ee_pos)
        assert.truthy(be_pos < bn_pos)
        assert.truthy(bn_pos < ee_pos)
    end)

    it("beginEach receives correct slot name, item name, and element type", function()
        local spy = run([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { } } }
            }
        ]])
        local call = first(spy.calls, "beginEach")
        assert.equals("items",  call.slot_name)
        assert.equals("item",   call.item_name)
        assert.equals("text",   call.element_type.kind)
    end)

    it("loop variable @item resolves with is_loop_var=true inside each block", function()
        local spy = run([[
            component C {
              slot items: list<text>;
              Column { each @items as item { Text { content: @item; } } }
            }
        ]])
        -- Find the Text beginNode call (inside the each block)
        local text_call
        for _, c in ipairs(spy.calls) do
            if c.method == "beginNode" and c.tag == "Text" then
                text_call = c
            end
        end
        assert.is_not_nil(text_call)
        local v = text_call.props[1].value
        assert.equals("slot_ref", v.kind)
        assert.equals("item",     v.slot_name)
        assert.is_true(v.is_loop_var)
    end)
end)

-- ============================================================================
-- run_source convenience
-- ============================================================================

describe("run_source", function()
    it("runs end-to-end from source text", function()
        local spy = new_spy()
        local result, err = vm.run_source("component C { Text { } }", spy)
        assert.is_nil(err)
        assert.is_not_nil(result)
    end)

    it("returns nil, errmsg on bad source", function()
        local spy = new_spy()
        local result, err = vm.run_source("not valid", spy)
        assert.is_nil(result)
        assert.is_string(err)
    end)
end)

-- ============================================================================
-- Error handling
-- ============================================================================

describe("error handling", function()
    it("vm.run returns nil, errmsg on invalid IR (bad slot ref)", function()
        -- Build an IR with a slot_ref property that references a nonexistent slot
        local ir = {
            component = {
                name  = "C",
                slots = {},
                tree  = {
                    tag          = "Text",
                    is_primitive = true,
                    properties   = { { name = "content", value = { kind = "slot_ref", slot_name = "missing" } } },
                    children     = {},
                },
            },
            imports = {},
        }
        local spy = new_spy()
        local result, err = vm.run(ir, spy)
        assert.is_nil(result)
        assert.is_string(err)
        assert.truthy(err:find("unresolved") or err:find("missing"))
    end)
end)
