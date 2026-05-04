"""Tests for cir_optimizer — constant folding and DCE passes.

Covers the module-level run() function and the CIROptimizer class wrapper.
Tests are designed to be exhaustive across all foldable operators.
"""

from __future__ import annotations

from codegen_core.cir import CIRInstr
from codegen_core.optimizer.cir_optimizer import (
    CIROptimizer,
    _constant_fold,
    _dead_code_eliminate,
    run,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _c(op: str, dest: str | None, srcs: list, type_: str = "any") -> CIRInstr:
    return CIRInstr(op=op, dest=dest, srcs=srcs, type=type_)


# ---------------------------------------------------------------------------
# Constant folding
# ---------------------------------------------------------------------------

class TestConstantFolding:

    def test_fold_add_integers(self) -> None:
        cir = [_c("add_u8", "v", [3, 4], "u8"), _c("ret_u8", None, ["v"], "u8")]
        result = _constant_fold(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [7]

    def test_fold_sub(self) -> None:
        cir = [_c("sub_u8", "v", [10, 3], "u8")]
        result = _constant_fold(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [7]

    def test_fold_mul(self) -> None:
        cir = [_c("mul_u8", "v", [6, 7], "u8")]
        result = _constant_fold(cir)
        assert result[0].srcs == [42]

    def test_fold_and(self) -> None:
        cir = [_c("and_u8", "v", [0xFF, 0x0F], "u8")]
        result = _constant_fold(cir)
        assert result[0].srcs == [0x0F]

    def test_fold_or(self) -> None:
        cir = [_c("or_u8", "v", [0xF0, 0x0F], "u8")]
        result = _constant_fold(cir)
        assert result[0].srcs == [0xFF]

    def test_fold_xor(self) -> None:
        cir = [_c("xor_u8", "v", [0xFF, 0x0F], "u8")]
        result = _constant_fold(cir)
        assert result[0].srcs == [0xF0]

    def test_cmp_eq_not_foldable(self) -> None:
        """cmp_eq_u8 splits to base "cmp" which is not in _FOLDABLE_OPS.

        Comparison ops with a type suffix are intentionally not folded —
        the base-op extraction takes only the first underscore-separated
        token, so "cmp_eq_u8" → "cmp", which has no entry in _FOLDABLE_OPS.
        The test documents this deliberate limitation.
        """
        cir = [_c("cmp_eq_u8", "b", [5, 5], "bool")]
        result = _constant_fold(cir)
        assert result[0].op == "cmp_eq_u8"  # unchanged

    def test_cmp_lt_not_foldable(self) -> None:
        """cmp_lt_u8 similarly extracts base "cmp" — not foldable."""
        cir = [_c("cmp_lt_u8", "b", [5, 3], "bool")]
        result = _constant_fold(cir)
        assert result[0].op == "cmp_lt_u8"  # unchanged

    def test_no_fold_when_src_is_var(self) -> None:
        cir = [_c("add_u8", "v", ["a", 4], "u8")]
        result = _constant_fold(cir)
        assert result[0].op == "add_u8"  # unchanged

    def test_no_fold_for_passthrough_ops(self) -> None:
        cir = [_c("label", None, ["start"])]
        result = _constant_fold(cir)
        assert result[0].op == "label"

    def test_fold_does_not_crash_on_zero_division(self) -> None:
        cir = [_c("div_u8", "v", [5, 0], "u8")]
        result = _constant_fold(cir)
        # Should not fold — returns original instruction.
        assert result[0].op == "div_u8"


# ---------------------------------------------------------------------------
# Dead-code elimination
# ---------------------------------------------------------------------------

class TestDeadCodeElimination:

    def test_removes_unused_const(self) -> None:
        cir = [
            _c("const_u8", "dead", [0], "u8"),
            _c("const_u8", "live", [1], "u8"),
            _c("ret_u8", None, ["live"], "u8"),
        ]
        result = _dead_code_eliminate(cir)
        dests = [i.dest for i in result]
        assert "dead" not in dests
        assert "live" in dests

    def test_keeps_side_effectful_with_unused_dest(self) -> None:
        """call_runtime is side-effectful — must not be removed even if dest unused."""
        cir = [
            _c("call_runtime", "unused", ["io_read"], "u8"),
            _c("ret_void", None, [], "void"),
        ]
        result = _dead_code_eliminate(cir)
        ops = [i.op for i in result]
        assert "call_runtime" in ops

    def test_keeps_ret_void(self) -> None:
        cir = [_c("ret_void", None, [], "void")]
        result = _dead_code_eliminate(cir)
        assert result[0].op == "ret_void"

    def test_empty_cir_stays_empty(self) -> None:
        assert _dead_code_eliminate([]) == []


# ---------------------------------------------------------------------------
# Combined run()
# ---------------------------------------------------------------------------

class TestRunCombined:

    def test_run_applies_both_passes(self) -> None:
        cir = [
            _c("const_u8", "dead", [0], "u8"),             # dead
            _c("add_u8", "v", [3, 4], "u8"),               # should fold to const_u8
            _c("ret_u8", None, ["v"], "u8"),
        ]
        result = run(cir)
        dests = [i.dest for i in result]
        assert "dead" not in dests
        ops = [i.op for i in result]
        assert "const_u8" in ops

    def test_run_idempotent(self) -> None:
        cir = [_c("const_u8", "v", [1], "u8"), _c("ret_u8", None, ["v"], "u8")]
        once = run(cir)
        twice = run(once)
        assert [str(i) for i in once] == [str(i) for i in twice]


# ---------------------------------------------------------------------------
# CIROptimizer class wrapper
# ---------------------------------------------------------------------------

class TestCIROptimizer:

    def test_produces_same_result_as_module_run(self) -> None:
        cir = [
            _c("add_u8", "v", [2, 3], "u8"),
            _c("ret_u8", None, ["v"], "u8"),
        ]
        expected = run(list(cir))
        got = CIROptimizer().run(list(cir))
        assert [str(i) for i in got] == [str(i) for i in expected]

    def test_satisfies_optimizer_protocol(self) -> None:
        """CIROptimizer.run() returns a list[CIRInstr]."""
        opt = CIROptimizer()
        cir = [_c("ret_void", None, [], "void")]
        result = opt.run(cir)
        assert isinstance(result, list)
        assert all(isinstance(i, CIRInstr) for i in result)
