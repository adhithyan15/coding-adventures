from __future__ import annotations

import pytest
from cil_bytecode_builder import CILBytecodeBuilder
from clr_bytecode_disassembler import disassemble_clr_method
from clr_pe_file import decode_clr_pe_file
from compiler_ir import (
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILHelper,
    CILMethodArtifact,
    CILProgramArtifact,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
)

from cli_assembly_writer import (
    CLIAssemblyConfig,
    CLIAssemblyWriterError,
    write_cli_assembly,
)


def test_write_tiny_entry_method_decodes_as_clr_pe() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(7).emit_ret()
    artifact = _artifact(
        CILMethodArtifact(
            "Main",
            builder.assemble(),
            max_stack=8,
            local_types=(),
            return_type="int32",
        )
    )

    assembly_artifact = write_cli_assembly(
        artifact,
        CLIAssemblyConfig(
            assembly_name="TinyDemo",
            module_name="TinyDemo.dll",
            type_name="DemoProgram",
        ),
    )
    assembly = decode_clr_pe_file(assembly_artifact.assembly_bytes)
    entry = assembly.get_entry_point_method()

    assert assembly.metadata_version == "v4.0.30319"
    assert assembly.entry_point_token == 0x06000001
    assert entry.name == "Main"
    assert entry.declaring_type == "DemoProgram"
    assert entry.signature.return_type == "int32"
    assert entry.header.format == "tiny"
    assert entry.il_bytes == bytes([0x1D, 0x2A])
    assert assembly_artifact.method_tokens == {"Main": 0x06000001}


def test_write_ir_lowered_fat_method_with_locals_and_internal_call() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("callee")]))
    program.add_instruction(IrInstruction(IrOp.RET))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("callee")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)])
    )
    program.add_instruction(IrInstruction(IrOp.RET))
    cil = lower_ir_to_cil_bytecode(program, CILBackendConfig(syscall_arg_reg=1))

    assembly = decode_clr_pe_file(write_cli_assembly(cil).assembly_bytes)
    entry = assembly.get_entry_point_method()
    callee = assembly.resolve_method_definition(0x06000002)
    body = disassemble_clr_method(assembly, entry)

    # CLR01: TypeDef row 0 is the ECMA-335 ``<Module>`` pseudo-type
    # (no methods); the user TypeDef is now at row 1 and owns every
    # MethodDef.
    assert assembly.type_definitions[0].full_name == "<Module>"
    assert assembly.type_definitions[0].method_tokens == ()
    assert assembly.type_definitions[1].method_tokens == (0x06000001, 0x06000002)
    assert entry.header.format == "fat"
    assert entry.local_count == 2
    assert callee.name == "callee"
    assert [instruction.opcode for instruction in body.instructions] == [
        "call",
        "stloc.1",
        "ldloc.1",
        "ret",
    ]
    assert body.instructions[0].operand.name == "callee"


def test_write_ir_lowered_helper_member_refs() -> None:
    program = IrProgram(entry_label="_start")
    program.add_data(IrDataDecl("tape", 16, 0))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_ADDR, [IrRegister(0), IrLabel("tape")])
    )
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_BYTE,
            [IrRegister(1), IrRegister(0), IrRegister(1)],
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET))
    cil = lower_ir_to_cil_bytecode(program, CILBackendConfig(syscall_arg_reg=1))
    assembly_artifact = write_cli_assembly(
        cil,
        CLIAssemblyConfig(helper_type_name="Generated.Runtime"),
    )
    assembly = decode_clr_pe_file(assembly_artifact.assembly_bytes)
    helper = assembly.resolve_member_reference(0x0A000001)

    assert assembly_artifact.helper_tokens[CILHelper.MEM_LOAD_BYTE] == 0x0A000001
    assert helper.declaring_type == "Generated.Runtime"
    assert helper.name == "__ca_mem_load_byte"
    assert helper.signature.parameter_types == ("int32",)
    assert helper.signature.return_type == "int32"


def test_signature_supports_string_array_parameter() -> None:
    method = CILMethodArtifact(
        "Main",
        bytes([0x2A]),
        max_stack=8,
        local_types=(),
        return_type="void",
        parameter_types=("string[]",),
    )
    assembly = decode_clr_pe_file(write_cli_assembly(_artifact(method)).assembly_bytes)

    assert assembly.get_entry_point_method().signature.parameter_types == ("string[]",)


def test_validation_errors_are_deterministic() -> None:
    with pytest.raises(CLIAssemblyWriterError, match="program must contain"):
        write_cli_assembly(
            CILProgramArtifact(
                entry_label="Main",
                methods=(),
                data_offsets={},
                data_size=0,
                helper_specs=(),
                token_provider=SequentialCILTokenProvider(()),
            )
        )

    method = CILMethodArtifact("Main", bytes([0x2A]), 8, ())
    with pytest.raises(CLIAssemblyWriterError, match="method names must be unique"):
        write_cli_assembly(_artifact(method, method))

    with pytest.raises(CLIAssemblyWriterError, match="type_name must not be empty"):
        write_cli_assembly(_artifact(method), CLIAssemblyConfig(type_name=""))

    bad_type = CILMethodArtifact("Main", bytes([0x2A]), 8, (), return_type="nativeint")
    with pytest.raises(CLIAssemblyWriterError, match="Unsupported CLI signature"):
        write_cli_assembly(_artifact(bad_type))


def _artifact(*methods: CILMethodArtifact) -> CILProgramArtifact:
    return CILProgramArtifact(
        entry_label=methods[0].name,
        methods=methods,
        data_offsets={},
        data_size=0,
        helper_specs=(),
        token_provider=SequentialCILTokenProvider(
            tuple(method.name for method in methods)
        ),
    )
