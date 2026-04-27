"""DebugSidecarWriter — accumulates source-location data during compilation.

The compiler calls this once per emitted IIRInstr to record the mapping from
instruction index to source location.  After compilation, ``finish()`` returns
the sidecar as an opaque ``bytes`` object that the reader can load.

Design notes
------------
The sidecar is stored as JSON serialised to UTF-8 bytes.  This is intentionally
simpler than the binary format in spec ``05d`` — it lets us get the full pipeline
working (compiler → sidecar → debugger / insight tools → DWARF emitter) before
investing in the compact binary format.  The ``finish()`` / ``DebugSidecarReader``
boundary is the only place that knows about the on-disk format, so swapping to
binary later is a single-file change.

The writer is append-only and not thread-safe.  Each compilation should use its
own writer instance.

Usage::

    writer = DebugSidecarWriter()
    file_id = writer.add_source_file("fibonacci.tetrad")

    writer.begin_function("fibonacci", start_instr=0, param_count=1)
    writer.declare_variable("fibonacci", reg_index=0, name="n",
                            type_hint="any", live_start=0, live_end=12)

    for instr_index, (instr, node) in enumerate(zip(instructions, ast_nodes)):
        writer.record("fibonacci", instr_index,
                      file_id=file_id, line=node.line, col=node.col)

    writer.end_function("fibonacci", end_instr=12)
    sidecar: bytes = writer.finish()
"""

from __future__ import annotations

import json


class DebugSidecarWriter:
    """Accumulates debug sidecar data during a single compilation.

    Parameters
    ----------
    None — create one instance per compilation unit (module / file).
    """

    def __init__(self) -> None:
        # source_files: list of {"path": str, "checksum": str (hex)}
        self._source_files: list[dict] = []
        self._source_file_index: dict[str, int] = {}

        # line_table: {fn_name: [{instr_index, file_id, line, col}]}
        self._line_table: dict[str, list[dict]] = {}

        # functions: {fn_name: {start_instr, end_instr, param_count}}
        self._functions: dict[str, dict] = {}

        # variables: {fn_name: [{reg_index, name, type_hint, live_start, live_end}]}
        self._variables: dict[str, list[dict]] = {}

    # ------------------------------------------------------------------
    # Source files
    # ------------------------------------------------------------------

    def add_source_file(self, path: str, checksum: bytes = b"") -> int:
        """Register a source file and return its file_id.

        Calling this multiple times with the same ``path`` is safe —
        subsequent calls return the same ``file_id`` without duplicating the
        entry.

        Parameters
        ----------
        path:
            Source file path (absolute or relative to the build directory).
        checksum:
            Optional SHA-256 of the source file at compile time.  Used by
            debug adapters to warn about stale binaries.  Pass ``b""`` to
            omit.

        Returns
        -------
        int
            0-based file_id for use in ``record()``.
        """
        if path in self._source_file_index:
            return self._source_file_index[path]
        file_id = len(self._source_files)
        self._source_files.append({
            "path": path,
            "checksum": checksum.hex(),
        })
        self._source_file_index[path] = file_id
        return file_id

    # ------------------------------------------------------------------
    # Line table
    # ------------------------------------------------------------------

    def record(
        self,
        fn_name: str,
        instr_index: int,
        *,
        file_id: int,
        line: int,
        col: int,
    ) -> None:
        """Record the source location of one emitted IIRInstr.

        Must be called for every instruction emitted by the compiler.
        Calling it out of order (by ``instr_index``) is allowed — the reader
        sorts by index at load time.

        Parameters
        ----------
        fn_name:
            Name of the function this instruction belongs to.
        instr_index:
            0-based index of the instruction within the function body.
        file_id:
            File identifier returned by ``add_source_file()``.
        line:
            1-based source line number.
        col:
            1-based source column number.
        """
        if fn_name not in self._line_table:
            self._line_table[fn_name] = []
        self._line_table[fn_name].append({
            "instr_index": instr_index,
            "file_id": file_id,
            "line": line,
            "col": col,
        })

    # ------------------------------------------------------------------
    # Execution units (functions)
    # ------------------------------------------------------------------

    def begin_function(
        self,
        fn_name: str,
        *,
        start_instr: int,
        param_count: int,
    ) -> None:
        """Register the start of a function's instruction range.

        Must be paired with ``end_function()``.

        Parameters
        ----------
        fn_name:
            Function name (must match the ``IIRFunction.name``).
        start_instr:
            Index of the first instruction in this function's body.
        param_count:
            Number of parameters (for display in call stack frames).
        """
        self._functions[fn_name] = {
            "start_instr": start_instr,
            "end_instr": None,
            "param_count": param_count,
        }

    def end_function(self, fn_name: str, *, end_instr: int) -> None:
        """Record the end of a function's instruction range.

        Parameters
        ----------
        fn_name:
            Function name (must match a prior ``begin_function`` call).
        end_instr:
            One-past-last instruction index (exclusive upper bound).
        """
        if fn_name in self._functions:
            self._functions[fn_name]["end_instr"] = end_instr

    # ------------------------------------------------------------------
    # Variables
    # ------------------------------------------------------------------

    def declare_variable(
        self,
        fn_name: str,
        *,
        reg_index: int,
        name: str,
        type_hint: str = "",
        live_start: int,
        live_end: int,
    ) -> None:
        """Record a named variable binding for a register.

        Parameters
        ----------
        fn_name:
            Function containing this variable.
        reg_index:
            IIR register index.
        name:
            Human-readable variable name.
        type_hint:
            Declared type (``"any"``, ``"u8"``, …), or ``""`` for no
            annotation.
        live_start:
            First instruction index at which this binding is valid.
        live_end:
            One-past-last instruction index (exclusive).
        """
        if fn_name not in self._variables:
            self._variables[fn_name] = []
        self._variables[fn_name].append({
            "reg_index": reg_index,
            "name": name,
            "type_hint": type_hint,
            "live_start": live_start,
            "live_end": live_end,
        })

    # ------------------------------------------------------------------
    # Serialisation
    # ------------------------------------------------------------------

    def finish(self) -> bytes:
        """Serialise the accumulated debug data to bytes.

        Returns
        -------
        bytes
            Opaque sidecar bytes.  Pass to ``DebugSidecarReader`` to query.
        """
        payload = {
            "version": 1,
            "source_files": self._source_files,
            "line_table": {
                fn: sorted(rows, key=lambda r: r["instr_index"])
                for fn, rows in self._line_table.items()
            },
            "functions": self._functions,
            "variables": self._variables,
        }
        return json.dumps(payload, separators=(",", ":")).encode("utf-8")
