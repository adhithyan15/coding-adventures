"""End-to-end integration tests for jit-core with a real VMCore.

These tests run actual IIR programs through the full pipeline:

    IIRFunction → specialise() → optimizer.run() → backend.compile()
                                                  → backend.run()

The SummingBackend is used for tests that want a predictable JIT return
value without implementing a real code generator.  A PassthroughBackend
is used to verify that dump_ir() and cache_stats() reflect real CIR.
"""

from __future__ import annotations

from conftest import MockBackend, SummingBackend, make_fn, make_instr, make_mod
from interpreter_ir import IIRFunction
from interpreter_ir.function import FunctionTypeStatus
from vm_core import VMCore

from jit_core import optimizer, specialise
from jit_core.core import JITCore

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _real_vm(**kwargs) -> VMCore:
    return VMCore(**kwargs)


def _identity_fn(name: str = "identity") -> IIRFunction:
    """f(x) → x  — minimal one-param function."""
    return make_fn(
        name,
        [("x", "u8")],
        make_instr("ret", srcs=["x"], type_hint="u8"),
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )


def _add_one_fn(name: str = "add_one") -> IIRFunction:
    """f(x) → x + 1  — arithmetic + return."""
    return make_fn(
        name,
        [("x", "u8")],
        make_instr("const", "one", [1], type_hint="u8"),
        make_instr("add", "result", ["x", "one"], type_hint="u8"),
        make_instr("ret", srcs=["result"], type_hint="u8"),
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )


def _const_fn(value: int, name: str = "const_fn") -> IIRFunction:
    """f() → constant value."""
    return make_fn(
        name,
        [],
        make_instr("const", "v", [value], type_hint="u8"),
        make_instr("ret", srcs=["v"], type_hint="u8"),
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )


# ---------------------------------------------------------------------------
# CIR pipeline — specialise + optimizer
# ---------------------------------------------------------------------------

class TestCIRPipeline:
    def test_specialise_identity(self):
        fn = _identity_fn()
        cir = specialise(fn)
        assert len(cir) >= 1
        assert any(c.op == "ret_u8" for c in cir)

    def test_specialise_add_one(self):
        fn = _add_one_fn()
        cir = specialise(fn)
        assert any(c.op == "const_u8" for c in cir)
        assert any(c.op == "add_u8" for c in cir)
        assert any(c.op == "ret_u8" for c in cir)

    def test_optimizer_folds_constants(self):
        """Two literal srcs → folded to a single const."""
        fn = make_fn(
            "fold",
            [],
            make_instr("add", "v", [3, 4], type_hint="u8"),
            make_instr("ret", srcs=["v"], type_hint="u8"),
        )
        cir = specialise(fn)
        cir = optimizer.run(cir)
        # The add_u8 with two literal srcs should be folded.
        ops = [c.op for c in cir]
        assert "add_u8" not in ops
        # Result is const_u8 with value 7
        const_instrs = [c for c in cir if c.op == "const_u8"]
        assert any(c.srcs == [7] for c in const_instrs)

    def test_optimizer_dce_removes_unused_const(self):
        """A const whose dest is never read should be eliminated."""
        fn = make_fn(
            "dead",
            [],
            make_instr("const", "dead_v", [99], type_hint="u8"),
            make_instr("const", "used_v", [1], type_hint="u8"),
            make_instr("ret", srcs=["used_v"], type_hint="u8"),
        )
        cir = specialise(fn)
        cir = optimizer.run(cir)
        dests = [c.dest for c in cir]
        assert "dead_v" not in dests


# ---------------------------------------------------------------------------
# VMCore — interpreter-only execution
# ---------------------------------------------------------------------------

class TestVMCoreInterpreter:
    def test_identity_returns_arg(self):
        fn = make_fn("f", [("x", "u8")], make_instr("ret", srcs=["x"], type_hint="u8"))
        mod = make_mod(fn)
        vm = _real_vm()
        result = vm.execute(mod, fn="f", args=[42])
        assert result == 42

    def test_add_one_returns_incremented(self):
        fn = _add_one_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        result = vm.execute(mod, fn="add_one", args=[5])
        assert result == 6

    def test_const_fn_returns_constant(self):
        fn = _const_fn(77)
        mod = make_mod(fn)
        vm = _real_vm()
        result = vm.execute(mod, fn="const_fn", args=[])
        assert result == 77

    def test_vm_metrics_track_calls(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        vm.execute(mod, fn="identity", args=[1])
        vm.execute(mod, fn="identity", args=[2])
        counts = vm.metrics().function_call_counts
        assert counts.get("identity", 0) >= 1


# ---------------------------------------------------------------------------
# JITCore.execute_with_jit — full pipeline
# ---------------------------------------------------------------------------

class TestExecuteWithJIT:
    def test_fully_typed_compiled_before_vm_execute(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend(return_value=99)
        jit = JITCore(vm=vm, backend=backend)

        jit.execute_with_jit(mod, fn="identity", args=[1])

        assert jit.is_compiled("identity")

    def test_jit_backend_compile_called(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)

        jit.execute_with_jit(mod, fn="identity", args=[1])

        assert len(backend.compile_calls) == 1

    def test_execute_after_jit_uses_backend(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend(return_value=42)
        jit = JITCore(vm=vm, backend=backend)

        jit.execute_with_jit(mod, fn="identity", args=[1])
        result = jit.execute("identity", [5])

        assert result == 42  # backend answer, not interpreter
        assert len(backend.run_calls) >= 1

    def test_untyped_fn_not_compiled_when_cold(self):
        fn = make_fn(
            "cold",
            [("x", "u8")],
            make_instr("ret", srcs=["x"], type_hint="u8"),
            type_status=FunctionTypeStatus.UNTYPED,
        )
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend, threshold_untyped=100)

        jit.execute_with_jit(mod, fn="cold", args=[1])

        assert not jit.is_compiled("cold")

    def test_summing_backend_end_to_end(self):
        """Two-param function: JIT returns sum of args."""
        fn = make_fn(
            "add",
            [("a", "u8"), ("b", "u8")],
            make_instr("add", "result", ["a", "b"], type_hint="u8"),
            make_instr("ret", srcs=["result"], type_hint="u8"),
            type_status=FunctionTypeStatus.FULLY_TYPED,
        )
        mod = make_mod(fn)
        vm = _real_vm()
        backend = SummingBackend()
        jit = JITCore(vm=vm, backend=backend)

        jit.execute_with_jit(mod, fn="add")
        result = jit.execute("add", [3, 4])

        assert result == 7  # SummingBackend returns sum(args)


# ---------------------------------------------------------------------------
# dump_ir and cache_stats reflect real CIR
# ---------------------------------------------------------------------------

class TestDumpIRAndStats:
    def test_dump_ir_contains_ret(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod, fn="identity")

        ir_str = jit.dump_ir("identity")
        assert "ret" in ir_str

    def test_cache_stats_backend_name(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod, fn="identity")

        stats = jit.cache_stats()
        assert stats["identity"]["backend"] == "mock"

    def test_cache_stats_compilation_time_positive(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod, fn="identity")

        stats = jit.cache_stats()
        assert stats["identity"]["compilation_time_ns"] >= 0

    def test_cache_stats_param_count(self):
        fn = make_fn(
            "three_params",
            [("a", "u8"), ("b", "u8"), ("c", "u8")],
            make_instr("ret_void"),
            type_status=FunctionTypeStatus.FULLY_TYPED,
        )
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend)
        jit.execute_with_jit(mod, fn="three_params")

        stats = jit.cache_stats()
        assert stats["three_params"]["param_count"] == 3


# ---------------------------------------------------------------------------
# Backend failure handling
# ---------------------------------------------------------------------------

class TestBackendFailure:
    def test_backend_compile_failure_returns_false(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        backend.fail_next_compile()
        jit = JITCore(vm=vm, backend=backend)
        jit._module = mod

        ok = jit.compile("identity")

        assert ok is False
        assert not jit.is_compiled("identity")

    def test_execution_falls_back_to_vm_on_backend_failure(self):
        fn = _identity_fn()
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        backend.fail_next_compile()
        jit = JITCore(vm=vm, backend=backend)

        # FULLY_TYPED tries to compile eagerly, but backend fails.
        result = jit.execute_with_jit(mod, fn="identity", args=[7])

        # VM interpreted result — identity returns its arg
        assert result == 7
        assert not jit.is_compiled("identity")


# ---------------------------------------------------------------------------
# Promote hot functions after execute_with_jit
# ---------------------------------------------------------------------------

class TestPromoteHotAfterExecution:
    def test_untyped_hot_fn_promoted_after_execution(self):
        """If an UNTYPED fn's call count crosses the threshold during VM
        execution, it is promoted to compiled in Phase 3."""
        fn = make_fn(
            "hot",
            [("x", "u8")],
            make_instr("ret", srcs=["x"], type_hint="u8"),
            type_status=FunctionTypeStatus.UNTYPED,
        )
        mod = make_mod(fn)
        vm = _real_vm()
        backend = MockBackend()
        jit = JITCore(vm=vm, backend=backend, threshold_untyped=1)

        # VM must execute 'hot' at least once for the call count to register.
        # We use "hot" as the entry-point so VM.execute increments its counter.
        jit.execute_with_jit(mod, fn="hot", args=[0])

        # After Phase 3 promotion, hot should now be compiled.
        assert jit.is_compiled("hot")
