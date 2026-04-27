-- draw-instructions-text — Terminal text renderer for Lua draw instruction scenes
-- ============================================================================
--
-- This renderer maps pixel-ish draw instruction coordinates onto a character
-- grid. It is deliberately small: enough to inspect scenes in logs, snapshots,
-- and terminal demos without pulling in a graphical backend.
--
-- Filled rectangles become block characters. Stroked rectangles and axis-
-- aligned lines become box-drawing characters. Text overwrites any underlying
-- box drawing so labels remain legible.

require("coding_adventures.draw_instructions")

local M = {}

M.VERSION = "0.1.0"

local H = "\226\148\128" -- U+2500
local V = "\226\148\130" -- U+2502
local TL = "\226\148\140" -- U+250C
local TR = "\226\148\144" -- U+2510
local BL = "\226\148\148" -- U+2514
local BR = "\226\148\152" -- U+2518
local FILL = "\226\150\136" -- U+2588

local function round(value)
    return math.floor((value or 0) + 0.5)
end

local function new_buffer(rows, cols)
    local chars = {}
    for row = 1, rows do
        chars[row] = {}
        for col = 1, cols do
            chars[row][col] = " "
        end
    end
    return { rows = rows, cols = cols, chars = chars }
end

local function default_clip(buf)
    return { min_row = 1, max_row = buf.rows, min_col = 1, max_col = buf.cols }
end

local function intersect_clip(a, b)
    return {
        min_row = math.max(a.min_row, b.min_row),
        max_row = math.min(a.max_row, b.max_row),
        min_col = math.max(a.min_col, b.min_col),
        max_col = math.min(a.max_col, b.max_col),
    }
end

local function in_clip(row, col, clip)
    return row >= clip.min_row and row <= clip.max_row
       and col >= clip.min_col and col <= clip.max_col
end

local function write(buf, row, col, value, clip)
    if row < 1 or row > buf.rows or col < 1 or col > buf.cols then return end
    if not in_clip(row, col, clip) then return end
    buf.chars[row][col] = value
end

local function to_col(x, scale_x)
    return round((x or 0) / scale_x) + 1
end

local function to_row(y, scale_y)
    return round((y or 0) / scale_y) + 1
end

local render_instruction

local function render_rect(buf, instr, opts, clip)
    local left = to_col(instr.x, opts.scale_x)
    local top = to_row(instr.y, opts.scale_y)
    local right = to_col((instr.x or 0) + (instr.width or 0), opts.scale_x)
    local bottom = to_row((instr.y or 0) + (instr.height or 0), opts.scale_y)

    if instr.stroke ~= nil or instr.fill == "transparent" then
        for col = left + 1, right - 1 do
            write(buf, top, col, H, clip)
            write(buf, bottom, col, H, clip)
        end
        for row = top + 1, bottom - 1 do
            write(buf, row, left, V, clip)
            write(buf, row, right, V, clip)
        end
        write(buf, top, left, TL, clip)
        write(buf, top, right, TR, clip)
        write(buf, bottom, left, BL, clip)
        write(buf, bottom, right, BR, clip)
        return
    end

    for row = top, bottom do
        for col = left, right do
            write(buf, row, col, FILL, clip)
        end
    end
end

local function render_text(buf, instr, opts, clip)
    local value = tostring(instr.value or "")
    local col = to_col(instr.x, opts.scale_x)
    if instr.align == "middle" then
        col = col - math.floor(#value / 2)
    elseif instr.align == "end" then
        col = col - #value + 1
    end
    local row = to_row(instr.y, opts.scale_y)
    for i = 1, #value do
        write(buf, row, col + i - 1, value:sub(i, i), clip)
    end
end

local function render_line(buf, instr, opts, clip)
    local c1 = to_col(instr.x1, opts.scale_x)
    local r1 = to_row(instr.y1, opts.scale_y)
    local c2 = to_col(instr.x2, opts.scale_x)
    local r2 = to_row(instr.y2, opts.scale_y)

    if r1 == r2 then
        if c2 < c1 then c1, c2 = c2, c1 end
        for col = c1, c2 do write(buf, r1, col, H, clip) end
    elseif c1 == c2 then
        if r2 < r1 then r1, r2 = r2, r1 end
        for row = r1, r2 do write(buf, row, c1, V, clip) end
    else
        local steps = math.max(math.abs(c2 - c1), math.abs(r2 - r1))
        for i = 0, steps do
            local t = steps == 0 and 0 or i / steps
            local row = round(r1 + (r2 - r1) * t)
            local col = round(c1 + (c2 - c1) * t)
            write(buf, row, col, "*", clip)
        end
    end
end

local function render_circle(buf, instr, opts, clip)
    write(buf, to_row(instr.cy, opts.scale_y), to_col(instr.cx, opts.scale_x), "o", clip)
end

local function render_group(buf, instr, opts, clip)
    for _, child in ipairs(instr.children or {}) do
        render_instruction(buf, child, opts, clip)
    end
end

local function render_clip(buf, instr, opts, clip)
    local child_clip = intersect_clip(clip, {
        min_row = to_row(instr.y, opts.scale_y),
        max_row = to_row((instr.y or 0) + (instr.height or 0), opts.scale_y),
        min_col = to_col(instr.x, opts.scale_x),
        max_col = to_col((instr.x or 0) + (instr.width or 0), opts.scale_x),
    })
    render_group(buf, instr, opts, child_clip)
end

render_instruction = function(buf, instr, opts, clip)
    if instr.kind == "rect" then
        render_rect(buf, instr, opts, clip)
    elseif instr.kind == "text" then
        render_text(buf, instr, opts, clip)
    elseif instr.kind == "line" then
        render_line(buf, instr, opts, clip)
    elseif instr.kind == "circle" then
        render_circle(buf, instr, opts, clip)
    elseif instr.kind == "group" then
        render_group(buf, instr, opts, clip)
    elseif instr.kind == "clip" then
        render_clip(buf, instr, opts, clip)
    else
        error("unsupported draw instruction kind: " .. tostring(instr.kind))
    end
end

local function buffer_to_string(buf)
    local lines = {}
    for row = 1, buf.rows do
        local line = table.concat(buf.chars[row])
        line = line:gsub("%s+$", "")
        lines[#lines + 1] = line
    end
    local result = table.concat(lines, "\n")
    result = result:gsub("[%s\n]+$", "")
    return result
end

function M.render_text(scene, opts)
    scene = scene or {}
    opts = opts or {}
    opts.scale_x = opts.scale_x or 8
    opts.scale_y = opts.scale_y or 16
    local cols = math.max(1, to_col(scene.width or 0, opts.scale_x))
    local rows = math.max(1, to_row(scene.height or 0, opts.scale_y))
    local buf = new_buffer(rows, cols)
    local clip = default_clip(buf)
    for _, instr in ipairs(scene.instructions or {}) do
        render_instruction(buf, instr, opts, clip)
    end
    return buffer_to_string(buf)
end

return M
