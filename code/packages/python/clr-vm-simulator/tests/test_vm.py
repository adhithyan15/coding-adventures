from __future__ import annotations

import pytest
from cil_bytecode_builder import CILBranchKind, CILBytecodeBuilder
from cli_assembly_writer import write_cli_assembly
from cli_runtime_model import CliValue
from clr_pe_file import CLRMemberReference, CLRMethodSignature, decode_clr_pe_file
from clr_pe_file.testing import hello_world_dll_bytes
from compiler_ir import IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from dartmouth_basic_ir_compiler import compile_basic
from dartmouth_basic_parser import parse_dartmouth_basic
from ir_to_cil_bytecode import (
    CILBackendConfig,
    CILHelper,
    CILHelperSpec,
    CILMethodArtifact,
    CILProgramArtifact,
    SequentialCILTokenProvider,
    lower_ir_to_cil_bytecode,
)

from clr_vm_simulator import CLRVM, CLRVMError, CLRVMStdlibHost, run_clr_entry_point


def test_runs_tiny_generated_entry_method() -> None:
    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(7).emit_ret()
    assembly_bytes = write_cli_assembly(
        _artifact(
            CILMethodArtifact(
                "Main",
                builder.assemble(),
                max_stack=8,
                local_types=(),
                return_type="int32",
            )
        )
    ).assembly_bytes

    result = run_clr_entry_point(assembly_bytes)

    assert result.return_value == CliValue.int32(7)
    assert result.output == ""
    assert [trace.opcode for trace in result.traces] == ["ldc.i4.7", "ret"]


def test_runs_ir_lowered_internal_method_call() -> None:
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

    result = run_clr_entry_point(write_cli_assembly(cil).assembly_bytes)

    assert result.return_value == CliValue.int32(42)
    assert [trace.opcode for trace in result.traces] == [
        "ldc.i4.s",
        "stloc.1",
        "ldloc.1",
        "ret",
        "call",
        "stloc.1",
        "ldloc.1",
        "ret",
    ]


def test_runs_branching_ir_program() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0)])
    )
    program.add_instruction(
        IrInstruction(IrOp.BRANCH_Z, [IrRegister(1), IrLabel("zero")])
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(99)])
    )
    program.add_instruction(IrInstruction(IrOp.RET))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("zero")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(5)])
    )
    program.add_instruction(IrInstruction(IrOp.RET))

    cil = lower_ir_to_cil_bytecode(program, CILBackendConfig(syscall_arg_reg=1))
    result = run_clr_entry_point(write_cli_assembly(cil).assembly_bytes)

    assert result.return_value == CliValue.int32(5)
    assert "brfalse.s" in [trace.opcode for trace in result.traces]


def test_runs_dartmouth_basic_print_through_clr_pipeline() -> None:
    source = "10 PRINT \"HELLO CLR\"\n20 END\n"
    ast = parse_dartmouth_basic(source)
    ir = compile_basic(ast).program
    cil = lower_ir_to_cil_bytecode(ir, CILBackendConfig(syscall_arg_reg=0))
    assembly = write_cli_assembly(cil).assembly_bytes

    result = run_clr_entry_point(assembly, max_steps=2000)

    assert result.output == "HELLO CLR\n"
    assert result.return_value == CliValue.int32(0)
    assert result.traces[-1].opcode == "ret"


def test_default_host_supports_console_writeline_member_ref() -> None:
    assembly = decode_clr_pe_file(hello_world_dll_bytes())
    result = CLRVM().run_entry_point(assembly)

    assert result.output == "Hello, world!\n"


def test_default_host_supports_direct_console_call() -> None:
    host = CLRVMStdlibHost()
    result = host.call_member(
        CLRMemberReference(
            token=0x0A000001,
            declaring_type="System.Console",
            name="WriteLine",
            signature=CLRMethodSignature(
                has_this=False,
                parameter_types=("string",),
                return_type="void",
            ),
        ),
        (CliValue.string("direct"),),
    )

    assert result is None
    assert "".join(host.output) == "direct\n"


def test_default_host_supports_compiler_helpers() -> None:
    host = CLRVMStdlibHost(memory_size=4)

    assert (
        host.call_member(
            _member("__ca_syscall", ("int32", "int32"), "int32"),
            (CliValue.int32(1), CliValue.int32(0o21)),
        )
        == CliValue.int32(0)
    )
    assert "".join(host.output) == "A"
    assert (
        host.call_member(
            _member("__ca_mem_load_byte", ("int32",), "int32"),
            (CliValue.int32(99),),
        )
        == CliValue.int32(0)
    )
    assert (
        host.call_member(
            _member("__ca_mem_store_byte", ("int32", "int32"), "void"),
            (CliValue.int32(2), CliValue.int32(255)),
        )
        is None
    )
    assert (
        host.call_member(
            _member("__ca_mem_load_byte", ("int32",), "int32"),
            (CliValue.int32(2),),
        )
        == CliValue.int32(255)
    )
    assert (
        host.call_member(
            _member("__ca_store_word", ("int32", "int32"), "void"),
            (CliValue.int32(12), CliValue.int32(345)),
        )
        is None
    )
    assert (
        host.call_member(
            _member("__ca_load_word", ("int32",), "int32"),
            (CliValue.int32(12),),
        )
        == CliValue.int32(345)
    )

    input_host = CLRVMStdlibHost(input_bytes=b"Q")
    assert (
        input_host.call_member(
            _member("__ca_syscall", ("int32", "int32"), "int32"),
            (CliValue.int32(2), CliValue.int32(0)),
        )
        == CliValue.int32(ord("Q"))
    )
    assert (
        input_host.call_member(
            _member("__ca_syscall", ("int32", "int32"), "int32"),
            (CliValue.int32(2), CliValue.int32(0)),
        )
        == CliValue.int32(0)
    )

    with pytest.raises(CLRVMError, match="unsupported compiler helper syscall"):
        host.call_member(
            _member("__ca_syscall", ("int32", "int32"), "int32"),
            (CliValue.int32(99), CliValue.int32(0)),
        )
    with pytest.raises(CLRVMError, match="unsupported CLR host call"):
        host.call_member(_member("Missing", (), "void"), ())


def test_compiler_helper_exit_stops_execution() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(7)])
    )
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(10)]))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(99)])
    )
    program.add_instruction(IrInstruction(IrOp.RET))
    cil = lower_ir_to_cil_bytecode(program)

    result = run_clr_entry_point(write_cli_assembly(cil).assembly_bytes)

    assert result.return_value == CliValue.int32(7)


def test_runs_argument_and_bitwise_instruction_mix() -> None:
    body = bytes(
        [
            0x1F,
            9,
            0x10,
            0,
            0x0E,
            0,
            0x1F,
            3,
            0x5F,
            0x17,
            0x60,
            0x17,
            0x62,
            0x17,
            0x63,
            0x2A,
        ]
    )
    method = CILMethodArtifact(
        "Main",
        body,
        max_stack=8,
        local_types=(),
        return_type="int32",
        parameter_types=("int32",),
    )

    result = run_clr_entry_point(write_cli_assembly(_artifact(method)).assembly_bytes)

    assert result.return_value == CliValue.int32(1)
    assert [trace.opcode for trace in result.traces] == [
        "ldc.i4.s",
        "starg.s",
        "ldarg.s",
        "ldc.i4.s",
        "and",
        "ldc.i4.1",
        "or",
        "ldc.i4.1",
        "shl",
        "ldc.i4.1",
        "shr",
        "ret",
    ]


def test_callvirt_static_member_ref_is_rejected() -> None:
    body = bytes([0x6F, 0x01, 0x00, 0x00, 0x0A, 0x2A])
    method = CILMethodArtifact(
        "Main",
        body,
        max_stack=8,
        local_types=(),
        return_type="void",
    )
    artifact = CILProgramArtifact(
        entry_label="Main",
        methods=(method,),
        data_offsets={},
        data_size=0,
        helper_specs=(
            CILHelperSpec(
                CILHelper.SYSCALL,
                "__ca_syscall",
                ("int32", "int32"),
                "int32",
            ),
        ),
        token_provider=SequentialCILTokenProvider(("Main",)),
    )

    with pytest.raises(CLRVMError, match="callvirt cannot target static"):
        run_clr_entry_point(write_cli_assembly(artifact).assembly_bytes)


def test_runs_null_and_comparison_values() -> None:
    null_method = CILMethodArtifact(
        "Main",
        bytes([0x00, 0x01, 0x2A]),
        max_stack=8,
        local_types=(),
        return_type="object",
    )
    null_result = run_clr_entry_point(
        write_cli_assembly(_artifact(null_method)).assembly_bytes
    )

    assert null_result.return_value == CliValue.null()
    assert [trace.opcode for trace in null_result.traces] == ["nop", "ldnull", "ret"]

    compare_cases = (
        (2, 2, 0x01, 1),
        (2, 1, 0x02, 1),
        (2, 1, 0x04, 0),
    )
    for lhs, rhs, op_byte, expected in compare_cases:
        method = CILMethodArtifact(
            "Main",
            bytes([0x16 + lhs, 0x16 + rhs, 0xFE, op_byte, 0x2A]),
            max_stack=8,
            local_types=(),
            return_type="int32",
        )
        result = run_clr_entry_point(
            write_cli_assembly(_artifact(method)).assembly_bytes
        )
        assert result.return_value == CliValue.int32(expected)


def test_runs_relational_branch_forms() -> None:
    cases = (
        (CILBranchKind.EQ, 2, 2, 7),
        (CILBranchKind.GE, 2, 2, 7),
        (CILBranchKind.GT, 2, 1, 7),
        (CILBranchKind.LE, 1, 2, 7),
        (CILBranchKind.LT, 1, 2, 7),
        (CILBranchKind.NE_UN, 1, 2, 7),
    )

    for kind, lhs, rhs, expected in cases:
        builder = CILBytecodeBuilder()
        builder.emit_ldc_i4(lhs)
        builder.emit_ldc_i4(rhs)
        builder.emit_branch(kind, "hit")
        builder.emit_ldc_i4(0)
        builder.emit_ret()
        builder.mark("hit")
        builder.emit_ldc_i4(expected)
        builder.emit_ret()
        method = CILMethodArtifact("Main", builder.assemble(), 8, (), "int32")

        result = run_clr_entry_point(
            write_cli_assembly(_artifact(method)).assembly_bytes
        )

        assert result.return_value == CliValue.int32(expected)


def test_vm_errors_are_deterministic() -> None:
    with pytest.raises(CLRVMError, match="max_steps"):
        CLRVM(max_steps=0)

    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(1).emit_ldc_i4(0).emit_div().emit_ret()
    assembly_bytes = write_cli_assembly(
        _artifact(CILMethodArtifact("Main", builder.assemble(), 8, ()))
    ).assembly_bytes

    with pytest.raises(CLRVMError, match="division by zero"):
        run_clr_entry_point(assembly_bytes)

    loop = CILMethodArtifact("Main", bytes([0x2B, 0xFE]), 8, (), "void")
    with pytest.raises(CLRVMError, match="exceeded max_steps"):
        run_clr_entry_point(
            write_cli_assembly(_artifact(loop)).assembly_bytes,
            max_steps=2,
        )

    missing_target = CILMethodArtifact("Main", bytes([0x2B, 0x08]), 8, (), "void")
    with pytest.raises(CLRVMError, match="no instruction"):
        run_clr_entry_point(write_cli_assembly(_artifact(missing_target)).assembly_bytes)


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


def _member(
    name: str,
    parameter_types: tuple[str, ...],
    return_type: str,
) -> CLRMemberReference:
    return CLRMemberReference(
        token=0x0A000001,
        declaring_type="CodingAdventures.Runtime.Helpers",
        name=name,
        signature=CLRMethodSignature(
            has_this=False,
            parameter_types=parameter_types,
            return_type=return_type,
        ),
    )
