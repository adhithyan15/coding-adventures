"""Errors raised by ``brainfuck-iir-compiler``.

Why a dedicated exception class
-------------------------------
Several distinct failure modes can occur while a Brainfuck program runs
under :class:`brainfuck_iir_compiler.BrainfuckVM`:

- The data pointer walks past the end of the configured tape.
- The data pointer walks below cell 0.
- The fuel cap (``max_steps``) is exhausted by a runaway loop.
- The user passed ``jit=True`` but the JIT path is not yet wired (BF05).

Lumping these into ``ValueError`` or ``RuntimeError`` would force callers
to inspect strings to distinguish them.  A single :class:`BrainfuckError`
class with a stable identity lets tests, REPL frontends, and notebook
kernels handle BF-level failures without touching message text.
"""

from __future__ import annotations


class BrainfuckError(Exception):
    """Raised by ``BrainfuckVM`` for Brainfuck-level execution failures.

    See :mod:`brainfuck_iir_compiler.errors` for the catalogue of
    conditions that surface as this exception.
    """
