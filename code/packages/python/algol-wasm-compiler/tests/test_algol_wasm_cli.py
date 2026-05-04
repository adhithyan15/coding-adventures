"""Tests for the ALGOL 60 to WASM command-line entry point."""

from __future__ import annotations

from pathlib import Path

import pytest
from wasm_runtime import WasmRuntime

from algol_wasm_compiler import MAX_SOURCE_LENGTH
from algol_wasm_compiler.cli import main

_FIXTURE_DIR = Path(__file__).with_name("fixtures")


def test_cli_compiles_source_file_to_explicit_output(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "answer.alg"
    output_path = tmp_path / "dist" / "answer.wasm"
    source_path.write_text("begin integer result; result := 7 end")

    assert main([str(source_path), "--output", str(output_path)]) == 0

    captured = capsys.readouterr()
    assert captured.out == f"{output_path}\n"
    assert captured.err == ""
    assert output_path.read_bytes().startswith(b"\x00asm")
    assert WasmRuntime().load_and_run(output_path.read_bytes(), "_start", []) == [7]


def test_cli_defaults_to_wasm_suffix(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "default.alg"
    output_path = tmp_path / "default.wasm"
    source_path.write_text("begin integer result; result := 3 + 4 end")

    assert main([str(source_path), "--quiet"]) == 0

    captured = capsys.readouterr()
    assert captured.out == ""
    assert captured.err == ""
    assert WasmRuntime().load_and_run(output_path.read_bytes(), "_start", []) == [7]


def test_cli_reports_compile_errors_without_writing_output(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "bad.alg"
    output_path = tmp_path / "bad.wasm"
    source_path.write_text("begin integer result; result := false end")

    assert main([str(source_path), "-o", str(output_path)]) == 1

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: [type-check]" in captured.err
    assert not output_path.exists()


def test_cli_rejects_oversized_files_before_parsing(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "too-large.alg"
    output_path = tmp_path / "too-large.wasm"
    source_path.write_text("x" * (MAX_SOURCE_LENGTH + 1))

    assert main([str(source_path), "-o", str(output_path)]) == 1

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: [source]" in captured.err
    assert not output_path.exists()


def test_cli_uses_cli_builder_help(capsys: pytest.CaptureFixture[str]) -> None:
    assert main(["--help"]) == 0

    captured = capsys.readouterr()
    assert "USAGE" in captured.out
    assert "algol60-wasm [OPTIONS] [COMMAND] <SOURCE>" in captured.out
    assert "COMMANDS" in captured.out
    assert "run" in captured.out
    assert "GLOBAL OPTIONS" in captured.out
    assert captured.err == ""


def test_cli_reports_cli_builder_parse_errors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert main(["--definitely-not-a-flag"]) == 2

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: error[unknown_flag]" in captured.err


def test_cli_run_executes_compiled_program_without_writing_wasm(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "run.alg"
    output_path = tmp_path / "run.wasm"
    source_path.write_text(
        "begin integer result; result := 7; "
        "print('RUN'); output(' '); print(result) end"
    )

    assert main(["run", str(source_path)]) == 0

    captured = capsys.readouterr()
    assert captured.out == "RUN 7"
    assert captured.err == ""
    assert not output_path.exists()


def test_cli_run_executes_full_surface_fixture_in_python_wasm_runtime(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "full-surface.alg"
    source_path.write_text((_FIXTURE_DIR / "full-surface.alg").read_text())

    assert main(["run", str(source_path), "--print-result"]) == 0

    captured = capsys.readouterr()
    assert captured.out == "COMPLETE 81"
    assert captured.err == "result: 81\n"


def test_cli_run_can_print_start_result(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "result.alg"
    source_path.write_text("begin integer result; result := 9 end")

    assert main(["run", str(source_path), "--print-result"]) == 0

    captured = capsys.readouterr()
    assert captured.out == ""
    assert captured.err == "result: 9\n"


def test_cli_run_reports_instruction_budget_traps(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "spin.alg"
    source_path.write_text("begin loop: goto loop end")

    assert main(["run", str(source_path), "--max-instructions", "32"]) == 1

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: [run]" in captured.err
    assert "instruction budget exhausted" in captured.err


def test_cli_run_rejects_nonpositive_instruction_budget(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    source_path = tmp_path / "budget.alg"
    source_path.write_text("begin integer result; result := 1 end")

    assert main(["run", str(source_path), "--max-instructions", "0"]) == 1

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: [run] --max-instructions must be at least 1" in captured.err
