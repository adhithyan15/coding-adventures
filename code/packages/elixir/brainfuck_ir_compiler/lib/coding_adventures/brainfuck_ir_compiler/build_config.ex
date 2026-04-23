defmodule CodingAdventures.BrainfuckIrCompiler.BuildConfig do
  @moduledoc """
  Compilation configuration — controls what the Brainfuck IR compiler emits.

  ## Composable flags

  Build modes are **composable flags**, not a fixed enum. A `BuildConfig`
  controls every aspect of compilation:

  - `insert_bounds_checks` — emit tape pointer range checks (debug builds).
    If the pointer goes out of bounds, the program jumps to `__trap_oob`.
    Costs ~2 extra instructions per pointer move.

  - `insert_debug_locs` — emit `COMMENT` instructions with source locations.
    These are stripped by the packager in release builds but aid debugging.

  - `mask_byte_arithmetic` — emit `AND_IMM v, v, 255` after every cell
    mutation (`INC`, `DEC`). Ensures cells stay in the 0–255 range per
    the Brainfuck spec. Backends that guarantee byte-width stores can
    skip this via an optimiser pass.

  - `tape_size` — number of cells in the tape (default: 30,000).

  ## Presets

  - `debug_config/0`   — all safety checks ON
  - `release_config/0` — safety checks OFF, masking ON

  New modes can be added without modifying existing code — just construct
  a `BuildConfig` with the desired flags.

  ## Examples

      # Debug build (bounds checks + debug locs)
      cfg = BuildConfig.debug_config()

      # Release build (minimal overhead)
      cfg = BuildConfig.release_config()

      # Custom: release with a smaller tape
      cfg = %{BuildConfig.release_config() | tape_size: 1000}
  """

  defstruct insert_bounds_checks: false,
            insert_debug_locs: false,
            mask_byte_arithmetic: true,
            tape_size: 30_000

  @type t :: %__MODULE__{
          insert_bounds_checks: boolean(),
          insert_debug_locs: boolean(),
          mask_byte_arithmetic: boolean(),
          tape_size: pos_integer()
        }

  @doc """
  Return a `BuildConfig` suitable for debug builds.

  All safety checks are enabled:
  - Bounds checking ON (detects tape overflow/underflow)
  - Debug location markers ON
  - Byte arithmetic masking ON
  - Tape size: 30,000 cells (canonical Brainfuck)
  """
  @spec debug_config() :: t()
  def debug_config do
    %__MODULE__{
      insert_bounds_checks: true,
      insert_debug_locs: true,
      mask_byte_arithmetic: true,
      tape_size: 30_000
    }
  end

  @doc """
  Return a `BuildConfig` suitable for release builds.

  Safety checks are disabled for maximum performance:
  - Bounds checking OFF
  - Debug location markers OFF
  - Byte arithmetic masking ON (correctness requirement)
  - Tape size: 30,000 cells (canonical Brainfuck)
  """
  @spec release_config() :: t()
  def release_config do
    %__MODULE__{
      insert_bounds_checks: false,
      insert_debug_locs: false,
      mask_byte_arithmetic: true,
      tape_size: 30_000
    }
  end
end
