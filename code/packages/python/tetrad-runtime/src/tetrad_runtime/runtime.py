"""``TetradRuntime`` — end-to-end Tetrad-on-LANG façade.

``TetradRuntime`` wires the four moving parts of a Tetrad-on-LANG run:

1.  ``tetrad_compiler.compile_program`` to produce a Tetrad ``CodeObject``
2.  ``code_object_to_iir`` to translate that into an ``IIRModule`` of
    standard LANG01 opcodes (plus the ``tetrad.move`` extension)
3.  ``vm_core.VMCore`` configured with:
       - ``u8_wrap=True`` (Tetrad's 8-bit register semantics)
       - the ``TETRAD_OPCODE_EXTENSIONS`` opcode table
       - four host-provided builtins (``__io_in``, ``__io_out``,
         ``__get_global``, ``__set_global``)
4.  Optional: ``jit_core.JITCore`` with an ``Intel4004Backend`` for
    profile-guided JIT compilation of FULLY_TYPED functions.

The class is a deliberate **thin façade**.  It does not try to mirror the
shape of the legacy ``TetradVM`` API (which expects branch-stat dicts,
feedback-vector state machines, etc.); those features live in vm-core's
profiler and metrics in a different shape and are exposed as needed via
``TetradRuntime.profiler_observations()``.

Globals
-------
Tetrad globals are shared across function calls.  ``TetradRuntime`` keeps
a per-instance dict ``self._globals`` and registers two builtins on the
VM that read and write that dict.  Globals are reset on each ``run()``
call so multiple runs on the same runtime don't leak state.

I/O
---
The constructor accepts ``io_in`` / ``io_out`` callables matching
``TetradVM``'s public signature.  They are wrapped as builtins so the
compiled IIR can reach them through ``call_builtin``.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

from interpreter_ir import IIRModule, SlotKind, SlotState
from jit_core import JITCore
from tetrad_compiler import compile_program
from tetrad_compiler.bytecode import CodeObject
from vm_core import BranchStats, DebugHooks, VMCore, VMTrace

from tetrad_runtime.iir_translator import (
    TETRAD_OPCODE_EXTENSIONS,
    code_object_to_iir,
)

__all__ = ["TetradRuntime", "compile_to_iir"]


def compile_to_iir(source: str, *, module_name: str = "tetrad-program") -> IIRModule:
    """Lex + parse + type-check + compile + translate to ``IIRModule``.

    Convenience entry point that runs the full Tetrad front-end and then
    hands the resulting ``CodeObject`` to ``code_object_to_iir``.
    """
    code = compile_program(source)
    return code_object_to_iir(code, module_name=module_name)


# Default I/O callables match the legacy ``TetradVM`` defaults: read from
# stdin, write to stdout.  Both wrap the result in a u8 mask so the
# downstream IIR sees a valid u8 value regardless of what the host returns.


def _default_io_in() -> int:
    return int(input()) & 0xFF


def _default_io_out(value: int) -> None:
    print(value)


class TetradRuntime:
    """End-to-end Tetrad-on-LANG runtime.

    Parameters
    ----------
    io_in:
        Callable returning a u8 for ``IO_IN`` instructions.  Defaults to
        ``int(input()) & 0xFF``.
    io_out:
        Callable accepting a u8 for ``IO_OUT`` instructions.  Defaults to
        ``print``.
    enable_jit:
        If True, ``run`` will use ``run_with_jit`` semantics by default.
        If False (default), only the interpreted path is used.

    Attributes
    ----------
    last_module:
        The most-recently-compiled ``IIRModule`` (or ``None`` if no
        program has been run yet).  Useful for tests and tooling that
        want to inspect the IIR.
    """

    def __init__(
        self,
        *,
        io_in: Callable[[], int] | None = None,
        io_out: Callable[[int], None] | None = None,
        enable_jit: bool = False,
    ) -> None:
        self._io_in: Callable[[], int] = io_in if io_in is not None else _default_io_in
        self._io_out: Callable[[int], None] = (
            io_out if io_out is not None else _default_io_out
        )
        self._enable_jit = enable_jit
        self.last_module: IIRModule | None = None
        # The most-recent VMCore — kept after ``run()`` returns so that
        # ``globals_snapshot`` can read vm._memory.  A fresh VM is created
        # for each ``run()`` to prevent cross-run state leakage.
        self._last_vm: VMCore | None = None
        # The most-recent DebugSidecar bytes produced by ``compile_with_debug``.
        # ``None`` until the first debug compilation.
        self._last_sidecar: bytes | None = None

    # ------------------------------------------------------------------
    # Public entry points
    # ------------------------------------------------------------------

    def run(self, source: str) -> Any:
        """Compile, translate, and execute a Tetrad source program.

        Returns whatever the program's ``main`` function (or the top-level
        statements, if no ``main`` exists) returns.  For Tetrad programs
        this is always either an ``int`` (the final accumulator value) or
        ``None`` (if the program ended with HALT).
        """
        module = compile_to_iir(source)
        return self.run_module(module)

    def run_module(self, module: IIRModule) -> Any:
        """Execute a pre-built ``IIRModule`` on the LANG interpreter.

        Most callers will use ``run(source)`` instead; this entry point
        exists for tooling that wants to assemble or modify the module
        before running it.
        """
        self.last_module = module
        vm = self._make_vm()
        self._last_vm = vm
        return vm.execute(module, fn=module.entry_point or "main")

    def run_with_jit(
        self,
        source: str,
        *,
        backend: Any = None,
    ) -> Any:
        """Run via ``jit_core.JITCore`` with a backend (default: Intel4004).

        If ``backend`` is None, an ``Intel4004Backend`` instance is created
        automatically.  Functions the backend cannot compile fall back to
        interpretation transparently — this is jit-core's standard
        deopt-on-fail behaviour.
        """
        if backend is None:
            from tetrad_runtime.intel4004_backend import Intel4004Backend
            backend = Intel4004Backend()

        module = compile_to_iir(source)
        self.last_module = module
        vm = self._make_vm()
        self._last_vm = vm
        jit = JITCore(vm, backend)
        return jit.execute_with_jit(module, fn=module.entry_point or "main")

    def run_code_object(self, code: CodeObject) -> Any:
        """Translate an existing ``CodeObject`` to IIR and execute.

        Useful when callers already have a ``CodeObject`` from
        ``tetrad_compiler.compile_program`` and want to run it through the
        LANG pipeline without re-parsing the source.
        """
        module = code_object_to_iir(code)
        return self.run_module(module)

    def compile_with_debug(
        self,
        source: str,
        source_path: str,
    ) -> tuple[IIRModule, bytes]:
        """Compile ``source`` and produce a ``DebugSidecar`` alongside the module.

        This is the debug-aware companion to ``compile_to_iir``.  It performs
        the same Tetrad → IIR translation and additionally builds a sidecar
        that maps every IIR instruction index in every function back to the
        original Tetrad source line and column.

        The sidecar bytes can be passed directly to
        ``debug_sidecar.DebugSidecarReader`` to answer:

        - ``reader.lookup(fn_name, iir_ip)``  → ``SourceLocation``
        - ``reader.find_instr(source_path, line)``  → ``int | None`` (IIR index)
        - ``reader.live_variables(fn_name, iir_ip)``  → ``list[Variable]``

        Parameters
        ----------
        source:
            Raw Tetrad source code.
        source_path:
            Path to the Tetrad source file — stored verbatim in the sidecar
            and used by ``find_instr`` when the debugger resolves a breakpoint
            by source file + line number.

        Returns
        -------
        tuple[IIRModule, bytes]
            ``(module, sidecar_bytes)`` — the module is identical to what
            ``compile_to_iir(source)`` would produce; ``sidecar_bytes`` is
            the freshly built DebugSidecar.
        """
        from tetrad_runtime.sidecar_builder import code_object_to_iir_with_sidecar

        code = compile_program(source)
        module, sidecar = code_object_to_iir_with_sidecar(code, source_path)
        self.last_module = module
        self._last_sidecar = sidecar
        return module, sidecar

    def run_with_debug(
        self,
        source: str,
        source_path: str,
        *,
        hooks: DebugHooks | None = None,
        breakpoints: dict[str, list[int]] | None = None,
    ) -> Any:
        """Compile and execute ``source`` with optional debug hooks and breakpoints.

        This combines ``compile_with_debug`` + ``run_module`` in one call,
        wiring up a debug adapter before execution begins.

        The typical workflow for a debug session:

        1. Build the sidecar via ``compile_with_debug`` (or use this method
           directly).
        2. Resolve source-line breakpoints to IIR indices via
           ``DebugSidecarReader.find_instr(source_path, line)``.
        3. Pass those indices as ``breakpoints`` here.
        4. Subclass ``DebugHooks`` and pass an instance as ``hooks``.
        5. In ``on_instruction`` inspect ``frame.ip`` — look up the source
           location with ``reader.lookup(frame.fn.name, frame.ip)``.

        Parameters
        ----------
        source:
            Raw Tetrad source code.
        source_path:
            Path to the Tetrad source file.  Passed to ``compile_with_debug``.
        hooks:
            Optional ``DebugHooks`` subclass.  If provided, it is attached to
            the VM before execution so ``on_instruction``, ``on_call``,
            ``on_return``, and ``on_exception`` fire at the appropriate points.
        breakpoints:
            Optional pre-set breakpoints.  Maps function name → list of IIR
            instruction indices at which to pause.  Resolve source-line
            breakpoints to IIR indices first via ``DebugSidecarReader``.

        Returns
        -------
        Any
            Same return value as ``run(source)``.
        """
        module, _sidecar = self.compile_with_debug(source, source_path)
        vm = self._make_vm()
        self._last_vm = vm

        if hooks is not None:
            vm.attach_debug_hooks(hooks)

        if breakpoints:
            for fn_name, indices in breakpoints.items():
                for idx in indices:
                    vm.set_breakpoint(idx, fn_name)

        return vm.execute(module, fn=module.entry_point or "main")

    # ------------------------------------------------------------------
    # Public introspection
    # ------------------------------------------------------------------

    @property
    def globals_snapshot(self) -> dict[str, Any]:
        """Read-only view of the globals at the end of the last run.

        Reads vm-core's memory map at the addresses assigned by the
        translator to each top-level global, looking up the address-to-name
        mapping from the IIR module's ``tetrad_globals`` attribute.
        Returns an empty dict if no run has happened yet.
        """
        if self._last_vm is None or self.last_module is None:
            return {}
        names = getattr(self.last_module, "tetrad_globals", []) or []
        return {
            name: int(self._last_vm.memory.get(addr, 0)) & 0xFF
            for addr, name in enumerate(names)
        }

    # ------------------------------------------------------------------
    # Legacy TetradVM API parity (LANG17 PR4).
    #
    # These wrappers re-project vm-core's generic, IIR-IP-keyed metric
    # surface into the exact signatures the legacy ``tetrad-vm``
    # ``TetradVM`` class exposed.  Existing callers can switch from
    # ``TetradVM → TetradRuntime`` without rewriting metric-reading code.
    #
    # Each method requires that the most recent ``run()`` populated
    # ``self._last_vm`` and ``self.last_module`` — an unrun runtime
    # returns ``None`` / empty rather than raising, matching the legacy
    # behaviour.
    # ------------------------------------------------------------------

    def hot_functions(self, threshold: int = 100) -> list[str]:
        """Return names of functions called at least ``threshold`` times.

        Mirrors ``TetradVM.hot_functions``.  Always returns an empty list
        if no run has happened yet.
        """
        if self._last_vm is None:
            return []
        return self._last_vm.hot_functions(threshold)

    def feedback_vector(self, fn_name: str) -> list[SlotState] | None:
        """Return the per-slot ``SlotState`` list for ``fn_name``.

        Mirrors ``TetradVM.feedback_vector``.  Returns ``None`` if the
        function has no slots, has never been called, or the runtime has
        not been used yet.

        The list is indexed by the slot index the Tetrad compiler assigned
        (from the bytecode's slot operand), reconstructed by walking
        ``IIRFunction.feedback_slots`` populated in the translator.
        Slots whose IIR instruction hasn't been observed yet are
        represented by a fresh UNINITIALIZED ``SlotState`` so the list
        is never sparse.
        """
        fn, _instrs = self._lookup_fn(fn_name)
        if fn is None:
            return None
        slot_map = fn.feedback_slots
        if not slot_map:
            return None
        max_slot = max(slot_map.keys())
        out: list[SlotState] = []
        for slot in range(max_slot + 1):
            iir_idx = slot_map.get(slot)
            if iir_idx is None:
                out.append(SlotState())
                continue
            instr = fn.instructions[iir_idx]
            out.append(instr.observed_slot or SlotState())
        return out

    def type_profile(self, fn_name: str, slot: int) -> SlotState | None:
        """Return the ``SlotState`` for one slot in ``fn_name``, or ``None``.

        Mirrors ``TetradVM.type_profile``.
        """
        vec = self.feedback_vector(fn_name)
        if vec is None or slot >= len(vec):
            return None
        return vec[slot]

    def call_site_shape(self, fn_name: str, slot: int) -> SlotKind:
        """Return the IC shape (``SlotKind``) of one CALL feedback slot.

        Mirrors ``TetradVM.call_site_shape``.  Returns
        ``SlotKind.UNINITIALIZED`` for unknown / unreached slots, matching
        legacy behaviour.
        """
        state = self.type_profile(fn_name, slot)
        if state is None:
            return SlotKind.UNINITIALIZED
        return state.kind

    def branch_profile(self, fn_name: str, tetrad_ip: int) -> BranchStats | None:
        """Return ``BranchStats`` for the branch at the original Tetrad IP.

        Mirrors ``TetradVM.branch_profile``.  ``tetrad_ip`` is the
        instruction index in the *original Tetrad bytecode*, not the IIR
        index.  We walk ``IIRFunction.source_map`` to find the IIR
        index where that Tetrad instr's translation starts, then
        consult ``vm_core.branch_profile`` keyed by IIR IP.

        The IIR translator emits the conditional branch (``jmp_if_true``
        / ``jmp_if_false``) as the only ``jmp_if_*`` instruction in the
        translation of a Tetrad JZ/JNZ — so the IIR's source-map start
        index is the right one to consult.
        """
        if self._last_vm is None:
            return None
        fn, _ = self._lookup_fn(fn_name)
        if fn is None:
            return None
        iir_ip = self._iir_ip_for_tetrad_ip(fn, tetrad_ip)
        if iir_ip is None:
            return None
        # Tetrad JZ / JNZ both translate to a small IIR sequence; the
        # ``jmp_if_*`` instruction lives one or two instructions deep.
        # Scan forward from iir_ip until we hit one (bounded by the start
        # of the next Tetrad source-map entry).
        iir_branch_ip = self._find_branch_in_translation(fn, iir_ip)
        if iir_branch_ip is None:
            return None
        return self._last_vm.branch_profile(fn_name, iir_branch_ip)

    def loop_iterations(self, fn_name: str) -> dict[int, int]:
        """Return ``{tetrad_ip: hit_count}`` for back-edges in ``fn_name``.

        Mirrors ``TetradVM.loop_iterations``.  Re-keys the IIR-IP-keyed
        counts back into Tetrad-IP space using ``IIRFunction.source_map``.
        Tetrad IPs that produced no IIR back-edge (e.g. forward jumps)
        are absent from the returned dict.
        """
        if self._last_vm is None:
            return {}
        fn, _ = self._lookup_fn(fn_name)
        if fn is None:
            return {}
        iir_loops = self._last_vm.loop_iterations(fn_name)
        if not iir_loops:
            return {}
        # Build a reverse map: iir_ip → tetrad_ip (using the start-of-
        # translation entries from source_map, then fanning forward to
        # every IIR ip in that Tetrad instr's translation).
        out: dict[int, int] = {}
        for iir_ip, count in iir_loops.items():
            tetrad_ip = self._tetrad_ip_for_iir_ip(fn, iir_ip)
            if tetrad_ip is not None:
                out[tetrad_ip] = count
        return out

    def reset_metrics(self) -> None:
        """Zero all aggregate counters on the live VM (if any).

        Mirrors ``TetradVM.reset_metrics``.  No-op if no run has happened.
        Per-instruction observations live on the IIR module and are NOT
        reset here — to clear those, run a fresh source through ``run``.
        """
        if self._last_vm is not None:
            self._last_vm.reset_metrics()

    def execute_traced(self, source: str) -> tuple[Any, list[VMTrace]]:
        """Run ``source`` and return ``(result, list[VMTrace])``.

        Mirrors ``TetradVM.execute_traced``.  Each ``VMTrace`` records
        one IIR instruction dispatch — note that one Tetrad bytecode
        instruction typically translates to several IIR instructions, so
        the trace is denser than legacy callers may expect.  Frontends
        that need Tetrad-IP-keyed traces can re-project through
        ``IIRFunction.source_map``.
        """
        module = compile_to_iir(source)
        self.last_module = module
        vm = self._make_vm()
        self._last_vm = vm
        return vm.execute_traced(module, fn=module.entry_point or "main")

    # ------------------------------------------------------------------
    # Legacy-API helpers
    # ------------------------------------------------------------------

    def _lookup_fn(self, fn_name: str):
        """Return ``(IIRFunction | None, instructions | None)``.

        Convenience helper for the metric wrappers that need to pull a
        function out of the most recently executed module.
        """
        if self.last_module is None:
            return None, None
        fn = self.last_module.get_function(fn_name)
        if fn is None:
            return None, None
        return fn, fn.instructions

    @staticmethod
    def _iir_ip_for_tetrad_ip(fn, tetrad_ip: int) -> int | None:
        """Resolve a Tetrad IP to the IIR IP at the start of its translation."""
        for iir_ip, t_ip, _col in fn.source_map:
            if t_ip == tetrad_ip:
                return iir_ip
        return None

    @staticmethod
    def _tetrad_ip_for_iir_ip(fn, iir_ip: int) -> int | None:
        """Reverse resolve: IIR IP → Tetrad IP it belongs to.

        A Tetrad instruction's translation occupies the IIR range
        ``[start, next_start)`` — find the largest ``start`` that is
        ``<= iir_ip``.
        """
        best: tuple[int, int] | None = None
        for iir_start, t_ip, _col in fn.source_map:
            if iir_start <= iir_ip and (best is None or iir_start > best[0]):
                best = (iir_start, t_ip)
        return best[1] if best is not None else None

    @staticmethod
    def _find_branch_in_translation(fn, iir_ip_start: int) -> int | None:
        """Find the conditional-branch IIR ip within one Tetrad translation.

        Returns the index of the first ``jmp_if_true`` / ``jmp_if_false``
        starting at or after ``iir_ip_start``, bounded by the start of
        the next source-map entry (i.e. the next Tetrad instr).
        """
        # Determine the upper bound (start of next Tetrad translation).
        next_starts = sorted(
            iir_start for iir_start, _, _ in fn.source_map if iir_start > iir_ip_start
        )
        upper = next_starts[0] if next_starts else len(fn.instructions)
        for ip in range(iir_ip_start, upper):
            instr = fn.instructions[ip]
            if instr.op in ("jmp_if_true", "jmp_if_false"):
                return ip
        return None

    # ------------------------------------------------------------------
    # Internal: vm-core construction with Tetrad opcodes and builtins.
    # ------------------------------------------------------------------

    def _make_vm(self) -> VMCore:
        """Create a fresh VMCore configured for Tetrad semantics.

        Each call returns a new VMCore so that profiler observations and
        call counts from one ``run()`` don't leak into the next.  Globals,
        which must persist within a single program run, are stored on
        ``self._globals`` and accessed through builtins that close over it.
        """
        vm = VMCore(
            opcodes=TETRAD_OPCODE_EXTENSIONS,
            u8_wrap=True,
            max_frames=4,    # match the historical TetradVM limit
        )
        # I/O builtins — close over self._io_in / self._io_out so callers can
        # supply test doubles via the constructor.
        vm.register_builtin("__io_in", lambda _args: self._io_in() & 0xFF)
        vm.register_builtin("__io_out", lambda args: self._io_out(int(args[0]) & 0xFF))
        return vm
