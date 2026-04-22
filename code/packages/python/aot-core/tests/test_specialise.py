"""Tests for aot_core.specialise — AOT specialization pass."""

from __future__ import annotations

from conftest import make_fn, make_instr

from aot_core.specialise import _spec_type, aot_specialise

# ---------------------------------------------------------------------------
# _spec_type helper
# ---------------------------------------------------------------------------

class TestSpecType:
    def test_type_hint_wins(self):
        instr = make_instr("add", "r", ["a", "b"], type_hint="u8")
        assert _spec_type(instr, {}) == "u8"

    def test_inferred_dest_used_when_hint_is_any(self):
        instr = make_instr("add", "r", ["a", "b"])
        assert _spec_type(instr, {"r": "u16"}) == "u16"

    def test_inferred_any_falls_back_to_any(self):
        instr = make_instr("add", "r", ["a", "b"])
        assert _spec_type(instr, {"r": "any"}) == "any"

    def test_no_inferred_gives_any(self):
        instr = make_instr("add", "r", ["a", "b"])
        assert _spec_type(instr, {}) == "any"

    def test_ret_uses_src_type(self):
        instr = make_instr("ret", srcs=["x"])
        assert _spec_type(instr, {"x": "u8"}) == "u8"

    def test_ret_src_not_in_env(self):
        instr = make_instr("ret", srcs=["x"])
        assert _spec_type(instr, {}) == "any"

    def test_ret_literal_src(self):
        instr = make_instr("ret", srcs=[42])
        assert _spec_type(instr, {}) == "any"  # literal ints are not str


# ---------------------------------------------------------------------------
# const translation
# ---------------------------------------------------------------------------

class TestAOTSpecialiseConst:
    def test_typed_hint_const(self):
        fn = make_fn("f", [], make_instr("const", "v", [10], type_hint="u16"))
        cir = aot_specialise(fn)
        assert len(cir) == 1
        assert cir[0].op == "const_u16"
        assert cir[0].srcs == [10]

    def test_untyped_const_infer_u8(self):
        fn = make_fn("f", [], make_instr("const", "v", [42]))
        cir = aot_specialise(fn, {"v": "u8"})
        assert cir[0].op == "const_u8"

    def test_const_bool_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", [True]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_bool"

    def test_const_float_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", [3.14]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_f64"

    def test_const_str_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", ["hello"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_str"

    def test_const_no_srcs(self):
        fn = make_fn("f", [], make_instr("const", "v"))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_u8"
        assert cir[0].srcs == [0]

    def test_const_u16_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", [256]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_u16"

    def test_const_u32_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", [65536]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_u32"

    def test_const_u64_literal(self):
        fn = make_fn("f", [], make_instr("const", "v", [0x1_0000_0000]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_u64"

    def test_const_none_value_gives_any(self):
        fn = make_fn("f", [], make_instr("const", "v", [None]))
        cir = aot_specialise(fn)
        assert cir[0].op == "const_any"


# ---------------------------------------------------------------------------
# ret translation
# ---------------------------------------------------------------------------

class TestAOTSpecialiseRet:
    def test_ret_typed_hint(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("ret", srcs=["x"], type_hint="u8"),
        )
        cir = aot_specialise(fn)
        assert cir[0].op == "ret_u8"

    def test_ret_via_inferred(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("ret", srcs=["x"]),
        )
        cir = aot_specialise(fn, {"x": "u8"})
        assert cir[0].op == "ret_u8"

    def test_ret_void(self):
        fn = make_fn("f", [], make_instr("ret_void"))
        cir = aot_specialise(fn)
        assert cir[0].op == "ret_void"
        assert cir[0].type == "void"

    def test_ret_any_when_unknown(self):
        fn = make_fn(
            "f", [],
            make_instr("ret", srcs=["x"]),
        )
        cir = aot_specialise(fn)
        assert cir[0].op == "ret_any"


# ---------------------------------------------------------------------------
# binary ops
# ---------------------------------------------------------------------------

class TestAOTSpecialiseBinary:
    def test_typed_add(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"], type_hint="u8"),
        )
        cir = aot_specialise(fn)
        ops = [c.op for c in cir]
        assert "add_u8" in ops

    def test_inferred_add(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"]),
        )
        cir = aot_specialise(fn, {"r": "u8"})
        assert any(c.op == "add_u8" for c in cir)

    def test_untyped_add_generic(self):
        fn = make_fn(
            "f", [("a", "any"), ("b", "any")],
            make_instr("add", "r", ["a", "b"]),
        )
        cir = aot_specialise(fn)
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "generic_add"

    def test_add_str_str_special(self):
        fn = make_fn(
            "f", [("a", "str"), ("b", "str")],
            make_instr("add", "r", ["a", "b"], type_hint="str"),
        )
        cir = aot_specialise(fn)
        assert cir[0].op == "call_runtime"
        assert cir[0].srcs[0] == "str_concat"

    def test_guards_emitted_when_hint_any(self):
        fn = make_fn(
            "f", [("a", "any"), ("b", "any")],
            make_instr("add", "r", ["a", "b"]),
        )
        cir = aot_specialise(fn, {"r": "u8"})
        guard_ops = [c.op for c in cir if c.op == "type_assert"]
        assert len(guard_ops) == 2  # one per variable src

    def test_no_guards_when_hint_typed(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("add", "r", ["a", "b"], type_hint="u8"),
        )
        cir = aot_specialise(fn)
        assert all(c.op != "type_assert" for c in cir)

    def test_cmp_inferred_to_bool(self):
        fn = make_fn(
            "f", [("a", "u8"), ("b", "u8")],
            make_instr("cmp_lt", "r", ["a", "b"]),
        )
        cir = aot_specialise(fn, {"r": "bool"})
        assert any(c.op == "cmp_lt_bool" for c in cir)


# ---------------------------------------------------------------------------
# unary ops
# ---------------------------------------------------------------------------

class TestAOTSpecialiseUnary:
    def test_typed_neg(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("neg", "r", ["x"], type_hint="u8"),
        )
        cir = aot_specialise(fn)
        assert any(c.op == "neg_u8" for c in cir)

    def test_inferred_neg(self):
        fn = make_fn(
            "f", [("x", "u8")],
            make_instr("neg", "r", ["x"]),
        )
        cir = aot_specialise(fn, {"r": "u8"})
        assert any(c.op == "neg_u8" for c in cir)

    def test_untyped_not_generic(self):
        fn = make_fn(
            "f", [("x", "any")],
            make_instr("not", "r", ["x"]),
        )
        cir = aot_specialise(fn)
        assert cir[0].op == "call_runtime"
        assert "generic_not" in cir[0].srcs

    def test_unary_guard_emitted(self):
        fn = make_fn(
            "f", [("x", "any")],
            make_instr("neg", "r", ["x"]),
        )
        cir = aot_specialise(fn, {"r": "u8"})
        guard_ops = [c.op for c in cir if c.op == "type_assert"]
        assert len(guard_ops) == 1


# ---------------------------------------------------------------------------
# passthrough ops
# ---------------------------------------------------------------------------

class TestAOTSpecialisePassthrough:
    def test_jmp_passes_through(self):
        fn = make_fn("f", [], make_instr("jmp", srcs=["loop_start"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "jmp"

    def test_label_passes_through(self):
        fn = make_fn("f", [], make_instr("label", srcs=["top"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "label"

    def test_jmp_if_false_passes_through(self):
        fn = make_fn("f", [], make_instr("jmp_if_false", srcs=["cond", "end"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "jmp_if_false"

    def test_call_passes_through(self):
        fn = make_fn("f", [], make_instr("call", "r", ["fn_name"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "call"

    def test_store_mem_passes_through(self):
        fn = make_fn("f", [], make_instr("store_mem", srcs=["addr", "val"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "store_mem"


# ---------------------------------------------------------------------------
# fallback / unknown ops
# ---------------------------------------------------------------------------

class TestAOTSpecialiseFallback:
    def test_unknown_op_emitted_as_is(self):
        fn = make_fn("f", [], make_instr("exotic_op", "r", ["x"]))
        cir = aot_specialise(fn)
        assert cir[0].op == "exotic_op"

    def test_none_inferred_arg(self):
        cir = aot_specialise(make_fn("f", []), None)
        assert cir == []
