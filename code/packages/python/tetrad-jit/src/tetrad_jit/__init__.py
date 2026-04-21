"""Tetrad JIT Compiler — Intel 4004 native code generator (spec TET05).

The JIT wraps a ``TetradVM`` and compiles hot Tetrad functions to Intel 4004
machine code.  Compiled binaries are executed on ``Intel4004Simulator``; all
other functions fall back to the interpreter.

Architecture
------------

The pipeline is::

    Tetrad bytecode (CodeObject)
        ↓ translate.py      — bytecode → SSA IR (virtual variables)
        ↓ passes.py         — constant folding + dead code elimination
        ↓ codegen_4004.py   — IR → Intel 4004 abstract assembly
        ↓ two-pass assembler — abstract assembly → 4004 binary (bytes)
        ↓ Intel4004Simulator — execute binary, return u8 result

Triggering
----------
Three tiers based on ``CodeObject.type_status``:

- ``FULLY_TYPED``     — compiled before the first interpreter call.
- ``PARTIALLY_TYPED`` — compiled after 10 interpreter calls.
- ``UNTYPED``         — compiled after 100 interpreter calls.

Deoptimisation
--------------
Operations not yet supported by the 4004 code generator (``mul``, ``div``,
``and``, ``or``, ``xor``, bitwise shifts, I/O, function calls) cause
``compile()`` to return ``False``.  The function is then executed by the
``TetradVM`` interpreter on every call.

Quick start::

    from tetrad_compiler import compile_program
    from tetrad_vm import TetradVM
    from tetrad_jit import TetradJIT

    code = compile_program("fn add(a: u8, b: u8) -> u8 { return a + b; }")
    vm   = TetradVM()
    jit  = TetradJIT(vm)
    jit.execute_with_jit(code)          # compiles FULLY_TYPED functions
    assert jit.is_compiled("add")
    assert jit.execute("add", [10, 20]) == 30
"""

from __future__ import annotations

from tetrad_compiler.bytecode import CodeObject
from tetrad_type_checker.types import FunctionTypeStatus
from tetrad_vm import TetradVM

from tetrad_jit.cache import JITCache, JITCacheEntry
from tetrad_jit.codegen_4004 import codegen, run_on_4004
from tetrad_jit.ir import IRInstr
from tetrad_jit.passes import optimize
from tetrad_jit.translate import translate

__all__ = ["TetradJIT"]

# ---------------------------------------------------------------------------
# Hot-function call-count thresholds
# ---------------------------------------------------------------------------

_THRESHOLDS: dict[FunctionTypeStatus, int] = {
    FunctionTypeStatus.FULLY_TYPED:     0,   # compile before first call
    FunctionTypeStatus.PARTIALLY_TYPED: 10,
    FunctionTypeStatus.UNTYPED:         100,
}


class TetradJIT:
    """Profile-guided Intel 4004 JIT compiler for Tetrad functions.

    Parameters
    ----------
    vm:
        A ``TetradVM`` instance.  The JIT wraps it: ``execute_with_jit``
        delegates to ``vm.execute`` for the interpreted tier.

    Usage
    -----
    ::

        jit = TetradJIT(vm)
        jit.execute_with_jit(code)     # run program; auto-compile hot fns
        jit.compile("add")             # manually compile a function
        result = jit.execute("add", [3, 4])  # run compiled or interpreted
    """

    def __init__(self, vm: TetradVM) -> None:
        self._vm = vm
        self._cache = JITCache()
        self._main_code: CodeObject | None = None

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _find_fn(self, fn_name: str) -> CodeObject | None:
        """Locate a function CodeObject by name in the loaded program."""
        if self._main_code is None:
            return None
        for fn in self._main_code.functions:
            if fn.name == fn_name:
                return fn
        return None

    def _compile_fn(self, fn: CodeObject) -> bool:
        """Translate, optimise, and codegen one function.

        Returns True on success, False on deopt.
        """
        t0 = JITCache.now_ns()
        try:
            ir: list[IRInstr] = translate(fn)
            ir = optimize(ir)
            binary = codegen(ir)
        except Exception:
            return False

        if binary is None:
            return False

        t1 = JITCache.now_ns()
        self._cache.put(JITCacheEntry(
            fn_name=fn.name,
            binary=binary,
            param_count=len(fn.params),
            ir=ir,
            compilation_time_ns=t1 - t0,
        ))
        return True

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def compile(self, fn_name: str) -> bool:
        """Compile *fn_name* from the loaded program to Intel 4004 binary.

        Returns ``True`` if compilation succeeded and the function is now
        cached.  Returns ``False`` if the function was not found, or if any
        IR instruction could not be encoded (deopt).

        The main CodeObject must first be loaded via ``execute_with_jit``
        (or by assigning ``jit._main_code`` directly in tests).
        """
        fn = self._find_fn(fn_name)
        if fn is None:
            return False
        return self._compile_fn(fn)

    def is_compiled(self, fn_name: str) -> bool:
        """Return ``True`` if *fn_name* has a cached 4004 binary."""
        return fn_name in self._cache

    def _promote_hot_functions(self) -> None:
        """Compile any uncompiled function whose interpreter call count has
        reached the tier threshold (PARTIALLY_TYPED=10, UNTYPED=100).

        Called after each interpreter run so that the *next* call to a newly
        hot function routes to the 4004 binary instead of the interpreter.
        """
        if self._main_code is None:
            return
        counts = self._vm.metrics().function_call_counts
        for fn in self._main_code.functions:
            if self.is_compiled(fn.name):
                continue
            threshold = _THRESHOLDS.get(fn.type_status)
            if threshold is None or threshold == 0:
                continue
            if counts.get(fn.name, 0) >= threshold:
                self._compile_fn(fn)

    def execute(self, fn_name: str, args: list[int]) -> int:
        """Execute *fn_name* with the given u8 arguments.

        If *fn_name* is compiled, the binary runs on ``Intel4004Simulator``.
        Otherwise the function is interpreted, call counts are updated, and any
        function that crosses its tier threshold is compiled immediately so that
        the *next* call uses the 4004 binary.

        Parameters
        ----------
        fn_name:
            Function name (must be in the loaded CodeObject's functions list).
        args:
            List of u8 argument values.  Extra args are silently ignored;
            missing args default to 0.

        Returns
        -------
        int:
            The u8 return value (0–255).
        """
        entry = self._cache.get(fn_name)
        if entry is not None:
            return run_on_4004(entry.binary, args)

        # Interpreter fallback: build a tiny call sequence and run it.
        fn = self._find_fn(fn_name)
        if fn is None:
            raise ValueError(f"function '{fn_name}' not found in loaded program")

        # Build a tiny synthetic CodeObject that pre-loads args into registers
        # and calls the function, so vm.execute() returns the function result.
        from tetrad_compiler.bytecode import Instruction, Op  # noqa: PLC0415
        fn_idx = next(
            i for i, f in enumerate(self._main_code.functions) if f.name == fn_name
        )
        instrs: list[Instruction] = []
        for i, arg in enumerate(args[: len(fn.params)]):
            instrs.append(Instruction(Op.LDA_IMM, [arg & 0xFF]))
            instrs.append(Instruction(Op.STA_REG, [i]))
        instrs.append(Instruction(Op.CALL, [fn_idx, len(fn.params), 0]))
        instrs.append(Instruction(Op.RET, []))
        synthetic = CodeObject(
            name="__call__",
            params=[],
            instructions=instrs,
            functions=self._main_code.functions,
            register_count=max(len(fn.params), 1),
        )
        result = self._vm.execute(synthetic)
        # After interpreting, promote any function that crossed its threshold.
        self._promote_hot_functions()
        return result

    def execute_with_jit(self, code: CodeObject) -> int:
        """Run *code* under the interpreter, auto-compiling hot functions.

        **Phase 1** — compile all ``FULLY_TYPED`` functions *before* the
        first interpreted instruction.

        **Phase 2** — find and execute the ``main`` function via the
        interpreter.  The top-level CodeObject's instruction list is just a
        HALT placeholder; actual execution begins at ``fn main()``.

        Returns the return value of ``main``, or 0 if no main function exists.
        """
        self._main_code = code

        # Phase 1: immediate compilation for FULLY_TYPED functions.
        for fn in code.functions:
            if fn.type_status is FunctionTypeStatus.FULLY_TYPED:
                self._compile_fn(fn)

        # Phase 2: find main and run it through the interpreter.
        # Build a synthetic CodeObject that runs main's instructions with the
        # full function list available so CALL instructions can resolve targets.
        main_fn = self._find_fn("main")
        if main_fn is None:
            return self._vm.execute(code)

        synthetic = CodeObject(
            name="__main__",
            params=[],
            instructions=main_fn.instructions,
            functions=code.functions,
            register_count=main_fn.register_count,
        )
        result = self._vm.execute(synthetic)
        # Phase 3: promote any function that turned hot during main's run.
        self._promote_hot_functions()
        return result

    def cache_stats(self) -> dict[str, dict]:
        """Return per-function cache statistics."""
        return self._cache.stats()

    def dump_ir(self, fn_name: str) -> str:
        """Return the post-optimization IR for *fn_name* as a human-readable string."""
        entry = self._cache.get(fn_name)
        if entry is None:
            return f"<not compiled: {fn_name!r}>"
        lines = [repr(instr) for instr in entry.ir]
        return "\n".join(lines)
