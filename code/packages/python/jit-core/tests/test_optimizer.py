"""Tests for the inline CIR optimizer (constant folding + DCE)."""

from __future__ import annotations

from jit_core import optimizer
from jit_core.cir import CIRInstr

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _instr(op: str, dest: str | None = None, srcs=None, type: str = "any") -> CIRInstr:
    return CIRInstr(op=op, dest=dest, srcs=srcs or [], type=type)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _ret(var: str, t: str = "u8") -> CIRInstr:
    return _instr(f"ret_{t}", None, [var], t)


def _prog(*instrs: CIRInstr) -> list[CIRInstr]:
    """Wrap instructions in a tiny program so DCE keeps the dest."""
    return list(instrs)


# ---------------------------------------------------------------------------
# Constant folding — arithmetic
# ---------------------------------------------------------------------------

class TestConstantFolding:
    def test_fold_add_u8(self):
        cir = _prog(_instr("add_u8", "v", [3, 4], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [7]

    def test_fold_sub_u8(self):
        cir = _prog(_instr("sub_u8", "v", [10, 3], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [7]

    def test_fold_mul_u8(self):
        cir = _prog(_instr("mul_u8", "v", [3, 4], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [12]

    def test_fold_div_int(self):
        cir = _prog(_instr("div_u8", "v", [10, 2], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [5]

    def test_fold_mod(self):
        cir = _prog(_instr("mod_u8", "v", [10, 3], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"
        assert result[0].srcs == [1]

    def test_fold_and(self):
        cir = _prog(_instr("and_u8", "v", [0b1010, 0b1100], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].srcs == [0b1000]

    def test_fold_or(self):
        cir = _prog(_instr("or_u8", "v", [0b1010, 0b0101], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].srcs == [0b1111]

    def test_fold_xor(self):
        cir = _prog(_instr("xor_u8", "v", [0b1010, 0b1100], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].srcs == [0b0110]

    def test_fold_shl(self):
        cir = _prog(_instr("shl_u8", "v", [1, 3], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].srcs == [8]

    def test_fold_shr(self):
        cir = _prog(_instr("shr_u8", "v", [16, 2], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].srcs == [4]

    def test_cmp_eq_not_foldable(self):
        # "cmp_eq_u8".split("_")[0] = "cmp" — not in _FOLDABLE_OPS
        # So comparison ops with literal srcs are NOT folded by the optimizer.
        cir = _prog(_instr("cmp_eq_u8", "v", [3, 3], "bool"), _ret("v", "bool"))
        result = optimizer.run(cir)
        assert result[0].op == "cmp_eq_u8"

    def test_cmp_lt_not_foldable(self):
        cir = _prog(_instr("cmp_lt_u8", "v", [2, 5], "bool"), _ret("v", "bool"))
        result = optimizer.run(cir)
        assert result[0].op == "cmp_lt_u8"

    def test_fold_f64(self):
        cir = _prog(_instr("add_f64", "v", [1.5, 2.5], "f64"), _ret("v", "f64"))
        result = optimizer.run(cir)
        assert result[0].srcs == [4.0]

    def test_fold_div_float(self):
        cir = _prog(_instr("div_f64", "v", [7.0, 2.0], "f64"), _ret("v", "f64"))
        result = optimizer.run(cir)
        assert result[0].srcs == [3.5]


# ---------------------------------------------------------------------------
# Constant folding — does NOT apply when srcs include variables
# ---------------------------------------------------------------------------

class TestConstantFoldingNoOp:
    def test_no_fold_variable_left(self):
        cir = _prog(_instr("add_u8", "v", ["x", 3], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "add_u8"

    def test_no_fold_variable_right(self):
        cir = _prog(_instr("add_u8", "v", [3, "x"], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "add_u8"

    def test_no_fold_both_variables(self):
        cir = _prog(_instr("add_u8", "v", ["x", "y"], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "add_u8"

    def test_no_fold_non_arithmetic_op(self):
        # call_runtime is side-effectful — kept even if dest unused
        cir = [_instr("call_runtime", "v", ["generic_add", 1, 2], "any")]
        result = optimizer.run(cir)
        assert result[0].op == "call_runtime"

    def test_no_fold_single_src(self):
        # fold requires exactly 2 srcs; neg_u8 has 1 src → not foldable
        cir = _prog(_instr("neg_u8", "v", [5], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "neg_u8"

    def test_no_fold_zero_srcs(self):
        cir = _prog(_instr("add_u8", "v", [], "u8"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "add_u8"

    def test_no_fold_on_div_by_zero(self):
        cir = _prog(_instr("div_u8", "v", [10, 0], "u8"), _ret("v"))
        result = optimizer.run(cir)
        # ZeroDivisionError → not folded
        assert result[0].op == "div_u8"


# ---------------------------------------------------------------------------
# Constant folding — _infer_literal_type paths (when type == "any")
# ---------------------------------------------------------------------------

class TestInferLiteralType:
    def test_infer_u8_result(self):
        cir = _prog(_instr("add_u8", "v", [1, 2], "any"), _ret("v"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u8"

    def test_infer_u16_result(self):
        cir = _prog(_instr("add_u16", "v", [300, 1], "any"), _ret("v", "u16"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u16"

    def test_infer_u32_result(self):
        cir = _prog(_instr("add_u32", "v", [70000, 1], "any"), _ret("v", "u32"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u32"

    def test_infer_u64_result(self):
        cir = _prog(_instr("add_u64", "v", [2**33, 1], "any"), _ret("v", "u64"))
        result = optimizer.run(cir)
        assert result[0].op == "const_u64"

    def test_infer_f64_result(self):
        cir = _prog(_instr("add_f64", "v", [1.0, 2.0], "any"), _ret("v", "f64"))
        result = optimizer.run(cir)
        assert result[0].op == "const_f64"


# ---------------------------------------------------------------------------
# Dead code elimination
# ---------------------------------------------------------------------------

class TestDCE:
    def test_removes_unused_const(self):
        cir = [
            _instr("const_u8", "dead", [99], "u8"),
            _instr("const_u8", "used", [1], "u8"),
            _instr("ret_u8", None, ["used"], "u8"),
        ]
        result = optimizer.run(cir)
        dests = [c.dest for c in result]
        assert "dead" not in dests

    def test_keeps_used_instruction(self):
        cir = [
            _instr("const_u8", "v", [5], "u8"),
            _instr("ret_u8", None, ["v"], "u8"),
        ]
        result = optimizer.run(cir)
        assert any(c.dest == "v" for c in result)

    def test_keeps_side_effect_ops_even_if_unused(self):
        cir = [
            _instr("call_runtime", "unused", ["side_effect"], "any"),
            _instr("ret_void", None, [], "void"),
        ]
        result = optimizer.run(cir)
        assert any(c.op == "call_runtime" for c in result)

    def test_keeps_ret_void(self):
        cir = [_instr("ret_void", None, [], "void")]
        result = optimizer.run(cir)
        assert result[0].op == "ret_void"

    def test_keeps_jmp(self):
        cir = [_instr("jmp", None, ["loop"], "void")]
        result = optimizer.run(cir)
        assert result[0].op == "jmp"

    def test_keeps_label(self):
        cir = [_instr("label", None, ["loop_start"], "void")]
        result = optimizer.run(cir)
        assert result[0].op == "label"

    def test_keeps_store_mem(self):
        cir = [_instr("store_mem", None, ["addr", "val"], "void")]
        result = optimizer.run(cir)
        assert result[0].op == "store_mem"

    def test_empty_cir(self):
        result = optimizer.run([])
        assert result == []

    def test_chain_dce_frees_intermediate(self):
        # a = const 3; b = a + 1; ret_void — b and a can be eliminated
        cir = [
            _instr("const_u8", "a", [3], "u8"),
            _instr("add_u8", "b", ["a", "1"], "u8"),
            _instr("ret_void", None, [], "void"),
        ]
        result = optimizer.run(cir)
        dests = [c.dest for c in result]
        assert "b" not in dests

    def test_type_assert_kept(self):
        cir = [_instr("type_assert", None, ["x", "u8"], "void")]
        result = optimizer.run(cir)
        assert result[0].op == "type_assert"
