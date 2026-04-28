"""Tests for aot_core.infer — static type inference."""

from __future__ import annotations

from conftest import make_fn, make_instr

from aot_core.infer import _literal_type, _promote, _resolve, infer_types

# ---------------------------------------------------------------------------
# _literal_type
# ---------------------------------------------------------------------------

class TestLiteralType:
    def test_bool_true(self):
        assert _literal_type(True) == "bool"

    def test_bool_false(self):
        assert _literal_type(False) == "bool"

    def test_u8(self):
        assert _literal_type(0) == "u8"
        assert _literal_type(255) == "u8"

    def test_u16(self):
        assert _literal_type(256) == "u16"
        assert _literal_type(65535) == "u16"

    def test_u32(self):
        assert _literal_type(65536) == "u32"
        assert _literal_type(0xFFFF_FFFF) == "u32"

    def test_u64(self):
        assert _literal_type(0x1_0000_0000) == "u64"

    def test_float(self):
        assert _literal_type(3.14) == "f64"

    def test_str(self):
        assert _literal_type("hello") == "str"

    def test_none(self):
        assert _literal_type(None) == "any"

    def test_object(self):
        assert _literal_type(object()) == "any"


# ---------------------------------------------------------------------------
# _promote
# ---------------------------------------------------------------------------

class TestPromote:
    def test_same_type_u8(self):
        assert _promote("u8", "u8") == "u8"

    def test_u8_u16_promotes_to_u16(self):
        assert _promote("u8", "u16") == "u16"

    def test_u16_u8_promotes_to_u16(self):
        assert _promote("u16", "u8") == "u16"

    def test_u8_f64_promotes_to_f64(self):
        assert _promote("u8", "f64") == "f64"

    def test_bool_u8_promotes_to_u8(self):
        assert _promote("bool", "u8") == "u8"

    def test_any_propagates(self):
        assert _promote("any", "u8") == "any"
        assert _promote("u8", "any") == "any"

    def test_str_with_numeric_is_any(self):
        assert _promote("str", "u8") == "any"
        assert _promote("u8", "str") == "any"

    def test_str_with_str_is_any(self):
        # _promote doesn't know about str+str → str; that's in _infer_instr
        assert _promote("str", "str") == "any"

    def test_u32_u64_promotes_to_u64(self):
        assert _promote("u32", "u64") == "u64"


# ---------------------------------------------------------------------------
# _resolve
# ---------------------------------------------------------------------------

class TestResolve:
    def test_bool_literal(self):
        assert _resolve(True, {}) == "bool"

    def test_int_literal(self):
        assert _resolve(42, {}) == "u8"

    def test_float_literal(self):
        assert _resolve(1.0, {}) == "f64"

    def test_str_var_in_env(self):
        assert _resolve("x", {"x": "u16"}) == "u16"

    def test_str_var_missing(self):
        assert _resolve("x", {}) == "any"

    def test_none_value(self):
        assert _resolve(None, {}) == "any"


# ---------------------------------------------------------------------------
# infer_types — basic cases
# ---------------------------------------------------------------------------

class TestInferTypesBasic:
    def test_params_seeded(self):
        fn = make_fn("f", [("x", "u8"), ("y", "u16")])
        env = infer_types(fn)
        assert env["x"] == "u8"
        assert env["y"] == "u16"

    def test_typed_const_from_hint(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "v", [42], type_hint="u16"),
        )
        env = infer_types(fn)
        assert env["v"] == "u16"

    def test_untyped_const_literal_u8(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "v", [10]),
        )
        env = infer_types(fn)
        assert env["v"] == "u8"

    def test_untyped_const_literal_bool(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "v", [True]),
        )
        env = infer_types(fn)
        assert env["v"] == "bool"

    def test_untyped_const_literal_float(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "v", [3.14]),
        )
        env = infer_types(fn)
        assert env["v"] == "f64"

    def test_untyped_const_no_srcs(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "v"),
        )
        env = infer_types(fn)
        assert env["v"] == "u8"  # 0 → u8

    def test_untyped_const_literal_str(self):
        fn = make_fn(
            "f", [],
            make_instr("const", "s", ["hello"]),
        )
        env = infer_types(fn)
        assert env["s"] == "str"

    def test_no_dest_skipped(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("ret", srcs=["x"]),
        )
        env = infer_types(fn)
        assert "None" not in env
        assert "x" in env


# ---------------------------------------------------------------------------
# infer_types — arithmetic / promotion
# ---------------------------------------------------------------------------

class TestInferTypesArithmetic:
    def test_add_u8_u8(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u8"

    def test_add_u8_u16_promotes(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u16")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u16"

    def test_add_u8_f64_promotes_to_f64(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "f64")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "f64"

    def test_add_str_str_is_str(self):
        fn = make_fn(
            "f", [("a", "str"), ("b", "str")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "str"

    def test_add_str_u8_is_any(self):
        fn = make_fn(
            "f", [("a", "str"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "any"

    def test_sub_u16_u8(self):
        fn = make_fn(
            "f", [("a", "u16"), ("b", "u8")],
            make_instr("sub", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u16"

    def test_mul_literal_literal(self):
        fn = make_fn(
            "f", [],
            make_instr("mul", "r", [3, 4]),
        )
        env = infer_types(fn)
        assert env["r"] == "u8"

    def test_div_float_int(self):
        fn = make_fn(
            "f", [],
            make_instr("div", "r", [3.0, 2]),
        )
        env = infer_types(fn)
        assert env["r"] == "f64"

    def test_arithmetic_with_any_src_gives_any(self):
        fn = make_fn(
            "f", [("a", "any"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "any"

    def test_bitwise_and(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("and", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u8"

    def test_shl_u16_u8(self):
        fn = make_fn(
            "f", [("a", "u16"), ("b", "u8")],
            make_instr("shl", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u16"

    def test_arithmetic_fewer_than_two_srcs(self):
        fn = make_fn(
            "f", [],
            make_instr("add", "r", [1]),
        )
        env = infer_types(fn)
        assert env["r"] == "any"


# ---------------------------------------------------------------------------
# infer_types — comparison ops
# ---------------------------------------------------------------------------

class TestInferTypesComparison:
    def test_cmp_lt_u8_u8_gives_bool(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("cmp_lt", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "bool"

    def test_cmp_eq_unknown_gives_any(self):
        fn = make_fn(
            "f", [("a", "any"), ("b", "u8")],
            make_instr("cmp_eq", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "any"

    def test_cmp_ge_u32_u32_gives_bool(self):
        fn = make_fn(
            "f", [("a", "u32"), ("b", "u32")],
            make_instr("cmp_ge", "r", ["a", "b"]),
        )
        env = infer_types(fn)
        assert env["r"] == "bool"

    def test_comparison_fewer_than_two_srcs(self):
        fn = make_fn(
            "f", [],
            make_instr("cmp_eq", "r", [1]),
        )
        env = infer_types(fn)
        assert env["r"] == "any"


# ---------------------------------------------------------------------------
# infer_types — unary ops
# ---------------------------------------------------------------------------

class TestInferTypesUnary:
    def test_neg_u8(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("neg", "r", ["x"]),
        )
        env = infer_types(fn)
        assert env["r"] == "u8"

    def test_not_bool(self):
        fn = make_fn(
            "f", [("x", "bool")],
            make_instr("not", "r", ["x"]),
        )
        env = infer_types(fn)
        assert env["r"] == "bool"

    def test_unary_no_srcs_gives_any(self):
        fn = make_fn(
            "f", [],
            make_instr("neg", "r"),
        )
        env = infer_types(fn)
        assert env["r"] == "any"


# ---------------------------------------------------------------------------
# infer_types — multi-instruction flow
# ---------------------------------------------------------------------------

class TestInferTypesFlow:
    def test_chain_of_instructions(self):
        """const → add → cmp: types should flow correctly."""
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("const", "one", [1]),
            make_instr("add", "sum", ["x", "one"]),
            make_instr("const", "ten", [10]),
            make_instr("cmp_lt", "flag", ["sum", "ten"]),
        )
        env = infer_types(fn)
        assert env["one"] == "u8"
        assert env["sum"] == "u8"
        assert env["ten"] == "u8"
        assert env["flag"] == "bool"

    def test_unknown_op_gives_any(self):
        fn = make_fn(
            "f", [],
            make_instr("load_mem", "v", ["ptr"]),
        )
        env = infer_types(fn)
        assert env.get("v", "any") == "any"

    def test_typed_hint_overrides_inference(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"], type_hint="u16"),
        )
        env = infer_types(fn)
        assert env["r"] == "u16"
