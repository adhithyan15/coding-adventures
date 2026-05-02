"""Tests for the ALGOL 60 to WASM command-line entry point."""

from __future__ import annotations

from pathlib import Path

import pytest
from wasm_runtime import WasmRuntime

from algol_wasm_compiler import MAX_SOURCE_LENGTH
from algol_wasm_compiler.cli import main


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
    assert "algol60-wasm [OPTIONS] <SOURCE>" in captured.out
    assert "GLOBAL OPTIONS" in captured.out
    assert captured.err == ""


def test_cli_reports_cli_builder_parse_errors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    assert main(["--definitely-not-a-flag"]) == 2

    captured = capsys.readouterr()
    assert captured.out == ""
    assert "algol60-wasm: error[unknown_flag]" in captured.err
