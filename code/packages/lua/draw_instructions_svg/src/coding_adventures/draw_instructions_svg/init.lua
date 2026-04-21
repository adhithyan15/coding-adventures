-- draw-instructions-svg — SVG renderer for Lua draw instruction scenes
-- ============================================================================
--
-- Draw instructions are backend-neutral data. This renderer turns that data
-- into a complete SVG document without smuggling drawing policy into the core
-- model. The mapping is intentionally direct:
--
--   rect   -> <rect>
--   text   -> <text>
--   line   -> <line>
--   circle -> <circle>
--   group  -> <g>
--   clip   -> <clipPath> + clipped <g>
--
-- The Lua draw_instructions package currently creates rect/text/line/circle,
-- group, and scene nodes. This renderer also understands the richer optional
-- fields used by sibling implementations, such as rect stroke metadata and
-- manually-authored clip nodes, so scenes can round-trip across languages.

require("coding_adventures.draw_instructions")

local M = {}

M.VERSION = "0.1.0"

local clip_counter = 0

local function xml_escape(value)
    local s = tostring(value or "")
    s = s:gsub("&", "&amp;")
    s = s:gsub("<", "&lt;")
    s = s:gsub(">", "&gt;")
    s = s:gsub('"', "&quot;")
    s = s:gsub("'", "&apos;")
    return s
end

local function sorted_keys(tbl)
    local keys = {}
    for key in pairs(tbl or {}) do
        keys[#keys + 1] = key
    end
    table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
    return keys
end

local function metadata_attrs(metadata)
    if metadata == nil then return "" end
    local parts = {}
    for _, key in ipairs(sorted_keys(metadata)) do
        parts[#parts + 1] = string.format(
            ' data-%s="%s"',
            xml_escape(key),
            xml_escape(metadata[key])
        )
    end
    return table.concat(parts)
end

local render_instruction

local function render_rect(instr)
    local stroke = ""
    if instr.stroke ~= nil then
        stroke = string.format(
            ' stroke="%s" stroke-width="%s"',
            xml_escape(instr.stroke),
            xml_escape(instr.stroke_width or 1)
        )
    end
    return string.format(
        '  <rect x="%s" y="%s" width="%s" height="%s" fill="%s"%s%s />',
        xml_escape(instr.x),
        xml_escape(instr.y),
        xml_escape(instr.width),
        xml_escape(instr.height),
        xml_escape(instr.fill or "none"),
        stroke,
        metadata_attrs(instr.metadata)
    )
end

local function render_text(instr)
    local weight = ""
    if instr.font_weight ~= nil and instr.font_weight ~= "normal" then
        weight = string.format(' font-weight="%s"', xml_escape(instr.font_weight))
    end
    return string.format(
        '  <text x="%s" y="%s" text-anchor="%s" font-family="%s" font-size="%s" fill="%s"%s%s>%s</text>',
        xml_escape(instr.x),
        xml_escape(instr.y),
        xml_escape(instr.align or "middle"),
        xml_escape(instr.font_family or "monospace"),
        xml_escape(instr.font_size or 16),
        xml_escape(instr.fill or "#000000"),
        weight,
        metadata_attrs(instr.metadata),
        xml_escape(instr.value)
    )
end

local function render_line(instr)
    return string.format(
        '  <line x1="%s" y1="%s" x2="%s" y2="%s" stroke="%s" stroke-width="%s"%s />',
        xml_escape(instr.x1),
        xml_escape(instr.y1),
        xml_escape(instr.x2),
        xml_escape(instr.y2),
        xml_escape(instr.stroke or "#000000"),
        xml_escape(instr.stroke_width or 1),
        metadata_attrs(instr.metadata)
    )
end

local function render_circle(instr)
    return string.format(
        '  <circle cx="%s" cy="%s" r="%s" fill="%s"%s />',
        xml_escape(instr.cx),
        xml_escape(instr.cy),
        xml_escape(instr.r),
        xml_escape(instr.fill or "#000000"),
        metadata_attrs(instr.metadata)
    )
end

local function render_group(instr)
    local lines = { "  <g" .. metadata_attrs(instr.metadata) .. ">" }
    for _, child in ipairs(instr.children or {}) do
        lines[#lines + 1] = render_instruction(child)
    end
    lines[#lines + 1] = "  </g>"
    return table.concat(lines, "\n")
end

local function render_clip(instr)
    clip_counter = clip_counter + 1
    local clip_id = "clip-" .. clip_counter
    local lines = {
        "  <defs>",
        string.format('  <clipPath id="%s">', clip_id),
        string.format(
            '  <rect x="%s" y="%s" width="%s" height="%s" />',
            xml_escape(instr.x),
            xml_escape(instr.y),
            xml_escape(instr.width),
            xml_escape(instr.height)
        ),
        "  </clipPath>",
        "  </defs>",
        string.format('  <g clip-path="url(#%s)"%s>', clip_id, metadata_attrs(instr.metadata)),
    }
    for _, child in ipairs(instr.children or {}) do
        lines[#lines + 1] = render_instruction(child)
    end
    lines[#lines + 1] = "  </g>"
    return table.concat(lines, "\n")
end

render_instruction = function(instr)
    if instr.kind == "rect" then
        return render_rect(instr)
    elseif instr.kind == "text" then
        return render_text(instr)
    elseif instr.kind == "line" then
        return render_line(instr)
    elseif instr.kind == "circle" then
        return render_circle(instr)
    elseif instr.kind == "group" then
        return render_group(instr)
    elseif instr.kind == "clip" then
        return render_clip(instr)
    end
    error("unsupported draw instruction kind: " .. tostring(instr.kind))
end

function M.render_svg(scene)
    clip_counter = 0
    scene = scene or {}
    local metadata = scene.metadata or {}
    local label = metadata.label or metadata.title or "draw instructions scene"
    local width = scene.width or 0
    local height = scene.height or 0
    local lines = {
        string.format(
            '<svg xmlns="http://www.w3.org/2000/svg" width="%s" height="%s" viewBox="0 0 %s %s" role="img" aria-label="%s">',
            xml_escape(width),
            xml_escape(height),
            xml_escape(width),
            xml_escape(height),
            xml_escape(label)
        ),
        string.format(
            '  <rect x="0" y="0" width="%s" height="%s" fill="%s" />',
            xml_escape(width),
            xml_escape(height),
            xml_escape(scene.background or "#ffffff")
        ),
    }
    for _, instr in ipairs(scene.instructions or {}) do
        lines[#lines + 1] = render_instruction(instr)
    end
    lines[#lines + 1] = "</svg>"
    return table.concat(lines, "\n")
end

return M
