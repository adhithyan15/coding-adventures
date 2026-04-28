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
from cil_bytecode_builder import (
    CILBuilder,
    CILMethodArtifact,
    CILProgramArtifact,
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
    builder = CILBuilder()
    builder.emit_ldc_i4(n)
    builder.emit_ret()
    body = builder.body()

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
        helper_specs=(),
        data_offsets={},
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


def test_diagnostic_current_failure_message(tmp_path: Path) -> None:
    """Diagnostic-only — captures the *current* dotnet error message
    as a baseline for tracking conformance progress.

    This test does NOT skip on missing dotnet (it's also useful as a
    pure unit test that the writer produces *some* output).  When
    dotnet is present it asserts that the failure mode is
    BadImageFormatException specifically — if a future CLR01 fix
    breaks differently (e.g. METHOD_NOT_FOUND), that's a sign of
    progress and this test is updated to reflect the new baseline.
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

    if not _DOTNET_AVAILABLE:
        pytest.skip("dotnet not available — diagnostic stops at byte production")

    asm_path = tmp_path / "Diagnostic.exe"
    cfg_path = tmp_path / "Diagnostic.runtimeconfig.json"
    asm_path.write_bytes(artifact.assembly_bytes)
    cfg_path.write_text(_runtimeconfig_for_net9())

    result = subprocess.run(
        ["dotnet", str(asm_path)],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )

    # As of pre-CLR01: BadImageFormatException, exit code 134.
    # When the fix lands and changes this, update the assertion.
    if result.returncode != 42:
        # Capture the failure mode for diagnostic purposes — if it
        # changes, we know the fix is making progress.
        assert "BadImageFormatException" in result.stderr or (
            result.returncode == 134
        ), (
            "Expected the *current* failure to be a BadImageFormat / 134; "
            f"got exit={result.returncode} stderr={result.stderr!r}.  "
            "If dotnet now accepts the assembly, this test should be "
            "removed and the smoke tests above will validate."
        )
