"""Core data types for debug-sidecar.

``SourceLocation``
    A frozen, hashable (file, line, col) triple.  The reader returns these
    in response to instruction-index queries.

``Variable``
    A register binding with a human name, type hint, and live range.  The
    live range is expressed as [live_start, live_end) instruction indices so
    the reader can answer "what variables are live at instruction N?".
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class SourceLocation:
    """Source position for one IIR instruction.

    Parameters
    ----------
    file:
        Source file path as registered with ``DebugSidecarWriter.add_source_file``.
    line:
        1-based line number.
    col:
        1-based column number.
    """

    file: str
    line: int
    col: int

    def __str__(self) -> str:
        return f"{self.file}:{self.line}:{self.col}"


@dataclass(frozen=True)
class Variable:
    """A named register binding valid over a range of instruction indices.

    Parameters
    ----------
    reg_index:
        IIR register index (matches the register number in the VM's frame).
    name:
        Human-readable variable name from the source program.
    type_hint:
        Declared type string (``"any"``, ``"u8"``, ``"Int"``, …).  Empty
        string if no type annotation was given.
    live_start:
        First instruction index at which this binding is valid (inclusive).
    live_end:
        One-past-last instruction index at which this binding is valid
        (exclusive).  A binding is live at instruction N when
        ``live_start <= N < live_end``.
    """

    reg_index: int
    name: str
    type_hint: str
    live_start: int
    live_end: int

    def is_live_at(self, instr_index: int) -> bool:
        """Return True if this variable is live at ``instr_index``."""
        return self.live_start <= instr_index < self.live_end
