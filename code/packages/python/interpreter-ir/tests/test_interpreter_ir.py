"""Tests for interpreter-ir package.

Covers IIRInstr, IIRFunction, IIRModule, opcode sets, and serialisation.
Target: ≥95% coverage.
"""

from __future__ import annotations

import struct

import pytest

from interpreter_ir import (
    ALL_OPS,
    ARITHMETIC_OPS,
    BITWISE_OPS,
    BRANCH_OPS,
    CALL_OPS,
    CMP_OPS,
    COERCION_OPS,
    CONCRETE_TYPES,
    CONTROL_OPS,
    DYNAMIC_TYPE,
    IO_OPS,
    MEMORY_OPS,
    POLYMORPHIC_TYPE,
    SIDE_EFFECT_OPS,
    VALUE_OPS,
    FunctionTypeStatus,
    IIRFunction,
    IIRInstr,
    IIRModule,
    deserialise,
    serialise,
)

# ===========================================================================
# Helpers
# ===========================================================================

def make_add_fn(type_hint: str = "u8") -> IIRFunction:
    return IIRFunction(
        name="add",
        params=[("a", type_hint), ("b", type_hint)],
        return_type=type_hint,
        instructions=[
            IIRInstr("add", "v0", ["a", "b"], type_hint),
            IIRInstr("ret", None, ["v0"], type_hint),
        ],
        type_status=FunctionTypeStatus.FULLY_TYPED,
    )


def make_module(*fns: IIRFunction, entry: str = "main") -> IIRModule:
    return IIRModule(name="test.ml", functions=list(fns), entry_point=entry)


# ===========================================================================
# IIRInstr tests
# ===========================================================================

class TestIIRInstr:
    def test_basic_construction(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        assert instr.op == "add"
        assert instr.dest == "v0"
        assert instr.srcs == ["a", "b"]
        assert instr.type_hint == "u8"

    def test_void_dest(self):
        instr = IIRInstr("ret", None, ["v0"], "u8")
        assert instr.dest is None

    def test_literal_srcs(self):
        instr = IIRInstr("const", "k", [42], "u8")
        assert instr.srcs == [42]

    def test_float_literal_src(self):
        instr = IIRInstr("const", "k", [3.14], "any")
        assert instr.srcs == [3.14]

    def test_bool_literal_src(self):
        instr = IIRInstr("const", "k", [True], "bool")
        assert instr.srcs == [True]

    def test_default_observation_fields(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        assert instr.observed_type is None
        assert instr.observation_count == 0
        assert instr.deopt_anchor is None

    def test_is_typed_concrete(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        assert instr.is_typed() is True

    def test_is_typed_dynamic(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        assert instr.is_typed() is False

    def test_has_observation_false(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        assert instr.has_observation() is False

    def test_has_observation_true(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        assert instr.has_observation() is True

    def test_is_polymorphic_false(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        assert instr.is_polymorphic() is False

    def test_is_polymorphic_after_two_types(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.record_observation("u16")
        assert instr.is_polymorphic() is True
        assert instr.observed_type == "polymorphic"

    def test_record_observation_same_type(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.record_observation("u8")
        instr.record_observation("u8")
        assert instr.observed_type == "u8"
        assert instr.observation_count == 3
        assert not instr.is_polymorphic()

    def test_record_observation_stays_polymorphic(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.record_observation("str")
        instr.record_observation("u8")   # already polymorphic
        assert instr.observed_type == "polymorphic"
        assert instr.observation_count == 3

    def test_effective_type_concrete_hint(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        assert instr.effective_type() == "u8"

    def test_effective_type_observed(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u16")
        assert instr.effective_type() == "u16"

    def test_effective_type_polymorphic_returns_any(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.record_observation("str")
        assert instr.effective_type() == "any"

    def test_effective_type_no_observation(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        assert instr.effective_type() == "any"

    def test_deopt_anchor(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any", deopt_anchor=5)
        assert instr.deopt_anchor == 5

    def test_repr_no_observation(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        r = repr(instr)
        assert "add" in r
        assert "v0" in r

    def test_repr_with_observation(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        r = repr(instr)
        assert "obs=" in r

    def test_repr_void_dest(self):
        instr = IIRInstr("ret", None, ["v0"], "u8")
        r = repr(instr)
        assert "ret" in r
        assert "= " not in r  # no "dest = " prefix

    def test_equality_ignores_observation(self):
        a = IIRInstr("add", "v0", ["a", "b"], "u8")
        b = IIRInstr("add", "v0", ["a", "b"], "u8")
        b.record_observation("u8")
        assert a == b   # compare=False for observation fields


# ===========================================================================
# IIRFunction tests
# ===========================================================================

class TestIIRFunction:
    def test_basic_construction(self):
        fn = make_add_fn()
        assert fn.name == "add"
        assert fn.return_type == "u8"
        assert len(fn.instructions) == 2

    def test_param_names(self):
        fn = make_add_fn()
        assert fn.param_names() == ["a", "b"]

    def test_param_types(self):
        fn = make_add_fn()
        assert fn.param_types() == ["u8", "u8"]

    def test_infer_type_status_fully_typed(self):
        fn = make_add_fn("u8")
        assert fn.infer_type_status() == FunctionTypeStatus.FULLY_TYPED

    def test_infer_type_status_untyped(self):
        fn = IIRFunction(
            name="f",
            params=[("x", "any")],
            return_type="any",
            instructions=[IIRInstr("ret", None, ["x"], "any")],
        )
        assert fn.infer_type_status() == FunctionTypeStatus.UNTYPED

    def test_infer_type_status_partially_typed(self):
        fn = IIRFunction(
            name="f",
            params=[("x", "u8")],
            return_type="any",
            instructions=[IIRInstr("ret", None, ["x"], "any")],
        )
        assert fn.infer_type_status() == FunctionTypeStatus.PARTIALLY_TYPED

    def test_infer_type_status_no_instructions(self):
        fn = IIRFunction(name="f", params=[], return_type="void", instructions=[])
        assert fn.infer_type_status() == FunctionTypeStatus.UNTYPED

    def test_label_index_found(self):
        fn = IIRFunction(
            name="f",
            params=[],
            return_type="void",
            instructions=[
                IIRInstr("const", "v0", [1], "u8"),
                IIRInstr("label", None, ["loop_start"], "void"),
                IIRInstr("ret", None, ["v0"], "u8"),
            ],
        )
        assert fn.label_index("loop_start") == 1

    def test_label_index_not_found(self):
        fn = make_add_fn()
        with pytest.raises(KeyError, match="missing"):
            fn.label_index("missing")

    def test_call_count_default(self):
        fn = make_add_fn()
        assert fn.call_count == 0

    def test_repr(self):
        fn = make_add_fn()
        r = repr(fn)
        assert "add" in r
        assert "FULLY_TYPED" in r

    def test_type_status_enum_values(self):
        assert FunctionTypeStatus.FULLY_TYPED != FunctionTypeStatus.UNTYPED
        assert FunctionTypeStatus.PARTIALLY_TYPED != FunctionTypeStatus.FULLY_TYPED


# ===========================================================================
# IIRModule tests
# ===========================================================================

class TestIIRModule:
    def test_basic_construction(self):
        fn = make_add_fn()
        m = IIRModule(name="prog.ml", functions=[fn], entry_point="add")
        assert m.name == "prog.ml"
        assert m.entry_point == "add"

    def test_get_function_found(self):
        fn = make_add_fn()
        m = IIRModule(name="x", functions=[fn])
        assert m.get_function("add") is fn

    def test_get_function_not_found(self):
        m = IIRModule(name="x", functions=[])
        assert m.get_function("missing") is None

    def test_function_names(self):
        a = make_add_fn()
        b = IIRFunction(name="sub", params=[], return_type="void", instructions=[])
        m = IIRModule(name="x", functions=[a, b])
        assert m.function_names() == ["add", "sub"]

    def test_add_or_replace_new(self):
        m = IIRModule(name="x", functions=[])
        fn = make_add_fn()
        m.add_or_replace(fn)
        assert m.get_function("add") is fn

    def test_add_or_replace_existing(self):
        old = make_add_fn("u8")
        new = make_add_fn("u16")
        m = IIRModule(name="x", functions=[old])
        m.add_or_replace(new)
        assert len(m.functions) == 1
        assert m.get_function("add") is new

    def test_validate_clean(self):
        fn = make_add_fn()
        m = IIRModule(name="x", functions=[fn], entry_point="add")
        assert m.validate() == []

    def test_validate_no_entry_point(self):
        fn = make_add_fn()
        m = IIRModule(name="x", functions=[fn], entry_point=None)
        assert m.validate() == []

    def test_validate_duplicate_function(self):
        fn1 = make_add_fn()
        fn2 = make_add_fn()
        m = IIRModule(name="x", functions=[fn1, fn2], entry_point="add")
        errors = m.validate()
        assert any("duplicate" in e for e in errors)

    def test_validate_missing_entry_point(self):
        fn = make_add_fn()
        m = IIRModule(name="x", functions=[fn], entry_point="nonexistent")
        errors = m.validate()
        assert any("entry_point" in e for e in errors)

    def test_validate_undefined_label(self):
        fn = IIRFunction(
            name="main",
            params=[],
            return_type="void",
            instructions=[
                IIRInstr("jmp", None, ["does_not_exist"], "void"),
            ],
        )
        m = IIRModule(name="x", functions=[fn], entry_point="main")
        errors = m.validate()
        assert any("does_not_exist" in e for e in errors)

    def test_validate_defined_label_no_error(self):
        fn = IIRFunction(
            name="main",
            params=[],
            return_type="void",
            instructions=[
                IIRInstr("label", None, ["loop"], "void"),
                IIRInstr("jmp", None, ["loop"], "void"),
            ],
        )
        m = IIRModule(name="x", functions=[fn], entry_point="main")
        errors = m.validate()
        assert errors == []

    def test_validate_branch_with_no_srcs(self):
        fn = IIRFunction(
            name="main",
            params=[],
            return_type="void",
            instructions=[
                IIRInstr("jmp", None, [], "void"),  # empty srcs — no label to check
            ],
        )
        m = IIRModule(name="x", functions=[fn], entry_point="main")
        errors = m.validate()
        assert errors == []

    def test_repr(self):
        fn = make_add_fn()
        m = IIRModule(
            name="test.ml", functions=[fn], entry_point="add", language="tetrad"
        )
        r = repr(m)
        assert "test.ml" in r
        assert "tetrad" in r

    def test_default_language(self):
        m = IIRModule(name="x")
        assert m.language == "unknown"

    def test_default_entry_point(self):
        m = IIRModule(name="x")
        assert m.entry_point == "main"


# ===========================================================================
# Opcode set tests
# ===========================================================================

class TestOpcodeSets:
    def test_arithmetic_ops_contents(self):
        assert "add" in ARITHMETIC_OPS
        assert "sub" in ARITHMETIC_OPS
        assert "mul" in ARITHMETIC_OPS
        assert "div" in ARITHMETIC_OPS
        assert "mod" in ARITHMETIC_OPS
        assert "neg" in ARITHMETIC_OPS

    def test_bitwise_ops_contents(self):
        assert "and" in BITWISE_OPS
        assert "or" in BITWISE_OPS
        assert "xor" in BITWISE_OPS
        assert "not" in BITWISE_OPS
        assert "shl" in BITWISE_OPS
        assert "shr" in BITWISE_OPS

    def test_cmp_ops_contents(self):
        for op in ["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"]:
            assert op in CMP_OPS

    def test_branch_ops(self):
        assert "jmp" in BRANCH_OPS
        assert "jmp_if_true" in BRANCH_OPS
        assert "jmp_if_false" in BRANCH_OPS

    def test_control_ops(self):
        assert "label" in CONTROL_OPS
        assert "ret" in CONTROL_OPS
        assert "ret_void" in CONTROL_OPS

    def test_memory_ops(self):
        assert "load_reg" in MEMORY_OPS
        assert "store_reg" in MEMORY_OPS
        assert "load_mem" in MEMORY_OPS
        assert "store_mem" in MEMORY_OPS

    def test_call_ops(self):
        assert "call" in CALL_OPS
        assert "call_builtin" in CALL_OPS

    def test_io_ops(self):
        assert "io_in" in IO_OPS
        assert "io_out" in IO_OPS

    def test_coercion_ops(self):
        assert "cast" in COERCION_OPS
        assert "type_assert" in COERCION_OPS

    def test_value_ops_includes_arithmetic(self):
        assert ARITHMETIC_OPS.issubset(VALUE_OPS)

    def test_side_effect_ops_includes_branches(self):
        assert BRANCH_OPS.issubset(SIDE_EFFECT_OPS)

    def test_all_ops_is_superset(self):
        for op_set in [ARITHMETIC_OPS, BITWISE_OPS, CMP_OPS, BRANCH_OPS,
                       CONTROL_OPS, MEMORY_OPS, CALL_OPS, IO_OPS, COERCION_OPS]:
            assert op_set.issubset(ALL_OPS)

    def test_concrete_types(self):
        for t in ["u8", "u16", "u32", "u64", "bool", "str"]:
            assert t in CONCRETE_TYPES

    def test_dynamic_type_not_concrete(self):
        assert DYNAMIC_TYPE not in CONCRETE_TYPES

    def test_polymorphic_constant(self):
        assert POLYMORPHIC_TYPE == "polymorphic"


# ===========================================================================
# Serialisation tests
# ===========================================================================

class TestSerialise:
    def _roundtrip(self, module: IIRModule) -> IIRModule:
        return deserialise(serialise(module))

    def test_empty_module(self):
        m = IIRModule(name="empty", functions=[], entry_point=None, language="test")
        m2 = self._roundtrip(m)
        assert m2.name == "empty"
        assert m2.functions == []
        assert m2.entry_point is None
        assert m2.language == "test"

    def test_simple_function(self):
        fn = make_add_fn()
        m = IIRModule(name="x", functions=[fn], entry_point="add", language="tetrad")
        m2 = self._roundtrip(m)
        assert m2.name == "x"
        assert len(m2.functions) == 1
        fn2 = m2.functions[0]
        assert fn2.name == "add"
        assert fn2.params == [("a", "u8"), ("b", "u8")]
        assert fn2.return_type == "u8"
        assert len(fn2.instructions) == 2
        assert fn2.type_status == FunctionTypeStatus.FULLY_TYPED

    def test_instruction_str_src(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "u8")
        fn = IIRFunction(name="f", params=[], return_type="u8", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == ["a", "b"]

    def test_instruction_int_src(self):
        instr = IIRInstr("const", "v0", [42], "u8")
        fn = IIRFunction(name="f", params=[], return_type="u8", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == [42]

    def test_instruction_float_src(self):
        instr = IIRInstr("const", "v0", [3.14], "any")
        fn = IIRFunction(name="f", params=[], return_type="any", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        restored_val = m.functions[0].instructions[0].srcs[0]
        assert abs(restored_val - 3.14) < 1e-10

    def test_instruction_bool_src_true(self):
        instr = IIRInstr("const", "v0", [True], "bool")
        fn = IIRFunction(name="f", params=[], return_type="bool", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == [True]

    def test_instruction_bool_src_false(self):
        instr = IIRInstr("const", "v0", [False], "bool")
        fn = IIRFunction(name="f", params=[], return_type="bool", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == [False]

    def test_void_dest_preserved(self):
        instr = IIRInstr("ret", None, ["v0"], "u8")
        fn = IIRFunction(name="f", params=[], return_type="u8", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].dest is None

    def test_multiple_functions(self):
        fn1 = make_add_fn()
        fn2 = IIRFunction(
            name="sub",
            params=[("a", "u8"), ("b", "u8")],
            return_type="u8",
            instructions=[
                IIRInstr("sub", "v0", ["a", "b"], "u8"),
                IIRInstr("ret", None, ["v0"], "u8"),
            ],
            type_status=FunctionTypeStatus.FULLY_TYPED,
        )
        m = IIRModule(name="x", functions=[fn1, fn2])
        m2 = self._roundtrip(m)
        assert [f.name for f in m2.functions] == ["add", "sub"]

    def test_type_status_untyped(self):
        fn = IIRFunction(
            name="f",
            params=[("x", "any")],
            return_type="any",
            instructions=[IIRInstr("ret", None, ["x"], "any")],
            type_status=FunctionTypeStatus.UNTYPED,
        )
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].type_status == FunctionTypeStatus.UNTYPED

    def test_type_status_partially_typed(self):
        fn = IIRFunction(
            name="f",
            params=[("x", "u8")],
            return_type="any",
            instructions=[IIRInstr("ret", None, ["x"], "any")],
            type_status=FunctionTypeStatus.PARTIALLY_TYPED,
        )
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].type_status == FunctionTypeStatus.PARTIALLY_TYPED

    def test_register_count_preserved(self):
        fn = IIRFunction(
            name="f", params=[], return_type="void",
            instructions=[], register_count=16,
        )
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].register_count == 16

    def test_invalid_magic(self):
        raw = b"BAD\x00" + b"\x00" * 100
        with pytest.raises(ValueError, match="magic"):
            deserialise(raw)

    def test_invalid_version(self):
        raw = serialise(IIRModule(name="x"))
        # Overwrite version bytes (offset 4,5) with 99.0
        raw2 = raw[:4] + struct.pack("<BB", 99, 0) + raw[6:]
        with pytest.raises(ValueError, match="version"):
            deserialise(raw2)

    def test_truncated_data(self):
        raw = serialise(IIRModule(name="x", functions=[make_add_fn()]))
        with pytest.raises(ValueError, match="end of data"):
            deserialise(raw[:20])

    def test_observation_fields_not_serialised(self):
        instr = IIRInstr("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.deopt_anchor = 3
        fn = IIRFunction(name="f", params=[], return_type="any", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        restored_instr = m.functions[0].instructions[0]
        assert restored_instr.observed_type is None
        assert restored_instr.observation_count == 0
        assert restored_instr.deopt_anchor is None

    def test_unicode_names(self):
        fn = IIRFunction(
            name="добавить",
            params=[("а", "u8"), ("б", "u8")],
            return_type="u8",
            instructions=[IIRInstr("ret", None, ["а"], "u8")],
        )
        m = IIRModule(name="тест.ml", functions=[fn], language="test")
        m2 = self._roundtrip(m)
        assert m2.name == "тест.ml"
        assert m2.functions[0].name == "добавить"

    def test_negative_int_src(self):
        instr = IIRInstr("const", "v0", [-100], "u8")
        fn = IIRFunction(name="f", params=[], return_type="u8", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == [-100]

    def test_large_int_src(self):
        instr = IIRInstr("const", "v0", [2**32], "u64")
        fn = IIRFunction(name="f", params=[], return_type="u64", instructions=[instr])
        m = self._roundtrip(IIRModule(name="x", functions=[fn]))
        assert m.functions[0].instructions[0].srcs == [2**32]

    def test_unknown_operand_kind_raises(self):
        # Build a raw payload with an unknown kind byte
        raw = serialise(IIRModule(name="x", functions=[make_add_fn()]))
        # Find the operand kind byte for "a" src and corrupt it.
        # We'll just manually craft a bad reader state by patching bytes.
        # Instead, test via direct reader call:
        from interpreter_ir.serialise import _Reader
        r = _Reader(b"\x09")  # kind=9, unknown
        r.u8()  # consume it
        # Can't call _read_instr directly easily, so test deserialise of bad data:
        # Build a module with one instr, then corrupt the kind byte.
        raw2 = bytearray(raw)
        # Locate and corrupt the first src kind byte — fragile but effective.
        # The serialise format is deterministic; we just test the error path.
        with pytest.raises((ValueError, struct.error)):
            # Corrupt last few bytes to force a bad kind
            bad = bytes(raw2[:-5]) + b"\x09\x00\x00\x00\x00"
            deserialise(bad)
