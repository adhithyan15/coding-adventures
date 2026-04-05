-- mosaic_emit_webcomponent — Web Components backend for the Mosaic compiler
-- ===========================================================================
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
--       Image { source: @avatar; }
--       Text { content: @name; }
--     }
--   }
--
-- This emitter produces a mosaic-profile-card.ts file:
--
--   // AUTO-GENERATED from ProfileCard.mosaic — do not edit
--
--   export class MosaicProfileCardElement extends HTMLElement {
--     private _shadow: ShadowRoot;
--     private _name: string = '';
--     private _avatar: string = '';
--     private _active: boolean = false;
--
--     constructor() { ... }
--     static get observedAttributes(): string[] { return ['name', 'avatar', 'active']; }
--     attributeChangedCallback(...) { ... }
--
--     set name(v: string) { this._name = v; this._render(); }
--     get name(): string { return this._name; }
--     ...
--
--     private _render(): void {
--       let html = '';
--       html += '<div style="display:flex;flex-direction:column">';
--       html += '<img>';
--       html += `${this._escapeHtml(this._name)}`;
--       html += '</span>';
--       html += '</div>';
--       this._shadow.innerHTML = html;
--     }
--   }
--
--   customElements.define('mosaic-profile-card', MosaicProfileCardElement);
--
-- # Architecture
--
-- Unlike the React backend (JSX string stack), the Web Components renderer
-- builds a flat list of RenderFragments during VM traversal, then serializes
-- them into `html +=` statements in the final `_render()` method.
--
-- # Tag Name Convention
--
-- PascalCase → kebab-case with "mosaic-" prefix:
--   ProfileCard → mosaic-profile-card
--   Button      → mosaic-button
--
-- # API
--
--   local renderer = MosaicEmitWebcomponent.new_renderer()
--   local result, err = vm.run(ir, renderer)
--   -- result.files[1].filename → "mosaic-profile-card.ts"
--
-- Convenience:
--   local result, err = MosaicEmitWebcomponent.emit(source)

local mosaic_vm       = require("coding_adventures.mosaic_vm")
local mosaic_analyzer = require("coding_adventures.mosaic_analyzer")

local M = {}
M.VERSION = "0.1.0"

-- ============================================================================
-- Helpers
-- ============================================================================

--- Convert PascalCase to kebab-case.
-- "ProfileCard" → "profile-card"
-- "HowItWorks"  → "how-it-works"
local function to_kebab_case(name)
    -- Insert '-' before each uppercase letter (except the first), then lowercase all
    local result = name:gsub("(%u)", function(c, pos)
        return "-" .. c:lower()
    end)
    return result:gsub("^%-", "")
end

--- Format RGBA color as CSS rgba() string.
local function rgba(r, g, b, a)
    local alpha = math.floor((a / 255) * 1000 + 0.5) / 1000
    return ("rgba(%d, %d, %d, %s)"):format(r, g, b, alpha)
end

--- Convert a dimension to a CSS string (dp/sp → px, % → %).
local function dim(value)
    if value.kind ~= "dimension" then return nil end
    if value.unit == "%" then return ("%g%%"):format(value.value) end
    return ("%gpx"):format(value.value)
end

--- Convert size value to CSS string.
local function size_value(value)
    if value.kind == "string" then
        if value.value == "fill" then return "100%" end
        if value.value == "wrap" then return "fit-content" end
        return value.value
    end
    return dim(value) or "auto"
end

--- Escape a string for use in a single-quoted JS string literal.
local function single_quote_escape(s)
    return s:gsub("\\", "\\\\"):gsub("'", "\\'")
end

--- Escape HTML entities in a literal string (for static text content).
local function escape_html_literal(s)
    return s:gsub("&", "&amp;"):gsub("<", "&lt;"):gsub(">", "&gt;")
end

--- Map a Mosaic node tag to its HTML tag name.
local function tag_to_html(tag)
    local map = {
        Column = "div", Row = "div", Box = "div", Spacer = "div", Scroll = "div",
        Stack  = "div",
        Text   = "span",
        Image  = "img",
        Icon   = "span",
        Divider = "hr",
    }
    return map[tag] or "div"
end

--- Return the TypeScript backing field type for a MosaicType.
local function ts_field_type(t)
    if t.kind == "text"      then return "string" end
    if t.kind == "number"    then return "number" end
    if t.kind == "bool"      then return "boolean" end
    if t.kind == "image"     then return "string" end
    if t.kind == "color"     then return "string" end
    if t.kind == "node"      then return "HTMLElement | null" end
    if t.kind == "component" then return "HTMLElement | null" end
    if t.kind == "list" then
        local inner = t.element_type
        if inner.kind == "node" or inner.kind == "component" then return "Element[]" end
        if inner.kind == "text"   then return "string[]" end
        if inner.kind == "number" then return "number[]" end
        if inner.kind == "bool"   then return "boolean[]" end
        return "unknown[]"
    end
    return "unknown"
end

--- Return the default initializer for a slot's backing field.
local function default_value_for_slot(slot)
    if slot.default_value then
        local v = slot.default_value
        if v.kind == "string" then return ("'%s'"):format(v.value) end
        if v.kind == "number" then return tostring(v.value) end
        if v.kind == "bool"   then return v.value and "true" or "false" end
    end
    local k = slot.type.kind
    if k == "text"   then return "''" end
    if k == "number" then return "0" end
    if k == "bool"   then return "false" end
    if k == "image"  then return "''" end
    if k == "color"  then return "''" end
    if k == "node" or k == "component" then return "null" end
    if k == "list"   then return "[]" end
    return "null"
end

--- Return true if the slot type can be set via an HTML attribute.
local function is_observable_type(t)
    return t.kind == "text" or t.kind == "number" or t.kind == "bool"
        or t.kind == "image" or t.kind == "color"
end

-- ============================================================================
-- Renderer
-- ============================================================================

--- Create a new Web Component renderer instance.
-- @return table  A renderer conforming to the mosaic_vm renderer interface.
function M.new_renderer()
    local self = {
        _component_name     = "",
        _slots              = {},
        _stack              = {},
        _needs_typescale_css = false,
    }

    -- -----------------------------------------------------------------------
    -- MosaicRenderer interface
    -- -----------------------------------------------------------------------

    function self:beginComponent(name, slots)
        self._component_name      = name
        self._slots               = slots
        self._stack               = { { kind = "component", fragments = {} } }
        self._needs_typescale_css = false
    end

    function self:endComponent()
        -- no-op
    end

    function self:emit()
        local content = self:_build_file()
        local tag_name = to_kebab_case(self._component_name)
        return { files = { { filename = "mosaic-" .. tag_name .. ".ts", content = content } } }
    end

    function self:beginNode(tag, is_primitive, properties, ctx)
        local frame = self:_build_node_frame(tag, is_primitive, properties)
        self._stack[#self._stack + 1] = frame
    end

    function self:endNode(_tag)
        local frame = table.remove(self._stack)
        local parent = self._stack[#self._stack]

        if frame.self_closing then
            parent.fragments[#parent.fragments + 1] = { kind = "self_closing", html = frame.open_html }
        else
            parent.fragments[#parent.fragments + 1] = { kind = "open_tag", html = frame.open_html }

            if frame.text_literal ~= nil then
                parent.fragments[#parent.fragments + 1] = {
                    kind = "open_tag",
                    html = escape_html_literal(frame.text_literal),
                }
            elseif frame.text_slot_expr ~= nil then
                parent.fragments[#parent.fragments + 1] = { kind = "slot_ref", expr = frame.text_slot_expr }
            else
                -- Append child fragments
                for _, frag in ipairs(frame.fragments) do
                    parent.fragments[#parent.fragments + 1] = frag
                end
            end

            -- Closing tag
            local close_tag
            if frame.tag == "Text" then
                close_tag = "span"
            else
                close_tag = tag_to_html(frame.tag)
            end
            parent.fragments[#parent.fragments + 1] = { kind = "close_tag", tag = close_tag }
        end
    end

    function self:renderSlotChild(slot_name, _slot_type, _ctx)
        local top = self._stack[#self._stack]
        top.fragments[#top.fragments + 1] = { kind = "slot_proj", slot_name = slot_name }
    end

    function self:beginWhen(slot_name, _ctx)
        self._stack[#self._stack + 1] = { kind = "when", slot_name = slot_name, fragments = {} }
    end

    function self:endWhen()
        local frame = table.remove(self._stack)
        local parent = self._stack[#self._stack]
        parent.fragments[#parent.fragments + 1] = { kind = "when_open", field = frame.slot_name }
        for _, frag in ipairs(frame.fragments) do
            parent.fragments[#parent.fragments + 1] = frag
        end
        parent.fragments[#parent.fragments + 1] = { kind = "when_close" }
    end

    function self:beginEach(slot_name, item_name, element_type, _ctx)
        local is_node_list = element_type.kind == "node" or element_type.kind == "component"
        self._stack[#self._stack + 1] = {
            kind         = "each",
            slot_name    = slot_name,
            item_name    = item_name,
            is_node_list = is_node_list,
            fragments    = {},
        }
    end

    function self:endEach()
        local frame = table.remove(self._stack)
        local parent = self._stack[#self._stack]
        parent.fragments[#parent.fragments + 1] = {
            kind         = "each_open",
            field        = frame.slot_name,
            item_name    = frame.item_name,
            is_node_list = frame.is_node_list,
        }
        for _, frag in ipairs(frame.fragments) do
            parent.fragments[#parent.fragments + 1] = frag
        end
        parent.fragments[#parent.fragments + 1] = { kind = "each_close" }
    end

    -- -----------------------------------------------------------------------
    -- Internal helpers
    -- -----------------------------------------------------------------------

    --- Build a NodeFrame for the given tag and resolved properties.
    function self:_build_node_frame(tag, is_primitive, properties)
        local styles   = {}  -- list of "css-prop:value" strings
        local attrs    = {}  -- list of HTML attribute strings
        local class_names = {}
        local text_literal    = nil
        local text_slot_expr  = nil
        local self_closing    = false
        local html_tag

        -- Base HTML tag and default styles from primitive type
        if is_primitive then
            if tag == "Box" then
                html_tag = "div"
                styles[#styles + 1] = "position:relative"
            elseif tag == "Column" then
                html_tag = "div"
                styles[#styles + 1] = "display:flex"
                styles[#styles + 1] = "flex-direction:column"
            elseif tag == "Row" then
                html_tag = "div"
                styles[#styles + 1] = "display:flex"
                styles[#styles + 1] = "flex-direction:row"
            elseif tag == "Text" then
                html_tag = "span"
            elseif tag == "Image" then
                html_tag = "img"
                self_closing = true
            elseif tag == "Spacer" then
                html_tag = "div"
                styles[#styles + 1] = "flex:1"
            elseif tag == "Scroll" then
                html_tag = "div"
                styles[#styles + 1] = "overflow:auto"
            elseif tag == "Divider" then
                html_tag = "hr"
                self_closing = true
                styles[#styles + 1] = "border:none"
                styles[#styles + 1] = "border-top:1px solid currentColor"
            elseif tag == "Icon" then
                html_tag = "span"
            elseif tag == "Stack" then
                html_tag = "div"
                styles[#styles + 1] = "position:relative"
            else
                html_tag = "div"
            end
        else
            html_tag = "div"  -- placeholder for component nodes
        end

        -- Apply each resolved property
        for _, prop in ipairs(properties) do
            local name  = prop.name
            local value = prop.value

            if name == "padding" then
                local d = dim(value)
                if d then styles[#styles + 1] = "padding:" .. d end

            elseif name == "padding-left" then
                local d = dim(value)
                if d then styles[#styles + 1] = "padding-left:" .. d end

            elseif name == "padding-right" then
                local d = dim(value)
                if d then styles[#styles + 1] = "padding-right:" .. d end

            elseif name == "padding-top" then
                local d = dim(value)
                if d then styles[#styles + 1] = "padding-top:" .. d end

            elseif name == "padding-bottom" then
                local d = dim(value)
                if d then styles[#styles + 1] = "padding-bottom:" .. d end

            elseif name == "gap" then
                local d = dim(value)
                if d then styles[#styles + 1] = "gap:" .. d end

            elseif name == "width" then
                styles[#styles + 1] = "width:" .. size_value(value)

            elseif name == "height" then
                styles[#styles + 1] = "height:" .. size_value(value)

            elseif name == "min-width" then
                local d = dim(value)
                if d then styles[#styles + 1] = "min-width:" .. d end

            elseif name == "max-width" then
                local d = dim(value)
                if d then styles[#styles + 1] = "max-width:" .. d end

            elseif name == "min-height" then
                local d = dim(value)
                if d then styles[#styles + 1] = "min-height:" .. d end

            elseif name == "max-height" then
                local d = dim(value)
                if d then styles[#styles + 1] = "max-height:" .. d end

            elseif name == "overflow" then
                if value.kind == "string" then
                    local map = { visible="visible", hidden="hidden", scroll="auto" }
                    local v = map[value.value]
                    if v then styles[#styles + 1] = "overflow:" .. v end
                end

            elseif name == "align" then
                if value.kind == "string" then
                    self:_apply_align(value.value, tag, styles)
                    if tag == "Box" then
                        -- ensure display:flex is present for Box
                        local has_flex = false
                        for _, s in ipairs(styles) do
                            if s == "display:flex" then has_flex = true break end
                        end
                        if not has_flex then
                            table.insert(styles, 1, "display:flex")
                        end
                    end
                end

            elseif name == "background" then
                if value.kind == "color" then
                    styles[#styles + 1] = "background-color:" .. rgba(value.r, value.g, value.b, value.a)
                end

            elseif name == "corner-radius" then
                local d = dim(value)
                if d then styles[#styles + 1] = "border-radius:" .. d end

            elseif name == "border-width" then
                local d = dim(value)
                if d then
                    styles[#styles + 1] = "border-width:" .. d
                    styles[#styles + 1] = "border-style:solid"
                end

            elseif name == "border-color" then
                if value.kind == "color" then
                    styles[#styles + 1] = "border-color:" .. rgba(value.r, value.g, value.b, value.a)
                end

            elseif name == "opacity" then
                if value.kind == "number" then
                    styles[#styles + 1] = "opacity:" .. tostring(value.value)
                end

            elseif name == "visible" then
                if value.kind == "bool" and not value.value then
                    styles[#styles + 1] = "display:none"
                end

            elseif name == "content" then
                if tag == "Text" then
                    if value.kind == "string" then
                        text_literal = value.value
                    elseif value.kind == "slot_ref" then
                        -- Generate: this._escapeHtml(this._slotName)
                        text_slot_expr = ("this._escapeHtml(this._%s)"):format(value.slot_name)
                    end
                end

            elseif name == "color" then
                if value.kind == "color" then
                    styles[#styles + 1] = "color:" .. rgba(value.r, value.g, value.b, value.a)
                end

            elseif name == "text-align" then
                if value.kind == "string" then
                    local map = { start="left", center="center", ["end"]="right" }
                    local v = map[value.value]
                    if v then styles[#styles + 1] = "text-align:" .. v end
                end

            elseif name == "font-weight" then
                if value.kind == "string" then
                    styles[#styles + 1] = "font-weight:" .. value.value
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
                    if value.kind == "string" then
                        attrs[#attrs + 1] = ('src="%s"'):format(escape_html_literal(value.value))
                    elseif value.kind == "slot_ref" then
                        -- Use a placeholder that will be replaced in _build_file
                        attrs[#attrs + 1] = ("src=\"__IMG_SRC_%s__\""):format(value.slot_name)
                    end
                end

            elseif name == "size" then
                local d = dim(value)
                if d and tag == "Image" then
                    styles[#styles + 1] = "width:" .. d
                    styles[#styles + 1] = "height:" .. d
                end

            elseif name == "shape" then
                if tag == "Image" and value.kind == "string" then
                    local shape_map = { circle = "50%", rounded = "8px" }
                    local r = shape_map[value.value]
                    if r then styles[#styles + 1] = "border-radius:" .. r end
                end

            elseif name == "fit" then
                if tag == "Image" and value.kind == "string" then
                    styles[#styles + 1] = "object-fit:" .. value.value
                end

            elseif name == "a11y-label" then
                if value.kind == "string" then
                    attrs[#attrs + 1] = ('aria-label="%s"'):format(escape_html_literal(value.value))
                elseif value.kind == "slot_ref" then
                    attrs[#attrs + 1] = ('aria-label="__ARIA_%s__"'):format(value.slot_name)
                end

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

        -- Post-process: a11y-role: heading on Text → h2
        if tag == "Text" then
            for i, attr in ipairs(attrs) do
                if attr == 'role="heading"' then
                    html_tag = "h2"
                    table.remove(attrs, i)
                    break
                end
            end
        end

        -- Build the opening HTML string
        local style_str = #styles > 0 and table.concat(styles, ";") or ""
        local parts = {}
        if style_str ~= "" then parts[#parts + 1] = ('style="%s"'):format(style_str) end
        if #class_names > 0 then parts[#parts + 1] = ('class="%s"'):format(table.concat(class_names, " ")) end
        for _, attr in ipairs(attrs) do parts[#parts + 1] = attr end

        local attr_str = #parts > 0 and (" " .. table.concat(parts, " ")) or ""
        local open_html = ("<" .. html_tag .. attr_str .. ">")

        return {
            kind           = "node",
            tag            = tag,
            html_tag       = html_tag,
            open_html      = open_html,
            self_closing   = self_closing,
            text_literal   = text_literal,
            text_slot_expr = text_slot_expr,
            fragments      = {},
        }
    end

    --- Apply alignment to styles list (CSS kebab-case).
    function self:_apply_align(align_value, tag, styles)
        if tag == "Column" then
            if align_value == "start"             then styles[#styles + 1] = "align-items:flex-start"
            elseif align_value == "center"        then styles[#styles + 1] = "align-items:center"
            elseif align_value == "end"           then styles[#styles + 1] = "align-items:flex-end"
            elseif align_value == "stretch"       then styles[#styles + 1] = "align-items:stretch"
            elseif align_value == "center-horizontal" then styles[#styles + 1] = "align-items:center"
            elseif align_value == "center-vertical"   then styles[#styles + 1] = "justify-content:center"
            end
        elseif tag == "Row" then
            if align_value == "start"             then styles[#styles + 1] = "align-items:flex-start"
            elseif align_value == "center"        then
                styles[#styles + 1] = "align-items:center"
                styles[#styles + 1] = "justify-content:center"
            elseif align_value == "end"           then
                styles[#styles + 1] = "align-items:flex-end"
                styles[#styles + 1] = "justify-content:flex-end"
            elseif align_value == "stretch"       then styles[#styles + 1] = "align-items:stretch"
            elseif align_value == "center-horizontal" then styles[#styles + 1] = "justify-content:center"
            elseif align_value == "center-vertical"   then styles[#styles + 1] = "align-items:center"
            end
        elseif tag == "Box" then
            if align_value == "start"             then styles[#styles + 1] = "align-items:flex-start"
            elseif align_value == "center"        then styles[#styles + 1] = "align-items:center"
            elseif align_value == "end"           then styles[#styles + 1] = "align-items:flex-end"
            elseif align_value == "stretch"       then styles[#styles + 1] = "align-items:stretch"
            elseif align_value == "center-horizontal" then styles[#styles + 1] = "align-items:center"
            elseif align_value == "center-vertical"   then styles[#styles + 1] = "justify-content:center"
            end
        end
    end

    --- Serialize render fragments into html += statement lines.
    function self:_serialize_fragments(fragments, indent)
        local lines = {}

        for _, frag in ipairs(fragments) do
            local k = frag.kind
            if k == "open_tag" then
                lines[#lines + 1] = ("%shtml += '%s';"):format(indent, single_quote_escape(frag.html))
            elseif k == "close_tag" then
                lines[#lines + 1] = ("%shtml += '</%s>';"):format(indent, frag.tag)
            elseif k == "self_closing" then
                lines[#lines + 1] = ("%shtml += '%s';"):format(indent, single_quote_escape(frag.html))
            elseif k == "slot_ref" then
                lines[#lines + 1] = ("%shtml += `${%s}`;"):format(indent, frag.expr)
            elseif k == "slot_proj" then
                lines[#lines + 1] = ('%shtml += \'<slot name="%s"></slot>\';'):format(indent, frag.slot_name)
            elseif k == "when_open" then
                lines[#lines + 1] = ("%sif (this._%s) {"):format(indent, frag.field)
            elseif k == "when_close" then
                lines[#lines + 1] = indent .. "}"
            elseif k == "each_open" then
                if frag.is_node_list then
                    lines[#lines + 1] = ("%sthis._%s.forEach((_item, _i) => {"):format(indent, frag.field)
                    lines[#lines + 1] = ('  %shtml += `<slot name="%s-${_i}"></slot>`;'):format(indent, frag.field)
                else
                    lines[#lines + 1] = ("%sthis._%s.forEach(%s => {"):format(indent, frag.field, frag.item_name)
                end
            elseif k == "each_close" then
                lines[#lines + 1] = indent .. "});"
            end
        end

        return lines
    end

    --- Assemble the complete .ts file content.
    function self:_build_file()
        local name       = self._component_name
        local class_name = "Mosaic" .. name .. "Element"
        local tag_name   = to_kebab_case(name)
        local element_tag = "mosaic-" .. tag_name

        -- Categorize slots
        local observable_slots = {}
        local node_slots       = {}
        local image_slots      = {}
        local list_slots       = {}
        for _, slot in ipairs(self._slots) do
            if is_observable_type(slot.type) then
                observable_slots[#observable_slots + 1] = slot
            end
            if slot.type.kind == "node" or slot.type.kind == "component" then
                node_slots[#node_slots + 1] = slot
            end
            if slot.type.kind == "image" then
                image_slots[#image_slots + 1] = slot
            end
            if slot.type.kind == "list" then
                list_slots[#list_slots + 1] = slot
            end
        end
        local has_node_slots = #node_slots > 0

        -- Backing field declarations
        local field_lines = {}
        for _, slot in ipairs(self._slots) do
            field_lines[#field_lines + 1] = ("  private _%s: %s = %s;"):format(
                slot.name, ts_field_type(slot.type), default_value_for_slot(slot)
            )
        end

        -- observedAttributes
        local observed_names = {}
        for _, slot in ipairs(observable_slots) do
            observed_names[#observed_names + 1] = ("'%s'"):format(slot.name)
        end

        -- attributeChangedCallback cases
        local attr_case_lines = {}
        for _, slot in ipairs(observable_slots) do
            local field = "_" .. slot.name
            local setter
            if slot.type.kind == "number" then
                local def_scalar = "0"
                if slot.default_value and slot.default_value.kind == "number" then
                    def_scalar = tostring(slot.default_value.value)
                end
                setter = ("this.%s = parseFloat(value ?? '%s');"):format(field, def_scalar)
            elseif slot.type.kind == "bool" then
                setter = ("this.%s = value !== null;"):format(field)
            else
                local def_str = "''"
                if slot.default_value and slot.default_value.kind == "string" then
                    def_str = ("'%s'"):format(slot.default_value.value)
                end
                setter = ("this.%s = value ?? %s;"):format(field, def_str)
            end
            attr_case_lines[#attr_case_lines + 1] =
                ("    case '%s': %s break;"):format(slot.name, setter)
        end

        -- Property setters/getters
        local setter_lines = {}
        for _, slot in ipairs(self._slots) do
            local field   = "_" .. slot.name
            local ts_type = ts_field_type(slot.type)
            if slot.type.kind == "node" or slot.type.kind == "component" then
                setter_lines[#setter_lines + 1] =
                    ("  set %s(v: HTMLElement) { this._projectSlot('%s', v); }"):format(slot.name, slot.name)
            elseif slot.type.kind == "image" then
                setter_lines[#setter_lines + 1] = ("  set %s(v: string) {"):format(slot.name)
                setter_lines[#setter_lines + 1] =  "    if (/^javascript:/i.test(v.trim())) return;"
                setter_lines[#setter_lines + 1] = ("    this.%s = v;"):format(field)
                setter_lines[#setter_lines + 1] =  "    this._render();"
                setter_lines[#setter_lines + 1] =  "  }"
                setter_lines[#setter_lines + 1] = ("  get %s(): string { return this.%s; }"):format(slot.name, field)
            else
                setter_lines[#setter_lines + 1] =
                    ("  set %s(v: %s) { this.%s = v; this._render(); }"):format(slot.name, ts_type, field)
                if slot.type.kind ~= "list" then
                    setter_lines[#setter_lines + 1] =
                        ("  get %s(): %s { return this.%s; }"):format(slot.name, ts_type, field)
                end
            end
        end

        -- Render body
        local root_fragments = self._stack[1].fragments
        local render_lines   = self:_serialize_fragments(root_fragments, "    ")

        -- Replace image source and aria-label placeholders
        local resolved_render_lines = {}
        for _, line in ipairs(render_lines) do
            -- __IMG_SRC_slotName__ → " + this._escapeHtml(this._slotName) + "
            line = line:gsub("__IMG_SRC_(%w+)__", function(slot_name)
                return '" + this._escapeHtml(this._' .. slot_name .. ') + "'
            end)
            -- __ARIA_slotName__ → " + this._escapeHtml(this._slotName) + "
            line = line:gsub("__ARIA_(%w+)__", function(slot_name)
                return '" + this._escapeHtml(this._' .. slot_name .. ') + "'
            end)
            resolved_render_lines[#resolved_render_lines + 1] = line
        end

        -- Assemble the file
        local lines = {
            ("// AUTO-GENERATED from %s.mosaic — do not edit"):format(name),
            "// Generated by mosaic-emit-webcomponent v1.0",
            ("// Source: %s.mosaic"):format(name),
            "//",
            ("// To modify this component, edit %s.mosaic and re-run the compiler."):format(name),
            "",
        }

        if self._needs_typescale_css then
            lines[#lines + 1] = "const MOSAIC_TYPE_SCALE_CSS = `"
            lines[#lines + 1] = "  .mosaic-heading-large { font-size: 2rem; font-weight: 700; line-height: 1.2; }"
            lines[#lines + 1] = "  .mosaic-heading-medium { font-size: 1.5rem; font-weight: 600; line-height: 1.3; }"
            lines[#lines + 1] = "  .mosaic-heading-small { font-size: 1.25rem; font-weight: 600; line-height: 1.4; }"
            lines[#lines + 1] = "  .mosaic-body-large { font-size: 1rem; line-height: 1.6; }"
            lines[#lines + 1] = "  .mosaic-body-medium { font-size: 0.875rem; line-height: 1.6; }"
            lines[#lines + 1] = "  .mosaic-body-small { font-size: 0.75rem; line-height: 1.5; }"
            lines[#lines + 1] = "  .mosaic-label { font-size: 0.875rem; font-weight: 500; }"
            lines[#lines + 1] = "  .mosaic-caption { font-size: 0.75rem; color: #666; }"
            lines[#lines + 1] = "`;"
            lines[#lines + 1] = ""
        end

        lines[#lines + 1] = ("export class %s extends HTMLElement {"):format(class_name)
        lines[#lines + 1] = "  private _shadow: ShadowRoot;"
        lines[#lines + 1] = ""

        if #field_lines > 0 then
            lines[#lines + 1] = "  // Backing fields for Mosaic slots"
            for _, l in ipairs(field_lines) do lines[#lines + 1] = l end
            lines[#lines + 1] = ""
        end

        lines[#lines + 1] = "  constructor() {"
        lines[#lines + 1] = "    super();"
        lines[#lines + 1] = "    this._shadow = this.attachShadow({ mode: 'open' });"
        lines[#lines + 1] = "  }"
        lines[#lines + 1] = ""

        if #observable_slots > 0 then
            lines[#lines + 1] = "  static get observedAttributes(): string[] {"
            lines[#lines + 1] = ("    return [%s];"):format(table.concat(observed_names, ", "))
            lines[#lines + 1] = "  }"
            lines[#lines + 1] = ""
            lines[#lines + 1] = "  attributeChangedCallback(name: string, _old: string | null, value: string | null): void {"
            lines[#lines + 1] = "    switch (name) {"
            for _, l in ipairs(attr_case_lines) do lines[#lines + 1] = l end
            lines[#lines + 1] = "    }"
            lines[#lines + 1] = "    this._render();"
            lines[#lines + 1] = "  }"
            lines[#lines + 1] = ""
        end

        if #setter_lines > 0 then
            lines[#lines + 1] = "  // Property setters and getters"
            for _, l in ipairs(setter_lines) do lines[#lines + 1] = l end
            lines[#lines + 1] = ""
        end

        if has_node_slots then
            lines[#lines + 1] = "  // Light DOM slot projection for node/component-type slots"
            lines[#lines + 1] = "  private _projectSlot(name: string, node: Element): void {"
            lines[#lines + 1] = "    const prev = this.querySelector(`[data-mosaic-slot=\"${name}\"]`);"
            lines[#lines + 1] = "    if (prev) prev.remove();"
            lines[#lines + 1] = "    node.setAttribute('slot', name);"
            lines[#lines + 1] = "    node.setAttribute('data-mosaic-slot', name);"
            lines[#lines + 1] = "    this.appendChild(node);"
            lines[#lines + 1] = "  }"
            lines[#lines + 1] = ""
        end

        lines[#lines + 1] = "  private _escapeHtml(s: string): string {"
        lines[#lines + 1] = "    return s"
        lines[#lines + 1] = "      .replace(/&/g, '&amp;')"
        lines[#lines + 1] = "      .replace(/</g, '&lt;')"
        lines[#lines + 1] = "      .replace(/>/g, '&gt;')"
        lines[#lines + 1] = "      .replace(/\"/g, '&quot;')"
        lines[#lines + 1] = "      .replace(/'/g, '&#39;');"
        lines[#lines + 1] = "  }"
        lines[#lines + 1] = ""

        lines[#lines + 1] = "  connectedCallback(): void { this._render(); }"
        lines[#lines + 1] = ""

        if has_node_slots then
            lines[#lines + 1] = "  disconnectedCallback(): void {"
            lines[#lines + 1] = "    [...this.querySelectorAll('[data-mosaic-slot]')].forEach((el) => el.remove());"
            lines[#lines + 1] = "  }"
            lines[#lines + 1] = ""
        end

        lines[#lines + 1] = "  private _render(): void {"
        lines[#lines + 1] = "    let html = '';"
        if self._needs_typescale_css then
            lines[#lines + 1] = "    html += `<style>${MOSAIC_TYPE_SCALE_CSS}</style>`;"
        end
        for _, l in ipairs(resolved_render_lines) do lines[#lines + 1] = l end
        lines[#lines + 1] = "    this._shadow.innerHTML = html;"
        lines[#lines + 1] = "  }"
        lines[#lines + 1] = "}"
        lines[#lines + 1] = ""
        lines[#lines + 1] = ("customElements.define('%s', %s);"):format(element_tag, class_name)

        return table.concat(lines, "\n")
    end

    return self
end

-- ============================================================================
-- Convenience API
-- ============================================================================

--- Emit a Web Component TypeScript file from Mosaic source text.
--
-- @param source string  Mosaic source text.
-- @return table|nil     { files = { { filename, content } } }, or nil on error.
-- @return nil|string    Error message on failure.
--
-- Example:
--
--   local emit = require("coding_adventures.mosaic_emit_webcomponent")
--   local result, err = emit.emit([[
--     component Button {
--       slot label: text;
--       Text { content: @label; }
--     }
--   ]])
--   -- result.files[1].filename → "mosaic-button.ts"
function M.emit(source)
    local ir, err = mosaic_analyzer.analyze(source)
    if not ir then
        return nil, err
    end
    local renderer = M.new_renderer()
    return mosaic_vm.run(ir, renderer)
end

return M
