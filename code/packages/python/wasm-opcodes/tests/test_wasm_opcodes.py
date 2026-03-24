"""Tests for wasm-opcodes.

Test plan:
  1.  Total opcode count is exactly 172.
  2.  Byte lookup: get_opcode(0x6A) returns i32.add.
  3.  Name lookup: get_opcode_by_name("i32.add") returns correct info.
  4.  Stack effects: i32.add pops 2, pushes 1.
  5.  i32.const has immediate "i32".
  6.  Memory loads have immediate "memarg".
  7.  Control instructions have correct immediates.
  8.  Unknown byte returns None.
  9.  Unknown name returns None.
  10. All opcodes have non-empty name and valid category.
  11. All opcode bytes are unique (no duplicates).
  12. All opcode names are unique.
  13. OPCODES and OPCODES_BY_NAME have same count.
  14. Version string is present.
  15. Conversion instructions have pop=1 push=1.
  16. select pops 3, pushes 1.
  17. Memory stores pop 2, push 0.
  18. OPCODES_BY_NAME is consistent with OPCODES.
"""

import wasm_opcodes
from wasm_opcodes import (
    OPCODES,
    OPCODES_BY_NAME,
    __version__,
    get_opcode,
    get_opcode_by_name,
)

# ---------------------------------------------------------------------------
# VALID CATEGORIES — the complete set of category strings used in this package
# ---------------------------------------------------------------------------
VALID_CATEGORIES = {
    "control",
    "parametric",
    "variable",
    "memory",
    "numeric_i32",
    "numeric_i64",
    "numeric_f32",
    "numeric_f64",
    "conversion",
}


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


class TestOpcodeCount:
    """The table must contain exactly 183 entries — no more, no less."""

    def test_total_count(self) -> None:
        # Count the instructions in each group from the spec:
        #   13 control + 2 parametric + 5 variable
        #   + 14 loads + 9 stores + 2 memory_mgmt
        #   + 30 i32 + 30 i64 + 21 f32 + 21 f64
        #   + 25 conversion
        #   = 172 total
        assert len(OPCODES) == 172

    def test_by_name_same_count(self) -> None:
        assert len(OPCODES_BY_NAME) == len(OPCODES)


class TestUniqueness:
    """Opcode bytes and names must be unique across the entire table."""

    def test_opcode_bytes_unique(self) -> None:
        # If bytes were duplicated, the dict comprehension would silently keep
        # only the last entry. Verify by counting the raw _RAW_TABLE length too.
        assert len(OPCODES) == len(wasm_opcodes._RAW_TABLE)

    def test_opcode_names_unique(self) -> None:
        all_names = [row[0] for row in wasm_opcodes._RAW_TABLE]
        assert len(all_names) == len(set(all_names))


class TestByteLookup:
    """get_opcode() returns correct entries for known bytes."""

    def test_i32_add(self) -> None:
        info = get_opcode(0x6A)
        assert info is not None
        assert info.name == "i32.add"
        assert info.opcode == 0x6A
        assert info.category == "numeric_i32"

    def test_unreachable(self) -> None:
        info = get_opcode(0x00)
        assert info is not None
        assert info.name == "unreachable"

    def test_nop(self) -> None:
        info = get_opcode(0x01)
        assert info is not None
        assert info.name == "nop"

    def test_memory_grow(self) -> None:
        info = get_opcode(0x40)
        assert info is not None
        assert info.name == "memory.grow"

    def test_i32_reinterpret_f32(self) -> None:
        info = get_opcode(0xBC)
        assert info is not None
        assert info.name == "i32.reinterpret_f32"

    def test_f64_reinterpret_i64(self) -> None:
        info = get_opcode(0xBF)
        assert info is not None
        assert info.name == "f64.reinterpret_i64"

    def test_unknown_byte_returns_none(self) -> None:
        assert get_opcode(0xFF) is None

    def test_another_unknown_byte(self) -> None:
        assert get_opcode(0xC0) is None

    def test_dict_lookup_same_as_function(self) -> None:
        assert OPCODES[0x6A] == get_opcode(0x6A)


class TestNameLookup:
    """get_opcode_by_name() returns correct entries for known mnemonics."""

    def test_i32_add(self) -> None:
        info = get_opcode_by_name("i32.add")
        assert info is not None
        assert info.opcode == 0x6A
        assert info.stack_pop == 2
        assert info.stack_push == 1

    def test_local_get(self) -> None:
        info = get_opcode_by_name("local.get")
        assert info is not None
        assert info.opcode == 0x20
        assert info.category == "variable"

    def test_unknown_name_returns_none(self) -> None:
        assert get_opcode_by_name("not_a_real_op") is None

    def test_empty_string_returns_none(self) -> None:
        assert get_opcode_by_name("") is None

    def test_dict_lookup_same_as_function(self) -> None:
        assert OPCODES_BY_NAME["i32.add"] == get_opcode_by_name("i32.add")


class TestStackEffects:
    """Verify pop/push counts for representative instructions."""

    def test_i32_add_pops_2_pushes_1(self) -> None:
        info = get_opcode(0x6A)
        assert info is not None
        assert info.stack_pop == 2
        assert info.stack_push == 1

    def test_i32_const_pops_0_pushes_1(self) -> None:
        info = get_opcode_by_name("i32.const")
        assert info is not None
        assert info.stack_pop == 0
        assert info.stack_push == 1

    def test_drop_pops_1_pushes_0(self) -> None:
        info = get_opcode_by_name("drop")
        assert info is not None
        assert info.stack_pop == 1
        assert info.stack_push == 0

    def test_select_pops_3_pushes_1(self) -> None:
        info = get_opcode_by_name("select")
        assert info is not None
        assert info.stack_pop == 3
        assert info.stack_push == 1

    def test_local_get_pops_0_pushes_1(self) -> None:
        info = get_opcode_by_name("local.get")
        assert info is not None
        assert info.stack_pop == 0
        assert info.stack_push == 1

    def test_local_set_pops_1_pushes_0(self) -> None:
        info = get_opcode_by_name("local.set")
        assert info is not None
        assert info.stack_pop == 1
        assert info.stack_push == 0

    def test_local_tee_pops_1_pushes_1(self) -> None:
        info = get_opcode_by_name("local.tee")
        assert info is not None
        assert info.stack_pop == 1
        assert info.stack_push == 1

    def test_memory_size_pops_0_pushes_1(self) -> None:
        info = get_opcode_by_name("memory.size")
        assert info is not None
        assert info.stack_pop == 0
        assert info.stack_push == 1

    def test_memory_grow_pops_1_pushes_1(self) -> None:
        info = get_opcode_by_name("memory.grow")
        assert info is not None
        assert info.stack_pop == 1
        assert info.stack_push == 1

    def test_all_memory_loads_pop1_push1(self) -> None:
        load_names = [
            "i32.load", "i64.load", "f32.load", "f64.load",
            "i32.load8_s", "i32.load8_u", "i32.load16_s", "i32.load16_u",
            "i64.load8_s", "i64.load8_u", "i64.load16_s", "i64.load16_u",
            "i64.load32_s", "i64.load32_u",
        ]
        for name in load_names:
            info = get_opcode_by_name(name)
            assert info is not None, f"{name} not found"
            assert info.stack_pop == 1, f"{name}: expected pop=1"
            assert info.stack_push == 1, f"{name}: expected push=1"

    def test_all_memory_stores_pop2_push0(self) -> None:
        store_names = [
            "i32.store", "i64.store", "f32.store", "f64.store",
            "i32.store8", "i32.store16", "i64.store8", "i64.store16", "i64.store32",
        ]
        for name in store_names:
            info = get_opcode_by_name(name)
            assert info is not None, f"{name} not found"
            assert info.stack_pop == 2, f"{name}: expected pop=2"
            assert info.stack_push == 0, f"{name}: expected push=0"

    def test_all_conversions_pop1_push1(self) -> None:
        conversion_names = [
            "i32.wrap_i64", "i32.trunc_f32_s", "i32.trunc_f32_u",
            "i32.trunc_f64_s", "i32.trunc_f64_u",
            "i64.extend_i32_s", "i64.extend_i32_u",
            "i64.trunc_f32_s", "i64.trunc_f32_u",
            "i64.trunc_f64_s", "i64.trunc_f64_u",
            "f32.convert_i32_s", "f32.convert_i32_u",
            "f32.convert_i64_s", "f32.convert_i64_u",
            "f32.demote_f64", "f64.convert_i32_s", "f64.convert_i32_u",
            "f64.convert_i64_s", "f64.convert_i64_u", "f64.promote_f32",
            "i32.reinterpret_f32", "i64.reinterpret_f64",
            "f32.reinterpret_i32", "f64.reinterpret_i64",
        ]
        for name in conversion_names:
            info = get_opcode_by_name(name)
            assert info is not None, f"{name} not found"
            assert info.stack_pop == 1, f"{name}: expected pop=1"
            assert info.stack_push == 1, f"{name}: expected push=1"


class TestImmediates:
    """Verify immediate encodings for representative instructions."""

    def test_i32_const_has_i32_immediate(self) -> None:
        info = get_opcode_by_name("i32.const")
        assert info is not None
        assert info.immediates == ("i32",)

    def test_i64_const_has_i64_immediate(self) -> None:
        info = get_opcode_by_name("i64.const")
        assert info is not None
        assert info.immediates == ("i64",)

    def test_f32_const_has_f32_immediate(self) -> None:
        info = get_opcode_by_name("f32.const")
        assert info is not None
        assert info.immediates == ("f32",)

    def test_f64_const_has_f64_immediate(self) -> None:
        info = get_opcode_by_name("f64.const")
        assert info is not None
        assert info.immediates == ("f64",)

    def test_all_memory_loads_have_memarg(self) -> None:
        load_names = [
            "i32.load", "i64.load", "f32.load", "f64.load",
            "i32.load8_s", "i32.load8_u", "i32.load16_s", "i32.load16_u",
            "i64.load8_s", "i64.load8_u", "i64.load16_s", "i64.load16_u",
            "i64.load32_s", "i64.load32_u",
        ]
        for name in load_names:
            info = get_opcode_by_name(name)
            assert info is not None
            assert info.immediates == ("memarg",), f"{name}: expected ('memarg',)"

    def test_all_memory_stores_have_memarg(self) -> None:
        store_names = [
            "i32.store", "i64.store", "f32.store", "f64.store",
            "i32.store8", "i32.store16", "i64.store8", "i64.store16", "i64.store32",
        ]
        for name in store_names:
            info = get_opcode_by_name(name)
            assert info is not None
            assert info.immediates == ("memarg",), f"{name}: expected ('memarg',)"

    def test_br_has_labelidx_immediate(self) -> None:
        info = get_opcode_by_name("br")
        assert info is not None
        assert info.immediates == ("labelidx",)

    def test_br_if_has_labelidx_immediate(self) -> None:
        info = get_opcode_by_name("br_if")
        assert info is not None
        assert info.immediates == ("labelidx",)

    def test_br_table_has_vec_labelidx_immediate(self) -> None:
        info = get_opcode_by_name("br_table")
        assert info is not None
        assert info.immediates == ("vec_labelidx",)

    def test_call_has_funcidx_immediate(self) -> None:
        info = get_opcode_by_name("call")
        assert info is not None
        assert info.immediates == ("funcidx",)

    def test_call_indirect_has_typeidx_tableidx(self) -> None:
        info = get_opcode_by_name("call_indirect")
        assert info is not None
        assert info.immediates == ("typeidx", "tableidx")

    def test_block_has_blocktype_immediate(self) -> None:
        info = get_opcode_by_name("block")
        assert info is not None
        assert info.immediates == ("blocktype",)

    def test_loop_has_blocktype_immediate(self) -> None:
        info = get_opcode_by_name("loop")
        assert info is not None
        assert info.immediates == ("blocktype",)

    def test_if_has_blocktype_immediate(self) -> None:
        info = get_opcode_by_name("if")
        assert info is not None
        assert info.immediates == ("blocktype",)

    def test_local_get_has_localidx(self) -> None:
        info = get_opcode_by_name("local.get")
        assert info is not None
        assert info.immediates == ("localidx",)

    def test_global_get_has_globalidx(self) -> None:
        info = get_opcode_by_name("global.get")
        assert info is not None
        assert info.immediates == ("globalidx",)

    def test_memory_size_has_memidx(self) -> None:
        info = get_opcode_by_name("memory.size")
        assert info is not None
        assert info.immediates == ("memidx",)

    def test_memory_grow_has_memidx(self) -> None:
        info = get_opcode_by_name("memory.grow")
        assert info is not None
        assert info.immediates == ("memidx",)

    def test_i32_add_has_no_immediates(self) -> None:
        info = get_opcode_by_name("i32.add")
        assert info is not None
        assert info.immediates == ()

    def test_conversions_have_no_immediates(self) -> None:
        info = get_opcode_by_name("i32.wrap_i64")
        assert info is not None
        assert info.immediates == ()


class TestCategories:
    """Verify categories for representative instructions."""

    def test_all_opcodes_have_valid_category(self) -> None:
        for info in OPCODES.values():
            assert info.category in VALID_CATEGORIES, (
                f"Opcode {info.name!r} (0x{info.opcode:02X}) has invalid "
                f"category {info.category!r}"
            )

    def test_all_opcodes_have_nonempty_name(self) -> None:
        for info in OPCODES.values():
            assert info.name, f"Opcode 0x{info.opcode:02X} has empty name"

    def test_control_category(self) -> None:
        assert get_opcode_by_name("unreachable").category == "control"  # type: ignore[union-attr]
        assert get_opcode_by_name("call").category == "control"  # type: ignore[union-attr]
        assert get_opcode_by_name("br_table").category == "control"  # type: ignore[union-attr]

    def test_parametric_category(self) -> None:
        assert get_opcode_by_name("drop").category == "parametric"  # type: ignore[union-attr]
        assert get_opcode_by_name("select").category == "parametric"  # type: ignore[union-attr]

    def test_variable_category(self) -> None:
        assert get_opcode_by_name("local.get").category == "variable"  # type: ignore[union-attr]
        assert get_opcode_by_name("global.set").category == "variable"  # type: ignore[union-attr]

    def test_memory_category(self) -> None:
        assert get_opcode_by_name("i32.load").category == "memory"  # type: ignore[union-attr]
        assert get_opcode_by_name("i64.store32").category == "memory"  # type: ignore[union-attr]
        assert get_opcode_by_name("memory.size").category == "memory"  # type: ignore[union-attr]

    def test_numeric_i32_category(self) -> None:
        assert get_opcode_by_name("i32.add").category == "numeric_i32"  # type: ignore[union-attr]
        assert get_opcode_by_name("i32.const").category == "numeric_i32"  # type: ignore[union-attr]

    def test_numeric_i64_category(self) -> None:
        assert get_opcode_by_name("i64.mul").category == "numeric_i64"  # type: ignore[union-attr]
        assert get_opcode_by_name("i64.eqz").category == "numeric_i64"  # type: ignore[union-attr]

    def test_numeric_f32_category(self) -> None:
        assert get_opcode_by_name("f32.add").category == "numeric_f32"  # type: ignore[union-attr]
        assert get_opcode_by_name("f32.sqrt").category == "numeric_f32"  # type: ignore[union-attr]

    def test_numeric_f64_category(self) -> None:
        assert get_opcode_by_name("f64.div").category == "numeric_f64"  # type: ignore[union-attr]
        assert get_opcode_by_name("f64.copysign").category == "numeric_f64"  # type: ignore[union-attr]

    def test_conversion_category(self) -> None:
        assert get_opcode_by_name("i32.wrap_i64").category == "conversion"  # type: ignore[union-attr]
        assert get_opcode_by_name("f64.promote_f32").category == "conversion"  # type: ignore[union-attr]


class TestConsistency:
    """Cross-check that OPCODES and OPCODES_BY_NAME are consistent."""

    def test_every_opcode_reachable_by_name(self) -> None:
        for byte_val, info in OPCODES.items():
            by_name = OPCODES_BY_NAME.get(info.name)
            assert by_name is not None, f"Name {info.name!r} not in OPCODES_BY_NAME"
            assert by_name.opcode == byte_val

    def test_every_name_reachable_by_opcode(self) -> None:
        for name, info in OPCODES_BY_NAME.items():
            by_opcode = OPCODES.get(info.opcode)
            assert by_opcode is not None, f"Opcode 0x{info.opcode:02X} not in OPCODES"
            assert by_opcode.name == name

    def test_opcode_info_is_frozen(self) -> None:
        """OpcodeInfo is a frozen dataclass — mutations must raise an error."""
        import dataclasses
        info = get_opcode(0x6A)
        assert info is not None
        try:
            info.name = "tampered"  # type: ignore[misc]
            raise AssertionError("Expected FrozenInstanceError but no exception raised")
        except dataclasses.FrozenInstanceError:
            pass  # expected


class TestSpecificOpcodes:
    """Spot-check a selection of opcodes from every category."""

    def test_end_opcode(self) -> None:
        info = get_opcode(0x0B)
        assert info is not None
        assert info.name == "end"
        assert info.stack_pop == 0
        assert info.stack_push == 0

    def test_return_opcode(self) -> None:
        info = get_opcode(0x0F)
        assert info is not None
        assert info.name == "return"

    def test_else_opcode(self) -> None:
        info = get_opcode(0x05)
        assert info is not None
        assert info.name == "else"

    def test_i64_div_s(self) -> None:
        info = get_opcode_by_name("i64.div_s")
        assert info is not None
        assert info.opcode == 0x7F
        assert info.stack_pop == 2

    def test_f32_nearest(self) -> None:
        info = get_opcode_by_name("f32.nearest")
        assert info is not None
        assert info.opcode == 0x90

    def test_f64_promote_f32(self) -> None:
        info = get_opcode_by_name("f64.promote_f32")
        assert info is not None
        assert info.opcode == 0xBB
        assert info.category == "conversion"

    def test_i64_rotl(self) -> None:
        info = get_opcode_by_name("i64.rotl")
        assert info is not None
        assert info.opcode == 0x89

    def test_i64_rotr(self) -> None:
        info = get_opcode_by_name("i64.rotr")
        assert info is not None
        assert info.opcode == 0x8A

    def test_i32_popcnt(self) -> None:
        info = get_opcode_by_name("i32.popcnt")
        assert info is not None
        assert info.opcode == 0x69
        assert info.stack_pop == 1
        assert info.stack_push == 1
