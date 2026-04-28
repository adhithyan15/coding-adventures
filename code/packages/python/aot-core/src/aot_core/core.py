"""AOTCore — the ahead-of-time compilation controller.

``AOTCore`` compiles an entire ``IIRModule`` to a ``.aot`` binary before any
user runs the program.  Unlike ``JITCore``, it:

- Runs **static type inference** (``infer.infer_types``) instead of consulting
  runtime profiler feedback.
- Compiles **every function** in the module (not just hot ones).
- Writes the result to a ``.aot`` binary rather than registering JIT handlers.
- Supports **cross-compilation** transparently via the backend protocol.

Compilation pipeline
--------------------
::

    IIRModule
        │
        ├── for each function:
        │       infer_types(fn)          → env: dict[str, str]
        │       aot_specialise(fn, env)  → list[CIRInstr]
        │       pipeline.compile(cir)    → bytes | None
        │       backend.compile(cir)     → bytes | None
        │
        │   fully compiled  →  fn_binaries
        │   uncompiled      →  untyped_fns  (→ vm-runtime IIR table)
        │
        ├── link(fn_binaries)            → (native_code, offsets)
        │
        ├── vm_runtime.serialise_iir_table(untyped_fns)  (if any)
        │
        └── snapshot.write(native_code, iir_table, entry_point)  → bytes

Optimization levels
-------------------
0 — no optimization (raw specialised CIR)
1 — constant folding + dead-code elimination (jit-core optimizer passes)
2 — same as 1 plus AOT-specific passes (function inlining, loop unrolling)
    Note: inlining and loop unrolling are future work; level 2 currently
    performs the same as level 1.

vm-runtime behaviour
--------------------
When a function cannot be fully specialized (all types remain ``"any"`` after
inference), it is considered *untyped* and emitted into the IIR table section:

- If ``vm_runtime`` is ``None``: the untyped function's IIR bytes are still
  written to the IIR table (so a simulator with Python ``vm-core`` can
  interpret them), but no pre-compiled runtime library is linked in.
- If a ``VmRuntime`` instance is provided: its ``library_bytes`` are appended
  after the IIR table so a native runtime can be linked at load time.
"""

from __future__ import annotations

import time
from typing import Any

from interpreter_ir import IIRModule
from interpreter_ir.function import FunctionTypeStatus

from codegen_core import CIROptimizer, CodegenPipeline
from codegen_core.backend import BackendProtocol  # re-exported for backwards compat

from aot_core import link as link_module
from aot_core import snapshot as snapshot_module
from aot_core.infer import infer_types
from aot_core.specialise import aot_specialise
from aot_core.stats import AOTStats
from aot_core.vm_runtime import VmRuntime


class AOTCore:
    """Ahead-of-time compilation controller.

    Parameters
    ----------
    backend:
        A backend implementing ``BackendProtocol``.  Responsible for
        translating ``CIRInstr`` lists to native binaries.
    vm_runtime:
        A ``VmRuntime`` instance to link into the binary when untyped functions
        are present.  ``None`` means no pre-compiled runtime is embedded
        (untyped functions still appear in the IIR table, but no library bytes
        are appended).
    optimization_level:
        0 = no optimization; 1 = constant fold + DCE; 2 = 1 + AOT passes.
    debug_info:
        Reserved for future use.  If ``True``, a debug section will be
        written once the debug-info format is specified.
    """

    def __init__(
        self,
        backend: BackendProtocol,
        vm_runtime: VmRuntime | None = None,
        optimization_level: int = 2,
        debug_info: bool = False,
    ) -> None:
        self._backend = backend
        self._vm_runtime = vm_runtime
        self._optimization_level = optimization_level
        self._debug_info = debug_info
        self._stats = AOTStats(optimization_level=optimization_level)
        # Build the codegen pipeline.  When optimization_level > 0, attach
        # the CIR optimizer (constant folding + DCE) from codegen-core.
        # This is the same CodegenPipeline[list[CIRInstr]] used by JITCore;
        # sharing it removes the aot-core → jit-core backwards dependency.
        _optimizer = CIROptimizer() if optimization_level > 0 else None
        self._pipeline: CodegenPipeline = CodegenPipeline(
            backend=backend, optimizer=_optimizer
        )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def compile(self, module: IIRModule) -> bytes:
        """Compile the entire ``module`` to a ``.aot`` binary.

        Parameters
        ----------
        module:
            The module to compile.  All functions are processed; the
            entry-point is assumed to be ``"main"``.

        Returns
        -------
        bytes
            Complete ``.aot`` binary (header + code section + optional IIR table).
        """
        t_start = time.monotonic_ns()

        fn_binaries: list[tuple[str, bytes]] = []
        untyped_fns = []

        for iir_fn in module.functions:
            binary = self._compile_fn(iir_fn)
            if binary is not None:
                fn_binaries.append((iir_fn.name, binary))
                self._stats.functions_compiled += 1
                self._stats.total_binary_size += len(binary)
            else:
                untyped_fns.append(iir_fn)
                self._stats.functions_untyped += 1

        # Link compiled functions into one code section.
        if fn_binaries:
            native_code, offsets = link_module.link(fn_binaries)
            ep_offset = link_module.entry_point_offset(offsets)
        else:
            native_code = b""
            ep_offset = 0

        # Serialise untyped functions into the IIR table.
        iir_table: bytes | None = None
        if untyped_fns:
            rt = self._vm_runtime or VmRuntime()
            iir_table = rt.serialise_iir_table(untyped_fns)

        self._stats.compilation_time_ns += time.monotonic_ns() - t_start

        data = snapshot_module.write(native_code, iir_table, ep_offset)

        # Append pre-compiled vm-runtime library bytes after the snapshot so a
        # native linker can locate them.  The header's vm_iir_table_offset
        # already points past the code section to the IIR table; the library
        # bytes follow without additional header entries.
        if iir_table is not None and self._vm_runtime and not self._vm_runtime.is_empty:
            data = data + self._vm_runtime.library_bytes

        return data

    def compile_to_file(self, module: IIRModule, path: str) -> None:
        """Compile ``module`` and write the ``.aot`` binary to ``path``.

        Parameters
        ----------
        module:
            The module to compile.
        path:
            Destination file path (e.g., ``"program.aot"``).
        """
        data = self.compile(module)
        with open(path, "wb") as fh:
            fh.write(data)

    def stats(self) -> AOTStats:
        """Return a snapshot of compilation statistics.

        The snapshot is cumulative across all ``compile()`` calls since this
        ``AOTCore`` instance was created.
        """
        return AOTStats(
            functions_compiled=self._stats.functions_compiled,
            functions_untyped=self._stats.functions_untyped,
            compilation_time_ns=self._stats.compilation_time_ns,
            total_binary_size=self._stats.total_binary_size,
            optimization_level=self._stats.optimization_level,
        )

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _compile_fn(self, fn: Any) -> bytes | None:
        """Infer, specialise, optimise, and compile one function.

        Delegates to ``self._pipeline.compile()`` which runs the CIR
        optimizer (constant fold + DCE) and then the backend.  The
        pipeline was built in ``__init__`` with the appropriate optimizer
        attached based on ``optimization_level``.

        Returns
        -------
        bytes
            Native binary produced by the backend.
        None
            If the function could not be compiled (backend returned ``None``,
            or an exception was raised).
        """
        try:
            inferred = infer_types(fn)
            cir = aot_specialise(fn, inferred)
            return self._pipeline.compile(cir)
        except Exception:
            return None

    def _is_fully_typed(self, fn: Any, inferred: dict[str, str]) -> bool:
        """Return True if all instruction types in ``fn`` can be resolved."""
        from interpreter_ir import IIRFunction
        if not isinstance(fn, IIRFunction):
            return False
        if fn.type_status == FunctionTypeStatus.FULLY_TYPED:
            return True
        # Check whether inference resolved all dest types.
        for instr in fn.instructions:
            if instr.dest is None:
                continue
            t = inferred.get(instr.dest, "any")
            if t == "any" and instr.type_hint == "any":
                return False
        return True
