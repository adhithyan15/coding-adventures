"""Tests for the jit-core specialization pass (specialise.py)."""

from __future__ import annotations

import pytest
from conftest import make_fn, make_instr

from jit_core.cir import CIRInstr
from jit_core.specialise import specialise

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _ops(cir: list[CIRInstr]) -> list[str]:
    return [i.op for i in cir]


def _types(cir: list[CIRInstr]) -> list[str]:
    return [i.type for i in cir]


# ---------------------------------------------------------------------------
# const translation
# ---------------------------------------------------------------------------

class TestConstTranslation:
    def test_const_bool_hint(self):
        fn = make_fn("f", [], make_instr("const", "v0", [True], type_hint="bool"))
        cir = specialise(fn)
        assert cir[0].op == "const_bool"
        assert cir[0].type == "bool"
        assert cir[0].srcs == [True]

    def test_const_bool_inferred_from_literal(self):
        # type_hint == "any" → _literal_type(True) → "bool"
        fn = make_fn("f", [], make_instr("const", "v0", [True]))
        cir = specialise(fn)
        assert cir[0].op == "const_bool"

    def test_const_any_unknown_type_inferred(self):
        # Non-standard literal type → "any"
        fn = make_fn("f", [], make_instr("const", "v0", [None]))
        cir = specialise(fn)
        assert cir[0].op == "const_any"

    def test_const_u8_inferred(self):
        # type_hint == "any" → infer from literal
        fn = make_fn("f", [], make_instr("const", "v0", [42]))
        cir = specialise(fn)
        assert cir[0].op == "const_u8"
        assert cir[0].type == "u8"

    def test_const_u16_inferred(self):
        fn = make_fn("f", [], make_instr("const", "v0", [1000]))
        cir = specialise(fn)
        assert cir[0].op == "const_u16"

    def test_const_u32_inferred(self):
        fn = make_fn("f", [], make_instr("const", "v0", [70_000]))
        cir = specialise(fn)
        assert cir[0].op == "const_u32"

    def test_const_u64_inferred(self):
        fn = make_fn("f", [], make_instr("const", "v0", [2**33]))
        cir = specialise(fn)
        assert cir[0].op == "const_u64"

    def test_const_f64_inferred(self):
        fn = make_fn("f", [], make_instr("const", "v0", [3.14]))
        cir = specialise(fn)
        assert cir[0].op == "const_f64"

    def test_const_str_inferred(self):
        fn = make_fn("f", [], make_instr("const", "v0", ["hello"]))
        cir = specialise(fn)
        assert cir[0].op == "const_str"

    def test_const_hint_overrides_literal(self):
        # type_hint wins over literal inference
        fn = make_fn("f", [], make_instr("const", "v0", [42], type_hint="u32"))
        cir = specialise(fn)
        assert cir[0].op == "const_u32"

    def test_const_empty_srcs_defaults_to_zero(self):
        fn = make_fn("f", [], make_instr("const", "v0"))
        cir = specialise(fn)
        assert cir[0].op == "const_u8"  # 0 → u8
        assert cir[0].srcs == [0]


# ---------------------------------------------------------------------------
# ret / ret_void
# ---------------------------------------------------------------------------

class TestRetTranslation:
    def test_ret_void(self):
        fn = make_fn("f", [], make_instr("ret_void"))
        cir = specialise(fn)
        assert cir[0].op == "ret_void"
        assert cir[0].type == "void"
        assert cir[0].dest is None

    def test_ret_with_type_hint(self):
        fn = make_fn("f", [], make_instr("ret", srcs=["v0"], type_hint="u8"))
        cir = specialise(fn)
        assert cir[0].op == "ret_u8"
        assert cir[0].type == "u8"

    def test_ret_any_falls_back(self):
        fn = make_fn("f", [], make_instr("ret", srcs=["v0"]))
        cir = specialise(fn)
        assert cir[0].op == "ret_any"
        assert cir[0].type == "any"

    def test_ret_uses_observed_type(self):
        instr = make_instr("ret", srcs=["v0"], observed_type="i32", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert cir[0].op == "ret_i32"


# ---------------------------------------------------------------------------
# Passthrough ops
# ---------------------------------------------------------------------------

class TestPassthroughOps:
    @pytest.mark.parametrize("op", [
        "label", "jmp", "call", "call_builtin",
        "cast", "type_assert", "load_reg", "store_reg",
        "load_mem", "store_mem", "io_in", "io_out",
    ])
    def test_passthrough(self, op):
        fn = make_fn("f", [], make_instr(op, "v0", ["v1"]))
        cir = specialise(fn)
        assert cir[0].op == op
        assert cir[0].srcs == ["v1"]

    def test_jmp_passthrough_no_guard(self):
        fn = make_fn("f", [], make_instr("jmp", srcs=["loop_start"]))
        cir = specialise(fn)
        assert len(cir) == 1
        assert cir[0].op == "jmp"


# ---------------------------------------------------------------------------
# Binary ops — generic path (type = "any")
# ---------------------------------------------------------------------------

class TestBinaryGenericPath:
    def test_add_any_emits_call_runtime(self):
        fn = make_fn("f", [], make_instr("add", "v0", ["a", "b"]))
        cir = specialise(fn)
        assert len(cir) == 1
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "generic_add"
        assert cir[0].type == "any"

    def test_sub_any_emits_call_runtime(self):
        fn = make_fn("f", [], make_instr("sub", "v0", ["a", "b"]))
        cir = specialise(fn)
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "generic_sub"

    @pytest.mark.parametrize("op", [
        "mul", "div", "mod", "and", "or", "xor", "shl", "shr",
        "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
    ])
    def test_binary_any_emits_call_runtime(self, op):
        fn = make_fn("f", [], make_instr(op, "v0", ["a", "b"]))
        cir = specialise(fn)
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == f"generic_{op}"


# ---------------------------------------------------------------------------
# Binary ops — concrete type path (with guards)
# ---------------------------------------------------------------------------

class TestBinaryTypedPath:
    def test_add_u8_with_type_hint(self):
        fn = make_fn("f", [], make_instr("add", "v0", ["a", "b"], type_hint="u8"))
        cir = specialise(fn)
        # type_hint is concrete → no guards needed
        assert len(cir) == 1
        assert cir[0].op == "add_u8"
        assert cir[0].type == "u8"

    def test_add_u8_with_observed_type_emits_guards(self):
        instr = make_instr("add", "v0", ["a", "b"], observed_type="u8", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        # two variable srcs → two guards + one add_u8
        assert len(cir) == 3
        assert cir[0].op == "type_assert"
        assert cir[0].srcs == ["a", "u8"]
        assert cir[1].op == "type_assert"
        assert cir[1].srcs == ["b", "u8"]
        assert cir[2].op == "add_u8"

    def test_guard_has_deopt_anchor(self):
        instr = make_instr(
            "add", "v0", ["a", "b"],
            observed_type="u8", observation_count=10,
            deopt_anchor=5,
        )
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert cir[0].deopt_to == 5
        assert cir[1].deopt_to == 5

    def test_no_guard_for_literal_src(self):
        # Only variable-name srcs get guards; literal ints do not.
        instr = make_instr("add", "v0", ["a", 2], observed_type="u8", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        # Only "a" is a variable → one guard
        guards = [c for c in cir if c.op == "type_assert"]
        assert len(guards) == 1
        assert guards[0].srcs[0] == "a"

    def test_typed_cmp_lt(self):
        fn = make_fn("f", [], make_instr("cmp_lt", "v0", ["a", "b"], type_hint="i32"))
        cir = specialise(fn)
        assert cir[0].op == "cmp_lt_i32"

    def test_typed_mul_f64(self):
        fn = make_fn("f", [], make_instr("mul", "v0", ["x", "y"], type_hint="f64"))
        cir = specialise(fn)
        assert cir[0].op == "mul_f64"


# ---------------------------------------------------------------------------
# Special (op, type) overrides
# ---------------------------------------------------------------------------

class TestSpecialOps:
    def test_add_str_emits_str_concat(self):
        fn = make_fn("f", [], make_instr("add", "v0", ["a", "b"], type_hint="str"))
        cir = specialise(fn)
        assert len(cir) == 1
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "str_concat"
        assert cir[0].type == "str"

    def test_jmp_if_false_bool_passes_through(self):
        # jmp_if_false is in _PASSTHROUGH_OPS — no transformation applied
        instr = make_instr("jmp_if_false", srcs=["cond", "lbl"], observed_type="bool", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert cir[-1].op == "jmp_if_false"

    def test_jmp_if_true_bool_passes_through(self):
        # jmp_if_true is in _PASSTHROUGH_OPS — no transformation applied
        instr = make_instr("jmp_if_true", srcs=["cond", "lbl"], observed_type="bool", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert cir[-1].op == "jmp_if_true"


# ---------------------------------------------------------------------------
# Unary ops
# ---------------------------------------------------------------------------

class TestUnaryOps:
    def test_neg_any_emits_call_runtime(self):
        fn = make_fn("f", [], make_instr("neg", "v0", ["a"]))
        cir = specialise(fn)
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "generic_neg"

    def test_neg_typed_emits_neg_type(self):
        fn = make_fn("f", [], make_instr("neg", "v0", ["a"], type_hint="f64"))
        cir = specialise(fn)
        assert cir[0].op == "neg_f64"

    def test_not_typed_emits_not_type(self):
        fn = make_fn("f", [], make_instr("not", "v0", ["a"], type_hint="bool"))
        cir = specialise(fn)
        assert cir[0].op == "not_bool"

    def test_unary_guard_emitted_when_observed(self):
        instr = make_instr("neg", "v0", ["a"], observed_type="i32", observation_count=10)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert cir[0].op == "type_assert"
        assert cir[1].op == "neg_i32"


# ---------------------------------------------------------------------------
# min_observations threshold
# ---------------------------------------------------------------------------

class TestMinObservations:
    def test_below_threshold_falls_back_to_any(self):
        instr = make_instr("add", "v0", ["a", "b"], observed_type="u8", observation_count=3)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        # count=3 < min_obs=5 → generic path
        assert cir[0].op == "call_runtime"

    def test_at_threshold_uses_observed_type(self):
        instr = make_instr("add", "v0", ["a", "b"], observed_type="u8", observation_count=5)
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=5)
        assert any(c.op == "add_u8" for c in cir)

    def test_polymorphic_falls_back_to_any(self):
        instr = make_instr("add", "v0", ["a", "b"])
        instr.record_observation("u8")
        instr.record_observation("str")  # now polymorphic
        fn = make_fn("f", [], instr)
        cir = specialise(fn, min_observations=1)
        assert cir[0].op == "call_runtime"


# ---------------------------------------------------------------------------
# Multi-instruction functions
# ---------------------------------------------------------------------------

class TestMultiInstrFn:
    def test_sequence_of_instrs(self):
        fn = make_fn(
            "f", [("a", "u8")],
            make_instr("const", "c", [1], type_hint="u8"),
            make_instr("add", "v0", ["a", "c"], type_hint="u8"),
            make_instr("ret", srcs=["v0"], type_hint="u8"),
        )
        cir = specialise(fn)
        ops = _ops(cir)
        assert "const_u8" in ops
        assert "add_u8" in ops
        assert "ret_u8" in ops

    def test_empty_function(self):
        fn = make_fn("f", [])
        cir = specialise(fn)
        assert cir == []

    def test_unknown_op_passes_through_as_fallback(self):
        fn = make_fn("f", [], make_instr("exotic_op", "v0", ["x"]))
        cir = specialise(fn)
        assert cir[0].op == "exotic_op"
