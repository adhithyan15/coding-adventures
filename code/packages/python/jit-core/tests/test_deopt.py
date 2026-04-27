"""Tests for deoptimization tracking and invalidation in JITCore."""

from __future__ import annotations

from conftest import MockBackend, make_fn, make_instr, make_mod
from interpreter_ir.function import FunctionTypeStatus
from test_tiers import MockVM

from jit_core.core import JITCore

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _typed_fn(name: str, status: FunctionTypeStatus = FunctionTypeStatus.FULLY_TYPED):
    return make_fn(
        name,
        [("a", "u8")],
        make_instr("ret", srcs=["a"], type_hint="u8"),
        type_status=status,
    )


def _build(fn_name: str = "f") -> tuple[JITCore, MockVM, MockBackend]:
    fn = _typed_fn(fn_name)
    mod = make_mod(fn)
    vm = MockVM()
    backend = MockBackend()
    jit = JITCore(vm=vm, backend=backend)
    jit.execute_with_jit(mod)
    return jit, vm, backend


# ---------------------------------------------------------------------------
# record_deopt — counter bookkeeping
# ---------------------------------------------------------------------------

class TestRecordDeopt:
    def test_record_deopt_increments_count(self):
        jit, _, _ = _build()
        assert jit.cache_stats()["f"]["deopt_count"] == 0
        jit.record_deopt("f")
        assert jit.cache_stats()["f"]["deopt_count"] == 1

    def test_record_deopt_multiple_times(self):
        jit, _, _ = _build()
        for _ in range(5):
            jit.record_deopt("f")
        assert jit.cache_stats()["f"]["deopt_count"] == 5

    def test_record_deopt_unknown_fn_is_safe(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        # No module loaded; no-op
        jit.record_deopt("ghost")  # must not raise


# ---------------------------------------------------------------------------
# Deopt rate threshold → invalidation
# ---------------------------------------------------------------------------

class TestDeoptRateInvalidation:
    def test_high_deopt_rate_invalidates_fn(self):
        jit, _, _ = _build()

        # Simulate 10 executions with 2 deopts (rate=0.2 > 0.1)
        entry = jit._cache.get("f")
        entry.exec_count = 10
        entry.deopt_count = 2

        # Trigger rate check
        jit.record_deopt("f")  # deopt_count → 3, rate=3/10=0.3

        assert not jit.is_compiled("f")
        assert "f" in jit._unspecializable

    def test_deopt_rate_at_limit_does_not_invalidate(self):
        jit, _, _ = _build()
        entry = jit._cache.get("f")
        # Exactly 0.1: 1 deopt out of 10 execs = 0.1 (not > 0.1)
        entry.exec_count = 10
        entry.deopt_count = 1
        # Don't call record_deopt yet — rate is exactly 0.1
        # Manually trigger check
        jit._check_deopt_rate(entry)

        assert jit.is_compiled("f")

    def test_deopt_rate_just_above_limit_invalidates(self):
        jit, _, _ = _build()
        entry = jit._cache.get("f")
        entry.exec_count = 10
        entry.deopt_count = 2  # rate = 0.2 > 0.1
        jit._check_deopt_rate(entry)

        assert not jit.is_compiled("f")

    def test_invalidated_fn_marked_unspecializable(self):
        jit, _, _ = _build()
        entry = jit._cache.get("f")
        entry.exec_count = 5
        entry.deopt_count = 2
        jit._check_deopt_rate(entry)

        assert "f" in jit._unspecializable

    def test_jit_handler_unregistered_after_invalidation(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod)

        assert "f" in vm.registered_handlers

        entry = jit._cache.get("f")
        entry.exec_count = 5
        entry.deopt_count = 2
        jit._check_deopt_rate(entry)

        assert "f" not in vm.registered_handlers
        assert "f" in vm.unregistered


# ---------------------------------------------------------------------------
# invalidate() — manual invalidation
# ---------------------------------------------------------------------------

class TestManualInvalidate:
    def test_invalidate_removes_compiled_fn(self):
        jit, _, _ = _build()
        assert jit.is_compiled("f")
        jit.invalidate("f")
        assert not jit.is_compiled("f")

    def test_invalidate_marks_unspecializable(self):
        jit, _, _ = _build()
        jit.invalidate("f")
        assert "f" in jit._unspecializable

    def test_invalidate_calls_unregister_on_vm(self):
        fn = _typed_fn("f")
        mod = make_mod(fn)
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod)

        jit.invalidate("f")

        assert "f" in vm.unregistered

    def test_invalidate_nonexistent_is_safe(self):
        vm = MockVM()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.invalidate("ghost")  # must not raise


# ---------------------------------------------------------------------------
# execute() — deopt tracking through the JIT handler
# ---------------------------------------------------------------------------

class TestExecuteDeoptTracking:
    def test_execute_increments_exec_count_on_jit_call(self):
        jit, _, backend = _build()
        jit.execute("f", [1])
        jit.execute("f", [2])
        stats = jit.cache_stats()["f"]
        assert stats["exec_count"] == 2

    def test_execute_triggers_invalidation_on_high_deopt_rate(self):
        jit, _, _ = _build()
        # Pre-load deopt stats so next execute tips over
        entry = jit._cache.get("f")
        entry.exec_count = 10
        entry.deopt_count = 2  # rate=0.2 > 0.1

        # execute() checks deopt rate after bumping exec_count
        jit.execute("f", [])
        # deopt_rate = 2/11 ≈ 0.18 — still > 0.1 → invalidated
        assert not jit.is_compiled("f")
