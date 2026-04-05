-- mosaic_vm — Generic tree-walking driver for Mosaic compiler backends
-- =====================================================================
--
-- # What is the MosaicVM?
--
-- The VM is the fourth stage of the Mosaic compiler pipeline:
--
--   Source text → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend → Output
--
-- The VM's responsibilities:
--
--   1. Traverse the MosaicIR tree depth-first.
--   2. Normalize every MosaicValue into a ResolvedValue:
--      - color_hex → parsed RGBA integers { r, g, b, a }
--      - dimension → already has { value, unit } from the analyzer
--      - ident     → folded into string
--      - slot_ref  → enriched with slot type info and loop-variable flag
--   3. Track the SlotContext (component slots + active each-loop scopes).
--   4. Call renderer methods in strict open-before-close order.
--
-- The VM does NOT know anything about output format. It has no knowledge of
-- React, Web Components, or any other platform. The renderer owns the output.
-- This mirrors the design of the JVM/CLR: the VM drives traversal; the
-- platform handles platform-specific behavior.
--
-- # Traversal Order
--
--   beginComponent(name, slots)
--     beginNode(tag, is_primitive, resolved_props, ctx)
--       [for each child:]
--         beginNode/endNode   ← child nodes
--         renderSlotChild     ← @slotName; children
--         beginWhen/endWhen   ← when blocks
--         beginEach/endEach   ← each blocks
--     endNode(tag)
--   endComponent()
--   emit() → result
--
-- # Renderer Interface (duck-typed)
--
-- The renderer is any Lua table with these method fields:
--
--   renderer:beginComponent(name, slots)
--   renderer:endComponent()
--   renderer:beginNode(tag, is_primitive, resolved_props, ctx)
--   renderer:endNode(tag)
--   renderer:renderSlotChild(slot_name, slot_type, ctx)
--   renderer:beginWhen(slot_name, ctx)
--   renderer:endWhen()
--   renderer:beginEach(slot_name, item_name, element_type, ctx)
--   renderer:endEach()
--   renderer:emit() → result
--
-- # Color Parsing
--
-- Hex colors use these expansion rules:
--   #rgb      → r=rr, g=gg, b=bb, a=255  (digit doubled)
--   #rrggbb   → r, g, b, a=255
--   #rrggbbaa → r, g, b, a
--
-- # SlotContext
--
-- The context tracks component slots and active each-loop scopes:
--
--   ctx = {
--     component_slots = { name → MosaicSlot },   (lookup table)
--     loop_scopes     = { { item_name, element_type }, ... },
--   }
--
-- Loop scopes are checked innermost-first for slot_ref resolution.

local mosaic_analyzer = require("coding_adventures.mosaic_analyzer")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Color Parsing
-- ============================================================================

--- Parse a hex color string into RGBA integer components.
--
-- Three forms are supported:
--   #rgb     → doubles each digit (e.g. #f0a → r=255, g=0, b=170)
--   #rrggbb  → 8-bit channels, alpha defaults to 255
--   #rrggbbaa → all four channels explicit
--
-- @param hex string  e.g. "#2563eb" or "#fff"
-- @return table      { kind="color", r, g, b, a }
local function parse_color(hex)
    local h = hex:sub(2)  -- strip leading '#'
    local r, g, b, a

    if #h == 3 then
        -- Three-digit shorthand: each hex digit is doubled
        -- e.g. #fff → r=255, g=255, b=255
        r = tonumber(h:sub(1,1) .. h:sub(1,1), 16)
        g = tonumber(h:sub(2,2) .. h:sub(2,2), 16)
        b = tonumber(h:sub(3,3) .. h:sub(3,3), 16)
        a = 255
    elseif #h == 6 then
        r = tonumber(h:sub(1,2), 16)
        g = tonumber(h:sub(3,4), 16)
        b = tonumber(h:sub(5,6), 16)
        a = 255
    elseif #h == 8 then
        r = tonumber(h:sub(1,2), 16)
        g = tonumber(h:sub(3,4), 16)
        b = tonumber(h:sub(5,6), 16)
        a = tonumber(h:sub(7,8), 16)
    else
        error("mosaic_vm: invalid color hex: " .. hex)
    end

    return { kind = "color", r = r, g = g, b = b, a = a }
end

-- ============================================================================
-- Value Resolution
-- ============================================================================

--- Resolve a MosaicValue into a ResolvedValue, given the current context.
--
-- ResolvedValue kinds:
--   string    → { kind="string",    value }
--   number    → { kind="number",    value }
--   bool      → { kind="bool",      value }
--   dimension → { kind="dimension", value, unit }
--   color     → { kind="color",     r, g, b, a }  (parsed from color_hex)
--   slot_ref  → { kind="slot_ref",  slot_name, slot_type, is_loop_var }
--   enum      → { kind="enum",      namespace, member }
--
-- Note: ident values are folded into string.
--
-- @param v   table  MosaicValue from the IR.
-- @param ctx table  Current SlotContext.
-- @return table     ResolvedValue.
local function resolve_value(v, ctx)
    local k = v.kind
    if k == "string" then
        return { kind = "string", value = v.value }
    elseif k == "number" then
        return { kind = "number", value = v.value }
    elseif k == "bool" then
        return { kind = "bool", value = v.value }
    elseif k == "ident" then
        -- Bare identifiers used as property values are strings at runtime
        return { kind = "string", value = v.value }
    elseif k == "dimension" then
        return { kind = "dimension", value = v.value, unit = v.unit }
    elseif k == "color_hex" then
        return parse_color(v.value)
    elseif k == "enum" then
        return { kind = "enum", namespace = v.namespace, member = v.member }
    elseif k == "slot_ref" then
        -- Check loop scopes innermost-first
        for i = #ctx.loop_scopes, 1, -1 do
            local scope = ctx.loop_scopes[i]
            if scope.item_name == v.slot_name then
                return {
                    kind        = "slot_ref",
                    slot_name   = v.slot_name,
                    slot_type   = scope.element_type,
                    is_loop_var = true,
                }
            end
        end
        -- Fall back to component slots
        local slot = ctx.component_slots[v.slot_name]
        if not slot then
            error("mosaic_vm: unresolved slot reference: @" .. v.slot_name)
        end
        return {
            kind        = "slot_ref",
            slot_name   = v.slot_name,
            slot_type   = slot.type,
            is_loop_var = false,
        }
    else
        error("mosaic_vm: unknown value kind: " .. tostring(k))
    end
end

-- ============================================================================
-- Tree Walking
-- ============================================================================

-- Forward declaration for mutual recursion (walk_node → walk_child → walk_node)
local walk_node

--- Walk a single child item, dispatching to the appropriate renderer method.
-- @param child    table  MosaicChild from the IR.
-- @param ctx      table  Current SlotContext.
-- @param renderer table  The backend renderer.
local function walk_child(child, ctx, renderer)
    local kind = child.kind

    if kind == "node" then
        walk_node(child.node, ctx, renderer)

    elseif kind == "slot_ref" then
        -- @slotName; used as a child element.
        -- Look up the slot type so the renderer knows what kind of content to project.
        local slot = ctx.component_slots[child.slot_name]
        if not slot then
            error("mosaic_vm: unknown slot in slot_ref child: @" .. child.slot_name)
        end
        renderer:renderSlotChild(child.slot_name, slot.type, ctx)

    elseif kind == "when" then
        -- Conditional block: when @show { ... }
        renderer:beginWhen(child.slot_name, ctx)
        for _, c in ipairs(child.children) do
            walk_child(c, ctx, renderer)
        end
        renderer:endWhen()

    elseif kind == "each" then
        -- Iteration block: each @items as item { ... }
        -- 1. Find the list slot to determine the element type.
        local list_slot = ctx.component_slots[child.slot_name]
        if not list_slot then
            error("mosaic_vm: unknown list slot: @" .. child.slot_name)
        end
        if list_slot.type.kind ~= "list" then
            error(("mosaic_vm: @%s is not a list slot"):format(child.slot_name))
        end
        local element_type = list_slot.type.element_type

        -- 2. Tell the renderer we're beginning an each block.
        renderer:beginEach(child.slot_name, child.item_name, element_type, ctx)

        -- 3. Push a loop scope so @item references resolve inside the block.
        local inner_ctx = {
            component_slots = ctx.component_slots,
            loop_scopes = {},
        }
        -- Copy existing scopes
        for _, s in ipairs(ctx.loop_scopes) do
            inner_ctx.loop_scopes[#inner_ctx.loop_scopes + 1] = s
        end
        -- Add the new scope
        inner_ctx.loop_scopes[#inner_ctx.loop_scopes + 1] = {
            item_name    = child.item_name,
            element_type = element_type,
        }

        -- 4. Walk the each body with the updated context.
        for _, c in ipairs(child.children) do
            walk_child(c, inner_ctx, renderer)
        end

        renderer:endEach()
    else
        error("mosaic_vm: unknown child kind: " .. tostring(kind))
    end
end

--- Walk a single MosaicNode: resolve properties, call beginNode, walk children, call endNode.
-- @param node     table  MosaicNode from the IR.
-- @param ctx      table  Current SlotContext.
-- @param renderer table  The backend renderer.
walk_node = function(node, ctx, renderer)
    -- Resolve all properties before calling beginNode so the renderer receives
    -- fully normalized values without needing to parse hex or split dimensions.
    local resolved = {}
    for _, prop in ipairs(node.properties or {}) do
        resolved[#resolved + 1] = {
            name  = prop.name,
            value = resolve_value(prop.value, ctx),
        }
    end

    renderer:beginNode(node.tag, node.is_primitive, resolved, ctx)

    for _, child in ipairs(node.children or {}) do
        walk_child(child, ctx, renderer)
    end

    renderer:endNode(node.tag)
end

-- ============================================================================
-- Public API
-- ============================================================================

--- Run the MosaicVM against a renderer.
--
-- Traverses the IR tree depth-first, calling renderer methods in strict
-- open-before-close order. Returns whatever renderer:emit() returns.
--
-- @param ir       table  MosaicIR from mosaic_analyzer.
-- @param renderer table  A backend renderer (duck-typed interface).
-- @return any            The result of renderer:emit().
-- @return nil|string     nil on success, error message on failure.
--
-- Example:
--
--   local vm    = require("coding_adventures.mosaic_vm")
--   local react = require("coding_adventures.mosaic_emit_react")
--
--   local ir, err = require("coding_adventures.mosaic_analyzer").analyze(source)
--   local renderer = react.new_renderer()
--   local result, vm_err = vm.run(ir, renderer)
--   -- result.files[1].filename → "MyComponent.tsx"
function M.run(ir, renderer)
    local ok, result = pcall(function()
        local comp = ir.component

        -- Build the initial SlotContext from the component's slot declarations.
        -- component_slots is a lookup table: name → MosaicSlot.
        local component_slots = {}
        for _, slot in ipairs(comp.slots or {}) do
            component_slots[slot.name] = slot
        end

        local ctx = {
            component_slots = component_slots,
            loop_scopes     = {},
        }

        renderer:beginComponent(comp.name, comp.slots or {})
        walk_node(comp.tree, ctx, renderer)
        renderer:endComponent()
        return renderer:emit()
    end)

    if ok then
        return result, nil
    else
        return nil, result
    end
end

--- Analyze source and run against a renderer in one step.
--
-- Convenience wrapper around mosaic_analyzer.analyze() + mosaic_vm.run().
--
-- @param source   string  Mosaic source text.
-- @param renderer table   A backend renderer.
-- @return any             The result of renderer:emit(), or nil on error.
-- @return nil|string      Error message on failure.
function M.run_source(source, renderer)
    local ir, err = mosaic_analyzer.analyze(source)
    if not ir then
        return nil, err
    end
    return M.run(ir, renderer)
end

return M
