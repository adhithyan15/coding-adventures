# frozen_string_literal: true

module CodingAdventures
  module Barcode2D
    # =========================================================================
    # ModuleGrid — the universal intermediate representation for 2D barcodes
    # =========================================================================
    #
    # A ModuleGrid is a 2D boolean grid where:
    #
    #   modules[row][col] == true   →  dark module (ink / filled)
    #   modules[row][col] == false  →  light module (background / empty)
    #
    # Row 0 is the top row. Column 0 is the leftmost column.
    # This matches the natural reading order in every 2D barcode standard.
    #
    # ## Immutability
    #
    # ModuleGrid is intentionally immutable (frozen). Use
    # Barcode2D.set_module(grid, row, col, dark) to produce a new grid with one
    # module changed. Encoders start with make_module_grid() and build the final
    # symbol by accumulating immutable set_module() updates — no mutation needed.
    #
    # This makes encoders easy to test: save any intermediate grid, compare,
    # discard if wrong, keep the old one if right.
    #
    # ## Module shapes
    #
    # - "square" — QR Code, Data Matrix, Aztec Code, PDF417 (the common case)
    # - "hex"    — MaxiCode (ISO/IEC 16023), which uses flat-top hexagons
    #
    # The shape is stored on the grid so that layout() can dispatch to the
    # right rendering path automatically.
    #
    # ## Struct fields
    #
    # cols         — number of columns (horizontal dimension)
    # rows         — number of rows    (vertical dimension)
    # modules      — 2D array of booleans, rows×cols, frozen
    # module_shape — "square" or "hex" (frozen string)
    #
    # Example — create a 3×3 grid and dark the center module:
    #
    #   grid = Barcode2D.make_module_grid(3, 3)
    #   grid2 = Barcode2D.set_module(grid, 1, 1, true)
    #   grid.modules[1][1]   # => false  (original unchanged)
    #   grid2.modules[1][1]  # => true
    ModuleGrid = Struct.new(:cols, :rows, :modules, :module_shape, keyword_init: true)

    # =========================================================================
    # ModuleRole constants
    # =========================================================================
    #
    # These symbolic constants identify the structural role of a module inside
    # its barcode symbol. They are purely informational — the renderer only
    # reads the boolean modules grid and ignores role annotations.
    #
    # Roles:
    #
    #   FINDER    — locator pattern that lets scanners orient the symbol.
    #               QR Code: three 7×7 corners. Data Matrix: L-shaped border.
    #
    #   SEPARATOR — quiet strip between a finder and the data area. Always light.
    #
    #   TIMING    — alternating dark/light calibration strip. Lets the scanner
    #               measure module size and compensate for perspective distortion.
    #
    #   ALIGNMENT — secondary locator pattern for large QR Code symbols to
    #               correct for lens distortion over large areas.
    #
    #   FORMAT    — encodes error correction level + mask indicator (QR),
    #               or other symbol-level metadata decoded before the data.
    #
    #   DATA      — one bit of an encoded data codeword. The message lives here.
    #
    #   ECC       — one bit of an error correction codeword (Reed-Solomon).
    #               Allows scanners to recover from partial symbol damage.
    #
    #   PADDING   — filler bits used when the message is shorter than capacity.
    module ModuleRole
      FINDER = "finder"
      SEPARATOR = "separator"
      TIMING = "timing"
      ALIGNMENT = "alignment"
      FORMAT = "format"
      DATA = "data"
      ECC = "ecc"
      PADDING = "padding"

      ALL = [FINDER, SEPARATOR, TIMING, ALIGNMENT, FORMAT, DATA, ECC, PADDING].freeze
    end
  end
end
