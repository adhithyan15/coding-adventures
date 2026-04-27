"""BEAM bytecode disassembly for a reusable pipeline-oriented toolchain."""

from __future__ import annotations

from dataclasses import dataclass

from beam_bytes_decoder import DecodedBeamModule, decode_beam_module
from beam_opcode_metadata import OTP_28_PROFILE, BeamOpcode, BeamProfile

TAG_NAMES = {
    0: "u",
    1: "i",
    2: "a",
    3: "x",
    4: "y",
    5: "f",
    6: "h",
    7: "z",
}


@dataclass(frozen=True)
class BeamOperand:
    """One decoded BEAM operand."""

    kind: str
    value: object


@dataclass(frozen=True)
class BeamInstruction:
    """One decoded BEAM instruction from the Code chunk."""

    offset: int
    opcode: BeamOpcode
    args: tuple[BeamOperand, ...]


@dataclass(frozen=True)
class BeamDisassembledModule:
    """A disassembled BEAM module."""

    profile: BeamProfile
    module_name: str
    instructions: tuple[BeamInstruction, ...]
    label_to_index: dict[int, int]
    imports: tuple[tuple[str, str, int], ...]
    exports: tuple[tuple[str, int, int], ...]

    def find_export(self, function: str, arity: int) -> int:
        """Return the instruction index for an exported function."""
        for name, export_arity, label in self.exports:
            if name == function and export_arity == arity:
                return self.label_to_index[label]
        msg = f"Export {function}/{arity} not found"
        raise KeyError(msg)


def _decode_u(data: bytes, offset: int) -> tuple[int, int]:
    first = data[offset]
    if (first & 0x08) == 0:
        return first >> 4, offset + 1
    if (first & 0x10) == 0:
        return ((first & 0xE0) << 3) | data[offset + 1], offset + 2

    length_code = first >> 5
    if length_code == 7:
        nested_length, next_offset = _decode_u(data, offset + 1)
        length = nested_length + 9
    else:
        next_offset = offset + 1
        length = length_code + 2
    end_offset = next_offset + length
    return int.from_bytes(data[next_offset:end_offset], "big", signed=False), end_offset


def _decode_value(
    data: bytes,
    offset: int,
    atoms: tuple[str | None, ...],
) -> tuple[BeamOperand, int]:
    first = data[offset]
    tag = first & 0b111
    tag_name = TAG_NAMES[tag]

    if tag_name == "z":
        msg = (
            "Extended z-tag operands are not yet supported "
            "by this initial disassembler slice"
        )
        raise NotImplementedError(msg)

    raw_value, next_offset = _decode_u(data, offset)
    if tag_name == "i":
        byte_width = next_offset - offset
        if byte_width > 2 and data[offset + (1 if byte_width > 1 else 0)] > 0x7F:
            bits = 8 * (next_offset - offset - 1)
            raw_value -= 1 << bits
        return BeamOperand(kind="integer", value=raw_value), next_offset
    if tag_name == "a":
        if raw_value == 0:
            return BeamOperand(kind="nil", value=[]), next_offset
        return BeamOperand(kind="atom", value=atoms[raw_value]), next_offset
    if tag_name == "x":
        return BeamOperand(kind="x", value=raw_value), next_offset
    if tag_name == "y":
        return BeamOperand(kind="y", value=raw_value), next_offset
    if tag_name == "f":
        return BeamOperand(kind="label", value=raw_value), next_offset
    if tag_name == "u":
        return BeamOperand(kind="u", value=raw_value), next_offset
    if tag_name == "h":
        return BeamOperand(kind="h", value=raw_value), next_offset
    msg = f"Unsupported tag {tag_name!r}"
    raise NotImplementedError(msg)


def disassemble_beam_module(module: DecodedBeamModule) -> BeamDisassembledModule:
    """Disassemble one decoded BEAM module."""
    code = module.code_header.code
    offset = 0
    instructions: list[BeamInstruction] = []
    label_to_index: dict[int, int] = {}

    while offset < len(code):
        start = offset
        opcode_value = code[offset]
        opcode = module.profile.opcode_by_number(opcode_value)
        offset += 1

        args: list[BeamOperand] = []
        for _ in range(opcode.arity):
            arg, offset = _decode_value(code, offset, module.atoms)
            args.append(arg)

        instruction = BeamInstruction(offset=start, opcode=opcode, args=tuple(args))
        if opcode.name == "label" and args and args[0].kind == "u":
            label_to_index[int(args[0].value)] = len(instructions)
        instructions.append(instruction)

    return BeamDisassembledModule(
        profile=module.profile,
        module_name=module.module_name,
        instructions=tuple(instructions),
        label_to_index=label_to_index,
        imports=tuple(
            (entry.module, entry.function, entry.arity) for entry in module.imports
        ),
        exports=tuple(
            (entry.function, entry.arity, entry.label) for entry in module.exports
        ),
    )


def disassemble_bytes(
    data: bytes,
    profile: BeamProfile = OTP_28_PROFILE,
) -> BeamDisassembledModule:
    """Decode and disassemble a raw `.beam` file in one step."""
    return disassemble_beam_module(decode_beam_module(data, profile))
