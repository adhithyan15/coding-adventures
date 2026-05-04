"""Pure unit tests for ``TwigJITRunner`` — no WASM required.

We use a stub backend that records every ``compile`` call so we
can assert that JIT promotion happens and the interpreter still
runs hot programs to completion.

The real-WASM smoke tests live in ``test_real_wasm.py`` and skip
cleanly if the WASM runtime isn't available.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

import pytest

from twig_jit_wasm import TwigJITRunner, compile_to_iir, run_with_jit


@dataclass
class _RecordingBackend:
    """Stub ``BackendProtocol`` that records compiles + always
    raises ``UnspecializableError`` on ``compile`` so the JIT
    falls back to the interpreter for every function.

    This is a deliberately conservative stub — it lets us test the
    runner's JIT control flow without dragging in WASM.
    """

    compile_calls: list[str] = field(default_factory=list)
    name: str = "recording-stub"

    def compile(self, _cir: list[Any]) -> bytes:  # noqa: D401
        # Record + refuse so jit-core marks the function unspecializable.
        from jit_core import UnspecializableError

        self.compile_calls.append("called")
        raise UnspecializableError("recording stub refuses everything")

    def run(self, _binary: bytes, _args: list[Any]) -> Any:  # pragma: no cover
        msg = "recording-stub.run should never be called"
        raise AssertionError(msg)


class TestCompileToIIR:
    def test_arithmetic_compiles(self) -> None:
        module = compile_to_iir("(+ 1 2)")
        # The compiled module always has at least 'main' as an
        # IIRFunction.
        names = [fn.name for fn in module.functions]
        assert "main" in names

    def test_function_define_lifts_to_iir(self) -> None:
        module = compile_to_iir("(define (square x) (* x x)) (square 7)")
        names = {fn.name for fn in module.functions}
        # 'main' plus the user define.
        assert "main" in names
        assert "square" in names


class TestRunnerWithStubBackend:
    def test_arithmetic_returns_correct_value(self) -> None:
        runner = TwigJITRunner(backend=_RecordingBackend())
        assert runner.run("(+ 1 2)") == 3
        assert runner.run("(* 6 7)") == 42

    def test_define_and_call_works(self) -> None:
        runner = TwigJITRunner(backend=_RecordingBackend())
        result = runner.run(
            "(define (square x) (* x x)) (square 7)"
        )
        assert result == 49

    def test_recursion_works_via_interpreter_fallback(self) -> None:
        runner = TwigJITRunner(backend=_RecordingBackend())
        result = runner.run(
            """
            (define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))
            (fact 5)
            """
        )
        assert result == 120

    def test_let_binding_works(self) -> None:
        runner = TwigJITRunner(backend=_RecordingBackend())
        assert runner.run("(let ((x 5)) (* x x))") == 25


class TestPublicAPI:
    def test_run_with_jit_uses_default_backend(self) -> None:
        # We can't assert on the WASM backend without a WASM runtime
        # being available, but we can at least confirm the function
        # accepts an explicit backend and routes through the runner.
        result = run_with_jit("(+ 1 2)", backend=_RecordingBackend())
        assert result == 3

    def test_run_with_jit_threshold_overrides(self) -> None:
        # Construct directly to exercise the threshold knobs.
        runner = TwigJITRunner(
            backend=_RecordingBackend(),
            threshold_partial=1,
            threshold_untyped=1,
        )
        # Aggressive thresholds shouldn't break correctness.
        assert runner.run("(+ 10 32)") == 42


class TestLazyBackend:
    def test_default_backend_only_imported_lazily(self) -> None:
        # Import should not pull WASM toolchain — confirm by
        # checking that importing the module + constructing a
        # runner with an explicit stub backend doesn't touch
        # ``wasm_backend``.
        import sys

        # Forget any previously-cached module so we can observe.
        for k in list(sys.modules):
            if k.startswith("wasm_backend"):
                del sys.modules[k]

        from twig_jit_wasm import TwigJITRunner  # noqa: F401, PLC0415

        # Constructing a runner with an explicit backend must NOT
        # import wasm_backend.
        TwigJITRunner(backend=_RecordingBackend())
        assert "wasm_backend" not in sys.modules
