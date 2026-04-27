# frozen_string_literal: true

module CodingAdventures
  module Barcode2D
    # =========================================================================
    # Default layout configuration
    # =========================================================================
    #
    # These defaults match the TypeScript reference implementation.
    #
    # | Field              | Default   | Why                                    |
    # |--------------------|-----------|----------------------------------------|
    # | module_size_px     | 10        | Produces a readable QR at ~210×210 px  |
    # | quiet_zone_modules | 4         | QR Code minimum per ISO/IEC 18004      |
    # | foreground         | "#000000" | Black ink on white paper               |
    # | background         | "#ffffff" | White paper                            |
    # | show_annotations   | false     | Off by default; opt-in for visualizers |
    # | module_shape       | "square"  | The overwhelmingly common case         |
    DEFAULT_BARCODE_2D_LAYOUT_CONFIG = {
      module_size_px: 10,
      quiet_zone_modules: 4,
      foreground: "#000000",
      background: "#ffffff",
      show_annotations: false,
      module_shape: "square"
    }.freeze

    module_function

    # =========================================================================
    # make_module_grid — create an all-light grid
    # =========================================================================
    #
    # Returns a new frozen ModuleGrid of the given dimensions, every module set
    # to false (light / background).
    #
    # This is the starting point for every 2D barcode encoder:
    #
    #   grid = Barcode2D.make_module_grid(21, 21)   # QR Code v1
    #   grid.modules[0][0]   # => false  (all light)
    #   grid.rows            # => 21
    #   grid.cols            # => 21
    #
    # Parameters:
    #   rows         — vertical dimension (number of rows)
    #   cols         — horizontal dimension (number of columns)
    #   module_shape — "square" (default) or "hex"
    def make_module_grid(rows, cols, module_shape: "square")
      modules = Array.new(rows) { Array.new(cols, false).freeze }.freeze
      ModuleGrid.new(
        cols: cols,
        rows: rows,
        modules: modules,
        module_shape: module_shape.freeze
      ).freeze
    end

    # =========================================================================
    # set_module — immutable single-module update
    # =========================================================================
    #
    # Returns a new ModuleGrid identical to grid except that the module at
    # (row, col) is set to dark.
    #
    # This function is PURE AND IMMUTABLE — it never modifies the input grid.
    # Only the affected row is re-allocated; all other rows are shared.
    #
    # ## Why immutability?
    #
    # Barcode encoders often need to backtrack — for example, trying different
    # QR mask patterns to minimize the penalty score. Immutable grids make this
    # trivial: save the grid before masking, evaluate the penalty, discard if the
    # score is worse, keep the old grid if it is better. No undo stack needed.
    #
    # ## Out-of-bounds
    #
    # Raises RangeError if row or col is outside the grid dimensions. This is
    # always a programming error in the encoder, not a user-facing validation
    # problem.
    #
    # Example:
    #
    #   g  = Barcode2D.make_module_grid(3, 3)
    #   g2 = Barcode2D.set_module(g, 1, 1, true)
    #   g.modules[1][1]   # => false  (original unchanged)
    #   g2.modules[1][1]  # => true
    def set_module(grid, row, col, dark)
      if row < 0 || row >= grid.rows
        raise RangeError, "set_module: row #{row} out of range [0, #{grid.rows - 1}]"
      end
      if col < 0 || col >= grid.cols
        raise RangeError, "set_module: col #{col} out of range [0, #{grid.cols - 1}]"
      end

      # Copy only the affected row — all other rows are shared (shallow copy).
      new_row = grid.modules[row].dup
      new_row[col] = dark
      new_row.freeze

      new_modules = grid.modules.each_with_index.map do |r, i|
        (i == row) ? new_row : r
      end.freeze

      ModuleGrid.new(
        cols: grid.cols,
        rows: grid.rows,
        modules: new_modules,
        module_shape: grid.module_shape
      ).freeze
    end

    # =========================================================================
    # layout — ModuleGrid → PaintScene
    # =========================================================================
    #
    # Converts a ModuleGrid into a PaintScene ready for the PaintVM.
    #
    # This is the ONLY function in the entire 2D barcode stack that knows about
    # pixels. Everything above this step works in abstract module units.
    # Everything below this step is handled by the paint backend (SVG, Canvas,
    # Metal, terminal, etc.).
    #
    # ## Square modules (the common case)
    #
    # Each dark module at (row, col) becomes one PaintRect:
    #
    #   quiet_zone_px = quiet_zone_modules * module_size_px
    #   x = quiet_zone_px + col * module_size_px
    #   y = quiet_zone_px + row * module_size_px
    #
    # Total symbol size (including quiet zone on all four sides):
    #
    #   total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
    #   total_height = (rows + 2 * quiet_zone_modules) * module_size_px
    #
    # The scene always starts with one background PaintRect covering the full
    # symbol, so the quiet zone and light modules are always filled even when
    # the backend default is transparent.
    #
    # ## Hex modules (MaxiCode)
    #
    # Each dark module at (row, col) becomes one PaintPath tracing a flat-top
    # regular hexagon. Odd-numbered rows are offset right by half a hex width:
    #
    #   Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
    #   Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
    #   Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
    #
    # Flat-top hexagon geometry:
    #
    #   hex_width  = module_size_px
    #   hex_height = module_size_px * (√3 / 2)
    #   circum_r   = module_size_px / √3
    #
    # ## Validation
    #
    # Raises InvalidBarcode2DConfigError if:
    #   - module_size_px <= 0
    #   - quiet_zone_modules < 0
    #   - config[:module_shape] != grid.module_shape
    #
    # Parameters:
    #   grid   — the ModuleGrid to render
    #   config — optional hash overriding DEFAULT_BARCODE_2D_LAYOUT_CONFIG
    #
    # Returns: a PaintScene (OpenStruct from CodingAdventures::PaintInstructions)
    def layout(grid, config = nil)
      # Merge partial config with defaults.
      cfg = DEFAULT_BARCODE_2D_LAYOUT_CONFIG.merge(config || {})

      # ── Validation ────────────────────────────────────────────────────────
      if cfg[:module_size_px] <= 0
        raise InvalidBarcode2DConfigError,
          "module_size_px must be > 0, got #{cfg[:module_size_px]}"
      end
      if cfg[:quiet_zone_modules] < 0
        raise InvalidBarcode2DConfigError,
          "quiet_zone_modules must be >= 0, got #{cfg[:quiet_zone_modules]}"
      end
      if cfg[:module_shape] != grid.module_shape
        raise InvalidBarcode2DConfigError,
          "config module_shape \"#{cfg[:module_shape]}\" does not match " \
          "grid module_shape \"#{grid.module_shape}\""
      end

      # Dispatch to the correct rendering path.
      if cfg[:module_shape] == "square"
        layout_square(grid, cfg)
      else
        layout_hex(grid, cfg)
      end
    end

    # =========================================================================
    # layout_square — internal helper for square-module grids
    # =========================================================================
    #
    # Called only by layout() after validation.
    #
    # Algorithm:
    #   1. Compute total pixel dimensions including quiet zone on all four sides.
    #   2. Emit one background PaintRect covering the entire symbol.
    #   3. For each dark module, emit one filled PaintRect.
    #
    # Light modules are implicitly covered by the background rect — no explicit
    # light rects are needed. Instruction count is proportional to the number of
    # dark modules, not the total grid size.
    def layout_square(grid, cfg)
      module_size_px = cfg[:module_size_px]
      quiet_zone_modules = cfg[:quiet_zone_modules]
      foreground = cfg[:foreground]
      background = cfg[:background]

      # Quiet zone in pixels on each side.
      quiet_zone_px = quiet_zone_modules * module_size_px

      # Total canvas dimensions including quiet zone on all four sides.
      total_width = (grid.cols + 2 * quiet_zone_modules) * module_size_px
      total_height = (grid.rows + 2 * quiet_zone_modules) * module_size_px

      instructions = []

      # 1. Background: a single rect covering the entire symbol including the
      #    quiet zone. Ensures light modules and the quiet zone are filled even
      #    when the backend's default is transparent.
      instructions << CodingAdventures::PaintInstructions.paint_rect(
        x: 0,
        y: 0,
        width: total_width,
        height: total_height,
        fill: background
      )

      # 2. One PaintRect per dark module.
      grid.rows.times do |row|
        grid.cols.times do |col|
          next unless grid.modules[row][col]

          # Pixel origin of this module (top-left corner of its square).
          x = quiet_zone_px + col * module_size_px
          y = quiet_zone_px + row * module_size_px

          instructions << CodingAdventures::PaintInstructions.paint_rect(
            x: x,
            y: y,
            width: module_size_px,
            height: module_size_px,
            fill: foreground
          )
        end
      end

      CodingAdventures::PaintInstructions.paint_scene(
        width: total_width,
        height: total_height,
        background: background,
        instructions: instructions
      )
    end

    # =========================================================================
    # layout_hex — internal helper for hex-module grids (MaxiCode)
    # =========================================================================
    #
    # Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons arranged
    # in an offset-row grid. Odd rows are shifted right by half a hex width to
    # interlock with even rows.
    #
    # ## Flat-top hexagon geometry
    #
    # A "flat-top" hexagon has two flat edges at top and bottom:
    #
    #    ___
    #   /   \     ← two vertices at top
    #  |     |
    #   \___/     ← two vertices at bottom
    #
    # For a regular flat-top hexagon with side length s:
    #
    #   hex_width  = s       (flat-to-flat = side length for flat-top regular hex)
    #   hex_height = s * (√3/2)  (vertical distance between row centers)
    #   circum_r   = s / √3      (center-to-vertex distance)
    #
    # ## Tiling offsets
    #
    #   cx = quiet_zone_px + col * hex_width + (row % 2) * (hex_width / 2)
    #   cy = quiet_zone_px + row * hex_height
    #
    # The +hex_width/2 on odd rows produces the standard brick-like offset.
    def layout_hex(grid, cfg)
      module_size_px = cfg[:module_size_px]
      quiet_zone_modules = cfg[:quiet_zone_modules]
      foreground = cfg[:foreground]
      background = cfg[:background]

      # Flat-top hexagon geometry.
      hex_width = module_size_px.to_f
      hex_height = module_size_px * (Math.sqrt(3) / 2.0)
      circum_r = module_size_px / Math.sqrt(3)

      quiet_zone_px = quiet_zone_modules * module_size_px

      # Total canvas size. The +hex_width/2 accounts for odd-row offsets so
      # the rightmost modules on odd rows don't clip outside the canvas.
      total_width = (grid.cols + 2 * quiet_zone_modules) * hex_width + hex_width / 2.0
      total_height = (grid.rows + 2 * quiet_zone_modules) * hex_height

      instructions = []

      # Background rect.
      instructions << CodingAdventures::PaintInstructions.paint_rect(
        x: 0,
        y: 0,
        width: total_width,
        height: total_height,
        fill: background
      )

      # One PaintPath per dark module.
      grid.rows.times do |row|
        grid.cols.times do |col|
          next unless grid.modules[row][col]

          # Center of this hexagon in pixel space.
          # Odd rows shift right by hex_width/2.
          cx = quiet_zone_px + col * hex_width + (row % 2) * (hex_width / 2.0)
          cy = quiet_zone_px + row * hex_height

          instructions << CodingAdventures::PaintInstructions.paint_path(
            build_flat_top_hex_path(cx, cy, circum_r),
            fill: foreground
          )
        end
      end

      CodingAdventures::PaintInstructions.paint_scene(
        width: total_width,
        height: total_height,
        background: background,
        instructions: instructions
      )
    end

    # =========================================================================
    # build_flat_top_hex_path — geometry helper
    # =========================================================================
    #
    # Builds the six PathCommand hashes for a flat-top regular hexagon centered
    # at (cx, cy) with circumscribed circle radius circum_r.
    #
    # Vertex formula:
    #
    #   vertex_i = ( cx + circum_r * cos(i * 60°),
    #                cy + circum_r * sin(i * 60°) )
    #
    # The path starts with a move_to to vertex 0, then five line_to commands to
    # vertices 1–5, then a close command.
    #
    #   Angle  cos(θ)   sin(θ)   vertex role
    #     0°    1        0        right midpoint
    #    60°    0.5      √3/2     bottom-right
    #   120°   -0.5      √3/2     bottom-left
    #   180°   -1        0        left midpoint
    #   240°   -0.5     -√3/2     top-left
    #   300°    0.5     -√3/2     top-right
    def build_flat_top_hex_path(cx, cy, circum_r)
      deg_to_rad = Math::PI / 180.0
      commands = []

      # First vertex: move_to
      angle0 = 0 * 60 * deg_to_rad
      commands << {
        kind: "move_to",
        x: cx + circum_r * Math.cos(angle0),
        y: cy + circum_r * Math.sin(angle0)
      }

      # Remaining 5 vertices: line_to
      (1..5).each do |i|
        angle = i * 60 * deg_to_rad
        commands << {
          kind: "line_to",
          x: cx + circum_r * Math.cos(angle),
          y: cy + circum_r * Math.sin(angle)
        }
      end

      # Close back to vertex 0.
      commands << {kind: "close"}

      commands
    end
  end
end
