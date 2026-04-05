-- mosaic_emit_react — React TSX backend for the Mosaic compiler
-- ==============================================================
--
-- # What does this emit?
--
-- Given a Mosaic component like:
--
--   component ProfileCard {
--     slot name: text;
--     slot avatar: image;
--     slot active: bool = false;
--
--     Column {
--       Image { source: @avatar; shape: circle; }
--       Text { content: @name; }
--       when @active {
--         Text { content: "Online"; color: #22c55e; }
--       }
--     }
--   }
--
-- This emitter produces a ProfileCard.tsx file like:
--
--   // AUTO-GENERATED from ProfileCard.mosaic — do not edit
--   import React from "react";
--
--   interface ProfileCardProps {
--     name: string;
--     avatar: string;
--     active?: boolean; // default: false
--   }
--
--   export function ProfileCard({
--     name,
--     avatar,
--     active = false,
--   }: ProfileCardProps): JSX.Element {
--     return (
--       <div style={{ display: "flex", flexDirection: "column" }}>
--         <img src={avatar} style={{ borderRadius: "50%" }} />
--         <span>{name}</span>
--         {active && (
--           <span style={{ color: "rgba(34, 197, 94, 1)" }}>Online</span>
--         )}
--       </div>
--     );
--   }
--
-- # Architecture: String Stack
--
-- The renderer maintains a stack of frames. Each open node pushes a new frame;
-- endNode pops the frame and builds the JSX string. Children accumulate in the
-- parent frame's `lines` list. This handles arbitrary nesting without lookahead.
--
-- # Primitive Node → JSX Element Mapping
--
--   Box     → <div style={{ position: "relative" }}>
--   Column  → <div style={{ display: "flex", flexDirection: "column" }}>
--   Row     → <div style={{ display: "flex", flexDirection: "row" }}>
--   Text    → <span> (or <h2> if a11y-role: heading)
--   Image   → <img ... /> (self-closing)
--   Spacer  → <div style={{ flex: 1 }}>
--   Scroll  → <div style={{ overflow: "auto" }}>
--   Divider → <hr ... /> (self-closing)
--   Icon    → <span> (placeholder)
--   Stack   → <div style={{ position: "relative" }}>
--
-- # API
--
--   local renderer = MosaicEmitReact.new_renderer()
--   local vm = require("coding_adventures.mosaic_vm")
--   local result, err = vm.run(ir, renderer)
--   -- result.files[1].filename → "MyComponent.tsx"
--   -- result.files[1].content  → "// AUTO-GENERATED ..."
--
-- Convenience one-shot:
--   local result, err = MosaicEmitReact.emit(source)

local mosaic_vm       = require("coding_adventures.mosaic_vm")
local mosaic_analyzer = require("coding_adventures.mosaic_analyzer")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Helpers
-- ============================================================================

--- Format an RGBA color as a CSS rgba() string.
-- Alpha is normalized from 0–255 to 0–1.
-- @param r number  Red   0–255
-- @param g number  Green 0–255
-- @param b number  Blue  0–255
-- @param a number  Alpha 0–255
-- @return string   e.g. "rgba(37, 99, 235, 1)"
local function rgba(r, g, b, a)
    local alpha = math.floor((a / 255) * 1000 + 0.5) / 1000
    return ("rgba(%d, %d, %d, %s)"):format(r, g, b, alpha)
end

--- Convert a ResolvedValue dimension to a CSS size string.
-- Both dp and sp map to px on the web.
-- @param value table  ResolvedValue
-- @return string|nil  e.g. "16px", "100%", or nil if not a dimension
local function dim(value)
    if value.kind ~= "dimension" then return nil end
    if value.unit == "%" then
        return ("%g%%"):format(value.value)
    end
    return ("%gpx"):format(value.value)
end

--- Convert a size ResolvedValue to a CSS string.
-- Handles fill → "100%", wrap → "fit-content", and dimensions.
local function size_value(value)
    if value.kind == "string" then
        if value.value == "fill" then return "100%" end
        if value.value == "wrap" then return "fit-content" end
        return value.value
    end
    return dim(value) or "auto"
end

--- Convert a ResolvedValue to a JSX children expression (for Text content).
local function value_to_jsx(value)
    if value.kind == "string" then return value.value end
    if value.kind == "number" then return tostring(value.value) end
    if value.kind == "bool"   then return tostring(value.value) end
    if value.kind == "slot_ref" then return ("{%s}"):format(value.slot_name) end
    return ""
end

--- Convert a ResolvedValue to a JSX attribute value (for src=, aria-label=, etc.).
local function attr_value(value)
    if value.kind == "string"   then return ('"%s"'):format(value.value) end
    if value.kind == "slot_ref" then return ("{%s}"):format(value.slot_name) end
    return '""'
end

--- Convert a MosaicType (from the IR) to a TypeScript type string.
local function slot_type_to_ts(t)
    if t.kind == "text"      then return "string" end
    if t.kind == "number"    then return "number" end
    if t.kind == "bool"      then return "boolean" end
    if t.kind == "image"     then return "string" end
    if t.kind == "color"     then return "string" end
    if t.kind == "node"      then return "React.ReactNode" end
    if t.kind == "component" then return ("React.ReactElement<%sProps>"):format(t.name) end
    if t.kind == "list" then
        local inner = t.element_type
        if inner.kind == "text"      then return "string[]" end
        if inner.kind == "number"    then return "number[]" end
        if inner.kind == "bool"      then return "boolean[]" end
        if inner.kind == "image"     then return "string[]" end
        if inner.kind == "color"     then return "string[]" end
        if inner.kind == "node"      then return "React.ReactNode[]" end
        if inner.kind == "component" then return ("Array<React.ReactElement<%sProps>>"):format(inner.name) end
        return "unknown[]"
    end
    return "unknown"
end

--- Convert a MosaicValue default to a TypeScript literal string.
local function default_value_literal(v)
    if v.kind == "string" then return ('"%s"'):format(v.value) end
    if v.kind == "number" then return tostring(v.value) end
    if v.kind == "bool"   then
        return v.value and "true" or "false"
    end
    return "undefined"
end

-- ============================================================================
-- Renderer
-- ============================================================================

--- Create a new React renderer instance.
-- @return table  A renderer conforming to the mosaic_vm renderer interface.
function M.new_renderer()
    local self = {
        _component_name      = "",
        _slots               = {},
        _stack               = {},
        _slot_component_imports = {},  -- set of component names from slot types
        _node_component_imports = {},  -- set of component names from node tags
        _needs_typescale_css = false,
    }

    -- -----------------------------------------------------------------------
    -- MosaicRenderer interface
    -- -----------------------------------------------------------------------

    function self:beginComponent(name, slots)
        self._component_name      = name
        self._slots               = slots
        self._stack               = { { kind = "component", lines = {} } }
        self._slot_component_imports = {}
        self._node_component_imports = {}
        self._needs_typescale_css = false

        -- Pre-scan slots for component-type imports
        for _, slot in ipairs(slots) do
            if slot.type.kind == "component" then
                self._slot_component_imports[slot.type.name] = true
            elseif slot.type.kind == "list" and slot.type.element_type.kind == "component" then
                self._slot_component_imports[slot.type.element_type.name] = true
            end
        end
    end

    function self:endComponent()
        -- no-op: endNode has already fired for root node
    end

    function self:emit()
        local content = self:_build_file()
        return { files = { { filename = self._component_name .. ".tsx", content = content } } }
    end

    function self:beginNode(tag, is_primitive, properties, ctx)
        local frame = self:_build_node_frame(tag, is_primitive, properties)
        self._stack[#self._stack + 1] = frame
    end

    function self:endNode(_tag)
        local frame = table.remove(self._stack)
        local jsx = self:_build_jsx_element(frame)
        self:_append_to_parent(jsx)
    end

    function self:renderSlotChild(slot_name, _slot_type, _ctx)
        self:_append_to_parent(("{%s}"):format(slot_name))
    end

    function self:beginWhen(slot_name, _ctx)
        self._stack[#self._stack + 1] = { kind = "when", slot_name = slot_name, lines = {} }
    end

    function self:endWhen()
        local frame = table.remove(self._stack)
        local children = frame.lines

        local body
        if #children == 1 then
            body = children[1]
        else
            local inner = {}
            for _, l in ipairs(children) do inner[#inner + 1] = "  " .. l end
            body = "<>\n" .. table.concat(inner, "\n") .. "\n</>"
        end

        -- Indent the body inside the conditional
        local indented_lines = {}
        for line in (body .. "\n"):gmatch("([^\n]*)\n") do
            indented_lines[#indented_lines + 1] = "  " .. line
        end
        local jsx = ("{%s && (\n%s\n)}"):format(
            frame.slot_name,
            table.concat(indented_lines, "\n")
        )
        self:_append_to_parent(jsx)
    end

    function self:beginEach(slot_name, item_name, _element_type, _ctx)
        self._stack[#self._stack + 1] = {
            kind      = "each",
            slot_name = slot_name,
            item_name = item_name,
            lines     = {},
        }
    end

    function self:endEach()
        local frame = table.remove(self._stack)
        local body_lines = {}
        for _, l in ipairs(frame.lines) do
            body_lines[#body_lines + 1] = "    " .. l
        end
        local jsx = (
            "{%s.map((%s, _index) => (\n" ..
            "  <React.Fragment key={_index}>\n" ..
            "%s\n" ..
            "  </React.Fragment>\n" ..
            "))}"
        ):format(
            frame.slot_name,
            frame.item_name,
            table.concat(body_lines, "\n")
        )
        self:_append_to_parent(jsx)
    end

    -- -----------------------------------------------------------------------
    -- Internal helpers
    -- -----------------------------------------------------------------------

    function self:_append_to_parent(content)
        local top = self._stack[#self._stack]
        if top then
            top.lines[#top.lines + 1] = content
        end
    end

    --- Build a NodeFrame for the given tag and resolved properties.
    function self:_build_node_frame(tag, is_primitive, properties)
        local styles      = {}
        local style_order = {}  -- preserve insertion order for consistent output
        local attrs       = {}
        local class_names = {}
        local text_content = nil
        local self_closing = false

        -- Helper to add a style entry
        local function add_style(k, v)
            if not styles[k] then
                style_order[#style_order + 1] = k
            end
            styles[k] = v
        end

        -- Base JSX tag and initial styles from primitive type
        local jsx_tag
        if is_primitive then
            if tag == "Box" then
                jsx_tag = "div"
                add_style("position", '"relative"')
            elseif tag == "Column" then
                jsx_tag = "div"
                add_style("display", '"flex"')
                add_style("flexDirection", '"column"')
            elseif tag == "Row" then
                jsx_tag = "div"
                add_style("display", '"flex"')
                add_style("flexDirection", '"row"')
            elseif tag == "Text" then
                jsx_tag = "span"
            elseif tag == "Image" then
                jsx_tag = "img"
                self_closing = true
            elseif tag == "Spacer" then
                jsx_tag = "div"
                add_style("flex", "1")
            elseif tag == "Scroll" then
                jsx_tag = "div"
                add_style("overflow", '"auto"')
            elseif tag == "Divider" then
                jsx_tag = "hr"
                self_closing = true
                add_style("border", '"none"')
                add_style("borderTop", '"1px solid currentColor"')
            elseif tag == "Icon" then
                jsx_tag = "span"
            elseif tag == "Stack" then
                jsx_tag = "div"
                add_style("position", '"relative"')
            else
                jsx_tag = "div"
            end
        else
            jsx_tag = tag
            self._node_component_imports[tag] = true
        end

        -- Apply each resolved property
        for _, prop in ipairs(properties) do
            local name  = prop.name
            local value = prop.value

            if name == "padding" then
                local d = dim(value)
                if d then add_style("padding", ('"' .. d .. '"')) end

            elseif name == "padding-left" then
                local d = dim(value)
                if d then add_style("paddingLeft", ('"' .. d .. '"')) end

            elseif name == "padding-right" then
                local d = dim(value)
                if d then add_style("paddingRight", ('"' .. d .. '"')) end

            elseif name == "padding-top" then
                local d = dim(value)
                if d then add_style("paddingTop", ('"' .. d .. '"')) end

            elseif name == "padding-bottom" then
                local d = dim(value)
                if d then add_style("paddingBottom", ('"' .. d .. '"')) end

            elseif name == "gap" then
                local d = dim(value)
                if d then add_style("gap", ('"' .. d .. '"')) end

            elseif name == "width" then
                add_style("width", ('"' .. size_value(value) .. '"'))

            elseif name == "height" then
                add_style("height", ('"' .. size_value(value) .. '"'))

            elseif name == "min-width" then
                local d = dim(value)
                if d then add_style("minWidth", ('"' .. d .. '"')) end

            elseif name == "max-width" then
                local d = dim(value)
                if d then add_style("maxWidth", ('"' .. d .. '"')) end

            elseif name == "min-height" then
                local d = dim(value)
                if d then add_style("minHeight", ('"' .. d .. '"')) end

            elseif name == "max-height" then
                local d = dim(value)
                if d then add_style("maxHeight", ('"' .. d .. '"')) end

            elseif name == "overflow" then
                if value.kind == "string" then
                    local overflow_map = { visible="visible", hidden="hidden", scroll="auto" }
                    local v = overflow_map[value.value]
                    if v then add_style("overflow", ('"' .. v .. '"')) end
                end

            elseif name == "align" then
                if value.kind == "string" then
                    self:_apply_align(value.value, tag, styles, style_order)
                end

            elseif name == "background" then
                if value.kind == "color" then
                    add_style("backgroundColor", ('"' .. rgba(value.r, value.g, value.b, value.a) .. '"'))
                end

            elseif name == "corner-radius" then
                local d = dim(value)
                if d then add_style("borderRadius", ('"' .. d .. '"')) end

            elseif name == "border-width" then
                local d = dim(value)
                if d then
                    add_style("borderWidth", ('"' .. d .. '"'))
                    add_style("borderStyle", '"solid"')
                end

            elseif name == "border-color" then
                if value.kind == "color" then
                    add_style("borderColor", ('"' .. rgba(value.r, value.g, value.b, value.a) .. '"'))
                end

            elseif name == "opacity" then
                if value.kind == "number" then
                    add_style("opacity", tostring(value.value))
                end

            elseif name == "shadow" then
                if value.kind == "enum" and value.namespace == "elevation" then
                    local shadow_map = {
                        none   = "none",
                        low    = "0 1px 3px rgba(0,0,0,0.12)",
                        medium = "0 4px 12px rgba(0,0,0,0.15)",
                        high   = "0 8px 24px rgba(0,0,0,0.20)",
                    }
                    local s = shadow_map[value.member]
                    if s then add_style("boxShadow", ('"' .. s .. '"')) end
                end

            elseif name == "visible" then
                if value.kind == "bool" and not value.value then
                    add_style("display", '"none"')
                end

            elseif name == "content" then
                if tag == "Text" then
                    text_content = value_to_jsx(value)
                end

            elseif name == "color" then
                if value.kind == "color" then
                    add_style("color", ('"' .. rgba(value.r, value.g, value.b, value.a) .. '"'))
                end

            elseif name == "text-align" then
                if value.kind == "string" then
                    local ta_map = { start="left", center="center", ["end"]="right" }
                    local v = ta_map[value.value]
                    if v then add_style("textAlign", ('"' .. v .. '"')) end
                end

            elseif name == "font-weight" then
                if value.kind == "string" then
                    add_style("fontWeight", ('"' .. value.value .. '"'))
                end

            elseif name == "style" then
                if value.kind == "enum" then
                    class_names[#class_names + 1] = ("mosaic-%s-%s"):format(value.namespace, value.member)
                    self._needs_typescale_css = true
                elseif value.kind == "string" then
                    class_names[#class_names + 1] = ("mosaic-%s"):format(value.value)
                    self._needs_typescale_css = true
                end

            elseif name == "source" then
                if tag == "Image" then
                    attrs[#attrs + 1] = ("src=%s"):format(attr_value(value))
                end

            elseif name == "size" then
                local d = dim(value)
                if d and tag == "Image" then
                    add_style("width", ('"' .. d .. '"'))
                    add_style("height", ('"' .. d .. '"'))
                end

            elseif name == "shape" then
                if tag == "Image" and value.kind == "string" then
                    local shape_map = { circle = "50%", rounded = "8px" }
                    local r = shape_map[value.value]
                    if r then add_style("borderRadius", ('"' .. r .. '"')) end
                end

            elseif name == "fit" then
                if tag == "Image" and value.kind == "string" then
                    add_style("objectFit", ('"' .. value.value .. '"'))
                end

            elseif name == "a11y-label" then
                attrs[#attrs + 1] = ("aria-label=%s"):format(attr_value(value))

            elseif name == "a11y-role" then
                if value.kind == "string" then
                    if value.value == "none" then
                        attrs[#attrs + 1] = 'aria-hidden="true"'
                    elseif value.value == "heading" then
                        attrs[#attrs + 1] = 'role="heading"'
                    elseif value.value == "image" then
                        attrs[#attrs + 1] = 'role="img"'
                    else
                        attrs[#attrs + 1] = ('role="%s"'):format(value.value)
                    end
                end

            elseif name == "a11y-hidden" then
                if value.kind == "bool" and value.value then
                    attrs[#attrs + 1] = 'aria-hidden="true"'
                end
            end
        end

        -- Post-process: a11y-role: heading → <h2> for Text
        if tag == "Text" then
            for i, attr in ipairs(attrs) do
                if attr == 'role="heading"' then
                    jsx_tag = "h2"
                    table.remove(attrs, i)
                    break
                end
            end
        end

        return {
            kind         = "node",
            tag          = tag,
            jsx_tag      = jsx_tag,
            styles       = styles,
            style_order  = style_order,
            attrs        = attrs,
            class_names  = class_names,
            text_content = text_content,
            self_closing = self_closing,
            lines        = {},
        }
    end

    --- Apply the Mosaic `align` property to the styles table.
    function self:_apply_align(align_value, tag, styles, style_order)
        local function add(k, v)
            if not styles[k] then style_order[#style_order + 1] = k end
            styles[k] = v
        end

        if tag == "Box" then
            add("display", '"flex"')
        end

        if tag == "Column" then
            if align_value == "start"             then add("alignItems", '"flex-start"')
            elseif align_value == "center"        then add("alignItems", '"center"')
            elseif align_value == "end"           then add("alignItems", '"flex-end"')
            elseif align_value == "stretch"       then add("alignItems", '"stretch"')
            elseif align_value == "center-horizontal" then add("alignItems", '"center"')
            elseif align_value == "center-vertical"   then add("justifyContent", '"center"')
            end
        elseif tag == "Row" then
            if align_value == "start"             then add("alignItems", '"flex-start"')
            elseif align_value == "center"        then
                add("alignItems", '"center"')
                add("justifyContent", '"center"')
            elseif align_value == "end"           then
                add("alignItems", '"flex-end"')
                add("justifyContent", '"flex-end"')
            elseif align_value == "stretch"       then add("alignItems", '"stretch"')
            elseif align_value == "center-horizontal" then add("justifyContent", '"center"')
            elseif align_value == "center-vertical"   then add("alignItems", '"center"')
            end
        elseif tag == "Box" then
            if align_value == "start"             then add("alignItems", '"flex-start"')
            elseif align_value == "center"        then add("alignItems", '"center"')
            elseif align_value == "end"           then add("alignItems", '"flex-end"')
            elseif align_value == "stretch"       then add("alignItems", '"stretch"')
            elseif align_value == "center-horizontal" then add("alignItems", '"center"')
            elseif align_value == "center-vertical"   then add("justifyContent", '"center"')
            end
        end
    end

    --- Convert a completed NodeFrame to a JSX element string.
    function self:_build_jsx_element(frame)
        local jsx_tag     = frame.jsx_tag
        local styles      = frame.styles
        local style_order = frame.style_order
        local attrs       = frame.attrs
        local class_names = frame.class_names
        local text_content = frame.text_content
        local self_closing = frame.self_closing
        local lines       = frame.lines

        -- Build JSX attribute parts
        local parts = {}

        if #style_order > 0 then
            local entries = {}
            for _, k in ipairs(style_order) do
                entries[#entries + 1] = k .. ": " .. styles[k]
            end
            parts[#parts + 1] = "style={{ " .. table.concat(entries, ", ") .. " }}"
        end

        if #class_names > 0 then
            parts[#parts + 1] = 'className="' .. table.concat(class_names, " ") .. '"'
        end

        for _, attr in ipairs(attrs) do
            parts[#parts + 1] = attr
        end

        local attr_str = #parts > 0 and (" " .. table.concat(parts, " ")) or ""

        if self_closing then
            return ("<" .. jsx_tag .. attr_str .. " />")
        end

        local children
        if text_content ~= nil then
            children = text_content
        else
            children = table.concat(lines, "\n")
        end

        if not children or children == "" then
            return ("<" .. jsx_tag .. attr_str .. " />")
        end

        if text_content ~= nil then
            -- Inline text: <span style={{...}}>{title}</span>
            return ("<" .. jsx_tag .. attr_str .. ">" .. children .. "</" .. jsx_tag .. ">")
        end

        -- Block children: indent each child line by 2 spaces
        local indented_parts = {}
        for line in (children .. "\n"):gmatch("([^\n]*)\n") do
            indented_parts[#indented_parts + 1] = "  " .. line
        end
        return (
            "<" .. jsx_tag .. attr_str .. ">\n" ..
            table.concat(indented_parts, "\n") ..
            "\n</" .. jsx_tag .. ">"
        )
    end

    --- Assemble the complete .tsx file content.
    function self:_build_file()
        local name = self._component_name

        -- Prop interface lines and function parameter lines
        local prop_lines  = {}
        local param_lines = {}

        for _, slot in ipairs(self._slots) do
            local ts_type = slot_type_to_ts(slot.type)
            local optional = slot.default_value and "?" or ""
            local comment  = ""
            if slot.default_value then
                comment = " // default: " .. default_value_literal(slot.default_value)
            end
            prop_lines[#prop_lines + 1] = ("  %s%s: %s;%s"):format(
                slot.name, optional, ts_type, comment
            )

            if slot.default_value then
                param_lines[#param_lines + 1] = ("  %s = %s,"):format(
                    slot.name, default_value_literal(slot.default_value)
                )
            else
                param_lines[#param_lines + 1] = ("  %s,"):format(slot.name)
            end
        end

        -- Collect import lines
        local import_lines = {}

        -- Slot-type component imports (sorted for determinism)
        local slot_imports = {}
        for comp_name in pairs(self._slot_component_imports) do
            slot_imports[#slot_imports + 1] = comp_name
        end
        table.sort(slot_imports)
        for _, comp_name in ipairs(slot_imports) do
            import_lines[#import_lines + 1] = ('import type { %sProps } from "./%s.js";'):format(comp_name, comp_name)
        end

        -- Node-level component imports (sorted)
        local node_imports = {}
        for comp_name in pairs(self._node_component_imports) do
            node_imports[#node_imports + 1] = comp_name
        end
        table.sort(node_imports)
        for _, comp_name in ipairs(node_imports) do
            import_lines[#import_lines + 1] = ('import { %s } from "./%s.js";'):format(comp_name, comp_name)
        end

        -- Root JSX content
        local root_lines = self._stack[1].lines
        local root_jsx = table.concat(root_lines, "\n")
        -- Indent 4 spaces (2 for return body, 2 for JSX root)
        local indented_root_parts = {}
        for line in (root_jsx .. "\n"):gmatch("([^\n]*)\n") do
            indented_root_parts[#indented_root_parts + 1] = "    " .. line
        end
        local indented_root = table.concat(indented_root_parts, "\n")

        -- Assemble the file
        local lines = {
            ("// AUTO-GENERATED from %s.mosaic — do not edit"):format(name),
            "// Generated by mosaic-emit-react v1.0",
            ("// Source: %s.mosaic"):format(name),
            "//",
            ("// To modify this component, edit %s.mosaic and re-run the compiler."):format(name),
            "",
            'import React from "react";',
        }

        if self._needs_typescale_css then
            lines[#lines + 1] = 'import "./mosaic-type-scale.css";'
        end

        if #import_lines > 0 then
            lines[#lines + 1] = ""
            for _, l in ipairs(import_lines) do
                lines[#lines + 1] = l
            end
        end

        lines[#lines + 1] = ""
        lines[#lines + 1] = ("interface %sProps {"):format(name)
        for _, l in ipairs(prop_lines) do lines[#lines + 1] = l end
        lines[#lines + 1] = "}"
        lines[#lines + 1] = ""
        lines[#lines + 1] = ("export function %s({"):format(name)
        for _, l in ipairs(param_lines) do lines[#lines + 1] = l end
        lines[#lines + 1] = ("}: %sProps): JSX.Element {"):format(name)
        lines[#lines + 1] = "  return ("
        lines[#lines + 1] = indented_root
        lines[#lines + 1] = "  );"
        lines[#lines + 1] = "}"

        return table.concat(lines, "\n")
    end

    return self
end

-- ============================================================================
-- Convenience API
-- ============================================================================

--- Emit a React TSX component from Mosaic source text.
--
-- One-shot convenience: analyzes the source and runs the React renderer.
--
-- @param source string  Mosaic source text.
-- @return table|nil     { files = { { filename, content } } }, or nil on error.
-- @return nil|string    Error message on failure.
--
-- Example:
--
--   local emit = require("coding_adventures.mosaic_emit_react")
--   local result, err = emit.emit([[
--     component Label {
--       slot text: text;
--       Text { content: @text; }
--     }
--   ]])
--   -- result.files[1].filename → "Label.tsx"
function M.emit(source)
    local ir, err = mosaic_analyzer.analyze(source)
    if not ir then
        return nil, err
    end
    local renderer = M.new_renderer()
    return mosaic_vm.run(ir, renderer)
end

return M
