# frozen_string_literal: true

# ============================================================================
# coding_adventures/barcode_2d — the top-level namespace entry point
# ============================================================================
#
# This file loads all sub-modules in dependency order:
#   1. version   — VERSION constant (no deps)
#   2. errors    — error classes (no deps)
#   3. module_grid — ModuleGrid struct + ModuleRole constants (no deps)
#   4. layout    — make_module_grid, set_module, layout() (needs the above)
#
# Callers should require "coding_adventures_barcode_2d" (the top-level file at
# lib/ root) which handles requiring paint_instructions first.

require_relative "barcode_2d/version"
require_relative "barcode_2d/errors"
require_relative "barcode_2d/module_grid"
require_relative "barcode_2d/layout"
