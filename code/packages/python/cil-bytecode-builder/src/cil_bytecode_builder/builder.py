"""Build Common Intermediate Language method-body bytecode.

The CLR stores executable method bodies as CIL byte streams. Most opcodes are
one byte, a few use the ``0xFE`` prefix, and branch operands are relative to the
instruction immediately after the branch. This module keeps those details in
one reusable place so compiler backends can emit readable operations instead of
hand-packed bytes.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from enum import IntEnum, StrEnum

_INT8_MIN = -128
_INT8_MAX = 127
_INT32_MIN = -(2**31)
_INT32_MAX = 2**31 - 1
_UINT16_MAX = 2**16 - 1
_UINT32_MAX = 2**32 - 1


class CILBuilderError(ValueError):
    """Raised when a CIL method body cannot be assembled."""


class CILOpcode(IntEnum):
    """CIL opcodes used by the compiler-backend MVP.

    The enum stores the first opcode byte. Two-byte opcodes such as ``ceq`` are
    emitted by helper methods because their second byte is the interesting part.
    """

    NOP = 0x00
    LDARG_0 = 0x02
    LDARG_1 = 0x03
    LDARG_2 = 0x04
    LDARG_3 = 0x05
    LDLOC_0 = 0x06
    LDLOC_1 = 0x07
    LDLOC_2 = 0x08
    LDLOC_3 = 0x09
    STLOC_0 = 0x0A
    STLOC_1 = 0x0B
    STLOC_2 = 0x0C
    STLOC_3 = 0x0D
    LDARG_S = 0x0E
    STARG_S = 0x10
    LDLOC_S = 0x11
    STLOC_S = 0x13
    LDC_I4_M1 = 0x15
    LDC_I4_0 = 0x16
    LDC_I4_1 = 0x17
    LDC_I4_2 = 0x18
    LDC_I4_3 = 0x19
    LDC_I4_4 = 0x1A
    LDC_I4_5 = 0x1B
    LDC_I4_6 = 0x1C
    LDC_I4_7 = 0x1D
    LDC_I4_8 = 0x1E
    LDC_I4_S = 0x1F
    LDC_I4 = 0x20
    DUP = 0x25
    POP = 0x26
    CALL = 0x28
    RET = 0x2A
    BR_S = 0x2B
    BRFALSE_S = 0x2C
    BRTRUE_S = 0x2D
    BEQ_S = 0x2E
    BGE_S = 0x2F
    BGT_S = 0x30
    BLE_S = 0x31
    BLT_S = 0x32
    BNE_UN_S = 0x33
    BR = 0x38
    BRFALSE = 0x39
    BRTRUE = 0x3A
    BEQ = 0x3B
    BGE = 0x3C
    BGT = 0x3D
    BLE = 0x3E
    BLT = 0x3F
    BNE_UN = 0x40
    ADD = 0x58
    SUB = 0x59
    MUL = 0x5A
    DIV = 0x5B
    AND = 0x5F
    OR = 0x60
    XOR = 0x61
    SHL = 0x62
    SHR = 0x63
    CALLVIRT = 0x6F
    LDSFLD = 0x7E
    STSFLD = 0x80
    NEWARR = 0x8D
    LDELEM_U1 = 0x91
    LDELEM_I4 = 0x94
    STELEM_I1 = 0x9C
    STELEM_I4 = 0x9E
    # Numeric conversion opcodes (ECMA-335 §III.1.6).
    # ``conv.u2`` converts the stack top to an unsigned 16-bit integer
    # (uint16 / char), zero-extending a shorter value or truncating a
    # longer one.  Used by the Twig CLR backend to convert an int32 byte
    # value to ``char`` before calling ``System.Console.Write(char)``.
    CONV_U2 = 0xD3
    PREFIX_FE = 0xFE


class CILBranchKind(StrEnum):
    """Branch families with short and long encodings."""

    ALWAYS = "br"
    FALSE = "brfalse"
    TRUE = "brtrue"
    EQ = "beq"
    GE = "bge"
    GT = "bgt"
    LE = "ble"
    LT = "blt"
    NE_UN = "bne.un"


_BRANCH_OPCODES: dict[CILBranchKind, tuple[int, int]] = {
    CILBranchKind.ALWAYS: (CILOpcode.BR_S, CILOpcode.BR),
    CILBranchKind.FALSE: (CILOpcode.BRFALSE_S, CILOpcode.BRFALSE),
    CILBranchKind.TRUE: (CILOpcode.BRTRUE_S, CILOpcode.BRTRUE),
    CILBranchKind.EQ: (CILOpcode.BEQ_S, CILOpcode.BEQ),
    CILBranchKind.GE: (CILOpcode.BGE_S, CILOpcode.BGE),
    CILBranchKind.GT: (CILOpcode.BGT_S, CILOpcode.BGT),
    CILBranchKind.LE: (CILOpcode.BLE_S, CILOpcode.BLE),
    CILBranchKind.LT: (CILOpcode.BLT_S, CILOpcode.BLT),
    CILBranchKind.NE_UN: (CILOpcode.BNE_UN_S, CILOpcode.BNE_UN),
}


@dataclass(frozen=True)
class _LabelMarker:
    name: str


@dataclass(frozen=True)
class _RawBytes:
    data: bytes


@dataclass(frozen=True)
class _BranchRef:
    kind: CILBranchKind
    label: str
    force_long: bool


type _Item = _LabelMarker | _RawBytes | _BranchRef


class CILBytecodeBuilder:
    """Two-pass CIL method-body assembler.

    The builder records a small stream of raw bytes, labels, and branch
    references. ``assemble()`` computes label offsets, promotes out-of-range
    short branches to long branches, and returns stable CIL bytes.
    """

    def __init__(self) -> None:
        self._items: list[_Item] = []

    def mark(self, label: str) -> CILBytecodeBuilder:
        """Mark the current byte offset with a label."""
        if not label:
            msg = "label must not be empty"
            raise CILBuilderError(msg)
        self._items.append(_LabelMarker(label))
        return self

    def emit_raw(self, data: bytes) -> CILBytecodeBuilder:
        """Append pre-encoded CIL bytes."""
        self._items.append(_RawBytes(bytes(data)))
        return self

    def emit_opcode(self, opcode: int | CILOpcode) -> CILBytecodeBuilder:
        """Append a one-byte opcode."""
        opcode_value = int(opcode)
        _require_u8(opcode_value, "opcode")
        return self.emit_raw(bytes([opcode_value]))

    def emit_ldc_i4(self, value: int) -> CILBytecodeBuilder:
        """Push a 32-bit integer using the most compact legal encoding."""
        return self.emit_raw(encode_ldc_i4(value))

    def emit_ldloc(self, index: int) -> CILBytecodeBuilder:
        """Load a local slot using short, ``.s``, or prefixed form."""
        return self.emit_raw(encode_ldloc(index))

    def emit_stloc(self, index: int) -> CILBytecodeBuilder:
        """Store to a local slot using short, ``.s``, or prefixed form."""
        return self.emit_raw(encode_stloc(index))

    def emit_ldarg(self, index: int) -> CILBytecodeBuilder:
        """Load an argument slot using short, ``.s``, or prefixed form."""
        return self.emit_raw(encode_ldarg(index))

    def emit_starg(self, index: int) -> CILBytecodeBuilder:
        """Store to an argument slot using ``.s`` or prefixed form."""
        return self.emit_raw(encode_starg(index))

    def emit_conv_u2(self) -> CILBytecodeBuilder:
        """Emit ``conv.u2`` — truncate/zero-extend the stack top to uint16.

        Used to convert an ``int32`` byte value to ``char`` (uint16) before
        calling ``System.Console.Write(char)`` in Twig's inline host-call
        path.  The opcode is ``0xD3`` per ECMA-335 §III.3.18.
        """
        return self.emit_opcode(CILOpcode.CONV_U2)

    def emit_token_instruction(
        self,
        opcode: int | CILOpcode,
        token: int,
    ) -> CILBytecodeBuilder:
        """Emit an instruction followed by a 4-byte metadata token."""
        opcode_value = int(opcode)
        _require_u8(opcode_value, "opcode")
        return self.emit_raw(bytes([opcode_value]) + encode_metadata_token(token))

    def emit_call(self, token: int) -> CILBytecodeBuilder:
        """Emit ``call`` with a MethodDef or MemberRef token."""
        return self.emit_token_instruction(CILOpcode.CALL, token)

    def emit_callvirt(self, token: int) -> CILBytecodeBuilder:
        """Emit ``callvirt`` with a MemberRef token."""
        return self.emit_token_instruction(CILOpcode.CALLVIRT, token)

    def emit_ldsfld(self, token: int) -> CILBytecodeBuilder:
        """Emit ``ldsfld`` with a Field token."""
        return self.emit_token_instruction(CILOpcode.LDSFLD, token)

    def emit_stsfld(self, token: int) -> CILBytecodeBuilder:
        """Emit ``stsfld`` with a Field token."""
        return self.emit_token_instruction(CILOpcode.STSFLD, token)

    def emit_newarr(self, token: int) -> CILBytecodeBuilder:
        """Emit ``newarr`` with an element TypeDef/TypeRef token."""
        return self.emit_token_instruction(CILOpcode.NEWARR, token)

    def emit_ceq(self) -> CILBytecodeBuilder:
        """Emit the two-byte ``ceq`` opcode."""
        return self.emit_raw(bytes([CILOpcode.PREFIX_FE, 0x01]))

    def emit_cgt(self) -> CILBytecodeBuilder:
        """Emit the two-byte ``cgt`` opcode."""
        return self.emit_raw(bytes([CILOpcode.PREFIX_FE, 0x02]))

    def emit_clt(self) -> CILBytecodeBuilder:
        """Emit the two-byte ``clt`` opcode."""
        return self.emit_raw(bytes([CILOpcode.PREFIX_FE, 0x04]))

    def emit_add(self) -> CILBytecodeBuilder:
        """Emit integer-compatible ``add``."""
        return self.emit_opcode(CILOpcode.ADD)

    def emit_sub(self) -> CILBytecodeBuilder:
        """Emit integer-compatible ``sub``."""
        return self.emit_opcode(CILOpcode.SUB)

    def emit_mul(self) -> CILBytecodeBuilder:
        """Emit integer-compatible ``mul``."""
        return self.emit_opcode(CILOpcode.MUL)

    def emit_div(self) -> CILBytecodeBuilder:
        """Emit integer-compatible ``div``."""
        return self.emit_opcode(CILOpcode.DIV)

    def emit_and(self) -> CILBytecodeBuilder:
        """Emit bitwise ``and`` (0x5F).

        Pops two int32 values from the evaluation stack, pushes their
        bitwise AND result.  Equivalent to ``a & b`` in C.
        """
        return self.emit_opcode(CILOpcode.AND)

    def emit_or(self) -> CILBytecodeBuilder:
        """Emit bitwise ``or`` (0x60).

        Pops two int32 values from the evaluation stack, pushes their
        bitwise OR result.  Equivalent to ``a | b`` in C.
        """
        return self.emit_opcode(CILOpcode.OR)

    def emit_xor(self) -> CILBytecodeBuilder:
        """Emit bitwise ``xor`` (0x61).

        Pops two int32 values from the evaluation stack, pushes their
        bitwise XOR result.  Equivalent to ``a ^ b`` in C.

        This is also the CIL primitive used to implement NOT:
        ``ldc.i4.m1`` (push -1 = 0xFFFF_FFFF) followed by ``xor``
        produces the bitwise complement of any int32 value.
        """
        return self.emit_opcode(CILOpcode.XOR)

    def emit_branch(
        self,
        kind: CILBranchKind,
        label: str,
        *,
        force_long: bool = False,
    ) -> CILBytecodeBuilder:
        """Emit a label branch that assembles to short or long form."""
        if not isinstance(kind, CILBranchKind):
            msg = f"unknown branch kind: {kind!r}"
            raise CILBuilderError(msg)
        if not label:
            msg = "branch label must not be empty"
            raise CILBuilderError(msg)
        self._items.append(_BranchRef(kind, label, force_long))
        return self

    def emit_ret(self) -> CILBytecodeBuilder:
        """Emit ``ret``."""
        return self.emit_opcode(CILOpcode.RET)

    def assemble(self) -> bytes:
        """Resolve labels and return method-body bytecode."""
        widths = self._initial_branch_widths()
        for _ in range(len(widths) + 1):
            label_offsets, item_offsets = self._measure(widths)
            promoted = False
            for index, item in enumerate(self._items):
                if not isinstance(item, _BranchRef) or widths[index] == 5:
                    continue
                target = self._target_offset(label_offsets, item.label)
                delta = target - (item_offsets[index] + widths[index])
                if delta < _INT8_MIN or delta > _INT8_MAX:
                    widths[index] = 5
                    promoted = True
            if not promoted:
                return self._encode(widths, label_offsets, item_offsets)
        msg = "branch promotion did not converge"
        raise CILBuilderError(msg)

    def _initial_branch_widths(self) -> dict[int, int]:
        widths: dict[int, int] = {}
        for index, item in enumerate(self._items):
            if isinstance(item, _BranchRef):
                widths[index] = 5 if item.force_long else 2
        return widths

    def _measure(
        self,
        branch_widths: dict[int, int],
    ) -> tuple[dict[str, int], dict[int, int]]:
        label_offsets: dict[str, int] = {}
        item_offsets: dict[int, int] = {}
        offset = 0
        for index, item in enumerate(self._items):
            item_offsets[index] = offset
            if isinstance(item, _LabelMarker):
                if item.name in label_offsets:
                    msg = f"duplicate label: {item.name}"
                    raise CILBuilderError(msg)
                label_offsets[item.name] = offset
            elif isinstance(item, _RawBytes):
                offset += len(item.data)
            else:
                offset += branch_widths[index]
        return label_offsets, item_offsets

    def _encode(
        self,
        branch_widths: dict[int, int],
        label_offsets: dict[str, int],
        item_offsets: dict[int, int],
    ) -> bytes:
        out = bytearray()
        for index, item in enumerate(self._items):
            if isinstance(item, _LabelMarker):
                continue
            if isinstance(item, _RawBytes):
                out.extend(item.data)
                continue
            out.extend(
                self._encode_branch(
                    item,
                    branch_widths[index],
                    label_offsets,
                    item_offsets[index],
                )
            )
        return bytes(out)

    def _encode_branch(
        self,
        item: _BranchRef,
        width: int,
        label_offsets: dict[str, int],
        item_offset: int,
    ) -> bytes:
        target = self._target_offset(label_offsets, item.label)
        delta = target - (item_offset + width)
        short_opcode, long_opcode = _BRANCH_OPCODES[item.kind]
        if width == 2:
            if delta < _INT8_MIN or delta > _INT8_MAX:
                msg = f"short branch to {item.label!r} is out of range"
                raise CILBuilderError(msg)
            return bytes([short_opcode, delta & 0xFF])
        if delta < _INT32_MIN or delta > _INT32_MAX:
            msg = f"long branch to {item.label!r} is outside int32 range"
            raise CILBuilderError(msg)
        return bytes([long_opcode]) + struct.pack("<i", delta)

    @staticmethod
    def _target_offset(label_offsets: dict[str, int], label: str) -> int:
        try:
            return label_offsets[label]
        except KeyError as exc:
            msg = f"unknown label: {label}"
            raise CILBuilderError(msg) from exc


def encode_ldc_i4(value: int) -> bytes:
    """Encode ``ldc.i4`` using the shortest form for a signed int32."""
    _require_int32(value, "ldc.i4 value")
    if value == -1:
        return bytes([CILOpcode.LDC_I4_M1])
    if 0 <= value <= 8:
        return bytes([CILOpcode.LDC_I4_0 + value])
    if _INT8_MIN <= value <= _INT8_MAX:
        return bytes([CILOpcode.LDC_I4_S, value & 0xFF])
    return bytes([CILOpcode.LDC_I4]) + encode_i4(value)


def encode_ldloc(index: int) -> bytes:
    """Encode a local load for slots 0 through 65535."""
    return _encode_indexed(index, CILOpcode.LDLOC_0, CILOpcode.LDLOC_S, 0x0C)


def encode_stloc(index: int) -> bytes:
    """Encode a local store for slots 0 through 65535."""
    return _encode_indexed(index, CILOpcode.STLOC_0, CILOpcode.STLOC_S, 0x0E)


def encode_ldarg(index: int) -> bytes:
    """Encode an argument load for slots 0 through 65535."""
    return _encode_indexed(index, CILOpcode.LDARG_0, CILOpcode.LDARG_S, 0x09)


def encode_starg(index: int) -> bytes:
    """Encode an argument store for slots 0 through 65535."""
    _require_u16(index, "argument index")
    if index <= 0xFF:
        return bytes([CILOpcode.STARG_S, index])
    return bytes([CILOpcode.PREFIX_FE, 0x0B]) + struct.pack("<H", index)


def encode_metadata_token(token: int) -> bytes:
    """Encode a CLI metadata token operand."""
    if token < 0 or token > _UINT32_MAX:
        msg = f"metadata token outside uint32 range: {token}"
        raise CILBuilderError(msg)
    return struct.pack("<I", token)


def encode_i4(value: int) -> bytes:
    """Encode a signed int32 immediate."""
    _require_int32(value, "int32 immediate")
    return struct.pack("<i", value)


def _encode_indexed(
    index: int,
    short_base: int,
    short_s: int,
    prefixed_second_byte: int,
) -> bytes:
    _require_u16(index, "slot index")
    if index <= 3:
        return bytes([short_base + index])
    if index <= 0xFF:
        return bytes([short_s, index])
    return bytes([CILOpcode.PREFIX_FE, prefixed_second_byte]) + struct.pack(
        "<H",
        index,
    )


def _require_int32(value: int, context: str) -> None:
    if value < _INT32_MIN or value > _INT32_MAX:
        msg = f"{context} outside int32 range: {value}"
        raise CILBuilderError(msg)


def _require_u16(value: int, context: str) -> None:
    if value < 0 or value > _UINT16_MAX:
        msg = f"{context} outside uint16 range: {value}"
        raise CILBuilderError(msg)


def _require_u8(value: int, context: str) -> None:
    if value < 0 or value > 0xFF:
        msg = f"{context} outside uint8 range: {value}"
        raise CILBuilderError(msg)
