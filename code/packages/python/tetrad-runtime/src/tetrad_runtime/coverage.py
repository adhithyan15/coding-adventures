"""Source-line coverage report for Tetrad programs (LANG18).

Why this module exists
----------------------
``VMCore`` (LANG18) records which IIR instruction indices were executed,
keyed by function name.  That data lives at the IIR level, not the Tetrad
source level.

This module takes the raw IIR coverage data from ``VMCore.coverage_data()``
and projects it back to source lines using the ``DebugSidecar`` built by
``sidecar_builder.code_object_to_iir_with_sidecar``.  The projection uses
``DebugSidecarReader.lookup(fn_name, ip)`` to convert each covered IIR
instruction index to a ``SourceLocation(file, line, col)``.

Data model
----------
The result is a ``LineCoverageReport`` — a list of ``CoveredLine`` records
(one per distinct ``(file, line)`` pair) plus helpers for querying it.

``CoveredLine.iir_hit_count``
    How many *distinct* IIR instruction indices at that source line were
    reached during execution.  A single source line typically compiles to
    several IIR instructions (e.g. a Tetrad ``ADD`` becomes
    ``load_var + add + store_var``).  If all three IIR instructions for
    that line ran, ``iir_hit_count`` is 3.  This is *not* an execution
    frequency — it does not count how many times a loop body ran.  For
    frequency, consult LANG17's ``BranchStats`` / ``loop_iterations``.

Public API
----------
``CoveredLine``         — dataclass: file, line, iir_hit_count
``LineCoverageReport``  — report + helpers; built by ``build_report``
``build_report``        — project IIR coverage → source-line coverage
"""

from __future__ import annotations

from dataclasses import dataclass, field

from debug_sidecar import DebugSidecarReader


@dataclass
class CoveredLine:
    """One source line that was reached during execution.

    Attributes
    ----------
    file:
        Source file path exactly as stored in the DebugSidecar (passed as
        ``source_path`` to ``code_object_to_iir_with_sidecar``).
    line:
        1-based source line number.
    iir_hit_count:
        Number of *distinct* IIR instruction indices at this source line
        that were executed.  A single Tetrad source line typically
        compiles to several IIR instructions; this counter reports how
        many of those IIR instructions ran — not how many times they ran.

    Example
    -------
    A source line ``y := x + 1`` might translate to three IIR instructions
    (``load x``, ``add 1``, ``store y``).  If all three were executed,
    ``iir_hit_count == 3``.
    """

    file: str
    line: int
    iir_hit_count: int


@dataclass
class LineCoverageReport:
    """Source-line coverage produced by composing IIR coverage with DebugSidecar.

    Attributes
    ----------
    covered_lines:
        All ``(file, line)`` pairs that were reached during execution,
        in (file, line) ascending order.

    Methods
    -------
    lines_for_file(path) -> list[int]
        Sorted line numbers for one source file.
    total_lines_covered() -> int
        Total number of distinct ``(file, line)`` pairs that were reached.
    """

    covered_lines: list[CoveredLine] = field(default_factory=list)

    def lines_for_file(self, path: str) -> list[int]:
        """Return sorted list of covered line numbers for ``path``.

        Returns an empty list if ``path`` does not appear in the report,
        or if no lines from that file were reached.

        Parameters
        ----------
        path:
            Source file path (must match exactly what was passed as
            ``source_path`` to ``code_object_to_iir_with_sidecar``).
        """
        return sorted(
            cl.line for cl in self.covered_lines if cl.file == path
        )

    def total_lines_covered(self) -> int:
        """Return the total number of distinct (file, line) pairs reached."""
        return len(self.covered_lines)

    def files(self) -> list[str]:
        """Return sorted list of unique source files present in this report."""
        return sorted({cl.file for cl in self.covered_lines})


def build_report(
    iir_coverage: dict[str, frozenset[int]],
    sidecar_bytes: bytes,
) -> LineCoverageReport:
    """Project IIR instruction coverage back to source lines via DebugSidecar.

    Algorithm
    ---------
    1. Construct a ``DebugSidecarReader`` from ``sidecar_bytes``.
    2. For every ``(fn_name, ip_set)`` in ``iir_coverage``:
       - For every ``ip`` in ``ip_set``, call
         ``reader.lookup(fn_name, ip)`` to get a ``SourceLocation``.
       - If a location is returned, accumulate the IIR hit count at
         ``(location.file, location.line)``.
    3. Build a ``CoveredLine`` for each distinct ``(file, line)`` pair,
       sorted by ``(file, line)``.
    4. Return a ``LineCoverageReport``.

    IIR instructions with no source-map entry (e.g. synthetic preamble
    instructions emitted by the translator that have no Tetrad bytecode
    counterpart) are silently skipped — ``lookup`` returns ``None`` for
    them.

    Parameters
    ----------
    iir_coverage:
        Output of ``VMCore.coverage_data()`` — maps function name to a
        frozenset of IIR instruction indices that were executed.
    sidecar_bytes:
        Raw bytes from ``code_object_to_iir_with_sidecar`` or
        ``TetradRuntime.compile_with_debug``.

    Returns
    -------
    LineCoverageReport
        Source-line coverage report.  Empty if ``iir_coverage`` is empty
        or if no IIR index can be mapped to a source line.
    """
    reader = DebugSidecarReader(sidecar_bytes)

    # Accumulate: (file, line) → IIR hit count.
    # Using a dict here keeps the insertion logic simple — each unique
    # (file, line) pair gets one entry regardless of how many IIR
    # instructions at that line were hit.
    hits: dict[tuple[str, int], int] = {}

    for fn_name, ip_set in iir_coverage.items():
        for ip in ip_set:
            loc = reader.lookup(fn_name, ip)
            if loc is None:
                continue
            key = (loc.file, loc.line)
            hits[key] = hits.get(key, 0) + 1

    # Sort for deterministic output — (file ascending, line ascending).
    covered = [
        CoveredLine(file=file, line=line, iir_hit_count=count)
        for (file, line), count in sorted(hits.items())
    ]

    return LineCoverageReport(covered_lines=covered)
