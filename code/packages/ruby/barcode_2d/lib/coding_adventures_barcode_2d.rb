# frozen_string_literal: true

# ============================================================================
# coding_adventures_barcode_2d — package entry point
# ============================================================================
#
# This is the file that `require "coding_adventures_barcode_2d"` loads. It pulls
# in paint_instructions (the dependency) first, then loads the barcode_2d
# namespace.
#
# ## What this package provides
#
# This package is the shared 2D barcode abstraction layer. It sits between
# format-specific encoders (QR Code, Data Matrix, Aztec Code, MaxiCode) and the
# PaintVM rendering backend. It provides two things:
#
#   1. ModuleGrid — the universal intermediate representation produced by every
#      2D barcode encoder. A 2D boolean grid: true = dark module, false = light.
#
#   2. layout() — converts a ModuleGrid into a PaintScene ready for the PaintVM.
#      This is the only function that knows about pixels. Everything above it
#      works in abstract module units; everything below is the paint backend.
#
# ## Pipeline
#
#   Input data
#     → format encoder (qr-code, data-matrix, aztec…)
#     → ModuleGrid            ← produced by the encoder
#     → Barcode2D.layout()    ← THIS PACKAGE converts to pixels
#     → PaintScene            ← consumed by paint-vm (P2D01)
#     → backend (SVG, Metal, Canvas, terminal…)
#
# ## Usage
#
#   require "coding_adventures_barcode_2d"
#
#   grid = CodingAdventures::Barcode2D.make_module_grid(21, 21)
#   grid = CodingAdventures::Barcode2D.set_module(grid, 0, 0, true)
#   scene = CodingAdventures::Barcode2D.layout(grid)
#   # scene is a PaintScene OpenStruct ready for PaintVM

require "coding_adventures_paint_instructions"
require_relative "coding_adventures/barcode_2d"
