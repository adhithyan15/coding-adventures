-- paint_instructions — Backend-neutral paint scene primitives
-- ================================================================
--
-- ## The Big Picture
--
-- Imagine you want to render a QR code. You have an abstract 21x21 grid of
-- dark/light modules, but you do NOT want to hard-code "draw SVG rectangles"
-- or "draw Metal quads" inside the QR encoder -- that would tie the encoder
-- to one specific backend.
--
-- Instead you write a tiny intermediate language: a list of paint
-- instructions that say "fill this rectangle with this colour", "place
-- these glyphs", etc. Any backend -- SVG, Canvas 2D, terminal ASCII, native
-- GPU -- can read that list and render it in its own way.
--
-- That is exactly what this module provides: a set of constructor functions
-- that build plain Lua tables tagged with a `kind` field. There are no
-- classes or metatables here on purpose -- every "instruction" is just a
-- table any caller can inspect, serialize, or pattern-match on `kind`
-- without needing to know this module's internals. This mirrors the style
-- already used by every other Lua package in this repo (see
-- code/packages/lua/cli_builder, which returns plain result tables the same
-- way).
--
-- ## Instructions at a glance
--
--   rect       -- axis-aligned rectangle, optionally filled and/or stroked
--   line       -- a stroked line segment between two points
--   glyph_run  -- pre-positioned literal glyphs (used by text pipelines)
--   group      -- a child list with an optional transform/opacity
--   clip       -- a rectangular clip region wrapping a child list
--   layer      -- a child list with a filter/blend-mode/opacity/transform
--   path       -- a filled polygon traced by move_to/line_to/close commands
--
-- ## Painter's algorithm
--
-- A PaintScene's `instructions` list is applied in order, like a painter
-- layering paint: earlier instructions go behind later ones.
--
-- ## Colour strings
--
-- Fill/stroke colours are CSS hex strings (`"#rgb"`, `"#rgba"`,
-- `"#rrggbb"`, `"#rrggbbaa"`) or the sentinel values `"transparent"` /
-- `"none"` / `""`, all three of which backends treat as "paint nothing".
--
-- ## Glyph IDs
--
-- `glyph_id` is a font-internal glyph index in the general contract, but
-- text-mode ("ascii") backends relax this to a literal Unicode code point --
-- see P2D02-paint-vm-ascii.md section "Glyph runs" for the rationale, and
-- code/specs/cowsay-paintvm-pipeline.md section 3.1 for why cowsay in
-- particular relies on that relaxation.

local M = {}

M.VERSION = "0.1.0"

-- copy_metadata returns a shallow copy of `metadata`, or a fresh empty table
-- when `metadata` is nil. Every instruction constructor stores its own copy
-- rather than aliasing the caller's table, so mutating a table the caller
-- still holds a reference to can never retroactively change an already-built
-- instruction.
local function copy_metadata(metadata)
    if metadata == nil then
        return {}
    end

    local copy = {}
    for key, value in pairs(metadata) do
        copy[key] = value
    end
    return copy
end

-- copy_list returns a shallow array copy of `list` (or an empty table when
-- `list` is nil), for the same aliasing-safety reason as copy_metadata.
local function copy_list(list)
    if list == nil then
        return {}
    end

    local copy = {}
    for i = 1, #list do
        copy[i] = list[i]
    end
    return copy
end

-- ============================================================================
-- rect
-- ============================================================================

-- paint_rect(x, y, width, height, fill, metadata, stroke, stroke_width)
--
-- An axis-aligned rectangle. Coordinates are in scene units with the origin
-- at the top-left corner; x increases to the right, y increases downward.
--
--   (x, y) ──────────────── x + width
--      |                         |
--      |         filled          |
--      |         rectangle       |
--      |                         |
--   y + height ──────────────────┘
--
-- `stroke` and `stroke_width` were added after this constructor's first
-- release, so they are trailing optional parameters (default: no stroke) --
-- every existing call site in this repo (e.g. barcode-2d, barcode_layout_1d)
-- keeps working unchanged, since Lua treats a missing trailing argument as
-- nil regardless of how many parameters the function declares.
function M.paint_rect(x, y, width, height, fill, metadata, stroke, stroke_width)
    return {
        kind = "rect",
        x = x,
        y = y,
        width = width,
        height = height,
        fill = fill or "#000000",
        metadata = copy_metadata(metadata),
        stroke = stroke or "",
        stroke_width = stroke_width or 0,
    }
end

-- ============================================================================
-- line
-- ============================================================================

-- paint_line(x1, y1, x2, y2, stroke, stroke_width, metadata)
--
-- A stroked line segment between two points. Unlike paint_rect, `stroke` is
-- required (not optional) -- a line with no stroke is invisible, so there is
-- no sensible default to fall back to.
function M.paint_line(x1, y1, x2, y2, stroke, stroke_width, metadata)
    return {
        kind = "line",
        x1 = x1,
        y1 = y1,
        x2 = x2,
        y2 = y2,
        stroke = stroke,
        stroke_width = stroke_width or 0,
        metadata = copy_metadata(metadata),
    }
end

-- ============================================================================
-- glyph_run
-- ============================================================================

-- paint_glyph_placement(glyph_id, x, y)
--
-- One glyph's position within a glyph_run. `glyph_id` is a font-internal
-- glyph index in the general contract; text-mode backends relax this to a
-- literal Unicode code point (see the module doc comment above).
function M.paint_glyph_placement(glyph_id, x, y)
    return {
        glyph_id = glyph_id,
        x = x,
        y = y,
    }
end

-- paint_glyph_run(glyphs, font_ref, font_size, fill, metadata)
--
-- Pre-positioned glyphs, each already placed in scene coordinates via
-- paint_glyph_placement. `font_ref`, `font_size`, and `fill` are required
-- fields on the general contract but are explicitly ignored by the ASCII
-- backend (there is no way to honor arbitrary font selection in a
-- terminal) -- any placeholder value (e.g. font_ref = "terminal-mono") is
-- correct for that backend.
function M.paint_glyph_run(glyphs, font_ref, font_size, fill, metadata)
    return {
        kind = "glyph_run",
        glyphs = copy_list(glyphs),
        font_ref = font_ref,
        font_size = font_size,
        fill = fill,
        metadata = copy_metadata(metadata),
    }
end

-- ============================================================================
-- Transform2D
-- ============================================================================

-- transform2d(a, b, c, d, e, f)
--
-- A six-value affine transform, matching the Canvas/SVG convention:
--
--   x' = a*x + c*y + e
--   y' = b*x + d*y + f
function M.transform2d(a, b, c, d, e, f)
    return { a = a, b = b, c = c, d = d, e = e, f = f }
end

-- identity_transform() -- no rotation, scale, or translation.
function M.identity_transform()
    return M.transform2d(1, 0, 0, 1, 0, 0)
end

-- is_identity_transform(transform) -- true for nil (no transform, meaning
-- "identity") and for a transform table whose six fields all match the
-- identity matrix.
function M.is_identity_transform(transform)
    if transform == nil then
        return true
    end
    return (transform.a or 1) == 1
        and (transform.b or 0) == 0
        and (transform.c or 0) == 0
        and (transform.d or 1) == 1
        and (transform.e or 0) == 0
        and (transform.f or 0) == 0
end

-- ============================================================================
-- group / clip / layer
-- ============================================================================

-- paint_group(children, opts)
--
-- A child list with an optional affine transform and opacity. `opts` is an
-- optional table with keys `transform`, `opacity`, `metadata` -- using an
-- options table (rather than more positional parameters) keeps call sites
-- readable once there is more than one or two optional fields, which is
-- exactly the situation group/layer are in.
--
--   paint.paint_group({ rect1, rect2 })
--   paint.paint_group({ rect1 }, { opacity = 0.5 })
function M.paint_group(children, opts)
    opts = opts or {}
    return {
        kind = "group",
        children = copy_list(children),
        transform = opts.transform,
        opacity = opts.opacity,
        metadata = copy_metadata(opts.metadata),
    }
end

-- paint_clip(x, y, width, height, children, metadata)
--
-- A rectangular clip region (in scene coordinates) wrapping a child list.
-- Children can only paint within the intersection of every enclosing clip
-- rectangle -- see P2D02-paint-vm-ascii.md section "Clipping".
function M.paint_clip(x, y, width, height, children, metadata)
    return {
        kind = "clip",
        x = x,
        y = y,
        width = width,
        height = height,
        children = copy_list(children),
        metadata = copy_metadata(metadata),
    }
end

-- paint_layer(children, opts)
--
-- A child list with a filter flag, blend mode, opacity, and transform.
-- `has_filters` is a simplified stand-in for a full filter-effect union --
-- no backend in this repository's Lua port implements pixel-level filters,
-- so all that matters for dispatch is whether to reject the layer.
--
-- `opts` keys: `has_filters` (boolean, default false), `blend_mode`
-- (string or nil; nil/"normal" means standard alpha compositing),
-- `opacity` (number or nil), `transform`, `metadata`.
function M.paint_layer(children, opts)
    opts = opts or {}
    return {
        kind = "layer",
        children = copy_list(children),
        has_filters = opts.has_filters or false,
        blend_mode = opts.blend_mode,
        opacity = opts.opacity,
        transform = opts.transform,
        metadata = copy_metadata(opts.metadata),
    }
end

-- ============================================================================
-- path
-- ============================================================================

-- paint_path builds a path instruction from a list of commands.
--
-- Each command is a table with a `kind` field:
--   { kind = "move_to", x = ..., y = ... }
--   { kind = "line_to", x = ..., y = ... }
--   { kind = "close" }
--
-- The `fill` string is the colour to use when rendering the closed path.
-- `metadata` is an optional table of key/value annotations.
function M.paint_path(commands, fill, metadata)
    return {
        kind = "path",
        commands = commands or {},
        fill = fill or "#000000",
        metadata = copy_metadata(metadata),
    }
end

-- ============================================================================
-- scene
-- ============================================================================

function M.paint_scene(width, height, instructions, background, metadata)
    return {
        width = width,
        height = height,
        instructions = instructions or {},
        background = background or "#ffffff",
        metadata = copy_metadata(metadata),
    }
end

function M.create_scene(width, height, instructions, background, metadata)
    return M.paint_scene(width, height, instructions, background, metadata)
end

return M
