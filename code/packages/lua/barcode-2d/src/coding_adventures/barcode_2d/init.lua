-- coding-adventures-barcode-2d
--
-- Shared 2D barcode abstraction layer.
--
-- This module provides the two building blocks every 2D barcode format needs:
--
--   1. ModuleGrid -- the universal intermediate representation produced by
--      every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
--      It is a 2D boolean grid: true = dark module, false = light module.
--
--   2. layout() -- the single function that converts abstract module
--      coordinates into pixel-level PaintScene instructions ready for the
--      PaintVM (P2D01) to render.
--
-- ## Where this fits in the pipeline
--
--   Input data
--     -> format encoder (qr-code, data-matrix, aztec...)
--     -> ModuleGrid          <- produced by the encoder
--     -> layout()            <- THIS MODULE converts to pixels
--     -> PaintScene          <- consumed by paint-vm (P2D01)
--     -> backend (SVG, Metal, Canvas, terminal...)
--
-- All coordinates before layout() are measured in "module units" -- abstract
-- grid steps. Only layout() multiplies by module_size_px to produce real
-- pixel coordinates. Encoders never need to know about screen resolution.
--
-- ## Supported module shapes
--
--   "square" (default): QR Code, Data Matrix, Aztec Code, PDF417.
--   Each module becomes a paint_rect instruction.
--
--   "hex" (flat-top hexagons): MaxiCode. Each module becomes a paint_path
--   instruction tracing six vertices.

local paint = require("coding_adventures.paint_instructions")

local M = {}

M.VERSION = "0.1.0"

-- ============================================================================
-- DEFAULT_CONFIG -- sensible defaults for layout()
-- ============================================================================
--
-- | Field               | Default    | Why                                      |
-- |---------------------|------------|------------------------------------------|
-- | module_size_px      | 10         | Produces a readable QR at 210x210 px     |
-- | quiet_zone_modules  | 4          | QR Code minimum per ISO/IEC 18004        |
-- | foreground          | "#000000"  | Black ink on white paper                 |
-- | background          | "#ffffff"  | White paper                              |
-- | show_annotations    | false      | Off by default; opt-in for visualizers   |
-- | module_shape        | "square"   | The overwhelmingly common case           |

M.DEFAULT_CONFIG = {
    module_size_px     = 10.0,
    quiet_zone_modules = 4,
    foreground         = "#000000",
    background         = "#ffffff",
    show_annotations   = false,
    module_shape       = "square",
}

-- ============================================================================
-- make_module_grid -- create an all-light grid
-- ============================================================================
--
-- Creates a new ModuleGrid with every module set to false (light).
-- This is the starting point for every 2D barcode encoder.
--
-- Parameters:
--   rows         -- number of rows (height)
--   cols         -- number of columns (width)
--   module_shape -- "square" (default) or "hex"
--
-- Returns a table:
--   {
--     rows         = <number>,
--     cols         = <number>,
--     modules      = <table of tables of bool>,  -- modules[row][col], 1-indexed
--     module_shape = <string>,
--   }
--
-- Example -- start a 21x21 QR Code v1 grid:
--
--   local grid = barcode_2d.make_module_grid(21, 21)
--   -- grid.modules[1][1] == false  (all light)
--   -- grid.rows == 21
--   -- grid.cols == 21
--
-- Note: Lua uses 1-based indexing. Row 1 is the top row. Column 1 is the
-- leftmost column. This matches Lua convention while preserving the visual
-- reading order used in barcode standards.

function M.make_module_grid(rows, cols, module_shape)
    module_shape = module_shape or "square"

    -- Build a 2D array of false values. Each row is an independent sub-table
    -- so that set_module() can replace individual rows without copying the
    -- entire grid. We initialise every entry to false (light module).
    local modules = {}
    for r = 1, rows do
        local row = {}
        for c = 1, cols do
            row[c] = false
        end
        modules[r] = row
    end

    return {
        rows         = rows,
        cols         = cols,
        modules      = modules,
        module_shape = module_shape,
    }
end

-- ============================================================================
-- set_module -- immutable single-module update
-- ============================================================================
--
-- Returns a new ModuleGrid identical to `grid` except that the module at
-- (row, col) is set to `dark`. This function is pure and immutable -- it
-- never modifies the input grid.
--
-- Only the affected row is re-allocated; all other rows are shared between
-- old and new grids (shallow copy of the outer table plus one row copy).
--
-- Parameters:
--   grid  -- a ModuleGrid table (from make_module_grid)
--   row   -- 1-based row index
--   col   -- 1-based column index
--   dark  -- true for a dark module, false for a light module
--
-- Errors:
--   Raises an error if row or col is outside the grid dimensions.
--
-- Example:
--
--   local g  = make_module_grid(3, 3)
--   local g2 = set_module(g, 2, 2, true)
--   -- g.modules[2][2]  == false   (original unchanged)
--   -- g2.modules[2][2] == true
--   -- g ~= g2          (different tables)

function M.set_module(grid, row, col, dark)
    -- Validate bounds (1-indexed). Programming errors in the encoder, not
    -- user-facing errors, so we raise immediately with a clear message.
    if row < 1 or row > grid.rows then
        error(string.format(
            "set_module: row %d out of range [1, %d]", row, grid.rows))
    end
    if col < 1 or col > grid.cols then
        error(string.format(
            "set_module: col %d out of range [1, %d]", col, grid.cols))
    end

    -- Copy only the affected row; all other rows are referenced directly
    -- from the original grid (they are never mutated).
    local new_row = {}
    for c = 1, grid.cols do
        new_row[c] = grid.modules[row][c]
    end
    new_row[col] = dark

    -- Shallow-copy the outer modules table, replacing just the one row.
    local new_modules = {}
    for r = 1, grid.rows do
        if r == row then
            new_modules[r] = new_row
        else
            new_modules[r] = grid.modules[r]
        end
    end

    return {
        rows         = grid.rows,
        cols         = grid.cols,
        modules      = new_modules,
        module_shape = grid.module_shape,
    }
end

-- ============================================================================
-- layout -- ModuleGrid -> PaintScene
-- ============================================================================
--
-- Convert a ModuleGrid into a PaintScene ready for the PaintVM.
--
-- This is the ONLY function in the entire 2D barcode stack that knows about
-- pixels. Everything above this step works in abstract module units.
-- Everything below this step is handled by the paint backend.
--
-- Parameters:
--   grid   -- a ModuleGrid table
--   config -- optional table with overrides for DEFAULT_CONFIG fields
--
-- Returns a PaintScene table (from paint_instructions.paint_scene).
--
-- Errors:
--   Raises an error ("InvalidBarcode2DConfigError: ...") if:
--     - module_size_px <= 0
--     - quiet_zone_modules < 0
--     - config.module_shape does not match grid.module_shape
--
-- Square modules (the common case):
--
--   quiet_zone_px = quiet_zone_modules * module_size_px
--   x = quiet_zone_px + (col - 1) * module_size_px    (converting 1-indexed to pixel)
--   y = quiet_zone_px + (row - 1) * module_size_px
--
--   total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
--   total_height = (rows + 2 * quiet_zone_modules) * module_size_px
--
-- Hex modules (MaxiCode):
--
--   A flat-top hexagon has two flat edges at the top and bottom:
--
--       ___
--      /   \      <- two vertices at top
--     |     |
--      \___/      <- two vertices at bottom
--
--   Hex geometry:
--     hex_width  = module_size_px
--     hex_height = module_size_px * (sqrt(3) / 2)   (row center-to-center step)
--     circum_r   = module_size_px / sqrt(3)          (center to vertex distance)
--
--   Center of module at (row, col), including quiet zone (row/col are 1-indexed):
--     cx = quiet_zone_px + (col - 1) * hex_width + ((row - 1) % 2) * (hex_width / 2)
--     cy = quiet_zone_px + (row - 1) * hex_height
--
--   Vertices of flat-top hex at (cx, cy) with circum_r:
--     vertex i (i = 0..5): ( cx + circum_r * cos(i * 60 deg),
--                             cy + circum_r * sin(i * 60 deg) )

function M.layout(grid, config)
    -- Merge caller-supplied config with defaults. Fields not supplied fall
    -- through to DEFAULT_CONFIG.
    local cfg = {}
    for k, v in pairs(M.DEFAULT_CONFIG) do
        cfg[k] = v
    end
    if config ~= nil then
        for k, v in pairs(config) do
            cfg[k] = v
        end
    end

    -- Validation
    if cfg.module_size_px <= 0 then
        error(string.format(
            "InvalidBarcode2DConfigError: module_size_px must be > 0, got %s",
            tostring(cfg.module_size_px)))
    end
    if cfg.quiet_zone_modules < 0 then
        error(string.format(
            "InvalidBarcode2DConfigError: quiet_zone_modules must be >= 0, got %s",
            tostring(cfg.quiet_zone_modules)))
    end
    if cfg.module_shape ~= grid.module_shape then
        error(string.format(
            'InvalidBarcode2DConfigError: config.module_shape "%s" does not match grid.module_shape "%s"',
            cfg.module_shape, grid.module_shape))
    end

    -- Dispatch to the correct rendering path.
    if cfg.module_shape == "square" then
        return M._layout_square(grid, cfg)
    else
        return M._layout_hex(grid, cfg)
    end
end

-- ============================================================================
-- _layout_square -- internal helper for square-module grids
-- ============================================================================
--
-- Renders a square-module ModuleGrid into a PaintScene.
-- Called only by layout() after validation.
--
-- Algorithm:
--   1. Compute total pixel dimensions including quiet zone.
--   2. Emit one background paint_rect covering the entire symbol.
--   3. For each dark module, emit one filled paint_rect.
--
-- Light modules are implicitly covered by the background rect -- no explicit
-- light rects are emitted. This keeps instruction count proportional to the
-- number of dark modules, not the total grid size.

function M._layout_square(grid, cfg)
    local s    = cfg.module_size_px
    local qz   = cfg.quiet_zone_modules
    local fg   = cfg.foreground
    local bg   = cfg.background

    -- Quiet zone in pixels on each side.
    local quiet_px = qz * s

    -- Total canvas dimensions (quiet zone on all four sides).
    local total_w = (grid.cols + 2 * qz) * s
    local total_h = (grid.rows + 2 * qz) * s

    local instructions = {}

    -- 1. Background: one rect covering the full symbol including quiet zone.
    --    This ensures light modules and the quiet zone are always filled even
    --    when the backend default is transparent.
    instructions[#instructions + 1] = paint.paint_rect(0, 0, total_w, total_h, bg)

    -- 2. One paint_rect per dark module.
    --    Row and col are 1-indexed; pixel origin is shifted by (col-1)*s and
    --    (row-1)*s from the quiet zone edge.
    for row = 1, grid.rows do
        for col = 1, grid.cols do
            if grid.modules[row][col] then
                local x = quiet_px + (col - 1) * s
                local y = quiet_px + (row - 1) * s
                instructions[#instructions + 1] = paint.paint_rect(x, y, s, s, fg)
            end
        end
    end

    return paint.paint_scene(total_w, total_h, instructions, bg)
end

-- ============================================================================
-- _layout_hex -- internal helper for hex-module grids (MaxiCode)
-- ============================================================================
--
-- Renders a hex-module ModuleGrid into a PaintScene.
-- Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
-- offset-row grid. Even-indexed rows (in 1-indexed terms: rows 1, 3, 5...) are
-- at the base position; odd-indexed rows (2, 4, 6...) are shifted right by
-- half a hexagon width to produce the standard interlocking tiling:
--
--   Row 1:  hex hex hex hex hex    (no offset)
--   Row 2:   hex hex hex hex hex   (offset right by hex_width/2)
--   Row 3:  hex hex hex hex hex    (no offset)
--
-- This matches the TypeScript implementation which offsets rows where
-- (row_0indexed % 2) == 1. In 1-indexed terms that is (row - 1) % 2 == 1.

function M._layout_hex(grid, cfg)
    local s    = cfg.module_size_px
    local qz   = cfg.quiet_zone_modules
    local fg   = cfg.foreground
    local bg   = cfg.background

    -- Flat-top hexagon geometry:
    --   hex_width  = one module width (flat-to-flat distance = side length)
    --   hex_height = vertical step between row centers = s * (sqrt(3) / 2)
    --   circum_r   = center-to-vertex radius = s / sqrt(3)
    --
    -- The circumscribed radius is derived from the flat-top hex constraint
    -- that the flat (horizontal) edge spans the full width s. For a regular
    -- hexagon with side length L, the flat-to-flat width is L (not 2L as for
    -- pointy-top). So L = s and circum_r = L / sqrt(3) = s / sqrt(3).
    local sqrt3     = math.sqrt(3)
    local hex_w     = s
    local hex_h     = s * (sqrt3 / 2)
    local circum_r  = s / sqrt3
    local deg_to_rad = math.pi / 180

    local quiet_px = qz * s

    -- Total canvas size. The extra hex_w/2 prevents the offset odd rows from
    -- clipping outside the canvas on the right.
    local total_w = (grid.cols + 2 * qz) * hex_w + hex_w / 2
    local total_h = (grid.rows + 2 * qz) * hex_h

    local instructions = {}

    -- Background rect.
    instructions[#instructions + 1] = paint.paint_rect(0, 0, total_w, total_h, bg)

    -- One paint_path per dark module.
    for row = 1, grid.rows do
        for col = 1, grid.cols do
            if grid.modules[row][col] then
                -- Convert 1-indexed row/col to 0-indexed offsets for
                -- the pixel computation (matches TypeScript where row
                -- and col are already 0-indexed).
                local r0 = row - 1
                local c0 = col - 1

                -- Center of this hexagon in pixel space.
                -- Odd 0-indexed rows shift right by hex_w/2 (offset tiling).
                local cx = quiet_px + c0 * hex_w + (r0 % 2) * (hex_w / 2)
                local cy = quiet_px + r0 * hex_h

                -- Build the six-vertex flat-top hexagon path.
                local commands = M._build_flat_top_hex_path(cx, cy, circum_r, deg_to_rad)
                instructions[#instructions + 1] = paint.paint_path(commands, fg)
            end
        end
    end

    return paint.paint_scene(total_w, total_h, instructions, bg)
end

-- ============================================================================
-- _build_flat_top_hex_path -- geometry helper
-- ============================================================================
--
-- Build the six path commands for a flat-top regular hexagon.
--
-- The six vertices are placed at angles 0, 60, 120, 180, 240, 300 degrees
-- from the center (cx, cy) at circumradius circum_r:
--
--   vertex_i = ( cx + circum_r * cos(i * 60 degrees),
--                cy + circum_r * sin(i * 60 degrees) )
--
--   angle  cos    sin    position
--     0      1      0     right midpoint
--    60     0.5   sqrt3/2   bottom-right
--   120    -0.5   sqrt3/2   bottom-left
--   180    -1      0     left midpoint
--   240    -0.5  -sqrt3/2   top-left
--   300     0.5  -sqrt3/2   top-right
--
-- The path starts with move_to at vertex 0, then five line_to commands to
-- vertices 1-5, then a close command to return to vertex 0.
--
-- Parameters:
--   cx          -- center x in pixels
--   cy          -- center y in pixels
--   circum_r    -- circumscribed circle radius (center to vertex) in pixels
--   deg_to_rad  -- precomputed math.pi / 180 (avoids repeated division)

function M._build_flat_top_hex_path(cx, cy, circum_r, deg_to_rad)
    local commands = {}

    -- Vertex 0: move_to
    local angle0 = 0 * 60 * deg_to_rad
    commands[1] = {
        kind = "move_to",
        x    = cx + circum_r * math.cos(angle0),
        y    = cy + circum_r * math.sin(angle0),
    }

    -- Vertices 1-5: line_to
    for i = 1, 5 do
        local angle = i * 60 * deg_to_rad
        commands[#commands + 1] = {
            kind = "line_to",
            x    = cx + circum_r * math.cos(angle),
            y    = cy + circum_r * math.sin(angle),
        }
    end

    -- Close the path back to vertex 0.
    commands[#commands + 1] = { kind = "close" }

    return commands
end

return M
