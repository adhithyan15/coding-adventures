"""Build configuration for the Nib IR compiler.

Overview
--------

The ``BuildConfig`` dataclass controls what the compiler emits. Build modes
are **composable flags**, not a fixed enum. This design lets you mix and
match features without combinatorial explosion:

  ``insert_debug_comments``  — emit COMMENT instructions with source info

Two presets are provided:

  ``debug_config()``   — debug comments enabled, useful for development
  ``release_config()`` — comments stripped, tightest IR output

Why composable flags instead of an enum?
-----------------------------------------

An enum ``{DEBUG, RELEASE}`` would work for two modes, but it breaks as
soon as you add a third (e.g., ``PROFILE``, ``FUZZING``, ``EMBEDDED``).
Composable flags scale to any combination without modifying existing code.
The open-closed principle in practice: open for extension, closed for
modification.

Why debug comments?
--------------------

Debug comments are ``COMMENT`` pseudo-instructions that explain what source
construct each IR sequence corresponds to. They produce no machine code —
the backend strips them. But when you're reading IR output by hand (which you
will do, because IR is the lingua franca of the compiler pipeline), comments
make it dramatically easier to trace ``LOAD_IMM v2, 5`` back to
``let x: u4 = 5;``.

In release builds we skip the ``COMMENT`` emit for a cleaner, smaller IR.
An optimizer pass (``comment_elision``) can also strip them post-hoc.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class BuildConfig:
    """Controls what the Nib IR compiler emits.

    Attributes:
        insert_debug_comments: Emit ``COMMENT`` instructions alongside compiled
            constructs. Each comment describes the source-level construct —
            e.g., ``"let x: u4 = 5"`` or ``"for i: u8 in 0..5"``. These are
            stripped by the packager in release builds but are invaluable
            when reading IR output during development.

    Example::

        cfg = BuildConfig(insert_debug_comments=True)
        cfg.insert_debug_comments  # True
    """

    insert_debug_comments: bool = True


def debug_config() -> BuildConfig:
    """Return a BuildConfig suitable for debug builds.

    Debug comments are enabled. Use this during development when you need
    to read IR output and trace it back to source constructs.

    Returns:
        A ``BuildConfig`` with debug comments enabled.

    Example::

        cfg = debug_config()
        cfg.insert_debug_comments  # True
    """
    return BuildConfig(insert_debug_comments=True)


def release_config() -> BuildConfig:
    """Return a BuildConfig suitable for release builds.

    Debug comments are disabled for the most compact, clean IR output.
    An optimizer pass can regenerate them for post-hoc analysis if needed.

    Returns:
        A ``BuildConfig`` with debug comments disabled.

    Example::

        cfg = release_config()
        cfg.insert_debug_comments  # False
    """
    return BuildConfig(insert_debug_comments=False)
