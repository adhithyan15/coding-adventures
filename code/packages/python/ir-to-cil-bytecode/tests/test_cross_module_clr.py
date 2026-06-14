"""Tests for TW04 Phase 4e — CLR cross-module CALL support.

Coverage targets
================
* ``CILBackendConfig.extra_callable_labels`` — force-include exported
  dep-module functions that no local CALL targets.
* ``CILBackendConfig.external_method_tokens`` — pre-assigned MethodDef
  token lookup for cross-module CALL instructions.
* ``SequentialCILTokenProvider(method_token_offset=N)`` — dep-module
  providers that start their MethodDef token numbering at an offset.
* ``_discover_callable_regions`` — cross-module label skipping.
* CALL lowering — cross-module branch in ``_emit_instruction``.
"""

from __future__ import annotations

import pytest

from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILBackendError,
    lower_ir_to_cil_bytecode,
)
from ir_to_cil_bytecode.backend import SequentialCILTokenProvider


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _reg(n: int) -> IrRegister:
    return IrRegister(index=n)


def _imm(v: int) -> IrImmediate:
    return IrImmediate(value=v)


def _lbl(name: str) -> IrLabel:
    return IrLabel(name=name)


def _simple_program(extra_instructions: list[IrInstruction]) -> IrProgram:
    """Build a minimal IrProgram with a ``main`` region and extra instructions."""
    base: list[IrInstruction] = [
        IrInstruction(IrOp.LABEL, (_lbl("main"),)),
        IrInstruction(IrOp.LOAD_IMM, (_reg(1), _imm(0))),
    ]
    return IrProgram(
        entry_label="main",
        instructions=base + extra_instructions + [
            IrInstruction(IrOp.RET, ()),
        ],
    )


# ---------------------------------------------------------------------------
# SequentialCILTokenProvider — method_token_offset
# ---------------------------------------------------------------------------


class TestSequentialTokenOffset:
    """SequentialCILTokenProvider honours ``method_token_offset``."""

    def test_zero_offset_starts_at_0x06000001(self) -> None:
        provider = SequentialCILTokenProvider(("main", "add"), method_token_offset=0)
        assert provider.method_token("main") == 0x06000001
        assert provider.method_token("add") == 0x06000002

    def test_nonzero_offset_shifts_base(self) -> None:
        """Dep module with 3 entry-module methods before it gets offset=3."""
        provider = SequentialCILTokenProvider(("add", "sub"), method_token_offset=3)
        assert provider.method_token("add") == 0x06000004
        assert provider.method_token("sub") == 0x06000005

    def test_helper_tokens_are_unaffected_by_offset(self) -> None:
        """MemberRef tokens (0x0A prefix) always start at 0x0A000001."""
        from ir_to_cil_bytecode.backend import CILHelper
        provider = SequentialCILTokenProvider(("main",), method_token_offset=10)
        assert provider.helper_token(CILHelper.SYSCALL) == 0x0A000005

    def test_unknown_method_raises(self) -> None:
        provider = SequentialCILTokenProvider(("main",), method_token_offset=0)
        with pytest.raises(CILBackendError, match="Unknown CIL method token"):
            provider.method_token("nonexistent")

    def test_large_offset(self) -> None:
        provider = SequentialCILTokenProvider(("f",), method_token_offset=100)
        assert provider.method_token("f") == 0x06000065  # 0x06000001 + 100


# ---------------------------------------------------------------------------
# extra_callable_labels — force-include exported functions
# ---------------------------------------------------------------------------


class TestExtraCallableLabels:
    """``extra_callable_labels`` forces exported functions into the TypeDef."""

    def _make_dep_module_program(self) -> IrProgram:
        """Dep module: 'main' stub + exported 'add' + 'sub'.

        Neither 'add' nor 'sub' is a CALL target locally — they're only
        called from other modules.  Without ``extra_callable_labels`` the
        backend would omit them.
        """
        return IrProgram(
            entry_label="main",
            instructions=[
                # main region (stub):
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                IrInstruction(IrOp.LOAD_IMM, (_reg(1), _imm(0))),
                IrInstruction(IrOp.RET, ()),
                # 'add' region:
                IrInstruction(IrOp.LABEL, (_lbl("add"),)),
                IrInstruction(IrOp.ADD, (_reg(1), _reg(2), _reg(3))),
                IrInstruction(IrOp.RET, ()),
                # 'sub' region:
                IrInstruction(IrOp.LABEL, (_lbl("sub"),)),
                IrInstruction(IrOp.SUB, (_reg(1), _reg(2), _reg(3))),
                IrInstruction(IrOp.RET, ()),
            ],
        )

    def test_without_extra_callable_labels_only_main_emitted(self) -> None:
        """Without the hint, only 'main' is discoverable (no local CALLs)."""
        prog = self._make_dep_module_program()
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(call_register_count=None),
        )
        # Only 'main' is a callable — 'add' and 'sub' are omitted.
        assert artifact.callable_labels == ("main",)

    def test_with_extra_callable_labels_exports_are_included(self) -> None:
        """With the hint, 'add' and 'sub' appear in the emitted methods."""
        prog = self._make_dep_module_program()
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                extra_callable_labels=("add", "sub"),
            ),
        )
        assert "add" in artifact.callable_labels
        assert "sub" in artifact.callable_labels

    def test_extra_callable_labels_preserves_ir_order(self) -> None:
        """Methods appear in IR label-position order, not set-insertion order."""
        prog = self._make_dep_module_program()
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                extra_callable_labels=("sub", "add"),  # reversed
            ),
        )
        names = artifact.callable_labels
        # 'add' label appears BEFORE 'sub' in the IR
        assert names.index("add") < names.index("sub")

    def test_extra_callable_labels_unknown_label_raises(self) -> None:
        """A label in ``extra_callable_labels`` that doesn't exist in the IR errors."""
        prog = _simple_program([])
        with pytest.raises(CILBackendError, match="Missing callable labels"):
            lower_ir_to_cil_bytecode(
                prog,
                CILBackendConfig(extra_callable_labels=("nonexistent",)),
            )


# ---------------------------------------------------------------------------
# external_method_tokens — cross-module CALL lowering
# ---------------------------------------------------------------------------


class TestExternalMethodTokens:
    """Cross-module CALL lowering via ``external_method_tokens``."""

    def _make_cross_module_program(self, cross_label: str) -> IrProgram:
        """Main program that calls a cross-module function."""
        return IrProgram(
            entry_label="main",
            instructions=[
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                # Marshal arg: move 10 into param register 2
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(10))),
                # Cross-module CALL — label contains '/'
                IrInstruction(IrOp.CALL, (_lbl(cross_label),)),
                # Store result from reg 1 (return slot)
                IrInstruction(IrOp.ADD_IMM, (_reg(1), _reg(1), _imm(0))),
                IrInstruction(IrOp.RET, ()),
            ],
        )

    def test_cross_module_call_without_token_raises(self) -> None:
        """A cross-module CALL with no pre-assigned token raises at lowering."""
        prog = self._make_cross_module_program("a/math/add")
        with pytest.raises(CILBackendError, match="No pre-assigned token"):
            lower_ir_to_cil_bytecode(
                prog,
                CILBackendConfig(call_register_count=None),
            )

    def test_cross_module_call_with_token_succeeds(self) -> None:
        """A cross-module CALL with a pre-assigned token lowers without error."""
        prog = self._make_cross_module_program("a/math/add")
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={"a/math/add": 0x06000004},
            ),
        )
        assert artifact.entry_label == "main"

    def test_cross_module_call_token_appears_in_bytecode(self) -> None:
        """The pre-assigned MethodDef token appears in the emitted bytecode.

        CIL ``call`` opcode: 0x28 followed by 4-byte token (little-endian).
        Token ``0x06000004`` → bytes ``[0x28, 0x04, 0x00, 0x00, 0x06]``.
        """
        prog = self._make_cross_module_program("b/util/double")
        token = 0x06000007
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={"b/util/double": token},
            ),
        )
        body = artifact.entry_method.body
        # Search for the 'call' instruction sequence in the method body.
        expected_token_bytes = token.to_bytes(4, "little")
        call_opcode = 0x28
        found = any(
            body[i] == call_opcode and body[i + 1:i + 5] == expected_token_bytes
            for i in range(len(body) - 4)
        )
        assert found, (
            f"Expected call 0x{token:08X} in bytecode, got: {body.hex()}"
        )

    def test_multiple_cross_module_calls(self) -> None:
        """Multiple cross-module CALL targets with different tokens all work."""
        prog = IrProgram(
            entry_label="main",
            instructions=[
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(1))),
                IrInstruction(IrOp.CALL, (_lbl("mod1/fn1"),)),
                IrInstruction(IrOp.ADD_IMM, (_reg(10), _reg(1), _imm(0))),
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(2))),
                IrInstruction(IrOp.CALL, (_lbl("mod2/fn2"),)),
                IrInstruction(IrOp.ADD, (_reg(1), _reg(10), _reg(1))),
                IrInstruction(IrOp.RET, ()),
            ],
        )
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={
                    "mod1/fn1": 0x06000003,
                    "mod2/fn2": 0x06000005,
                },
            ),
        )
        body = artifact.entry_method.body
        # Both tokens should appear in the bytecode.
        for token in (0x06000003, 0x06000005):
            token_bytes = token.to_bytes(4, "little")
            assert any(
                body[i] == 0x28 and body[i + 1:i + 5] == token_bytes
                for i in range(len(body) - 4)
            ), f"Token 0x{token:08X} not found in bytecode"

    def test_cross_module_call_does_not_appear_in_callable_regions(self) -> None:
        """Cross-module call targets are NOT added to the local callable regions."""
        prog = self._make_cross_module_program("x/y/z")
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={"x/y/z": 0x06000002},
            ),
        )
        # Only "main" is a callable region — "x/y/z" is external.
        assert artifact.callable_labels == ("main",)

    def test_local_and_cross_module_calls_coexist(self) -> None:
        """Local CALL and cross-module CALL can appear in the same program."""
        prog = IrProgram(
            entry_label="main",
            instructions=[
                IrInstruction(IrOp.LABEL, (_lbl("helper"),)),
                IrInstruction(IrOp.ADD_IMM, (_reg(1), _reg(2), _imm(0))),
                IrInstruction(IrOp.RET, ()),
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                # Local call to 'helper':
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(5))),
                IrInstruction(IrOp.CALL, (_lbl("helper"),)),
                # Cross-module call to 'ext/mod/fn':
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(3))),
                IrInstruction(IrOp.CALL, (_lbl("ext/mod/fn"),)),
                IrInstruction(IrOp.ADD, (_reg(1), _reg(1), _reg(1))),
                IrInstruction(IrOp.RET, ()),
            ],
        )
        artifact = lower_ir_to_cil_bytecode(
            prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={"ext/mod/fn": 0x06000009},
            ),
        )
        # 'helper' and 'main' are local callables; 'ext/mod/fn' is external.
        assert "helper" in artifact.callable_labels
        assert "main" in artifact.callable_labels
        assert "ext/mod/fn" not in artifact.callable_labels


# ---------------------------------------------------------------------------
# Combined: extra_callable_labels + external_method_tokens
# ---------------------------------------------------------------------------


class TestExtraCallableAndExternalTokensTogether:
    """Verify both fields work correctly in combination (dep module scenario)."""

    def test_dep_module_with_exports_and_calls_to_entry(self) -> None:
        """Dep module exports 'add', 'sub'; entry module uses a cross-module token."""
        # Dep module program: exports add + sub; no local cross-module calls.
        dep_prog = IrProgram(
            entry_label="main",
            instructions=[
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                IrInstruction(IrOp.LOAD_IMM, (_reg(1), _imm(0))),
                IrInstruction(IrOp.RET, ()),
                IrInstruction(IrOp.LABEL, (_lbl("add"),)),
                IrInstruction(IrOp.ADD, (_reg(1), _reg(2), _reg(3))),
                IrInstruction(IrOp.RET, ()),
                IrInstruction(IrOp.LABEL, (_lbl("sub"),)),
                IrInstruction(IrOp.SUB, (_reg(1), _reg(2), _reg(3))),
                IrInstruction(IrOp.RET, ()),
            ],
        )
        # Compile dep module with offset=2 (entry module has 2 main methods)
        dep_artifact = lower_ir_to_cil_bytecode(
            dep_prog,
            CILBackendConfig(
                call_register_count=None,
                extra_callable_labels=("add", "sub"),
            ),
            token_provider=SequentialCILTokenProvider(
                ("main", "add", "sub"),
                method_token_offset=2,
            ),
        )
        # dep module has 3 methods (main, add, sub) all starting at offset 2:
        # main → 0x06000003, add → 0x06000004, sub → 0x06000005
        assert "add" in dep_artifact.callable_labels
        assert "sub" in dep_artifact.callable_labels
        assert len(dep_artifact.callable_labels) == 3

    def test_entry_module_calling_dep_module(self) -> None:
        """Entry module calls dep module 'a_math/add' via external token."""
        entry_prog = IrProgram(
            entry_label="main",
            instructions=[
                IrInstruction(IrOp.LABEL, (_lbl("main"),)),
                IrInstruction(IrOp.LOAD_IMM, (_reg(2), _imm(40))),
                IrInstruction(IrOp.LOAD_IMM, (_reg(3), _imm(2))),
                IrInstruction(IrOp.CALL, (_lbl("a/math/add"),)),
                IrInstruction(IrOp.ADD_IMM, (_reg(1), _reg(1), _imm(0))),
                IrInstruction(IrOp.RET, ()),
            ],
        )
        # Token for "a/math/add": dep module's 'add' is at row 2 (offset=1):
        # entry has 1 method (main → row 1), dep add → row 2 → token 0x06000002
        artifact = lower_ir_to_cil_bytecode(
            entry_prog,
            CILBackendConfig(
                call_register_count=None,
                external_method_tokens={"a/math/add": 0x06000002},
            ),
            token_provider=SequentialCILTokenProvider(
                ("main",),
                method_token_offset=0,
            ),
        )
        body = artifact.entry_method.body
        # 'call 0x06000002' should appear in the body
        target = (0x06000002).to_bytes(4, "little")
        found = any(
            body[i] == 0x28 and body[i + 1:i + 5] == target
            for i in range(len(body) - 4)
        )
        assert found
