"""Tests for JITCore tier-based compilation thresholds.

Three tiers:
  FULLY_TYPED     → compiled eagerly before the first interpreted call
  PARTIALLY_TYPED → compiled after threshold_partial interpreted calls
  UNTYPED         → compiled after threshold_untyped interpreted calls
"""

from __future__ import annotations

from typing import Any

import pytest
from conftest import MockBackend, make_fn, make_instr, make_mod
from interpreter_ir.function import FunctionTypeStatus
from vm_core import VMMetrics

from jit_core.core import JITCore
from jit_core.errors import UnspecializableError

# ---------------------------------------------------------------------------
# Mock VM
# ---------------------------------------------------------------------------

class MockVM:
    """Minimal VMCore stand-in for unit tests.

    ``call_counts`` is pre-populated to simulate what the interpreter has
    observed.  ``execute()`` returns ``return_value`` without actually
    running any IIR instructions.
    """

    def __init__(self, return_value: Any = None, call_counts: dict | None = None) -> None:
        self._return_value = return_value
        self._call_counts: dict[str, int] = call_counts or {}
        self.executed: list[tuple[str, list]] = []
        self.registered_handlers: dict[str, Any] = {}
        self.unregistered: list[str] = []

    def execute(self, module, *, fn: str = "main", args=None) -> Any:
        self.executed.append((fn, args or []))
        return self._return_value

    def metrics(self) -> VMMetrics:
        return VMMetrics(function_call_counts=dict(self._call_counts))

    def register_jit_handler(self, fn_name: str, handler: Any) -> None:
        self.registered_handlers[fn_name] = handler

    def unregister_jit_handler(self, fn_name: str) -> None:
        self.unregistered.append(fn_name)
        self.registered_handlers.pop(fn_name, None)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _typed_fn(name: str, status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED):
    return make_fn(
        name,
        [("a", "u8")],
        make_instr("ret", srcs=["a"], type_hint="u8"),
        type_status=status,
    )


def _jit(backend: MockBackend, vm: MockVM, **kwargs) -> JITCore:
    return JITCore(vm=vm, backend=backend, **kwargs)


# ---------------------------------------------------------------------------
# FULLY_TYPED — eager compilation
# ---------------------------------------------------------------------------

class TestFullyTypedTier:
    def test_fully_typed_compiled_before_execute(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("f")

    def test_fully_typed_backend_received_compile_call(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)

        assert len(backend.compile_calls) == 1

    def test_fully_typed_jit_handler_registered(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)

        assert "f" in vm.registered_handlers

    def test_fully_typed_not_recompiled_on_second_execute_with_jit(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)
        jit.execute_with_jit(mod)

        # Should only compile once
        assert len(backend.compile_calls) == 1

    def test_multiple_fully_typed_compiled_eagerly(self):
        fns = [_typed_fn(n, FunctionTypeStatus.FULLY_TYPED) for n in ("f", "g", "h")]
        mod = make_mod(*fns)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)

        assert all(jit.is_compiled(n) for n in ("f", "g", "h"))
        assert len(backend.compile_calls) == 3


# ---------------------------------------------------------------------------
# PARTIALLY_TYPED — compile after threshold_partial calls
# ---------------------------------------------------------------------------

class TestPartiallyTypedTier:
    def test_below_threshold_not_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.PARTIALLY_TYPED)
        mod = make_mod(fn)
        # 5 calls, threshold=10 → not hot enough
        vm = MockVM(call_counts={"f": 5})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_partial=10)

        jit.execute_with_jit(mod)

        assert not jit.is_compiled("f")

    def test_at_threshold_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.PARTIALLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 10})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_partial=10)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("f")

    def test_above_threshold_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.PARTIALLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 50})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_partial=10)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("f")

    def test_zero_call_count_not_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.PARTIALLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_partial=1)

        jit.execute_with_jit(mod)

        assert not jit.is_compiled("f")


# ---------------------------------------------------------------------------
# UNTYPED — compile after threshold_untyped calls
# ---------------------------------------------------------------------------

class TestUntypedTier:
    def test_below_threshold_not_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.UNTYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 50})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_untyped=100)

        jit.execute_with_jit(mod)

        assert not jit.is_compiled("f")

    def test_at_threshold_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.UNTYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 100})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_untyped=100)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("f")

    def test_custom_threshold(self):
        fn = _typed_fn("f", FunctionTypeStatus.UNTYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 3})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_untyped=3)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("f")


# ---------------------------------------------------------------------------
# Mixed tiers in the same module
# ---------------------------------------------------------------------------

class TestMixedTiers:
    def test_fully_typed_and_untyped_hot(self):
        fully = _typed_fn("ft", FunctionTypeStatus.FULLY_TYPED)
        hot = _typed_fn("hot", FunctionTypeStatus.UNTYPED)
        cold = _typed_fn("cold", FunctionTypeStatus.UNTYPED)
        mod = make_mod(fully, hot, cold)
        vm = MockVM(call_counts={"ft": 0, "hot": 200, "cold": 5})
        backend = MockBackend()
        jit = _jit(backend, vm, threshold_untyped=100)

        jit.execute_with_jit(mod)

        assert jit.is_compiled("ft")
        assert jit.is_compiled("hot")
        assert not jit.is_compiled("cold")


# ---------------------------------------------------------------------------
# JITCore.compile — manual compilation
# ---------------------------------------------------------------------------

class TestManualCompile:
    def test_compile_returns_true_on_success(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        assert jit.compile("f") is True

    def test_compile_returns_false_when_no_module(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)

        assert jit.compile("f") is False

    def test_compile_returns_false_when_fn_missing(self):
        mod = make_mod(_typed_fn("g"))
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        assert jit.compile("f") is False

    def test_compile_raises_for_unspecializable(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod
        jit._unspecializable.add("f")

        with pytest.raises(UnspecializableError):
            jit.compile("f")

    def test_compile_returns_false_when_backend_returns_none(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        backend.fail_next_compile()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        assert jit.compile("f") is False
        assert not jit.is_compiled("f")


# ---------------------------------------------------------------------------
# JITCore.execute — JIT dispatch vs interpreter fallback
# ---------------------------------------------------------------------------

class TestExecuteDispatch:
    def test_execute_uses_jit_when_compiled(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM(return_value=99)
        backend = MockBackend(return_value=42)
        jit = _jit(backend, vm)

        jit.execute_with_jit(mod)
        result = jit.execute("f", [])

        assert result == 42  # JIT result, not vm result

    def test_execute_falls_back_to_vm_when_not_compiled(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM(return_value=7)
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        result = jit.execute("f", [])

        assert result == 7

    def test_execute_returns_none_when_no_module(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)

        assert jit.execute("f") is None

    def test_execute_increments_exec_count(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = _jit(backend, vm)
        jit.execute_with_jit(mod)

        jit.execute("f", [])
        jit.execute("f", [])

        stats = jit.cache_stats()
        assert stats["f"]["exec_count"] == 2


# ---------------------------------------------------------------------------
# is_compiled and cache_stats
# ---------------------------------------------------------------------------

class TestCacheStats:
    def test_is_compiled_false_before_compile(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        assert not jit.is_compiled("f")

    def test_is_compiled_true_after_compile(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod
        jit.compile("f")

        assert jit.is_compiled("f")

    def test_cache_stats_empty_initially(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        assert jit.cache_stats() == {}

    def test_cache_stats_after_compile(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod
        jit.compile("f")

        stats = jit.cache_stats()
        assert "f" in stats
        assert stats["f"]["backend"] == "mock"

    def test_dump_ir_empty_before_compile(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        assert jit.dump_ir("f") == ""

    def test_dump_ir_after_compile(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod
        jit.compile("f")

        ir_str = jit.dump_ir("f")
        assert isinstance(ir_str, str)
        assert len(ir_str) > 0


# ---------------------------------------------------------------------------
# Internal edge-case coverage
# ---------------------------------------------------------------------------

class RaisingBackend:
    """Backend that raises during compile (tests exception handling path)."""
    name = "raising"
    def compile(self, cir):  # noqa: ANN001
        raise RuntimeError("deliberate compile error")
    def run(self, binary, args):  # noqa: ANN001
        return None


class TestInternalCoverage:
    def test_promote_hot_functions_no_module(self):
        # _promote_hot_functions early-returns when _module is None
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        # _module is None by default; calling this should not raise
        jit._promote_hot_functions()

    def test_promote_skips_unspecializable_fn(self):
        fn = _typed_fn("f", FunctionTypeStatus.UNTYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 999})
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend, threshold_untyped=1)
        jit._module = mod
        jit._unspecializable.add("f")

        jit._promote_hot_functions()

        assert not jit.is_compiled("f")

    def test_promote_skips_fn_with_no_threshold_entry(self):
        fn = _typed_fn("f", FunctionTypeStatus.PARTIALLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM(call_counts={"f": 999})
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod
        # Remove the PARTIALLY_TYPED threshold so it returns None
        del jit._thresholds[FunctionTypeStatus.PARTIALLY_TYPED]

        jit._promote_hot_functions()

        assert not jit.is_compiled("f")

    def test_compile_fn_returns_false_on_exception(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = RaisingBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        result = jit.compile("f")

        assert result is False

    def test_jit_handler_callable_directly(self):
        # The handler closure registered with the VM should be callable
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend(return_value=77)
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod)

        handler = vm.registered_handlers["f"]
        result = handler([1, 2])

        assert result == 77

    def test_jit_handler_increments_exec_count(self):
        fn = _typed_fn("f", FunctionTypeStatus.FULLY_TYPED)
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod)

        handler = vm.registered_handlers["f"]
        handler([])
        handler([])

        entry = jit._cache.get("f")
        assert entry.exec_count == 2
