"""VCD writer — Value Change Dump format per IEEE 1364-2005 §18.

Streaming text writer. Produces a ``.vcd`` file readable by GTKWave, surfer,
ModelSim, and every other waveform viewer.

Design intentionally decoupled from any specific simulator: emit value changes
via ``value_change(time, signal_id, value)``. An attach helper for hardware-vm
(or any callback-style emitter) is provided in ``attach_to_callback_emitter``.
"""

from __future__ import annotations

import datetime
import string
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import TextIO


@dataclass(frozen=True, slots=True)
class VarDef:
    """One variable declaration in the VCD."""

    name: str
    width: int
    var_id: str
    kind: str = "wire"  # 'wire' | 'reg' | 'integer' | 'real'


@dataclass
class Scope:
    """A scope in the VCD hierarchy. Holds variables and child scopes."""

    name: str
    kind: str = "module"
    vars: list[VarDef] = field(default_factory=list)
    children: list[Scope] = field(default_factory=list)


class _IdAllocator:
    """Generates compact printable-ASCII identifiers ('!' through '~').

    First 94 use 1 char; thereafter 2-char combinations. Practically unlimited."""

    _CHARS = string.printable.replace(string.whitespace, "")[: ord("~") - ord("!") + 1]

    def __init__(self) -> None:
        self._next = 0

    def alloc(self) -> str:
        # Encode self._next in base-94 using printable ASCII offset from '!' (33).
        n = self._next
        self._next += 1
        chars = []
        while True:
            chars.append(chr(33 + (n % 94)))
            n //= 94
            if n == 0:
                break
            n -= 1
        return "".join(chars)


class VcdWriter:
    """Streaming VCD writer.

    Two phases:
    1. Header — declare timescale, scopes, variables. Call ``end_definitions()``.
    2. Body — call ``time(t)`` to advance, then ``value_change(var_id, value)``
       repeatedly. Or call the convenience ``value_change(time, var_id, value)``
       which handles time-tracking internally.

    Close with ``close()`` or use as a context manager.
    """

    def __init__(self, path: str | Path, timescale: str = "1ps") -> None:
        self.path = Path(path)
        self.timescale = timescale
        self._fh: TextIO | None = None
        self._id_alloc = _IdAllocator()
        self._defs_ended = False
        self._cur_time: int = -1  # -1 = no #t emitted yet
        self._values: dict[str, int | str] = {}  # last-emitted values; for $dumpvars
        self._var_defs: dict[str, VarDef] = {}  # var_id -> VarDef
        self._scopes: list[Scope] = []  # current open-scope stack

    def __enter__(self) -> VcdWriter:
        self.open()
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    # ------------------------------------------------------------------
    # Header
    # ------------------------------------------------------------------

    def open(self) -> None:
        self._fh = self.path.open("w", encoding="utf-8")
        self._write_header_preamble()

    def close(self) -> None:
        if self._fh is not None:
            self._fh.close()
            self._fh = None

    def _w(self, line: str) -> None:
        if self._fh is None:
            raise RuntimeError("VcdWriter not open")
        self._fh.write(line)
        if not line.endswith("\n"):
            self._fh.write("\n")

    def _write_header_preamble(self) -> None:
        date_str = datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%d %H:%M:%S UTC")
        self._w(f"$date {date_str} $end")
        self._w("$version Silicon-Stack VCD Writer 0.1.0 $end")
        self._w(f"$timescale {self.timescale} $end")

    def open_scope(self, name: str, kind: str = "module") -> Scope:
        if self._defs_ended:
            raise RuntimeError("cannot open_scope after end_definitions")
        self._w(f"$scope {kind} {name} $end")
        scope = Scope(name=name, kind=kind)
        if self._scopes:
            self._scopes[-1].children.append(scope)
        self._scopes.append(scope)
        return scope

    def close_scope(self) -> None:
        if not self._scopes:
            raise RuntimeError("close_scope without matching open_scope")
        self._w("$upscope $end")
        self._scopes.pop()

    def declare(self, name: str, width: int, kind: str = "wire") -> str:
        """Declare a variable and return its compact VCD id."""
        if self._defs_ended:
            raise RuntimeError("cannot declare after end_definitions")
        if width < 1:
            raise ValueError(f"width must be >= 1, got {width}")
        var_id = self._id_alloc.alloc()
        var_def = VarDef(name=name, width=width, var_id=var_id, kind=kind)
        self._var_defs[var_id] = var_def
        if self._scopes:
            self._scopes[-1].vars.append(var_def)
        # In VCD, multi-bit names are written as `name [w-1:0]`
        if width > 1:
            self._w(f"$var {kind} {width} {var_id} {name} [{width - 1}:0] $end")
        else:
            self._w(f"$var {kind} {width} {var_id} {name} $end")
        return var_id

    def end_definitions(self) -> None:
        # Close any open scopes that the user forgot.
        while self._scopes:
            self.close_scope()
        self._w("$enddefinitions $end")
        self._defs_ended = True

    # ------------------------------------------------------------------
    # Body
    # ------------------------------------------------------------------

    def time(self, t: int) -> None:
        if not self._defs_ended:
            self.end_definitions()
        if t < self._cur_time:
            raise ValueError(f"time must not decrease: {t} < {self._cur_time}")
        if t != self._cur_time:
            self._w(f"#{t}")
            self._cur_time = t

    def dump_initial(self, values: dict[str, int | str]) -> None:
        """Emit a $dumpvars block with initial values for every declared var.

        ``values`` maps var_id -> initial value. Vars not in the dict default to 0.
        """
        if self._cur_time == -1:
            self.time(0)
        self._w("$dumpvars")
        for var_id, var_def in self._var_defs.items():
            v = values.get(var_id, 0)
            self._w(self._format_value_change(var_def, v))
            self._values[var_id] = v
        self._w("$end")

    def value_change(
        self, time_or_var: int | str, var_id: str | None = None, value: int | str | None = None
    ) -> None:
        """Emit a value change. Two call signatures:

        1. ``value_change(time, var_id, value)`` — advance time first, then emit.
        2. ``value_change(var_id, value)`` — emit at the current time."""
        if isinstance(time_or_var, int):
            assert var_id is not None
            assert value is not None
            self.time(time_or_var)
            target_id = var_id
            new_value = value
        else:
            # Two-arg form: (var_id, value)
            assert var_id is not None
            target_id = time_or_var
            new_value = var_id  # type: ignore[assignment]
            value = var_id  # placate type-checker

        if target_id not in self._var_defs:
            raise KeyError(f"unknown var_id: {target_id!r}")

        # Skip if value didn't change
        if self._values.get(target_id) == new_value:
            return
        self._values[target_id] = new_value

        var_def = self._var_defs[target_id]
        self._w(self._format_value_change(var_def, new_value))

    def _format_value_change(self, var_def: VarDef, value: int | str) -> str:
        var_id = var_def.var_id
        if var_def.kind == "real":
            return f"r{value} {var_id}"
        if isinstance(value, str):
            # Already a VCD value string (e.g., 'x' or 'z' or '1010xz')
            if var_def.width == 1:
                return f"{value}{var_id}"
            return f"b{value} {var_id}"
        # Integer
        if var_def.width == 1:
            return f"{int(value) & 1}{var_id}"
        # Multi-bit: emit as binary, no leading zeros (VCD convention)
        bits = bin(int(value) & ((1 << var_def.width) - 1))[2:]
        return f"b{bits} {var_id}"


# ----------------------------------------------------------------------------
# Convenience: attach to a callback-style event emitter (e.g., hardware-vm)
# ----------------------------------------------------------------------------


def attach_to_callback_emitter(
    writer: VcdWriter,
    *,
    name_to_var_id: dict[str, str],
) -> Callable[[object], None]:
    """Return a callback suitable for hardware-vm's ``vm.subscribe(callback)``.

    The returned function expects an object with attributes
    ``time`` (int), ``signal`` (str), and ``new_value`` (int) — exactly the
    shape of ``hardware_vm.Event``.

    ``name_to_var_id`` maps signal names to the VCD var_ids returned by
    ``writer.declare(...)``."""

    def cb(event: object) -> None:
        signal = getattr(event, "signal", None)
        time_ = getattr(event, "time", 0)
        value = getattr(event, "new_value", 0)
        if signal is None:
            return
        var_id = name_to_var_id.get(signal)
        if var_id is None:
            return
        writer.value_change(time_, var_id, value)

    return cb
