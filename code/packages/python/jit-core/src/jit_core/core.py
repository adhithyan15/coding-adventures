"""JITCore — the top-level JIT compilation and dispatch API.

JITCore monitors a VMCore execution, detects hot functions, compiles them
through a registered backend, and registers native handlers with the VM so
subsequent calls bypass the interpreter entirely.

Compilation pipeline
--------------------
::

    IIRFunction (with feedback vectors from VMProfiler)
        │
        ▼ specialise()
    list[CIRInstr]   (typed, with guards)
        │
        ▼ optimizer.run()
    list[CIRInstr]   (constant-folded, DCE'd)
        │
        ▼ backend.compile()
    bytes            (native binary)
        │
        ▼ backend.run()   (registered as JIT handler in VMCore)
    return value

Tiered compilation
------------------
Three tiers control when a function is compiled:

    FULLY_TYPED     → compile eagerly before the first interpreted call
    PARTIALLY_TYPED → compile after ``threshold_partial`` interpreted calls
    UNTYPED         → compile after ``threshold_untyped`` interpreted calls

A threshold of ``0`` means "compile before any interpreted call".

Deoptimization
--------------
The JIT handler wrapper tracks ``exec_count`` and ``deopt_count`` (via
``record_deopt()``).  When ``deopt_rate > 0.1`` the compiled function is
invalidated and the function is marked unspecializable — it runs interpreted
forever after.

Thread safety
-------------
Not thread-safe.  Each thread should own its own ``JITCore`` instance.
"""

from __future__ import annotations

from typing import Any

from interpreter_ir import IIRFunction, IIRModule
from interpreter_ir.function import FunctionTypeStatus
from vm_core import VMCore

from jit_core import optimizer
from jit_core.backend import BackendProtocol
from jit_core.cache import JITCache, JITCacheEntry
from jit_core.errors import UnspecializableError
from jit_core.specialise import specialise

# Deopt rate above which a function is permanently invalidated.
_DEOPT_RATE_LIMIT: float = 0.1


class JITCore:
    """Generic JIT compilation engine.

    Parameters
    ----------
    vm:
        The ``VMCore`` instance to attach to.  JITCore registers JIT handlers
        here so the VM's dispatch loop calls compiled code instead of
        interpreting.
    backend:
        A backend implementing ``BackendProtocol``.  Responsible for
        translating ``CIRInstr`` lists to native binaries and executing them.
    threshold_fully_typed:
        Call count threshold for ``FULLY_TYPED`` functions.  ``0`` means
        compile before the first interpreted call.
    threshold_partial:
        Call count threshold for ``PARTIALLY_TYPED`` functions.
    threshold_untyped:
        Call count threshold for ``UNTYPED`` functions.
    min_observations:
        Minimum profiler observation count before an observed type is trusted
        for specialization.
    """

    def __init__(
        self,
        vm: VMCore,
        backend: BackendProtocol,
        threshold_fully_typed: int = 0,
        threshold_partial: int = 10,
        threshold_untyped: int = 100,
        min_observations: int = 5,
    ) -> None:
        self._vm = vm
        self._backend = backend
        self._cache = JITCache()
        self._module: IIRModule | None = None
        self._unspecializable: set[str] = set()
        self._min_observations = min_observations
        self._thresholds: dict[FunctionTypeStatus, int] = {
            FunctionTypeStatus.FULLY_TYPED: threshold_fully_typed,
            FunctionTypeStatus.PARTIALLY_TYPED: threshold_partial,
            FunctionTypeStatus.UNTYPED: threshold_untyped,
        }

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def execute_with_jit(
        self,
        module: IIRModule,
        *,
        fn: str = "main",
        args: list[Any] | None = None,
    ) -> Any:
        """Run ``module`` under the interpreter with JIT compilation.

        Phase 1: compile all ``FULLY_TYPED`` functions eagerly (before the
        first interpreted call).
        Phase 2: run ``fn`` via the interpreter (JIT handlers fire for any
        functions that were already compiled).
        Phase 3: after Phase 2, promote any function whose call count crossed
        its tier threshold during Phase 2.

        Parameters
        ----------
        module:
            The module to execute.
        fn:
            Entry-point function name.
        args:
            Arguments for the entry-point function.

        Returns
        -------
        Any
            Return value of ``fn``, or ``None``.
        """
        self._module = module

        # Phase 1 — eager compilation of FULLY_TYPED functions.
        for iir_fn in module.functions:
            if (
                iir_fn.type_status == FunctionTypeStatus.FULLY_TYPED
                and not self.is_compiled(iir_fn.name)
                and iir_fn.name not in self._unspecializable
            ):
                self._compile_fn(iir_fn)

        # Phase 2 — interpreted execution.
        result = self._vm.execute(module, fn=fn, args=args)

        # Phase 3 — promote functions that turned hot during Phase 2.
        self._promote_hot_functions()

        return result

    def compile(self, fn_name: str) -> bool:
        """Manually compile ``fn_name``.

        Returns
        -------
        bool
            ``True`` on success, ``False`` if the function was not found,
            cannot be compiled, or the backend returned ``None``.

        Raises
        ------
        UnspecializableError
            If the function has been marked unspecializable.
        """
        if fn_name in self._unspecializable:
            raise UnspecializableError(
                f"function {fn_name!r} is marked unspecializable (deopt rate exceeded)"
            )
        if self._module is None:
            return False
        fn = self._module.get_function(fn_name)
        if fn is None:
            return False
        return self._compile_fn(fn)

    def execute(self, fn_name: str, args: list[Any] | None = None) -> Any:
        """Execute ``fn_name`` using the compiled binary or the interpreter.

        If the function is compiled, the JIT handler runs directly.
        If not, it falls back to interpreted execution and then attempts
        to promote the function if it has turned hot.

        Parameters
        ----------
        fn_name:
            Name of the function to call.
        args:
            Positional arguments.

        Returns
        -------
        Any
            Return value, or ``None`` for void functions.
        """
        if self._module is None:
            return None

        entry = self._cache.get(fn_name)
        if entry is not None:
            result = self._backend.run(entry.binary, args or [])
            entry.exec_count += 1
            self._check_deopt_rate(entry)
            return result

        result = self._vm.execute(self._module, fn=fn_name, args=args)
        self._promote_hot_functions()
        return result

    def is_compiled(self, fn_name: str) -> bool:
        """Return ``True`` if ``fn_name`` has a cached native binary."""
        return fn_name in self._cache

    def cache_stats(self) -> dict[str, dict]:
        """Return per-function JIT cache statistics."""
        return self._cache.stats()

    def dump_ir(self, fn_name: str) -> str:
        """Return the post-optimization CIR for ``fn_name`` as a string.

        Returns an empty string if the function has not been compiled.
        """
        entry = self._cache.get(fn_name)
        if entry is None:
            return ""
        return "\n".join(str(instr) for instr in entry.ir)

    def invalidate(self, fn_name: str) -> None:
        """Remove the compiled version of ``fn_name`` and mark it as
        unspecializable so it is never re-compiled."""
        self._cache.invalidate(fn_name)
        self._unspecializable.add(fn_name)
        self._vm.unregister_jit_handler(fn_name)

    def record_deopt(self, fn_name: str) -> None:
        """Increment the deopt counter for ``fn_name``.

        Called by the deopt stub when a compiled function falls back to the
        interpreter.  If the deopt rate exceeds the limit, the function is
        invalidated.
        """
        entry = self._cache.get(fn_name)
        if entry is None:
            return
        entry.deopt_count += 1
        self._check_deopt_rate(entry)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _promote_hot_functions(self) -> None:
        """Compile any function that has crossed its tier threshold."""
        if self._module is None:
            return
        counts = self._vm.metrics().function_call_counts
        for iir_fn in self._module.functions:
            if self.is_compiled(iir_fn.name):
                continue
            if iir_fn.name in self._unspecializable:
                continue
            threshold = self._thresholds.get(iir_fn.type_status)
            if threshold is None:
                continue
            if threshold == 0:
                # FULLY_TYPED functions are compiled eagerly in execute_with_jit.
                continue
            if counts.get(iir_fn.name, 0) >= threshold:
                self._compile_fn(iir_fn)

    def _compile_fn(self, fn: IIRFunction) -> bool:
        """Specialise, optimise, and compile ``fn`` via the backend.

        Returns ``True`` on success, ``False`` on any failure.  On success,
        registers a JIT handler with ``vm-core``.
        """
        t0 = JITCache.now_ns()
        try:
            cir = specialise(fn, min_observations=self._min_observations)
            cir = optimizer.run(cir)
            binary = self._backend.compile(cir)
        except Exception:
            return False

        if binary is None:
            return False

        t1 = JITCache.now_ns()

        entry = JITCacheEntry(
            fn_name=fn.name,
            binary=binary,
            backend_name=self._backend.name,
            param_count=len(fn.params),
            ir=cir,
            compilation_time_ns=t1 - t0,
        )
        self._cache.put(entry)

        # Register a JIT handler so vm-core bypasses the interpreter.
        def _jit_handler(args: list[Any]) -> Any:
            result = self._backend.run(entry.binary, args)
            entry.exec_count += 1
            return result

        self._vm.register_jit_handler(fn.name, _jit_handler)
        return True

    def _check_deopt_rate(self, entry: JITCacheEntry) -> None:
        """Invalidate ``entry`` if its deopt rate exceeds the limit."""
        if entry.exec_count > 0 and entry.deopt_rate > _DEOPT_RATE_LIMIT:
            self.invalidate(entry.fn_name)
