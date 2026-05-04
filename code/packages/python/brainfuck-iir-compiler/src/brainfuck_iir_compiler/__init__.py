"""``brainfuck-iir-compiler`` — Brainfuck through the LANG pipeline.

This package supplies the missing piece between the Brainfuck parser
(``brainfuck`` / BF01) and the generic ``vm-core`` interpreter
(``vm-core`` / LANG02).  Specifically:

- :func:`compile_to_iir` — turn a parsed Brainfuck AST into an
  :class:`interpreter_ir.IIRModule`.
- :func:`compile_source` — convenience wrapper that lexes + parses + compiles.
- :class:`BrainfuckVM` — a thin adapter around ``vm-core`` that wires up
  ``putchar`` / ``getchar`` builtins and applies u8 wrap.  ``vm.run(source)``
  returns the program's stdout as ``bytes``.

See ``code/specs/BF04-brainfuck-iir-compiler.md`` for the design notes.
"""

from __future__ import annotations

from brainfuck_iir_compiler.compiler import compile_source, compile_to_iir
from brainfuck_iir_compiler.errors import BrainfuckError
from brainfuck_iir_compiler.vm import BrainfuckVM

__all__ = [
    "compile_source",
    "compile_to_iir",
    "BrainfuckVM",
    "BrainfuckError",
]
