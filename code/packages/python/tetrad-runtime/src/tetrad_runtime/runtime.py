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

from interpreter_ir import IIRModule
from jit_core import JITCore
from tetrad_compiler import compile_program
from tetrad_compiler.bytecode import CodeObject
from vm_core import VMCore

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
