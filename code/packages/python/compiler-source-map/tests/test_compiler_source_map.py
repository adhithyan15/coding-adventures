"""Tests for the compiler_source_map package.

Tests cover:
- SourcePosition: construction, str, frozen equality
- SourceToAst: add, lookup_by_node_id (found, not found)
- AstToIr: add, lookup_by_ast_node_id, lookup_by_ir_id
- IrToIr: add_mapping, add_deletion, lookup_by_original_id, lookup_by_new_id
- IrToMachineCode: add, lookup_by_ir_id, lookup_by_mc_offset
- SourceMapChain.new(): initial state
- SourceMapChain.add_optimizer_pass()
- SourceMapChain.source_to_mc(): forward lookup (found, not found, incomplete)
- SourceMapChain.mc_to_source(): reverse lookup (found, not found, incomplete)
- Multiple optimizer passes: identity + contraction
- Optimizer deletion: deleted IR IDs drop from forward results
- Prologue instructions with no AST mapping → reverse returns None
"""

from __future__ import annotations

import pytest

from compiler_source_map import (
    AstToIr,
    AstToIrEntry,
    IrToIr,
    IrToIrEntry,
    IrToMachineCode,
    IrToMachineCodeEntry,
    SourceMapChain,
    SourcePosition,
    SourceToAst,
    SourceToAstEntry,
)


# =============================================================================
# Test helpers
# =============================================================================


def build_test_chain() -> SourceMapChain:
    """Create a fully-populated source map chain for testing.

    The chain represents a tiny Brainfuck program "+." compiled through
    the full pipeline::

        Source: "+" at (1,1) → AST node 0 → IR [2, 3, 4, 5]
                "." at (1,2) → AST node 1 → IR [6, 7]
        Prologue IR [0, 1] have no AST mapping.
        Identity optimiser pass: every IR ID maps to itself.
        Machine code: each IR instruction has a known offset and length.

    Layout:

    ======  ======  =========  ==========
    IR ID   Opcode  MC Offset  MC Length
    ======  ======  =========  ==========
    0       LOAD_ADDR  0x00    8
    1       LOAD_IMM   0x08    4
    2       LOAD_BYTE  0x0C    8
    3       ADD_IMM    0x14    4
    4       AND_IMM    0x18    4
    5       STORE_BYTE 0x1C    8
    6       LOAD_BYTE  0x24    8
    7       SYSCALL    0x2C    8
    ======  ======  =========  ==========
    """
    chain = SourceMapChain.new()

    # Segment 1: SourceToAst
    chain.source_to_ast.add(SourcePosition("hello.bf", 1, 1, 1), ast_node_id=0)
    chain.source_to_ast.add(SourcePosition("hello.bf", 1, 2, 1), ast_node_id=1)

    # Segment 2: AstToIr
    chain.ast_to_ir.add(ast_node_id=0, ir_ids=[2, 3, 4, 5])  # "+" → 4 instructions
    chain.ast_to_ir.add(ast_node_id=1, ir_ids=[6, 7])         # "." → 2 instructions

    # Segment 3: IrToIr (identity pass — every IR maps to itself)
    identity = IrToIr(pass_name="identity")
    for i in range(8):
        identity.add_mapping(i, [i])
    chain.add_optimizer_pass(identity)

    # Segment 4: IrToMachineCode
    mc = IrToMachineCode()
    mc.add(ir_id=0, mc_offset=0x00, mc_length=8)
    mc.add(ir_id=1, mc_offset=0x08, mc_length=4)
    mc.add(ir_id=2, mc_offset=0x0C, mc_length=8)
    mc.add(ir_id=3, mc_offset=0x14, mc_length=4)
    mc.add(ir_id=4, mc_offset=0x18, mc_length=4)
    mc.add(ir_id=5, mc_offset=0x1C, mc_length=8)
    mc.add(ir_id=6, mc_offset=0x24, mc_length=8)
    mc.add(ir_id=7, mc_offset=0x2C, mc_length=8)
    chain.ir_to_machine_code = mc

    return chain


# =============================================================================
# SourcePosition tests
# =============================================================================


class TestSourcePosition:
    """Tests for SourcePosition."""

    def test_construction(self) -> None:
        pos = SourcePosition(file="test.bf", line=3, column=7, length=1)
        assert pos.file == "test.bf"
        assert pos.line == 3
        assert pos.column == 7
        assert pos.length == 1

    def test_str(self) -> None:
        pos = SourcePosition(file="test.bf", line=3, column=7, length=1)
        assert str(pos) == "test.bf:3:7 (len=1)"

    def test_str_multi_char(self) -> None:
        pos = SourcePosition(file="prog.bas", line=2, column=1, length=5)
        assert str(pos) == "prog.bas:2:1 (len=5)"

    def test_equality(self) -> None:
        p1 = SourcePosition("a.bf", 1, 1, 1)
        p2 = SourcePosition("a.bf", 1, 1, 1)
        assert p1 == p2

    def test_inequality(self) -> None:
        p1 = SourcePosition("a.bf", 1, 1, 1)
        p2 = SourcePosition("a.bf", 1, 2, 1)
        assert p1 != p2

    def test_frozen(self) -> None:
        """SourcePosition is immutable (frozen dataclass)."""
        pos = SourcePosition("a.bf", 1, 1, 1)
        with pytest.raises(Exception):
            pos.line = 2  # type: ignore[misc]

    def test_hashable(self) -> None:
        """SourcePosition is hashable and can be used in sets."""
        p1 = SourcePosition("a.bf", 1, 1, 1)
        p2 = SourcePosition("a.bf", 1, 2, 1)
        p3 = SourcePosition("a.bf", 1, 1, 1)  # duplicate of p1
        s = {p1, p2, p3}
        assert len(s) == 2


# =============================================================================
# SourceToAst tests
# =============================================================================


class TestSourceToAst:
    """Tests for SourceToAst segment."""

    def test_add_single_entry(self) -> None:
        s = SourceToAst()
        pos = SourcePosition("a.bf", 1, 1, 1)
        s.add(pos, ast_node_id=42)
        assert len(s.entries) == 1
        assert s.entries[0].ast_node_id == 42
        assert s.entries[0].pos == pos

    def test_add_multiple_entries(self) -> None:
        s = SourceToAst()
        s.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=10)
        s.add(SourcePosition("a.bf", 1, 2, 1), ast_node_id=11)
        assert len(s.entries) == 2

    def test_lookup_by_node_id_found(self) -> None:
        s = SourceToAst()
        s.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=10)
        s.add(SourcePosition("a.bf", 1, 2, 1), ast_node_id=11)
        pos = s.lookup_by_node_id(11)
        assert pos is not None
        assert pos.column == 2

    def test_lookup_by_node_id_not_found(self) -> None:
        s = SourceToAst()
        s.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=10)
        pos = s.lookup_by_node_id(999)
        assert pos is None

    def test_lookup_by_node_id_empty(self) -> None:
        s = SourceToAst()
        assert s.lookup_by_node_id(0) is None

    def test_entry_type(self) -> None:
        """SourceToAstEntry has the right fields."""
        entry = SourceToAstEntry(
            pos=SourcePosition("x.bf", 1, 1, 1),
            ast_node_id=5,
        )
        assert entry.ast_node_id == 5


# =============================================================================
# AstToIr tests
# =============================================================================


class TestAstToIr:
    """Tests for AstToIr segment."""

    def test_add_single(self) -> None:
        a = AstToIr()
        a.add(ast_node_id=0, ir_ids=[10, 11, 12])
        assert len(a.entries) == 1
        assert a.entries[0].ir_ids == [10, 11, 12]

    def test_add_multiple(self) -> None:
        a = AstToIr()
        a.add(5, [20, 21])
        a.add(6, [22])
        assert len(a.entries) == 2

    def test_lookup_by_ast_node_id_found(self) -> None:
        a = AstToIr()
        a.add(5, [20, 21])
        a.add(6, [22])
        ids = a.lookup_by_ast_node_id(5)
        assert ids == [20, 21]

    def test_lookup_by_ast_node_id_not_found(self) -> None:
        a = AstToIr()
        a.add(5, [20, 21])
        ids = a.lookup_by_ast_node_id(999)
        assert ids is None

    def test_lookup_by_ir_id_found(self) -> None:
        a = AstToIr()
        a.add(5, [20, 21])
        a.add(6, [22])
        # IR ID 21 belongs to AST node 5
        assert a.lookup_by_ir_id(21) == 5
        # IR ID 22 belongs to AST node 6
        assert a.lookup_by_ir_id(22) == 6

    def test_lookup_by_ir_id_not_found(self) -> None:
        a = AstToIr()
        a.add(5, [20, 21])
        assert a.lookup_by_ir_id(999) == -1

    def test_lookup_by_ir_id_empty(self) -> None:
        a = AstToIr()
        assert a.lookup_by_ir_id(0) == -1

    def test_entry_type(self) -> None:
        entry = AstToIrEntry(ast_node_id=3, ir_ids=[7, 8, 9])
        assert entry.ast_node_id == 3
        assert len(entry.ir_ids) == 3


# =============================================================================
# IrToIr tests
# =============================================================================


class TestIrToIr:
    """Tests for IrToIr segment."""

    def test_identity_pass(self) -> None:
        m = IrToIr(pass_name="identity")
        for i in range(5):
            m.add_mapping(i, [i])

        for i in range(5):
            ids = m.lookup_by_original_id(i)
            assert ids == [i], f"Identity pass: ID {i} should map to [{i}]"

    def test_reverse_lookup(self) -> None:
        m = IrToIr(pass_name="identity")
        for i in range(5):
            m.add_mapping(i, [i])

        for i in range(5):
            orig = m.lookup_by_new_id(i)
            assert orig == i, f"Reverse: new ID {i} should come from {i}"

    def test_contraction(self) -> None:
        """A contraction pass folds IDs 7, 8, 9 into ID 100."""
        m = IrToIr(pass_name="contraction")
        m.add_mapping(7, [100])
        m.add_mapping(8, [100])
        m.add_mapping(9, [100])

        for orig in [7, 8, 9]:
            ids = m.lookup_by_original_id(orig)
            assert ids == [100], f"Contraction: ID {orig} should map to [100]"

        # Reverse: 100 maps back to the first original found (7)
        assert m.lookup_by_new_id(100) == 7

    def test_deletion(self) -> None:
        m = IrToIr(pass_name="dead_store")
        m.add_mapping(1, [1])  # preserved
        m.add_deletion(2)      # deleted

        # ID 1 is preserved
        assert m.lookup_by_original_id(1) == [1]

        # ID 2 is deleted → lookup returns None
        assert m.lookup_by_original_id(2) is None
        assert 2 in m.deleted

    def test_lookup_by_new_id_not_found(self) -> None:
        m = IrToIr(pass_name="test")
        m.add_mapping(1, [1])
        assert m.lookup_by_new_id(999) == -1

    def test_lookup_by_original_id_not_found(self) -> None:
        m = IrToIr(pass_name="test")
        m.add_mapping(1, [1])
        assert m.lookup_by_original_id(999) is None

    def test_pass_name(self) -> None:
        m = IrToIr(pass_name="my_pass")
        assert m.pass_name == "my_pass"

    def test_entry_type(self) -> None:
        entry = IrToIrEntry(original_id=5, new_ids=[10, 11])
        assert entry.original_id == 5
        assert entry.new_ids == [10, 11]

    def test_multiple_new_ids(self) -> None:
        """One original ID can map to multiple new IDs (splitting)."""
        m = IrToIr(pass_name="split")
        m.add_mapping(1, [100, 101, 102])
        ids = m.lookup_by_original_id(1)
        assert ids == [100, 101, 102]


# =============================================================================
# IrToMachineCode tests
# =============================================================================


class TestIrToMachineCode:
    """Tests for IrToMachineCode segment."""

    def test_add(self) -> None:
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        assert len(mc.entries) == 1
        assert mc.entries[0].mc_offset == 0x20
        assert mc.entries[0].mc_length == 4

    def test_lookup_by_ir_id_found(self) -> None:
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        mc.add(ir_id=6, mc_offset=0x24, mc_length=8)

        offset, length = mc.lookup_by_ir_id(5)
        assert offset == 0x20 and length == 4

    def test_lookup_by_ir_id_not_found(self) -> None:
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        offset, length = mc.lookup_by_ir_id(999)
        assert offset == -1 and length == 0

    def test_lookup_by_mc_offset_start(self) -> None:
        """Exact start of an instruction's range returns that IR ID."""
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)  # bytes 0x20..0x23
        assert mc.lookup_by_mc_offset(0x20) == 5

    def test_lookup_by_mc_offset_middle(self) -> None:
        """Middle of an instruction's range returns that IR ID."""
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        assert mc.lookup_by_mc_offset(0x22) == 5

    def test_lookup_by_mc_offset_last_byte(self) -> None:
        """Last byte of an instruction's range returns that IR ID."""
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)  # 0x23 is the last byte
        assert mc.lookup_by_mc_offset(0x23) == 5

    def test_lookup_by_mc_offset_past_end(self) -> None:
        """One byte past the end returns -1."""
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        assert mc.lookup_by_mc_offset(0x24) == -1

    def test_lookup_by_mc_offset_not_found(self) -> None:
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)
        assert mc.lookup_by_mc_offset(0xFF) == -1

    def test_multiple_entries(self) -> None:
        mc = IrToMachineCode()
        mc.add(ir_id=5, mc_offset=0x20, mc_length=4)  # 0x20..0x23
        mc.add(ir_id=6, mc_offset=0x24, mc_length=8)  # 0x24..0x2B

        assert mc.lookup_by_mc_offset(0x24) == 6
        assert mc.lookup_by_mc_offset(0x2B) == 6

    def test_entry_type(self) -> None:
        entry = IrToMachineCodeEntry(ir_id=3, mc_offset=0x14, mc_length=4)
        assert entry.ir_id == 3
        assert entry.mc_offset == 0x14
        assert entry.mc_length == 4


# =============================================================================
# SourceMapChain.new() tests
# =============================================================================


class TestSourceMapChainNew:
    """Tests for SourceMapChain.new() initial state."""

    def test_source_to_ast_not_none(self) -> None:
        chain = SourceMapChain.new()
        assert chain.source_to_ast is not None

    def test_ast_to_ir_not_none(self) -> None:
        chain = SourceMapChain.new()
        assert chain.ast_to_ir is not None

    def test_ir_to_machine_code_is_none(self) -> None:
        """IrToMachineCode starts None — the backend fills it."""
        chain = SourceMapChain.new()
        assert chain.ir_to_machine_code is None

    def test_ir_to_ir_is_empty(self) -> None:
        chain = SourceMapChain.new()
        assert chain.ir_to_ir == []

    def test_source_to_ast_empty(self) -> None:
        chain = SourceMapChain.new()
        assert chain.source_to_ast.entries == []

    def test_ast_to_ir_empty(self) -> None:
        chain = SourceMapChain.new()
        assert chain.ast_to_ir.entries == []


# =============================================================================
# SourceMapChain.add_optimizer_pass tests
# =============================================================================


class TestAddOptimizerPass:
    """Tests for adding optimizer passes to the chain."""

    def test_add_one_pass(self) -> None:
        chain = SourceMapChain.new()
        m = IrToIr(pass_name="identity")
        chain.add_optimizer_pass(m)
        assert len(chain.ir_to_ir) == 1
        assert chain.ir_to_ir[0].pass_name == "identity"

    def test_add_multiple_passes(self) -> None:
        chain = SourceMapChain.new()
        chain.add_optimizer_pass(IrToIr(pass_name="pass1"))
        chain.add_optimizer_pass(IrToIr(pass_name="pass2"))
        assert len(chain.ir_to_ir) == 2
        assert chain.ir_to_ir[1].pass_name == "pass2"


# =============================================================================
# SourceMapChain.source_to_mc (forward lookup) tests
# =============================================================================


class TestForwardLookup:
    """Tests for the forward composite lookup (source → machine code)."""

    def test_plus_maps_to_four_mc_entries(self) -> None:
        """The '+' character at (1,1) maps to 4 MC entries (IR IDs 2,3,4,5)."""
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 1, 1, 1))
        assert results is not None
        assert len(results) == 4

    def test_plus_mc_ir_ids(self) -> None:
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 1, 1, 1))
        assert results is not None
        ir_ids = {r.ir_id for r in results}
        assert ir_ids == {2, 3, 4, 5}

    def test_plus_first_mc_offset(self) -> None:
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 1, 1, 1))
        assert results is not None
        # First MC entry for '+' is at offset 0x0C (IR #2, LOAD_BYTE)
        assert results[0].mc_offset == 0x0C

    def test_dot_maps_to_two_mc_entries(self) -> None:
        """The '.' character at (1,2) maps to 2 MC entries (IR IDs 6,7)."""
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 1, 2, 1))
        assert results is not None
        assert len(results) == 2

    def test_dot_mc_ir_ids(self) -> None:
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 1, 2, 1))
        assert results is not None
        ir_ids = {r.ir_id for r in results}
        assert ir_ids == {6, 7}

    def test_missing_position_returns_none(self) -> None:
        chain = build_test_chain()
        results = chain.source_to_mc(SourcePosition("hello.bf", 99, 1, 1))
        assert results is None

    def test_incomplete_chain_returns_none(self) -> None:
        """Forward lookup returns None when IrToMachineCode is missing."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=0)
        chain.ast_to_ir.add(ast_node_id=0, ir_ids=[1, 2])
        # No ir_to_machine_code set
        result = chain.source_to_mc(SourcePosition("a.bf", 1, 1, 1))
        assert result is None

    def test_no_ast_mapping_for_source_returns_none(self) -> None:
        """Source position found in SourceToAst but not in AstToIr."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("a.bf", 1, 1, 1), ast_node_id=0)
        # ast_to_ir has no entry for ast_node_id=0
        mc = IrToMachineCode()
        chain.ir_to_machine_code = mc
        result = chain.source_to_mc(SourcePosition("a.bf", 1, 1, 1))
        assert result is None


# =============================================================================
# SourceMapChain.mc_to_source (reverse lookup) tests
# =============================================================================


class TestReverseLookup:
    """Tests for the reverse composite lookup (machine code → source)."""

    def test_0x14_maps_to_plus(self) -> None:
        """MC offset 0x14 → IR #3 (ADD_IMM) → AST node 0 → '+' at (1,1)."""
        chain = build_test_chain()
        pos = chain.mc_to_source(0x14)
        assert pos is not None
        assert pos.file == "hello.bf"
        assert pos.line == 1
        assert pos.column == 1

    def test_0x2C_maps_to_dot(self) -> None:
        """MC offset 0x2C → IR #7 (SYSCALL) → AST node 1 → '.' at (1,2)."""
        chain = build_test_chain()
        pos = chain.mc_to_source(0x2C)
        assert pos is not None
        assert pos.column == 2

    def test_middle_of_instruction(self) -> None:
        """MC offset 0x0E (middle of LOAD_BYTE, IR #2) → '+' at (1,1)."""
        chain = build_test_chain()
        pos = chain.mc_to_source(0x0E)
        assert pos is not None
        assert pos.column == 1

    def test_out_of_range_returns_none(self) -> None:
        chain = build_test_chain()
        pos = chain.mc_to_source(0xFF)
        assert pos is None

    def test_incomplete_chain_returns_none(self) -> None:
        """Reverse lookup returns None when IrToMachineCode is missing."""
        chain = SourceMapChain.new()
        pos = chain.mc_to_source(0)
        assert pos is None

    def test_prologue_maps_to_none(self) -> None:
        """Prologue instructions (IR #0, #1) have no AST mapping → None."""
        chain = build_test_chain()
        # IR #0 is the prologue LOAD_ADDR at offset 0x00
        pos = chain.mc_to_source(0x00)
        assert pos is None


# =============================================================================
# Multiple optimizer passes
# =============================================================================


class TestMultipleOptimizerPasses:
    """Tests for chains with more than one optimizer pass."""

    def test_two_passes_forward(self) -> None:
        """Forward lookup through two passes: identity then contraction."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("t.bf", 1, 1, 1), ast_node_id=0)
        chain.ast_to_ir.add(ast_node_id=0, ir_ids=[1, 2, 3])

        # Pass 1: identity (1→1, 2→2, 3→3)
        pass1 = IrToIr(pass_name="identity")
        pass1.add_mapping(1, [1])
        pass1.add_mapping(2, [2])
        pass1.add_mapping(3, [3])
        chain.add_optimizer_pass(pass1)

        # Pass 2: contraction (1,2,3 → 100)
        pass2 = IrToIr(pass_name="contraction")
        pass2.add_mapping(1, [100])
        pass2.add_mapping(2, [100])
        pass2.add_mapping(3, [100])
        chain.add_optimizer_pass(pass2)

        mc = IrToMachineCode()
        mc.add(ir_id=100, mc_offset=0x00, mc_length=4)
        chain.ir_to_machine_code = mc

        results = chain.source_to_mc(SourcePosition("t.bf", 1, 1, 1))
        assert results is not None and len(results) > 0

    def test_two_passes_reverse(self) -> None:
        """Reverse lookup through two passes: 100 → 1 (via pass2) → 1 (via pass1)."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("t.bf", 1, 1, 1), ast_node_id=0)
        chain.ast_to_ir.add(ast_node_id=0, ir_ids=[1, 2, 3])

        pass1 = IrToIr(pass_name="identity")
        pass1.add_mapping(1, [1])
        pass1.add_mapping(2, [2])
        pass1.add_mapping(3, [3])
        chain.add_optimizer_pass(pass1)

        pass2 = IrToIr(pass_name="contraction")
        pass2.add_mapping(1, [100])
        pass2.add_mapping(2, [100])
        pass2.add_mapping(3, [100])
        chain.add_optimizer_pass(pass2)

        mc = IrToMachineCode()
        mc.add(ir_id=100, mc_offset=0x00, mc_length=4)
        chain.ir_to_machine_code = mc

        pos = chain.mc_to_source(0x00)
        assert pos is not None
        assert pos.line == 1 and pos.column == 1


# =============================================================================
# Optimizer deletion
# =============================================================================


class TestOptimizerDeletion:
    """Tests for chains where some IR instructions are deleted."""

    def test_deletion_removes_ir_from_forward_results(self) -> None:
        """Deleted IR IDs do not appear in the forward lookup results."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("t.bf", 1, 1, 1), ast_node_id=0)
        chain.ast_to_ir.add(ast_node_id=0, ir_ids=[1, 2])

        # Optimizer deletes IR ID 2, preserves IR ID 1
        pass_ = IrToIr(pass_name="dead_store")
        pass_.add_mapping(1, [1])
        pass_.add_deletion(2)
        chain.add_optimizer_pass(pass_)

        mc = IrToMachineCode()
        mc.add(ir_id=1, mc_offset=0x00, mc_length=4)
        chain.ir_to_machine_code = mc

        results = chain.source_to_mc(SourcePosition("t.bf", 1, 1, 1))
        assert results is not None
        assert len(results) == 1
        assert results[0].ir_id == 1

    def test_all_deleted_returns_none(self) -> None:
        """If all IR IDs are deleted, forward lookup returns None."""
        chain = SourceMapChain.new()
        chain.source_to_ast.add(SourcePosition("t.bf", 1, 1, 1), ast_node_id=0)
        chain.ast_to_ir.add(ast_node_id=0, ir_ids=[1, 2])

        pass_ = IrToIr(pass_name="dead_store")
        pass_.add_deletion(1)
        pass_.add_deletion(2)
        chain.add_optimizer_pass(pass_)

        mc = IrToMachineCode()
        chain.ir_to_machine_code = mc

        results = chain.source_to_mc(SourcePosition("t.bf", 1, 1, 1))
        assert results is None
