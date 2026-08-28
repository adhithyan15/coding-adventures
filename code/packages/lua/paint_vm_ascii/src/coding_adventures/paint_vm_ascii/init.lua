-- paint_vm_ascii — Terminal backend for coding_adventures.paint_instructions
-- ================================================================
--
-- Implements the full P2D02-paint-vm-ascii.md contract: filled/stroked
-- rectangles, lines (with a proper Bresenham diagonal case), glyph runs, and
-- plain (untransformed, unfiltered, fully opaque) groups/clips/layers.
--
-- ## Coordinate mapping
--
--   char_col = round(scene_x / scale_x)
--   char_row = round(scene_y / scale_y)
--
-- Default scale factors are scale_x = 8, scale_y = 16 (roughly matching
-- monospace glyph metrics).
--
-- ## Failure philosophy
--
-- Every other language in this repo's cowsay-paintvm-ascii rollout
-- (csharp/fsharp/haskell/java/kotlin/dart/swift) reports errors through a
-- typed Result/Either value. Lua has no sum types, and this package's own
-- prior rect-only version already used `error(...)` for its one failure
-- case ("unsupported paint instruction kind"). This version keeps that
-- idiom rather than inventing a parallel Result-table convention: every
-- failure calls `error()` with a message prefixed "paint_vm_ascii: ", which
-- satisfies the spec's "must fail loudly with an error rather than degrade
-- silently" requirement exactly as well as a typed Err value would, and
-- matches how code/packages/perl/paint-vm-ascii (die/eval) already does the
-- same thing in this repo's other "script language" port.
--
-- Callers who want to trap a failure (e.g. a CLI wanting a clean error
-- message instead of a raw traceback) can wrap a `render` call in `pcall`,
-- same as any other Lua function that can raise.

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- Options
-- ============================================================================

local DEFAULT_SCALE_X = 8
local DEFAULT_SCALE_Y = 16

local function scale_x(options)
    if options == nil or options.scale_x == nil then
        return DEFAULT_SCALE_X
    end
    return options.scale_x
end

local function scale_y(options)
    if options == nil or options.scale_y == nil then
        return DEFAULT_SCALE_Y
    end
    return options.scale_y
end

-- ============================================================================
-- Small numeric helpers
-- ============================================================================

-- is_finite(value) -- false for NaN and +-infinity. Lua has no `isnan`
-- builtin; the standard trick is that NaN is the only value that is not
-- equal to itself.
local function is_finite(value)
    return value == value and value ~= math.huge and value ~= -math.huge
end

local function ceil_div(numerator, denominator)
    return -((-numerator) // denominator)
end

-- Cell-coordinate values are saturated to this bound (rather than left as a
-- raw rounded result) so a large-but-ordinary finite number can never land
-- on an extreme Lua integer. Without this, a clip extent rounding to an
-- extreme value could defeat clamp_col/clamp_row downstream via integer
-- overflow in the `max_col - 1` they compute, un-clamping any shape nested
-- in that clip and reopening the unbounded-iteration DoS the clip clamping
-- exists to prevent (this is the same reasoning code/packages/dart/paint-vm-ascii
-- and code/packages/swift/PaintVmAscii apply -- see their `_cellBound` /
-- `cellBound` constants). A billion cells in either direction is far beyond
-- any real rendered scene (scenes are additionally capped at MAX_AXIS_CELLS
-- per axis below) while leaving enormous headroom below Lua's 64-bit
-- integer range for clamp_col/clamp_row's arithmetic to stay overflow-free.
local CELL_BOUND = 1000000000

-- to_cell(coordinate, scale) -- converts a scene coordinate to a character
-- cell index, rounding half away from zero (matching every other language
-- port's rounding convention, e.g. C#'s
-- Math.Round(..., MidpointRounding.AwayFromZero)).
local function to_cell(coordinate, scale)
    local scaled = coordinate / scale
    if scaled ~= scaled then -- NaN: NaN is the only value unequal to itself.
        return 0
    end
    if scaled >= CELL_BOUND then
        return CELL_BOUND
    end
    if scaled <= -CELL_BOUND then
        return -CELL_BOUND
    end
    if scaled >= 0 then
        return math.floor(scaled + 0.5)
    end
    return -math.floor(-scaled + 0.5)
end

-- ============================================================================
-- Clip bounds
-- ============================================================================

local function new_clip(min_col, min_row, max_col, max_row)
    return { min_col = min_col, min_row = min_row, max_col = max_col, max_row = max_row }
end

local function clip_inside(clip, row, col)
    return row >= clip.min_row and row < clip.max_row
        and col >= clip.min_col and col < clip.max_col
end

-- clamp_col/clamp_row clamp a cell coordinate into a clip's own bounds.
-- Used before building any range that iterates between two cell coordinates
-- (rect fill/stroke, line endpoints), so a caller-supplied geometry with a
-- huge (but valid) extent can't force iteration far beyond the actual
-- clipped surface -- bounded by the clip's own size instead of by caller
-- input.
local function clamp_col(clip, value)
    return math.max(clip.min_col, math.min(value, clip.max_col - 1))
end

local function clamp_row(clip, value)
    return math.max(clip.min_row, math.min(value, clip.max_row - 1))
end

local function clip_intersect(parent, child)
    return new_clip(
        math.max(parent.min_col, child.min_col),
        math.max(parent.min_row, child.min_row),
        math.min(parent.max_col, child.max_col),
        math.min(parent.max_row, child.max_row)
    )
end

-- ============================================================================
-- Buffer
-- ============================================================================
--
-- The buffer is two parallel dense 2D arrays (`chars`, `tags`), sized
-- exactly rows x cols and allocated once up front -- the same shape this
-- package's original rect-only version used, and the same shape
-- code/packages/perl/paint-vm-ascii uses. Scenes rendered by this backend
-- are terminal-sized and hard-capped (see MAX_AXIS_CELLS below), so eager
-- allocation stays cheap; a sparse table keyed by (row, col) pairs would add
-- complexity for no benefit at this scale.

local FLAG_UP = 1
local FLAG_RIGHT = 2
local FLAG_DOWN = 4
local FLAG_LEFT = 8
local FLAG_FILL = 16
local FLAG_TEXT = 32 -- marks a cell as holding literal glyph text.

local BOX_CHARACTERS = {
    [(FLAG_LEFT | FLAG_RIGHT)] = "\u{2500}",
    [(FLAG_UP | FLAG_DOWN)] = "\u{2502}",
    [(FLAG_DOWN | FLAG_RIGHT)] = "\u{250C}",
    [(FLAG_DOWN | FLAG_LEFT)] = "\u{2510}",
    [(FLAG_UP | FLAG_RIGHT)] = "\u{2514}",
    [(FLAG_UP | FLAG_LEFT)] = "\u{2518}",
    [(FLAG_LEFT | FLAG_RIGHT | FLAG_DOWN)] = "\u{252C}",
    [(FLAG_LEFT | FLAG_RIGHT | FLAG_UP)] = "\u{2534}",
    [(FLAG_UP | FLAG_DOWN | FLAG_RIGHT)] = "\u{251C}",
    [(FLAG_UP | FLAG_DOWN | FLAG_LEFT)] = "\u{2524}",
    [(FLAG_UP | FLAG_DOWN | FLAG_LEFT | FLAG_RIGHT)] = "\u{253C}",
    [FLAG_RIGHT] = "\u{2500}",
    [FLAG_LEFT] = "\u{2500}",
    [FLAG_UP] = "\u{2502}",
    [FLAG_DOWN] = "\u{2502}",
}

local FILL_CHAR = "\u{2588}"

local function new_buffer(rows, cols)
    local chars = {}
    local tags = {}
    for row = 1, rows do
        local char_row = {}
        local tag_row = {}
        for col = 1, cols do
            char_row[col] = " "
            tag_row[col] = 0
        end
        chars[row] = char_row
        tags[row] = tag_row
    end
    return { rows = rows, cols = cols, chars = chars, tags = tags }
end

local function resolve_box_char(flags)
    local directions = flags & (FLAG_UP | FLAG_RIGHT | FLAG_DOWN | FLAG_LEFT)
    if directions ~= 0 and BOX_CHARACTERS[directions] then
        return BOX_CHARACTERS[directions]
    end
    if (flags & FLAG_FILL) ~= 0 then
        return FILL_CHAR
    end
    return "+"
end

-- write_tag merges a directional/fill flag into a cell (used by rect/line so
-- intersecting strokes combine into the correct box-drawing glyph). A cell
-- that already holds literal text (from a glyph_run) is never overwritten --
-- text priority, per P2D02's rendering rules.
local function write_tag(buffer, clip, row, col, flags)
    if not clip_inside(clip, row, col) then
        return
    end
    -- clip is always a subset of the buffer's own [0, rows) x [0, cols)
    -- extent (see render()'s construction), so an in-clip cell is always an
    -- in-buffer cell too.
    local existing = buffer.tags[row + 1][col + 1]
    if (existing & FLAG_TEXT) ~= 0 then
        return
    end
    local merged = existing | flags
    buffer.tags[row + 1][col + 1] = merged
    buffer.chars[row + 1][col + 1] = resolve_box_char(merged)
end

-- write_char writes a literal glyph into a cell, unconditionally overwriting
-- whatever box-drawing/fill was there before (text priority, per P2D02).
local function write_char(buffer, clip, row, col, text)
    if not clip_inside(clip, row, col) then
        return
    end
    buffer.chars[row + 1][col + 1] = text
    buffer.tags[row + 1][col + 1] = FLAG_TEXT
end

-- buffer_to_string renders the buffer to the final document string: each
-- line has its trailing spaces trimmed, and trailing blank lines at the end
-- of the document are trimmed too (both spec-mandated, P2D02 "Trimming").
local function buffer_to_string(buffer)
    local lines = {}
    for row = 1, buffer.rows do
        local line = table.concat(buffer.chars[row])
        lines[row] = (line:gsub("%s+$", ""))
    end

    local last_non_blank = 0
    for i = 1, #lines do
        if #lines[i] > 0 then
            last_non_blank = i
        end
    end

    if last_non_blank == 0 then
        return ""
    end

    local trimmed = {}
    for i = 1, last_non_blank do
        trimmed[i] = lines[i]
    end
    return table.concat(trimmed, "\n")
end

-- ============================================================================
-- Validation
-- ============================================================================

local function valid_rectangle(instruction)
    return instruction.width >= 0 and instruction.height >= 0
end

local function valid_line(instruction)
    return is_finite(instruction.x1) and is_finite(instruction.y1)
        and is_finite(instruction.x2) and is_finite(instruction.y2)
end

-- valid_clip validates the individual fields *and* the x+width / y+height
-- extents used to build the clip's cell bounds -- two individually-finite
-- values near the top of the float range can still sum to +-infinity under
-- IEEE-754 arithmetic, so checking the fields alone isn't sufficient to
-- guarantee to_cell() never sees a non-finite input.
local function valid_clip(instruction)
    return is_finite(instruction.x) and is_finite(instruction.y)
        and is_finite(instruction.width) and is_finite(instruction.height)
        and instruction.width >= 0 and instruction.height >= 0
        and is_finite(instruction.x + instruction.width)
        and is_finite(instruction.y + instruction.height)
end

local function is_identity_transform(transform)
    if transform == nil then
        return true
    end
    return (transform.a or 1) == 1 and (transform.b or 0) == 0
        and (transform.c or 0) == 0 and (transform.d or 1) == 1
        and (transform.e or 0) == 0 and (transform.f or 0) == 0
end

local function assert_plain_group(group)
    if not is_identity_transform(group.transform) then
        error("paint_vm_ascii: group with a non-identity transform is not supported")
    end
    if group.opacity ~= nil and group.opacity ~= 1.0 then
        error("paint_vm_ascii: group with non-default opacity is not supported")
    end
end

local function assert_plain_layer(layer)
    if not is_identity_transform(layer.transform) then
        error("paint_vm_ascii: layer with a non-identity transform is not supported")
    end
    if layer.opacity ~= nil and layer.opacity ~= 1.0 then
        error("paint_vm_ascii: layer with non-default opacity is not supported")
    end
    if layer.has_filters then
        error("paint_vm_ascii: layer with filters is not supported")
    end
    if layer.blend_mode ~= nil and layer.blend_mode ~= "normal" then
        error("paint_vm_ascii: layer with a non-normal blend mode is not supported")
    end
end

local function visible_paint(paint)
    if paint == nil then
        return false
    end
    local trimmed = paint:match("^%s*(.-)%s*$")
    return trimmed ~= "" and trimmed ~= "transparent" and trimmed ~= "none"
end

-- ============================================================================
-- Glyph safety filter
-- ============================================================================
--
-- ASCII-backend-specific relaxation of the general PaintGlyphRun contract:
-- glyph_id is treated as a literal Unicode code point here (no font
-- resolution happens in a terminal), per P2D02-paint-vm-ascii.md.
--
-- Control characters, bidi-control code points, and UTF-16 surrogate code
-- points are replaced with "?" so a crafted message can't inject terminal
-- escape sequences (e.g. glyph_id = 0x1B, ESC) or ill-formed UTF-8. Lua's
-- `utf8.char` does NOT validate its input the way, say, Rust's `char::from_u32`
-- does -- it will happily emit the (invalid) byte sequence for a surrogate
-- code point (U+D800-U+DFFF) if asked, since surrogates only exist as a
-- UTF-16 encoding artifact and have no meaning as UTF-8. Rejecting them here
-- mirrors the equivalent check in every already-merged sibling
-- (Java/Kotlin's char cast can only *represent* a lone surrogate rather than
-- reject it, so they filter explicitly too; Dart's larger code-point range
-- needs the same filter for the same reason).
local function is_safe_terminal_code_point(code_point)
    if code_point < 0x20 then
        return false
    end
    if code_point >= 0x7f and code_point <= 0x9f then
        return false
    end
    if code_point >= 0xd800 and code_point <= 0xdfff then
        return false
    end
    if code_point == 0x200e or code_point == 0x200f or code_point == 0x061c then
        return false
    end
    if code_point >= 0x202a and code_point <= 0x202e then
        return false
    end
    if code_point >= 0x2066 and code_point <= 0x2069 then
        return false
    end
    return true
end

local function to_safe_terminal_glyph(code_point)
    if type(code_point) ~= "number" then
        return "?"
    end
    if code_point ~= math.floor(code_point) then
        return "?"
    end
    if code_point < 0 or code_point > 0x10ffff then
        return "?"
    end
    if not is_safe_terminal_code_point(code_point) then
        return "?"
    end
    -- utf8.char can still raise on values it considers malformed (Lua's
    -- library rejects values above 0x7FFFFFFF, which cannot occur here, but
    -- guarding with pcall keeps this function total regardless of Lua
    -- version quirks -- a single bad glyph must never abort the whole
    -- render).
    local ok, result = pcall(utf8.char, code_point)
    if ok then
        return result
    end
    return "?"
end

-- ============================================================================
-- Rect
-- ============================================================================

local function render_rectangle(clip, instruction, sx, sy, buffer)
    local c1 = clamp_col(clip, to_cell(instruction.x, sx))
    local r1 = clamp_row(clip, to_cell(instruction.y, sy))
    local c2 = clamp_col(clip, to_cell(instruction.x + instruction.width, sx))
    local r2 = clamp_row(clip, to_cell(instruction.y + instruction.height, sy))

    if visible_paint(instruction.fill) then
        for row = r1, r2 do
            for col = c1, c2 do
                write_tag(buffer, clip, row, col, FLAG_FILL)
            end
        end
    end

    if instruction.stroke ~= nil and instruction.stroke:match("^%s*(.-)%s*$") ~= "" then
        write_tag(buffer, clip, r1, c1, FLAG_DOWN | FLAG_RIGHT)
        write_tag(buffer, clip, r1, c2, FLAG_DOWN | FLAG_LEFT)
        write_tag(buffer, clip, r2, c1, FLAG_UP | FLAG_RIGHT)
        write_tag(buffer, clip, r2, c2, FLAG_UP | FLAG_LEFT)
        for col = c1 + 1, c2 - 1 do
            write_tag(buffer, clip, r1, col, FLAG_LEFT | FLAG_RIGHT)
            write_tag(buffer, clip, r2, col, FLAG_LEFT | FLAG_RIGHT)
        end
        for row = r1 + 1, r2 - 1 do
            write_tag(buffer, clip, row, c1, FLAG_UP | FLAG_DOWN)
            write_tag(buffer, clip, row, c2, FLAG_UP | FLAG_DOWN)
        end
    end
end

-- ============================================================================
-- Line (horizontal/vertical fast paths + Bresenham for the diagonal case)
-- ============================================================================

local function render_line(clip, instruction, sx, sy, buffer)
    -- Clamped into the clip's own bounds before use -- an out-of-range but
    -- otherwise valid (finite) endpoint can't force iteration or Bresenham
    -- stepping far beyond the actual clipped surface.
    local c1 = clamp_col(clip, to_cell(instruction.x1, sx))
    local r1 = clamp_row(clip, to_cell(instruction.y1, sy))
    local c2 = clamp_col(clip, to_cell(instruction.x2, sx))
    local r2 = clamp_row(clip, to_cell(instruction.y2, sy))

    if r1 == r2 then
        local min_col = math.min(c1, c2)
        local max_col = math.max(c1, c2)
        for col = min_col, max_col do
            local flags
            if min_col == max_col then
                flags = FLAG_LEFT | FLAG_RIGHT
            elseif col == min_col then
                flags = FLAG_RIGHT
            elseif col == max_col then
                flags = FLAG_LEFT
            else
                flags = FLAG_LEFT | FLAG_RIGHT
            end
            write_tag(buffer, clip, r1, col, flags)
        end
        return
    end

    if c1 == c2 then
        local min_row = math.min(r1, r2)
        local max_row = math.max(r1, r2)
        for row = min_row, max_row do
            local flags
            if min_row == max_row then
                flags = FLAG_UP | FLAG_DOWN
            elseif row == min_row then
                flags = FLAG_DOWN
            elseif row == max_row then
                flags = FLAG_UP
            else
                flags = FLAG_UP | FLAG_DOWN
            end
            write_tag(buffer, clip, row, c1, flags)
        end
        return
    end

    local delta_row = math.abs(r2 - r1)
    local delta_col = math.abs(c2 - c1)
    local step_row = (r1 < r2) and 1 or -1
    local step_col = (c1 < c2) and 1 or -1
    local diagonal_flags = (delta_col > delta_row) and (FLAG_LEFT | FLAG_RIGHT) or (FLAG_UP | FLAG_DOWN)

    -- The error term is seeded to delta_col - delta_row (the standard
    -- Bresenham initialization), NOT 0. Seeding from 0 lets `row` overshoot
    -- `r2` for some slopes (verified: delta_row=1, delta_col=3) without the
    -- loop's break condition (row == r2 and col == c2) ever becoming true
    -- again, hanging forever -- a real bug found in the haskell/java/kotlin
    -- ports of this exact algorithm (GitHub issue #12093) and independently
    -- avoided from the start in the dart and swift ports. This port follows
    -- the dart/swift precedent and seeds correctly from the first line of
    -- the loop.
    local row = r1
    local col = c1
    local error_term = delta_col - delta_row
    while true do
        write_tag(buffer, clip, row, col, diagonal_flags)
        if row == r2 and col == c2 then
            break
        end
        local doubled = 2 * error_term
        if doubled > -delta_row then
            error_term = error_term - delta_row
            col = col + step_col
        end
        if doubled < delta_col then
            error_term = error_term + delta_col
            row = row + step_row
        end
    end
end

-- ============================================================================
-- Glyph run
-- ============================================================================

-- A glyph with a non-finite position is skipped rather than passed to
-- to_cell() -- unlike a malformed rect/line/clip, a single bad glyph
-- placement doesn't need to fail the whole render.
local function render_glyph_run(clip, instruction, sx, sy, buffer)
    for _, glyph in ipairs(instruction.glyphs or {}) do
        if is_finite(glyph.x) and is_finite(glyph.y) then
            local row = to_cell(glyph.y, sy)
            local col = to_cell(glyph.x, sx)
            write_char(buffer, clip, row, col, to_safe_terminal_glyph(glyph.glyph_id))
        end
    end
end

-- ============================================================================
-- Top-level render
-- ============================================================================

-- Upper bound on the number of character cells a rendered scene may occupy,
-- both in total and per axis. Scene dimensions are otherwise only checked
-- for being non-negative, so without this a caller-supplied width/height of
-- e.g. one billion would force buffer allocation (and buffer_to_string) to
-- work through an enormous number of cells even with zero drawing
-- instructions -- a denial-of-service unrelated to (and not fixed by) the
-- per-instruction clip clamping. The per-axis bound is required in addition
-- to the product bound: a zero-width, huge-height scene has a product of
-- zero (passing a product-only check) while still forcing an unbounded
-- traversal along the surviving axis. 2000x2000 (a generous terminal-sized
-- canvas) is cheap to fully materialize either way. Mirrors
-- code/packages/dart/paint-vm-ascii's `_maxAxisCells` / `_maxBufferCells`.
local MAX_AXIS_CELLS = 2000
local MAX_BUFFER_CELLS = MAX_AXIS_CELLS * MAX_AXIS_CELLS

-- Upper bound on how deeply group/clip/layer children may nest. dispatch()
-- recurses one Lua call frame per nesting level with no other bound on
-- depth, so a scene built from deeply nested wrapper instructions (each with
-- a single child) could otherwise exhaust the C call stack -- a fatal "stack
-- overflow" that, unlike every other error this backend reports, cannot be
-- trapped with pcall. 64 levels is far beyond any real scene (this
-- package's own scenes are always flat: one glyph_run per line, no nesting)
-- while stopping a pathological scene long before it threatens the stack.
-- Mirrors dart/swift's SceneTooDeep cap.
local MAX_NESTING_DEPTH = 64

local dispatch -- forward declaration; group/clip/layer recurse through it.

local function dispatch_children(clip, children, sx, sy, buffer, depth)
    local next_depth = depth + 1
    if next_depth > MAX_NESTING_DEPTH then
        error("paint_vm_ascii: scene nests groups/clips/layers more than " .. MAX_NESTING_DEPTH .. " levels deep")
    end
    for _, child in ipairs(children or {}) do
        dispatch(clip, child, sx, sy, buffer, next_depth)
    end
end

function dispatch(clip, instruction, sx, sy, buffer, depth)
    local kind = instruction.kind

    if kind == "rect" then
        if not valid_rectangle(instruction) then
            error("paint_vm_ascii: rect has negative width or height")
        end
        render_rectangle(clip, instruction, sx, sy, buffer)
    elseif kind == "line" then
        if not valid_line(instruction) then
            error("paint_vm_ascii: line has a non-finite coordinate")
        end
        render_line(clip, instruction, sx, sy, buffer)
    elseif kind == "glyph_run" then
        render_glyph_run(clip, instruction, sx, sy, buffer)
    elseif kind == "group" then
        assert_plain_group(instruction)
        dispatch_children(clip, instruction.children, sx, sy, buffer, depth)
    elseif kind == "clip" then
        if not valid_clip(instruction) then
            error("paint_vm_ascii: clip has a non-finite or negative-size geometry")
        end
        local child_clip = new_clip(
            to_cell(instruction.x, sx),
            to_cell(instruction.y, sy),
            to_cell(instruction.x + instruction.width, sx),
            to_cell(instruction.y + instruction.height, sy)
        )
        dispatch_children(clip_intersect(clip, child_clip), instruction.children, sx, sy, buffer, depth)
    elseif kind == "layer" then
        assert_plain_layer(instruction)
        dispatch_children(clip, instruction.children, sx, sy, buffer, depth)
    elseif kind == "path" then
        error("paint_vm_ascii: path instructions are not supported by this backend")
    else
        error("paint_vm_ascii: unsupported paint instruction kind: " .. tostring(kind))
    end
end

-- render(scene, options) -> string
--
-- Renders a PaintScene (from coding_adventures.paint_instructions) to a
-- terminal-friendly string. `options` is an optional table with `scale_x` /
-- `scale_y` keys; both default to the spec's documented defaults (8, 16).
--
-- Raises (via Lua's `error`) on any geometry that is invalid, out of bounds,
-- or uses an instruction/feature this backend does not support -- see the
-- module doc comment above for why this uses `error` rather than a typed
-- result.
function M.render(scene, options)
    local sx = scale_x(options)
    local sy = scale_y(options)
    -- `sx <= 0` alone does not reject NaN (every comparison against NaN is
    -- false, including `<= 0`), so a NaN scale would otherwise slip past
    -- this check and reach ceil_div() below -- explicitly require
    -- is_finite() too, not just the sign check.
    if type(sx) ~= "number" or not is_finite(sx) or sx <= 0 then
        error("paint_vm_ascii: options.scale_x must be a positive number, got: " .. tostring(sx))
    end
    if type(sy) ~= "number" or not is_finite(sy) or sy <= 0 then
        error("paint_vm_ascii: options.scale_y must be a positive number, got: " .. tostring(sy))
    end
    -- Same NaN gap as above: `scene.width < 0` is false for NaN, which
    -- would otherwise reach ceil_div()/the scene-size cap check with a NaN
    -- cols/rows value (NaN comparisons there are also false, silently
    -- bypassing MAX_AXIS_CELLS/MAX_BUFFER_CELLS).
    if not is_finite(scene.width) or not is_finite(scene.height)
        or scene.width < 0 or scene.height < 0 then
        error("paint_vm_ascii: scene width/height must be finite and non-negative, got: "
            .. tostring(scene.width) .. "x" .. tostring(scene.height))
    end

    local cols = ceil_div(scene.width, sx)
    local rows = ceil_div(scene.height, sy)
    if cols > MAX_AXIS_CELLS or rows > MAX_AXIS_CELLS or cols * rows > MAX_BUFFER_CELLS then
        error("paint_vm_ascii: scene is too large to render ("
            .. cols .. "x" .. rows .. " cells, max " .. MAX_AXIS_CELLS .. "x" .. MAX_AXIS_CELLS .. ")")
    end

    local clip = new_clip(0, 0, cols, rows)
    local buffer = new_buffer(rows, cols)

    for _, instruction in ipairs(scene.instructions or {}) do
        dispatch(clip, instruction, sx, sy, buffer, 0)
    end

    return buffer_to_string(buffer)
end

return M
