"""Tests for the Prolog VM command-line interface."""

from __future__ import annotations

from pathlib import Path

import pytest

from prolog_vm_compiler.cli import main


def test_cli_runs_inline_ad_hoc_query_with_bytecode_values(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). parent(homer, lisa).",
        "--query",
        "parent(homer, Who)",
        "--backend",
        "bytecode",
        "--values",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == ["bart.", "lisa."]


def test_cli_runs_file_embedded_query_with_named_answers(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    source_path = tmp_path / "family.pl"
    source_path.write_text(
        "parent(homer, bart).\n"
        "parent(homer, lisa).\n"
        "?- parent(homer, Who).\n",
        encoding="utf-8",
    )

    status = main([str(source_path), "--backend", "bytecode"])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "Who = lisa.",
    ]


def test_cli_runs_project_file_graph_with_query_module(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    family_path = tmp_path / "family.pl"
    family_path.write_text(
        ":- module(family, [ancestor/2]).\n"
        "ancestor(homer, bart).\n"
        "ancestor(homer, lisa).\n",
        encoding="utf-8",
    )
    app_path = tmp_path / "app.pl"
    app_path.write_text(
        ":- module(app, []).\n"
        ":- use_module(family, [ancestor/2]).\n",
        encoding="utf-8",
    )

    status = main([
        str(app_path),
        str(family_path),
        "--query",
        "ancestor(homer, Who)",
        "--query-module",
        "app",
        "--backend",
        "bytecode",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "Who = lisa.",
    ]


def test_cli_help_is_generated_by_cli_builder(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main(["--help"])

    assert status == 0
    output = capsys.readouterr().out
    assert "prolog-vm" in output
    assert "--query" in output


def test_cli_no_solution_returns_false_and_nonzero(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "parent(marge, Who)",
    ])

    assert status == 1
    assert capsys.readouterr().out == "false.\n"


def test_cli_rejects_missing_input(capsys: pytest.CaptureFixture[str]) -> None:
    status = main(["--query", "parent(homer, Who)"])

    assert status == 2
    assert "provide --source or at least one Prolog file" in capsys.readouterr().err
