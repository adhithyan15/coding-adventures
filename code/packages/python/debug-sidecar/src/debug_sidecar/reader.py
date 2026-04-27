"""DebugSidecarReader — queries source-location data at runtime.

The reader is the consumer side of the debug sidecar pipeline.  It loads the
bytes produced by ``DebugSidecarWriter.finish()`` and answers three kinds of
questions:

1. **Offset → source** (debugger paused):
   ``lookup(fn_name, instr_index)`` → ``SourceLocation | None``

2. **Source → offset** (setting a breakpoint):
   ``find_instr(file, line)`` → ``int | None``

3. **Variable inspection** (Variables panel):
   ``live_variables(fn_name, at_instr)`` → ``list[Variable]``

Usage::

    reader = DebugSidecarReader(sidecar_bytes)

    # Where did instruction 7 in 'fibonacci' come from?
    loc = reader.lookup("fibonacci", 7)
    if loc:
        print(f"Stopped at {loc}")   # "fibonacci.tetrad:3:5"

    # Set a breakpoint on line 10 of fibonacci.tetrad
    idx = reader.find_instr("fibonacci.tetrad", 10)
    if idx is not None:
        vm.set_breakpoint(idx, "fibonacci")

    # What variables are live when the debugger pauses at instruction 5?
    for var in reader.live_variables("fibonacci", 5):
        print(f"  {var.name} (reg {var.reg_index}): {var.type_hint}")
"""

from __future__ import annotations

import bisect
import json

from debug_sidecar.types import SourceLocation, Variable


class DebugSidecarReader:
    """Answers debug queries from a compiled sidecar.

    Parameters
    ----------
    data:
        Bytes returned by ``DebugSidecarWriter.finish()``.

    Raises
    ------
    ValueError
        If ``data`` is not a valid sidecar (wrong version or corrupt JSON).
    """

    def __init__(self, data: bytes) -> None:
        try:
            payload = json.loads(data.decode("utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError) as exc:
            raise ValueError(f"invalid sidecar data: {exc}") from exc

        if payload.get("version") != 1:
            raise ValueError(
                f"unsupported sidecar version: {payload.get('version')!r}"
            )

        self._source_files: list[dict] = payload.get("source_files", [])
        self._raw_line_table: dict[str, list[dict]] = payload.get("line_table", {})
        self._functions: dict[str, dict] = payload.get("functions", {})
        self._raw_variables: dict[str, list[dict]] = payload.get("variables", {})

        # Pre-build sorted index: {fn_name: sorted list of instr_index values}
        # Used for bisect lookups in lookup().
        self._sorted_indices: dict[str, list[int]] = {
            fn: [row["instr_index"] for row in rows]
            for fn, rows in self._raw_line_table.items()
        }

    # ------------------------------------------------------------------
    # Source location lookup (offset → source)
    # ------------------------------------------------------------------

    def lookup(self, fn_name: str, instr_index: int) -> SourceLocation | None:
        """Return the source location of instruction ``instr_index`` in ``fn_name``.

        Uses the last row whose index is ≤ ``instr_index``, exactly as the
        DWARF line-number lookup algorithm works.  This means a generated
        instruction that maps to no explicit record still gets the location of
        the nearest preceding recorded instruction.

        Returns ``None`` if the function has no debug information or if
        ``instr_index`` is before the first recorded instruction.

        Parameters
        ----------
        fn_name:
            Function name as registered with the writer.
        instr_index:
            0-based instruction index within the function body.

        Returns
        -------
        SourceLocation | None
        """
        rows = self._raw_line_table.get(fn_name)
        if not rows:
            return None

        indices = self._sorted_indices[fn_name]
        # bisect_right gives the insertion point after any existing equal values.
        pos = bisect.bisect_right(indices, instr_index) - 1
        if pos < 0:
            return None

        row = rows[pos]
        file_id = row["file_id"]
        if file_id >= len(self._source_files):
            return None

        return SourceLocation(
            file=self._source_files[file_id]["path"],
            line=row["line"],
            col=row["col"],
        )

    # ------------------------------------------------------------------
    # Reverse lookup (source → offset)
    # ------------------------------------------------------------------

    def find_instr(self, file: str, line: int) -> int | None:
        """Return the first instruction index that maps to ``(file, line)``.

        Scans all functions for a row matching the given file and line number.
        Returns the instruction index with the lowest index (first instruction
        on that source line), or ``None`` if the line is not reachable.

        Parameters
        ----------
        file:
            Source file path (must match a path registered with the writer).
        line:
            1-based line number.

        Returns
        -------
        int | None
            First instruction index that originated at ``file:line``.
        """
        # Resolve file path to file_id
        file_id = None
        for i, sf in enumerate(self._source_files):
            if sf["path"] == file:
                file_id = i
                break
        if file_id is None:
            return None

        best: int | None = None
        for rows in self._raw_line_table.values():
            for row in rows:
                if row["file_id"] == file_id and row["line"] == line:
                    if best is None or row["instr_index"] < best:
                        best = row["instr_index"]
        return best

    # ------------------------------------------------------------------
    # Variable inspection
    # ------------------------------------------------------------------

    def live_variables(self, fn_name: str, at_instr: int) -> list[Variable]:
        """Return all variables live at instruction ``at_instr`` in ``fn_name``.

        A variable is live at ``at_instr`` when
        ``live_start <= at_instr < live_end``.

        Parameters
        ----------
        fn_name:
            Function name.
        at_instr:
            0-based instruction index.

        Returns
        -------
        list[Variable]
            Variables alive at the given instruction, sorted by register index.
        """
        raw = self._raw_variables.get(fn_name, [])
        result = [
            Variable(
                reg_index=v["reg_index"],
                name=v["name"],
                type_hint=v["type_hint"],
                live_start=v["live_start"],
                live_end=v["live_end"],
            )
            for v in raw
            if v["live_start"] <= at_instr < v["live_end"]
        ]
        result.sort(key=lambda v: v.reg_index)
        return result

    # ------------------------------------------------------------------
    # Metadata queries
    # ------------------------------------------------------------------

    def source_files(self) -> list[str]:
        """Return the list of source file paths registered in this sidecar."""
        return [sf["path"] for sf in self._source_files]

    def function_names(self) -> list[str]:
        """Return the list of function names that have debug information."""
        return list(self._functions.keys())

    def function_range(self, fn_name: str) -> tuple[int, int] | None:
        """Return the (start_instr, end_instr) range for ``fn_name``.

        Returns ``None`` if the function was not registered with the writer.
        ``end_instr`` may be ``None`` if ``end_function()`` was never called.
        """
        fn = self._functions.get(fn_name)
        if fn is None:
            return None
        end = fn.get("end_instr")
        if end is None:
            return None
        return (fn["start_instr"], end)
