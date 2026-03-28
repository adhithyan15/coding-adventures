-- coding_adventures.draw_instructions
-- ============================================================================
--
-- BACKEND-NEUTRAL DRAWING INSTRUCTION SET
--
-- This module defines a set of plain Lua tables that describe drawing
-- operations: rectangles, text, lines, circles, and groups. These "drawing
-- instructions" are data — they describe WHAT to draw, not HOW to draw it.
-- A separate backend (SVG renderer, HTML canvas, terminal, etc.) interprets
-- them to produce actual output.
--
-- WHY INSTRUCTION-BASED DRAWING?
-- --------------------------------
-- Decoupling the description of a drawing from its rendering is a powerful
-- pattern. It lets you:
--   - Serialize drawings to JSON or other formats
--   - Render the same drawing in different backends (SVG, PNG, terminal)
--   - Inspect, transform, and diff drawings programmatically
--   - Attach arbitrary metadata to any instruction for tooling
--
-- This pattern is used in many real systems: PDF content streams, Skia's
-- picture recording, Flutter's render objects, React's virtual DOM.
--
-- INSTRUCTION KINDS
-- -----------------
-- Each instruction is a table with a `kind` field:
--
--   "rect"    — filled rectangle
--   "text"    — text label
--   "line"    — straight line between two points
--   "circle"  — filled circle
--   "group"   — container for child instructions
--
-- Every instruction also carries a `metadata` table for arbitrary key-value
-- annotations (e.g., ids, tooltips, event handlers).
--
-- COORDINATE SYSTEM
-- -----------------
-- All coordinates are in abstract units. The origin (0, 0) is at the top-left
-- corner. X increases to the right; Y increases downward.
--
-- COLOR FORMAT
-- ------------
-- Colors are CSS-style hex strings: "#rrggbb" (e.g., "#ff0000" = red).
-- The defaults match common visualization conventions:
--   - Rect fill:      "#000000" (black)
--   - Text fill:      "#000000" (black)
--   - Line stroke:    "#000000" (black)
--   - Circle fill:    "#000000" (black)
--   - Scene bg:       "#ffffff" (white)
--
-- Usage:
--   local Draw = require("coding_adventures.draw_instructions")
--   local r = Draw.draw_rect(10, 20, 100, 50, "#ff0000")
--   local t = Draw.draw_text(10, 20, "Hello")
--   local s = Draw.create_scene(800, 600, {r, t})
--
-- ============================================================================

local M = {}

M.VERSION = "0.1.0"

-- ---------------------------------------------------------------------------
-- Internal helper: copy_metadata
-- ---------------------------------------------------------------------------

-- Make a shallow copy of the metadata table, or return {} if nil.
-- This ensures each instruction has its own metadata table and callers can
-- modify it freely without aliasing issues.
local function copy_metadata(meta)
    if meta == nil then return {} end
    local copy = {}
    for k, v in pairs(meta) do
        copy[k] = v
    end
    return copy
end

-- ---------------------------------------------------------------------------
-- draw_rect(x, y, width, height, fill, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a rectangle drawing instruction.
--
-- A rectangle is the most common shape in UI/visualization work. It's defined
-- by its top-left corner (x, y) and its dimensions (width × height).
--
-- Parameters:
--   x        (number)  — left edge in abstract units
--   y        (number)  — top edge in abstract units
--   width    (number)  — horizontal extent
--   height   (number)  — vertical extent
--   fill     (string)  — fill color (default "#000000")
--   metadata (table)   — arbitrary annotations (default {})
--
-- Returns: { kind="rect", x, y, width, height, fill, metadata }
function M.draw_rect(x, y, width, height, fill, metadata)
    return {
        kind     = "rect",
        x        = x,
        y        = y,
        width    = width,
        height   = height,
        fill     = fill or "#000000",
        metadata = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------
-- draw_text(x, y, value, fill, font_family, font_size, align, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a text drawing instruction.
--
-- Text instructions render a string at a given position. The optional
-- typography parameters let backends style the text appropriately.
--
-- Parameters:
--   x           (number)  — x coordinate of the text anchor
--   y           (number)  — y coordinate of the text anchor (baseline)
--   value       (string)  — the text to render
--   fill        (string)  — text color (default "#000000")
--   font_family (string)  — CSS font-family hint (default "monospace")
--   font_size   (number)  — font size in points/pixels (default 16)
--   align       (string)  — text anchor: "start", "middle", or "end"
--                           (default "middle")
--   metadata    (table)   — arbitrary annotations (default {})
--
-- Returns: { kind="text", x, y, value, fill, font_family, font_size, align, metadata }
function M.draw_text(x, y, value, fill, font_family, font_size, align, metadata)
    return {
        kind        = "text",
        x           = x,
        y           = y,
        value       = value,
        fill        = fill        or "#000000",
        font_family = font_family or "monospace",
        font_size   = font_size   or 16,
        align       = align       or "middle",
        metadata    = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------
-- draw_line(x1, y1, x2, y2, stroke, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a line segment drawing instruction.
--
-- A line connects point (x1, y1) to point (x2, y2). It uses a stroke color
-- (no fill — lines have no interior).
--
-- Parameters:
--   x1, y1  (number) — start point
--   x2, y2  (number) — end point
--   stroke  (string) — line color (default "#000000")
--   metadata (table) — arbitrary annotations (default {})
--
-- Returns: { kind="line", x1, y1, x2, y2, stroke, metadata }
function M.draw_line(x1, y1, x2, y2, stroke, metadata)
    return {
        kind     = "line",
        x1       = x1,
        y1       = y1,
        x2       = x2,
        y2       = y2,
        stroke   = stroke or "#000000",
        metadata = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------
-- draw_circle(cx, cy, r, fill, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a circle drawing instruction.
--
-- A circle is defined by its center (cx, cy) and radius r.
--
-- Parameters:
--   cx       (number) — x coordinate of the center
--   cy       (number) — y coordinate of the center
--   r        (number) — radius
--   fill     (string) — fill color (default "#000000")
--   metadata (table)  — arbitrary annotations (default {})
--
-- Returns: { kind="circle", cx, cy, r, fill, metadata }
function M.draw_circle(cx, cy, r, fill, metadata)
    return {
        kind     = "circle",
        cx       = cx,
        cy       = cy,
        r        = r,
        fill     = fill or "#000000",
        metadata = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------
-- draw_group(children, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a group instruction that contains child instructions.
--
-- Groups are how you compose complex drawings from simpler parts. A backend
-- can apply transformations (translation, scaling, opacity) to an entire group
-- at once. In SVG, this maps to a <g> element.
--
-- Parameters:
--   children (table) — ordered list of drawing instructions
--   metadata (table) — arbitrary annotations (default {})
--
-- Returns: { kind="group", children, metadata }
function M.draw_group(children, metadata)
    return {
        kind     = "group",
        children = children or {},
        metadata = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------
-- create_scene(width, height, instructions, background, metadata)
-- ---------------------------------------------------------------------------
--
-- Create a top-level scene (the root of a drawing).
--
-- A scene has fixed dimensions and a background color. All drawing
-- instructions are rendered within these bounds. This is analogous to the
-- <svg> root element or an HTML canvas element.
--
-- Parameters:
--   width        (number) — scene width in abstract units
--   height       (number) — scene height in abstract units
--   instructions (table)  — list of root-level drawing instructions
--   background   (string) — background color (default "#ffffff")
--   metadata     (table)  — arbitrary annotations (default {})
--
-- Returns:
--   { width, height, background, instructions, metadata }
function M.create_scene(width, height, instructions, background, metadata)
    return {
        width        = width,
        height       = height,
        background   = background   or "#ffffff",
        instructions = instructions or {},
        metadata     = copy_metadata(metadata),
    }
end

-- ---------------------------------------------------------------------------

return M
