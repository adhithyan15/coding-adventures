"""Build configuration for the Brainfuck IR compiler.

Overview
--------

The ``BuildConfig`` dataclass controls what the compiler emits. Build modes
are **composable flags**, not a fixed enum. This design lets you mix and
match features without combinatorial explosion:

  ``insert_bounds_checks``  — emit tape pointer range checks (debug builds)
  ``insert_debug_locs``     — emit source location markers in IR comments
  ``mask_byte_arithmetic``  — AND 0xFF after every cell mutation
  ``tape_size``             — configurable tape length (default 30,000 cells)

Two presets are provided:

  ``debug_config()``   — all safety checks enabled, useful for development
  ``release_config()`` — safety checks off, byte masking on, fastest output

Why composable flags instead of an enum?
-----------------------------------------

An enum ``{DEBUG, RELEASE}`` would work for two modes, but it breaks as
soon as you add a third (e.g., ``PROFILE``, ``FUZZING``, ``EMBEDDED``).
Composable flags scale to any combination without modifying existing code.
The open-closed principle in practice: open for extension, closed for
modification.

Why default tape size 30,000?
------------------------------

Urban Müller's original Brainfuck interpreter from 1993 used 30,000 cells.
This has become the de facto standard. Programs written for the canonical
implementation expect at least 30,000 cells, so we honour that default.

Why mask byte arithmetic?
--------------------------

Brainfuck cells are 8-bit values (0-255). The ``AND_IMM v, v, 255``
instruction forces wrap-around after every increment or decrement. This
matches the spec: if a cell at 255 is incremented, it becomes 0, not 256.

Some backends guarantee byte-width stores automatically (the CPU's store
instruction truncates to 8 bits), so an optimizer pass (``mask_elision``)
can remove these AND_IMM instructions for those targets. The flag lets the
compiler emit correct code first; the optimizer handles the platform.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class BuildConfig:
    """Controls what the Brainfuck IR compiler emits.

    Attributes:
        insert_bounds_checks: Emit tape pointer range checks before every
            pointer move (``<`` and ``>``). If the pointer goes out of bounds,
            the program jumps to ``__trap_oob``. Costs ~2 instructions per
            pointer move. Useful for debugging Brainfuck programs.
        insert_debug_locs: Emit ``COMMENT`` instructions with source locations.
            These are stripped by the packager in release builds but help
            when reading IR output during development.
        mask_byte_arithmetic: Emit ``AND_IMM v, v, 255`` after every cell
            mutation (``+``, ``-``). Ensures cells stay in the 0-255 range.
            Backends that guarantee byte-width stores can skip this.
        tape_size: The number of cells in the Brainfuck tape. Default 30,000,
            matching the original specification.

    Example::

        cfg = BuildConfig(
            insert_bounds_checks=True,
            insert_debug_locs=True,
            mask_byte_arithmetic=True,
            tape_size=30000,
        )
    """

    insert_bounds_checks: bool = False
    insert_debug_locs: bool = False
    mask_byte_arithmetic: bool = True
    tape_size: int = 30000


def debug_config() -> BuildConfig:
    """Return a BuildConfig suitable for debug builds.

    All safety checks are enabled. Use this during development when you
    need to catch bugs in Brainfuck programs and see detailed IR output.

    Returns:
        A ``BuildConfig`` with all safety features enabled.

    Example::

        cfg = debug_config()
        cfg.insert_bounds_checks  # True
        cfg.tape_size             # 30000
    """
    return BuildConfig(
        insert_bounds_checks=True,
        insert_debug_locs=True,
        mask_byte_arithmetic=True,
        tape_size=30000,
    )


def release_config() -> BuildConfig:
    """Return a BuildConfig suitable for release builds.

    Safety checks are disabled for maximum performance. Byte masking is
    kept because correctness is not negotiable — cells must wrap around.
    (An optimizer pass can remove masking for backends that guarantee it.)

    Returns:
        A ``BuildConfig`` with safety checks disabled and masking enabled.

    Example::

        cfg = release_config()
        cfg.insert_bounds_checks  # False
        cfg.mask_byte_arithmetic  # True
    """
    return BuildConfig(
        insert_bounds_checks=False,
        insert_debug_locs=False,
        mask_byte_arithmetic=True,
        tape_size=30000,
    )
