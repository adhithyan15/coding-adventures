"""Real-``dotnet`` conformance tests for ``cli-assembly-writer``.

These tests are the *target* of the CLR01 conformance work.  Today
they all fail with ``System.BadImageFormatException: File is
corrupt`` — the in-house writer produces assemblies that work on
``clr-vm-simulator`` but real .NET rejects.  Each chunk of the
CLR01 fix should knock another test (or another error message) off
the failure list.

The tests gate themselves behind a ``has_dotnet()`` probe so CI
without the .NET SDK skips them rather than failing — same pattern
the repo uses for git/curl/etc-dependent tests.

What "passes" means
-------------------
A test passes when:

1. The writer produces an assembly the real ``dotnet`` runtime
   loads without ``BadImageFormatException``.
2. The program executes and exits with the expected code.

Both criteria must hold for every test in this file before the
CLR01 work is considered done.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest
from cil_bytecode_builder import CILBytecodeBuilder
from ir_to_cil_bytecode import (
    CILMethodArtifact,
    CILProgramArtifact,
    SequentialCILTokenProvider,
)
from cli_assembly_writer import CLIAssemblyConfig, write_cli_assembly


def _has_dotnet() -> bool:
    """Probe for a working ``dotnet`` CLI on PATH."""
    if shutil.which("dotnet") is None:
        return False
    try:
        result = subprocess.run(
            ["dotnet", "--version"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        return result.returncode == 0
    except (subprocess.SubprocessError, OSError):
        return False


_DOTNET_AVAILABLE = _has_dotnet()
_skip_if_no_dotnet = pytest.mark.skipif(
    not _DOTNET_AVAILABLE,
    reason="dotnet SDK not available; skipping real-runtime conformance test",
)


def _runtimeconfig_for_net9() -> str:
    """The minimal ``<name>.runtimeconfig.json`` real .NET expects.

    Without this file alongside the assembly, ``dotnet <name>.exe``
    fails before even loading the PE because it can't pick a runtime.
    """
    return json.dumps({
        "runtimeOptions": {
            "tfm": "net9.0",
            "framework": {
                "name": "Microsoft.NETCore.App",
                "version": "9.0.0",
            },
        },
    })


def _build_minimal_return_n_program(n: int) -> CILProgramArtifact:
    """Build the smallest CIL program possible: ``Main`` returns ``n``.

    Used as the target of the conformance fix — nothing else exercises
    cli-assembly-writer's metadata layout more directly.
    """
    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(n)
    builder.emit_ret()
    body = builder.assemble()

    method = CILMethodArtifact(
        name="Main",
        body=body,
        return_type="int32",
        parameter_types=(),
        local_types=(),
        max_stack=8,
    )
    return CILProgramArtifact(
        entry_label="Main",
        methods=(method,),
        data_offsets={},
        data_size=0,
        helper_specs=(),
        token_provider=SequentialCILTokenProvider(("Main",)),
    )


@_skip_if_no_dotnet
def test_return_42_runs_on_real_dotnet(tmp_path: Path) -> None:
    """The simplest possible smoke test: ``return 42``.

    Currently fails with ``System.BadImageFormatException`` — the
    target of the CLR01 conformance fix.  When this passes, the
    minimum viable real-.NET writer is done.
    """
    program = _build_minimal_return_n_program(42)
    artifact = write_cli_assembly(
        program,
        CLIAssemblyConfig(
            assembly_name="ReturnFortyTwo",
            module_name="ReturnFortyTwo.exe",
            type_name="ReturnFortyTwo",
        ),
    )

    asm_path = tmp_path / "ReturnFortyTwo.exe"
    cfg_path = tmp_path / "ReturnFortyTwo.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )

    # The expected return code is 42 — the value Main returned.
    # If we instead see 134 with "BadImageFormatException", the
    # writer's output isn't loading; that's the conformance bug
    # CLR01 fixes.
    assert result.returncode == 42, (
        f"dotnet rejected the assembly or returned the wrong exit code.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


@_skip_if_no_dotnet
def test_return_zero_runs_on_real_dotnet(tmp_path: Path) -> None:
    """Sanity twin: returning 0 is distinguishable from "process
    crashed before main ran" if we test against multiple return
    values."""
    program = _build_minimal_return_n_program(0)
    artifact = write_cli_assembly(
        program,
        CLIAssemblyConfig(
            assembly_name="ReturnZero",
            module_name="ReturnZero.exe",
            type_name="ReturnZero",
        ),
    )

    asm_path = tmp_path / "ReturnZero.exe"
    cfg_path = tmp_path / "ReturnZero.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )

    assert result.returncode == 0, (
        f"dotnet exited with {result.returncode}; "
        f"stderr={result.stderr!r}"
    )


# ──────────────────────────────────────────────────────────────────────────
# CLR02 Phase 2b — multi-TypeDef metadata
# ──────────────────────────────────────────────────────────────────────────


def _build_main_returning_42_program(
    extra_types: tuple[object, ...] = (),
) -> CILProgramArtifact:
    """Build a CLR01-shape program that returns 42, optionally with
    extra TypeDefs alongside the main user type.

    Used by the Phase 2b multi-TypeDef tests to verify that adding
    extra interfaces / classes doesn't break the load path — the
    user's main ``Main`` method still has to run and return 42.
    """
    builder = CILBytecodeBuilder()
    builder.emit_ldc_i4(42)
    builder.emit_ret()
    body = builder.assemble()
    main = CILMethodArtifact(
        name="Main",
        body=body,
        return_type="int32",
        parameter_types=(),
        local_types=(),
        max_stack=8,
    )
    return CILProgramArtifact(
        entry_label="Main",
        methods=(main,),
        data_offsets={},
        data_size=0,
        helper_specs=(),
        token_provider=SequentialCILTokenProvider(("Main",)),
        extra_types=extra_types,
    )


@_skip_if_no_dotnet
def test_extra_interface_typedef_loads_on_real_dotnet(tmp_path: Path) -> None:
    """A second TypeDef row (an abstract IClosure interface) added
    alongside the main user type — real ``dotnet`` must still load
    and run ``Main`` for exit code 42.

    This is the smallest possible multi-TypeDef test: it exercises
    the variable TypeDef table layout, the abstract-method codepath
    (RVA=0, ``MethodAttributes`` = abstract+virtual+newslot), the
    interface ``Flags`` constant, and the instance ``MethodSig``
    blob (``HASTHIS`` bit set).
    """
    from ir_to_cil_bytecode import CILTypeArtifact

    iclosure = CILTypeArtifact(
        name="IClosure",
        namespace="CodingAdventures",
        is_interface=True,
        extends=None,
        methods=(
            CILMethodArtifact(
                name="Apply",
                body=b"",
                max_stack=0,
                local_types=(),
                return_type="int32",
                parameter_types=("int32",),
                is_instance=True,
                is_abstract=True,
            ),
        ),
    )
    program = _build_main_returning_42_program(extra_types=(iclosure,))
    artifact = write_cli_assembly(
        program,
        CLIAssemblyConfig(
            assembly_name="WithIClosure",
            module_name="WithIClosure.exe",
            type_name="WithIClosure",
        ),
    )
    asm_path = tmp_path / "WithIClosure.exe"
    cfg_path = tmp_path / "WithIClosure.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    assert result.returncode == 42, (
        f"dotnet rejected the multi-TypeDef assembly.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


@_skip_if_no_dotnet
def test_concrete_class_with_field_and_interfaceimpl_loads(
    tmp_path: Path,
) -> None:
    """An IClosure interface + a concrete class that ``Implements``
    it, with one ``int32`` instance field.  Exercises the Field
    table + InterfaceImpl table + same-module TypeDef-to-TypeDef
    references.

    The concrete class's ``Apply`` is emitted as a *static*
    placeholder method that returns 42 — we're proving the metadata
    plumbing here, not yet the semantic correctness of an instance
    Apply.  Phase 2c will fill in the instance bodies.
    """
    from ir_to_cil_bytecode import CILFieldArtifact, CILTypeArtifact

    iclosure = CILTypeArtifact(
        name="IClosure",
        namespace="CodingAdventures",
        is_interface=True,
        extends=None,
        methods=(
            CILMethodArtifact(
                name="Apply",
                body=b"",
                max_stack=0,
                local_types=(),
                return_type="int32",
                parameter_types=("int32",),
                is_instance=True,
                is_abstract=True,
            ),
        ),
    )
    placeholder = CILBytecodeBuilder()
    placeholder.emit_ldc_i4(42)
    placeholder.emit_ret()
    closure = CILTypeArtifact(
        name="Closure_lambda_0",
        namespace="CodingAdventures",
        extends="System.Object",
        implements=("CodingAdventures.IClosure",),
        fields=(CILFieldArtifact(name="n", type="int32"),),
        methods=(
            CILMethodArtifact(
                name="ApplyStatic",
                body=placeholder.assemble(),
                max_stack=8,
                local_types=(),
                return_type="int32",
                parameter_types=("int32",),
            ),
        ),
    )
    program = _build_main_returning_42_program(
        extra_types=(iclosure, closure),
    )
    artifact = write_cli_assembly(
        program,
        CLIAssemblyConfig(
            assembly_name="WithClosure",
            module_name="WithClosure.exe",
            type_name="WithClosure",
        ),
    )
    asm_path = tmp_path / "WithClosure.exe"
    cfg_path = tmp_path / "WithClosure.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    assert result.returncode == 42, (
        f"dotnet rejected the assembly with closure class.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


# ──────────────────────────────────────────────────────────────────────────
# CLR02 Phase 2c — closures end-to-end through the full pipeline
# ──────────────────────────────────────────────────────────────────────────


@pytest.mark.xfail(
    reason=(
        "CLR02 Phase 2c lowering is structural only — closure refs are "
        "managed pointers but the existing CLR backend uses int32-uniform "
        "locals/args, so storing a closure ref into a local typed as "
        "int32 either truncates the pointer or trips the GC.  The next "
        "phase (typed register pool, parallel object-slot tracking) "
        "wires the runtime end-to-end.  Test stays in the file so the "
        "shape stays right; it'll flip to passing when the typed pool "
        "lands."
    ),
    strict=True,
)
@_skip_if_no_dotnet
def test_make_adder_closure_returns_42_on_real_dotnet(tmp_path: Path) -> None:
    """The headline CLR02 Phase 2c test:
    ``((make-adder 7) 35) → 42`` end-to-end on real ``dotnet``.

    Hand-built IR mirroring what twig-clr-compiler will eventually
    emit for ``(define (make-adder n) (lambda (x) (+ x n)))
    ((make-adder 7) 35)``.  Exercises the full Phase 2c pipeline:

    * MAKE_CLOSURE → ``newobj Closure_lambda::.ctor(int32)``.
    * APPLY_CLOSURE → ``callvirt int32 IClosure::Apply(int32)``.
    * Auto-emitted IClosure interface + per-lambda Closure_lambda
      TypeDef with one int32 instance field, a ctor, and an Apply
      method whose body reads the captured ``n`` from the field.
    * MemberRef to ``System.Object::.ctor`` so the closure ctor can
      chain into the base class.
    """
    from compiler_ir import (
        IDGenerator,
        IrImmediate,
        IrInstruction,
        IrLabel,
        IrOp,
        IrProgram,
        IrRegister,
    )
    from ir_to_cil_bytecode import CILBackendConfig, lower_ir_to_cil_bytecode

    def lbl(name: str) -> IrLabel:
        return IrLabel(name=name)

    def reg(i: int) -> IrRegister:
        return IrRegister(index=i)

    def imm(v: int) -> IrImmediate:
        return IrImmediate(value=v)

    gen = IDGenerator()
    program = IrProgram(entry_label="Main")

    # _lambda_0(N, X) — captures-first layout: r2 = N (capture),
    # r3 = X (explicit arg).  Body: r1 = r2 + r3.
    program.add_instruction(IrInstruction(IrOp.LABEL, [lbl("_lambda_0")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.ADD, [reg(1), reg(2), reg(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # make_adder(n): returns MAKE_CLOSURE(_lambda_0, [n]).
    program.add_instruction(IrInstruction(IrOp.LABEL, [lbl("make_adder")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.MAKE_CLOSURE,
            [reg(1), lbl("_lambda_0"), imm(1), reg(2)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    # Main(): closure = make_adder(7); return APPLY_CLOSURE(closure, [35]).
    program.add_instruction(IrInstruction(IrOp.LABEL, [lbl("Main")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [reg(2), imm(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.CALL, [lbl("make_adder")], id=gen.next())
    )
    # Stash the closure ref in a holding reg so the arg-staging move
    # for APPLY_CLOSURE doesn't clobber it.
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [reg(10), reg(1), imm(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [reg(11), imm(35)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.APPLY_CLOSURE,
            [reg(1), reg(10), imm(1), reg(11)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    cil_program = lower_ir_to_cil_bytecode(
        program,
        CILBackendConfig(
            call_register_count=None,
            closure_free_var_counts={"_lambda_0": 1},
        ),
    )
    artifact = write_cli_assembly(
        cil_program,
        CLIAssemblyConfig(
            assembly_name="ClosureAdder",
            module_name="ClosureAdder.exe",
            type_name="ClosureAdder",
        ),
    )

    asm_path = tmp_path / "ClosureAdder.exe"
    cfg_path = tmp_path / "ClosureAdder.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    assert result.returncode == 42, (
        f"closure pipeline broke at runtime.\n"
        f"  exit code: {result.returncode}\n"
        f"  stdout: {result.stdout!r}\n"
        f"  stderr: {result.stderr!r}"
    )


def test_writer_produces_nonempty_output() -> None:
    """Pure unit test — the writer produces some PE bytes for a
    minimal return-42 program.  Pre-CLR01 this test was where we
    captured the BadImageFormat baseline; CLR01 has landed (real
    dotnet now exits 42 — see the smoke tests above), so this is
    just a non-emptiness check now.
    """
    program = _build_minimal_return_n_program(42)
    artifact = write_cli_assembly(
        program,
        CLIAssemblyConfig(
            assembly_name="Diagnostic",
            module_name="Diagnostic.exe",
            type_name="Diagnostic",
        ),
    )
    assert len(artifact.assembly_bytes) > 0
    assert artifact.assembly_bytes[:2] == b"MZ"
