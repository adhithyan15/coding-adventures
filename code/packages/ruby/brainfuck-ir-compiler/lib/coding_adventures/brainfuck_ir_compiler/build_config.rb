# frozen_string_literal: true

# ==========================================================================
# BuildConfig — Compilation Flags for the Brainfuck AOT Compiler
# ==========================================================================
#
# Build modes are composable flags, not a fixed enum. A BuildConfig
# object controls every aspect of compilation:
#
#   insert_bounds_checks  — emit tape pointer range checks (debug builds)
#   insert_debug_locs     — emit COMMENT instructions with source locations
#   mask_byte_arithmetic  — AND 0xFF after every cell mutation (correctness)
#   tape_size             — number of tape cells (default 30,000)
#
# ── Presets ─────────────────────────────────────────────────────────────────
#
#   BuildConfig.debug_config   — all safety checks ON
#   BuildConfig.release_config — bounds checks OFF, masking ON
#
# ── Design rationale ────────────────────────────────────────────────────────
#
# Using composable boolean flags rather than an enum (DebugMode, ReleaseMode)
# means:
#
#   1. New options can be added without breaking existing callers — just add
#      a new flag with a sensible default.
#   2. Unusual combinations are possible (e.g., bounds checks ON but debug
#      locs OFF) for benchmarking or partial hardening.
#   3. The presets are just sugar for the most common combinations.
# ==========================================================================

module CodingAdventures
  module BrainfuckIrCompiler
    # BuildConfig holds the compilation flags for one compiler invocation.
    #
    # Attributes:
    #   insert_bounds_checks  [Boolean] emit tape bounds checks before > and <
    #   insert_debug_locs     [Boolean] emit COMMENT source-location markers
    #   mask_byte_arithmetic  [Boolean] emit AND_IMM 255 after + and -
    #   tape_size             [Integer] number of tape cells (default 30000)
    BuildConfig = Struct.new(:insert_bounds_checks,
                             :insert_debug_locs,
                             :mask_byte_arithmetic,
                             :tape_size,
                             keyword_init: true) do
      # debug_config returns a BuildConfig suitable for debug builds.
      # All safety checks are enabled.
      #
      # ┌────────────────────────┬───────┐
      # │ insert_bounds_checks   │  true │
      # │ insert_debug_locs      │  true │
      # │ mask_byte_arithmetic   │  true │
      # │ tape_size              │ 30000 │
      # └────────────────────────┴───────┘
      def self.debug_config
        new(
          insert_bounds_checks: true,
          insert_debug_locs: true,
          mask_byte_arithmetic: true,
          tape_size: 30_000
        )
      end

      # release_config returns a BuildConfig suitable for release builds.
      # Bounds checks are disabled for maximum performance; byte masking
      # stays enabled for correctness.
      #
      # ┌────────────────────────┬───────┐
      # │ insert_bounds_checks   │ false │
      # │ insert_debug_locs      │ false │
      # │ mask_byte_arithmetic   │  true │
      # │ tape_size              │ 30000 │
      # └────────────────────────┴───────┘
      def self.release_config
        new(
          insert_bounds_checks: false,
          insert_debug_locs: false,
          mask_byte_arithmetic: true,
          tape_size: 30_000
        )
      end
    end
  end
end
