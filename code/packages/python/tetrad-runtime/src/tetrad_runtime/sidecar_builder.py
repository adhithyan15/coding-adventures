"""Build a DebugSidecar alongside an IIRModule during Tetrad compilation.

Why this module exists
----------------------
The Tetrad pipeline produces two kinds of source-position data at different
stages, and neither stage alone is enough for the debugger:

1.  **``CodeObject.source_map``** — populated by the Tetrad compiler's
    ``_emit()`` function — maps *Tetrad bytecode instruction index* (``tetrad_ip``)
    to a *source location* ``(line, col)``.  This is in the old-style Tetrad
    world.

2.  **``IIRFunction.source_map``** — populated by the IIR translator
    (``iir_translator._translate_function``) — maps *IIR instruction start
    index* (``iir_start``) to the *Tetrad bytecode index* ``(tetrad_ip, 0)``.
    This is in the new IIR world, but carries bytecode indices, not source lines.

The debugger needs a third form: *IIR instruction index → source location*.
This module composes the two maps:

    CodeObject.source_map:  (tetrad_ip=7)  → (line=3, col=5)
    IIRFunction.source_map: (iir_start=14) → (tetrad_ip=7)
    ──────────────────────────────────────────────────────────────────────────
    Composed:               (iir_start=14) → (line=3, col=5)

The result is written into a ``DebugSidecar`` via ``DebugSidecarWriter``.
The ``DebugSidecarReader`` then answers:

    reader.lookup("main", ip=15) → SourceLocation("prog.tetrad", line=3, col=5)
    reader.find_instr("prog.tetrad", line=3) → 14  # for vm.set_breakpoint(14, "main")

Variable declarations
---------------------
For each function the sidecar also records:

- **Parameters** — one ``Variable`` per ``code.params[i]`` with ``reg_index=i``,
  ``name=param_name``, ``type_hint="u8"``, live for the full function body.

- **Locals** — one ``Variable`` per entry in ``code.var_names`` beyond the
  parameter count, with ``type_hint="u8"``, live for the full function body.
  Exact live-start positions (the first ``STA_VAR`` for each local) are a
  future improvement; declaring them live from 0 is conservative but correct
  for the Variables panel — uninitialized locals will just show ``0``.

The ``reg_index`` field is the ``var_names`` ordinal for ordering in the debug
panel.  The debug adapter should prefer resolving variables by *name* (using
``frame.name_to_reg``) rather than by index, because IIR named slots are not
positionally stable across different Tetrad translations.

Public API
----------
::

    from tetrad_runtime.sidecar_builder import code_object_to_iir_with_sidecar
    from debug_sidecar import DebugSidecarReader

    runtime = TetradRuntime()
    module, sidecar = runtime.compile_with_debug(source, "myprogram.tetrad")
    reader = DebugSidecarReader(sidecar)

    # Set a breakpoint at source line 10
    iir_idx = reader.find_instr("myprogram.tetrad", 10)
    vm.set_breakpoint(iir_idx, "main")
"""

from __future__ import annotations

from debug_sidecar import DebugSidecarWriter
from interpreter_ir import IIRFunction, IIRModule
from tetrad_compiler.bytecode import CodeObject
from tetrad_runtime.iir_translator import ENTRY_FN_NAME, code_object_to_iir

__all__ = ["code_object_to_iir_with_sidecar"]


def code_object_to_iir_with_sidecar(
    main: CodeObject,
    source_path: str,
    module_name: str = "tetrad-program",
) -> tuple[IIRModule, bytes]:
    """Translate a Tetrad ``CodeObject`` to an ``IIRModule`` + ``DebugSidecar``.

    This is the debug-aware sibling of ``code_object_to_iir``.  It performs
    the same translation and additionally produces a sidecar that maps every
    IIR instruction index in every function back to the original Tetrad source
    line and column.

    The function is non-destructive: it calls ``code_object_to_iir`` with the
    same arguments so the returned ``IIRModule`` is identical to what a plain
    ``code_object_to_iir`` call would produce — the only addition is the
    sidecar bytes.

    Parameters
    ----------
    main:
        Top-level ``CodeObject`` returned by
        ``tetrad_compiler.compile_program``.  Its ``functions`` list contains
        the user-defined functions; its ``var_names`` list contains global
        variable names; its ``source_map`` contains ``(tetrad_ip, line, col)``
        for every instruction emitted by the compiler.
    source_path:
        Path to the Tetrad source file.  Stored verbatim in the sidecar and
        used by ``DebugSidecarReader.find_instr(source_path, line)`` when the
        debug adapter sets a breakpoint.
    module_name:
        Passed through to ``code_object_to_iir``.  Defaults to
        ``"tetrad-program"``.

    Returns
    -------
    tuple[IIRModule, bytes]
        ``(module, sidecar_bytes)`` — pass ``sidecar_bytes`` to
        ``DebugSidecarReader`` to use the debug information.

    Examples
    --------
    ::

        from tetrad_compiler import compile_program
        from tetrad_runtime.sidecar_builder import code_object_to_iir_with_sidecar
        from debug_sidecar import DebugSidecarReader
        from vm_core import VMCore

        code = compile_program(source)
        module, sidecar = code_object_to_iir_with_sidecar(code, "fib.tetrad")
        reader = DebugSidecarReader(sidecar)

        # Map source line 5 to an IIR instruction index
        bp_idx = reader.find_instr("fib.tetrad", 5)
        if bp_idx is not None:
            vm.set_breakpoint(bp_idx, "fibonacci")
    """
    # Step 1: Perform the standard Tetrad → IIR translation.
    module = code_object_to_iir(main, module_name=module_name)

    # Step 2: Build the sidecar.
    writer = DebugSidecarWriter()
    file_id = writer.add_source_file(source_path)

    # The synthetic __entry__ function wraps the top-level CodeObject (<main>).
    iir_entry = module.get_function(ENTRY_FN_NAME)
    if iir_entry is not None:
        _build_fn_sidecar(writer, iir_entry, main, file_id)

    # Each user-defined function maps to the corresponding CodeObject in
    # main.functions.
    for fn_code in main.functions:
        iir_fn = module.get_function(fn_code.name)
        if iir_fn is not None:
            _build_fn_sidecar(writer, iir_fn, fn_code, file_id)

    return module, writer.finish()


def _build_fn_sidecar(
    writer: DebugSidecarWriter,
    iir_fn: IIRFunction,
    code: CodeObject,
    file_id: int,
) -> None:
    """Populate the sidecar with debug information for one function.

    This is an internal helper called once per function in
    ``code_object_to_iir_with_sidecar``.

    Algorithm
    ---------
    1.  Build a lookup table ``{tetrad_ip: (line, col)}`` from
        ``code.source_map``.

        ``code.source_map`` is a list of ``(tetrad_ip, line, col)`` triples,
        one per instruction the Tetrad compiler emitted via ``_emit()``.

    2.  Walk ``iir_fn.source_map`` — a list of ``(iir_start, tetrad_ip, 0)``
        triples written by the IIR translator — and for each entry look up the
        corresponding ``(line, col)`` from the table built in step 1.

    3.  Call ``writer.record(fn_name, iir_start, ...)`` for every entry that
        has a known source location.  Entries whose ``tetrad_ip`` has no
        corresponding ``code.source_map`` entry (e.g. synthetic instructions
        added by the translator) are silently skipped.

    The ``DebugSidecarReader.lookup(fn, ip)`` method uses
    *nearest-preceding-entry* semantics (like DWARF), so recording the
    *start* of each Tetrad instruction's IIR translation is sufficient — the
    reader correctly attributes intermediate IIR instructions (that have no
    explicit entry) to the source location of the nearest preceding recorded
    start.

    Parameters
    ----------
    writer:
        The ``DebugSidecarWriter`` being built.
    iir_fn:
        The translated ``IIRFunction``.
    code:
        The original Tetrad ``CodeObject`` that was translated.
    file_id:
        File identifier returned by ``writer.add_source_file()``.
    """
    n_instrs = len(iir_fn.instructions)

    # Build lookup: tetrad_ip → (line, col)
    # CodeObject.source_map is populated by _emit() as (tetrad_ip, line, col).
    # Lines are 1-based; skip entries with line == 0 (synthetic HALT, etc.).
    tetrad_to_loc: dict[int, tuple[int, int]] = {
        tetrad_ip: (line, col)
        for tetrad_ip, line, col in code.source_map
        if line > 0
    }

    writer.begin_function(
        iir_fn.name,
        start_instr=0,
        param_count=len(code.params),
    )

    # Declare parameters: each param occupies a positional register slot.
    # The IIR translator pre-binds them via `tetrad.move _ri ← param_name`
    # at function entry.  For the Variables panel we declare them with their
    # original source names and reg_index=i (their natural ordering).
    for i, param_name in enumerate(code.params):
        writer.declare_variable(
            iir_fn.name,
            reg_index=i,
            name=param_name,
            type_hint="u8",
            live_start=0,
            live_end=n_instrs,
        )

    # Declare locals: var_names entries beyond the parameter count are
    # local variables introduced by `let` statements.  They are stored as
    # named IIR slots (not positional registers), live for the full function
    # for conservative correctness.
    for j in range(len(code.params), len(code.var_names)):
        writer.declare_variable(
            iir_fn.name,
            reg_index=j,
            name=code.var_names[j],
            type_hint="u8",
            live_start=0,
            live_end=n_instrs,
        )

    # Record source locations using the composed map.
    #
    # IIRFunction.source_map contains one entry per Tetrad bytecode
    # instruction: (iir_start, tetrad_ip, 0).  The iir_start is the IIR
    # instruction index where that Tetrad instruction's translation begins.
    # Multiple consecutive IIR instructions may share the same Tetrad source
    # location; the nearest-preceding lookup in DebugSidecarReader handles
    # them correctly without needing an explicit record for each.
    for iir_start, tetrad_ip, _unused in iir_fn.source_map:
        loc = tetrad_to_loc.get(tetrad_ip)
        if loc is not None:
            line, col = loc
            writer.record(
                iir_fn.name,
                iir_start,
                file_id=file_id,
                line=line,
                col=col,
            )

    writer.end_function(iir_fn.name, end_instr=n_instrs)
