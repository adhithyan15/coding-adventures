"""Tests for aot_core.core — AOTCore compilation controller."""

from __future__ import annotations

import json
import os
import tempfile

from conftest import MockAOTBackend, make_fn, make_instr, make_mod
from interpreter_ir.function import FunctionTypeStatus

from aot_core.core import AOTCore
from aot_core.snapshot import read as read_snap
from aot_core.stats import AOTStats
from aot_core.vm_runtime import VmRuntime

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _fully_typed_fn(name: str = "main"):
    return make_fn(
        name,
        [("x", "u8")],
        make_instr("const", "one", [1], type_hint="u8"),
        make_instr("add", "r", ["x", "one"], type_hint="u8"),
        make_instr("ret", srcs=["r"], type_hint="u8"),
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )


def _untyped_fn(name: str = "dynamic"):
    return make_fn(
        name,
        [("x", "any")],
        make_instr("ret", srcs=["x"]),
        type_status=FunctionTypeStatus.UNTYPED,
    )


# ---------------------------------------------------------------------------
# Basic compile() → .aot binary
# ---------------------------------------------------------------------------

class TestAOTCoreCompile:
    def test_returns_bytes(self):
        fn = _fully_typed_fn()
        mod = make_mod(fn)
        aot = AOTCore(backend=MockAOTBackend())
        result = aot.compile(mod)
        assert isinstance(result, bytes)

    def test_binary_has_valid_magic(self):
        mod = make_mod(_fully_typed_fn())
        aot = AOTCore(backend=MockAOTBackend())
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.native_code == b"\xde\xad"  # MockAOTBackend default

    def test_empty_module(self):
        mod = make_mod()
        aot = AOTCore(backend=MockAOTBackend())
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.native_code == b""
        assert snap.iir_table is None

    def test_backend_compile_called_per_fn(self):
        backend = MockAOTBackend()
        mod = make_mod(_fully_typed_fn("f1"), _fully_typed_fn("f2"))
        AOTCore(backend=backend).compile(mod)
        assert len(backend.compile_calls) == 2

    def test_two_fns_code_concatenated(self):
        backend = MockAOTBackend(return_value=b"\xAA\xBB")
        mod = make_mod(_fully_typed_fn("f1"), _fully_typed_fn("f2"))
        raw = AOTCore(backend=backend).compile(mod)
        snap = read_snap(raw)
        assert snap.native_code == b"\xAA\xBB\xAA\xBB"

    def test_entry_point_offset_is_main(self):
        backend = MockAOTBackend(return_value=b"\x01\x02")
        mod = make_mod(_fully_typed_fn("helper"), _fully_typed_fn("main"))
        raw = AOTCore(backend=backend).compile(mod)
        snap = read_snap(raw)
        # main is second fn, offset = len(b"\x01\x02") = 2
        assert snap.entry_point_offset == 2


# ---------------------------------------------------------------------------
# Untyped functions → IIR table
# ---------------------------------------------------------------------------

class TestAOTCoreUntypedFns:
    def test_untyped_fn_goes_to_iir_table(self):
        # A backend that returns None (can't compile generic CIR) routes the
        # function to the IIR table instead of the code section.
        mod = make_mod(_untyped_fn())
        aot = AOTCore(backend=MockAOTBackend(fail=True))
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.has_vm_runtime
        assert snap.iir_table is not None

    def test_iir_table_contains_fn_name(self):
        mod = make_mod(_untyped_fn("dynamic_fn"))
        # The backend returns None for untyped (can't compile "any" types).
        backend = MockAOTBackend(fail=True)
        aot = AOTCore(backend=backend)
        raw = aot.compile(mod)
        snap = read_snap(raw)
        records = json.loads(snap.iir_table.decode())
        names = [r["name"] for r in records]
        assert "dynamic_fn" in names

    def test_vm_runtime_library_appended_when_provided(self):
        mod = make_mod(_untyped_fn())
        backend = MockAOTBackend(fail=True)
        rt = VmRuntime(library_bytes=b"\xFF\xFE\xFD")
        aot = AOTCore(backend=backend, vm_runtime=rt)
        raw = aot.compile(mod)
        # vm-runtime bytes should appear somewhere in the binary.
        assert b"\xFF\xFE\xFD" in raw

    def test_mixed_module_partial_compile(self):
        # One typed fn + one untyped fn.
        typed = _fully_typed_fn("compiled")
        untyped = _untyped_fn("dynamic")
        mod = make_mod(typed, untyped)
        # Backend succeeds for typed, fails for untyped.
        call_count = [0]
        class SelectiveBackend:
            name = "selective"
            def compile(self, cir):
                call_count[0] += 1
                # First call compiles typed fn; second fails (untyped → any).
                if call_count[0] == 1:
                    return b"\xCC"
                return None
            def run(self, binary, args):
                return None

        aot = AOTCore(backend=SelectiveBackend())
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.native_code == b"\xCC"
        assert snap.has_vm_runtime


# ---------------------------------------------------------------------------
# Backend compile failure
# ---------------------------------------------------------------------------

class TestAOTCoreBackendFailure:
    def test_backend_returns_none_routes_to_iir(self):
        fn = _fully_typed_fn()
        mod = make_mod(fn)
        backend = MockAOTBackend(fail=True)
        aot = AOTCore(backend=backend)
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.has_vm_runtime
        assert snap.native_code == b""

    def test_backend_exception_routes_to_iir(self):
        fn = _fully_typed_fn()
        mod = make_mod(fn)

        class ExplodingBackend:
            name = "exploding"
            def compile(self, cir):
                raise RuntimeError("kaboom")
            def run(self, binary, args):
                return None

        aot = AOTCore(backend=ExplodingBackend())
        raw = aot.compile(mod)
        snap = read_snap(raw)
        assert snap.has_vm_runtime


# ---------------------------------------------------------------------------
# Optimization levels
# ---------------------------------------------------------------------------

class TestAOTCoreOptimization:
    def test_optimization_level_0_no_folding(self):
        fn = make_fn(
            "main", [],
            make_instr("add", "r", [3, 4], type_hint="u8"),
            make_instr("ret", srcs=["r"], type_hint="u8"),
        )
        mod = make_mod(fn)

        received_cir = []
        class CapturingBackend:
            name = "cap"
            def compile(self, cir):
                received_cir.extend(cir)
                return b"\x00"
            def run(self, binary, args):
                return None

        AOTCore(backend=CapturingBackend(), optimization_level=0).compile(mod)
        ops = [c.op for c in received_cir]
        assert "add_u8" in ops  # not folded

    def test_optimization_level_1_folds_constants(self):
        fn = make_fn(
            "main", [],
            make_instr("add", "r", [3, 4], type_hint="u8"),
            make_instr("ret", srcs=["r"], type_hint="u8"),
        )
        mod = make_mod(fn)

        received_cir = []
        class CapturingBackend:
            name = "cap"
            def compile(self, cir):
                received_cir.extend(cir)
                return b"\x00"
            def run(self, binary, args):
                return None

        AOTCore(backend=CapturingBackend(), optimization_level=1).compile(mod)
        ops = [c.op for c in received_cir]
        assert "add_u8" not in ops
        const_instrs = [c for c in received_cir if c.op == "const_u8"]
        assert any(c.srcs == [7] for c in const_instrs)

    def test_optimization_level_2_same_as_1(self):
        fn = make_fn(
            "main", [],
            make_instr("add", "r", [3, 4], type_hint="u8"),
            make_instr("ret", srcs=["r"], type_hint="u8"),
        )
        mod = make_mod(fn)

        received_cir = []
        class CapturingBackend:
            name = "cap"
            def compile(self, cir):
                received_cir.extend(cir)
                return b"\x00"
            def run(self, binary, args):
                return None

        AOTCore(backend=CapturingBackend(), optimization_level=2).compile(mod)
        ops = [c.op for c in received_cir]
        assert "add_u8" not in ops


# ---------------------------------------------------------------------------
# compile_to_file
# ---------------------------------------------------------------------------

class TestAOTCoreCompileToFile:
    def test_writes_valid_aot_file(self):
        fn = _fully_typed_fn()
        mod = make_mod(fn)
        aot = AOTCore(backend=MockAOTBackend())
        with tempfile.NamedTemporaryFile(suffix=".aot", delete=False) as f:
            path = f.name
        try:
            aot.compile_to_file(mod, path)
            with open(path, "rb") as f:
                data = f.read()
            snap = read_snap(data)
            assert snap.native_code == b"\xde\xad"
        finally:
            os.unlink(path)

    def test_file_is_non_empty(self):
        fn = _fully_typed_fn()
        mod = make_mod(fn)
        aot = AOTCore(backend=MockAOTBackend())
        with tempfile.NamedTemporaryFile(suffix=".aot", delete=False) as f:
            path = f.name
        try:
            aot.compile_to_file(mod, path)
            assert os.path.getsize(path) > 0
        finally:
            os.unlink(path)


# ---------------------------------------------------------------------------
# stats()
# ---------------------------------------------------------------------------

class TestAOTCoreStats:
    def test_stats_type(self):
        aot = AOTCore(backend=MockAOTBackend())
        s = aot.stats()
        assert isinstance(s, AOTStats)

    def test_functions_compiled_counted(self):
        mod = make_mod(_fully_typed_fn("f1"), _fully_typed_fn("f2"))
        aot = AOTCore(backend=MockAOTBackend())
        aot.compile(mod)
        s = aot.stats()
        assert s.functions_compiled == 2

    def test_functions_untyped_counted(self):
        mod = make_mod(_untyped_fn())
        backend = MockAOTBackend(fail=True)
        aot = AOTCore(backend=backend)
        aot.compile(mod)
        s = aot.stats()
        assert s.functions_untyped == 1

    def test_total_binary_size(self):
        backend = MockAOTBackend(return_value=b"\xAA\xBB\xCC")
        mod = make_mod(_fully_typed_fn("f1"), _fully_typed_fn("f2"))
        aot = AOTCore(backend=backend)
        aot.compile(mod)
        s = aot.stats()
        assert s.total_binary_size == 6  # 3 bytes × 2 fns

    def test_compilation_time_positive(self):
        mod = make_mod(_fully_typed_fn())
        aot = AOTCore(backend=MockAOTBackend())
        aot.compile(mod)
        s = aot.stats()
        assert s.compilation_time_ns >= 0

    def test_optimization_level_in_stats(self):
        aot = AOTCore(backend=MockAOTBackend(), optimization_level=0)
        assert aot.stats().optimization_level == 0

    def test_stats_accumulate_across_compiles(self):
        mod = make_mod(_fully_typed_fn())
        aot = AOTCore(backend=MockAOTBackend())
        aot.compile(mod)
        aot.compile(mod)
        s = aot.stats()
        assert s.functions_compiled == 2

    def test_stats_snapshot_is_independent(self):
        mod = make_mod(_fully_typed_fn())
        aot = AOTCore(backend=MockAOTBackend())
        aot.compile(mod)
        s1 = aot.stats()
        aot.compile(mod)
        s2 = aot.stats()
        assert s1.functions_compiled == 1
        assert s2.functions_compiled == 2


# ---------------------------------------------------------------------------
# _is_fully_typed helper
# ---------------------------------------------------------------------------

class TestIsFullyTyped:
    def test_fully_typed_status(self):
        fn = _fully_typed_fn()
        from aot_core.infer import infer_types
        env = infer_types(fn)
        aot = AOTCore(backend=MockAOTBackend())
        assert aot._is_fully_typed(fn, env)

    def test_not_iir_function(self):
        aot = AOTCore(backend=MockAOTBackend())
        assert not aot._is_fully_typed("not_a_fn", {})

    def test_untyped_fn_with_any_dest(self):
        fn = make_fn(
            "f", [("x", "any")],
            make_instr("ret", srcs=["x"]),
            type_status=FunctionTypeStatus.UNTYPED,
        )
        from aot_core.infer import infer_types
        env = infer_types(fn)
        aot = AOTCore(backend=MockAOTBackend())
        # ret has no dest, so _is_fully_typed returns True (no unresolved dests).
        assert aot._is_fully_typed(fn, env)

    def test_untyped_fn_with_any_dest_instruction(self):
        fn = make_fn(
            "f", [("x", "any"), ("y", "any")],
            make_instr("add", "r", ["x", "y"]),  # r will be "any"
            make_instr("ret", srcs=["r"]),
            type_status=FunctionTypeStatus.UNTYPED,
        )
        from aot_core.infer import infer_types
        env = infer_types(fn)
        aot = AOTCore(backend=MockAOTBackend())
        assert not aot._is_fully_typed(fn, env)
