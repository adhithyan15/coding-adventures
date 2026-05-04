"""Tests for the Prolog VM command-line interface."""

from __future__ import annotations

import io
import json
import sys
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


def test_cli_repeated_queries_share_committed_runtime_state(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        ":- dynamic(memo/1).",
        "--query",
        "assertz(memo(saved))",
        "--query",
        "memo(Value)",
        "--commit",
        "--backend",
        "bytecode",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "true.",
        "Value = saved.",
    ]


def test_cli_check_compiles_source_without_queries(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--check",
    ])

    assert status == 0
    assert capsys.readouterr().out == "ok.\n"


def test_cli_check_json_reports_query_counts(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        ":- initialization(true). parent(homer, bart). ?- parent(homer, Who).",
        "--check",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload == {
        "backend": "structured",
        "initialization_query_count": 1,
        "initialized": True,
        "mode": "check",
        "source_query_count": 1,
        "success": True,
    }


def test_cli_check_rejects_ad_hoc_queries(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--check",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--check cannot be combined with --query" in capsys.readouterr().err


def test_cli_lists_source_query_variables(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who). ?- true.",
        "--list-source-queries",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "query 0: Who",
        "query 1: (no variables)",
    ]


def test_cli_lists_source_query_variables_as_json(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--list-source-queries",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload == {
        "mode": "source_queries",
        "queries": [
            {"index": 0, "variables": ["Who"]},
        ],
        "source_query_count": 1,
        "success": True,
    }


def test_cli_list_source_queries_rejects_execution_modes(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--list-source-queries",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--list-source-queries cannot be combined with --query" in (
        capsys.readouterr().err
    )


def test_cli_json_output_serializes_named_answers(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "parent(homer, Who)",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload == {
        "answer_count": 1,
        "answers": [
            {
                "bindings": {
                    "Who": {"type": "atom", "value": "bart"},
                },
                "residual_constraints": [],
            },
        ],
        "query": "parent(homer, Who)",
        "success": True,
    }


def test_cli_jsonl_output_streams_repeated_query_records(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "parent(homer, Who)",
        "--query",
        "parent(marge, Who)",
        "--format",
        "jsonl",
    ])

    records = [
        json.loads(line)
        for line in capsys.readouterr().out.splitlines()
    ]

    assert status == 1
    assert records[0]["success"] is True
    assert records[0]["answer_count"] == 1
    assert records[0]["answers"][0]["bindings"]["Who"] == {
        "type": "atom",
        "value": "bart",
    }
    assert records[1] == {
        "answer_count": 0,
        "answers": [],
        "query": "parent(marge, Who)",
        "success": False,
    }


def test_cli_repeated_queries_report_script_failure(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "parent(homer, Who)",
        "--query",
        "parent(marge, Who)",
    ])

    assert status == 1
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "false.",
    ]


def test_cli_interactive_loop_runs_queries_from_stdin(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        sys,
        "stdin",
        io.StringIO("parent(homer, Who)\nhalt.\n"),
    )

    status = main([
        "--source",
        "parent(homer, bart).",
        "--interactive",
        "--backend",
        "bytecode",
    ])

    assert status == 0
    assert capsys.readouterr().out == "Who = bart.\n"


def test_cli_interactive_loop_preserves_committed_setup_queries(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(sys, "stdin", io.StringIO("memo(Value)\n:q\n"))

    status = main([
        "--source",
        ":- dynamic(memo/1).",
        "--query",
        "assertz(memo(saved))",
        "--interactive",
        "--commit",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "true.",
        "Value = saved.",
    ]


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


def test_cli_runs_all_file_embedded_queries(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    source_path = tmp_path / "family.pl"
    source_path.write_text(
        "parent(homer, bart).\n"
        "?- parent(homer, Who).\n"
        "?- parent(marge, Who).\n",
        encoding="utf-8",
    )

    status = main([
        str(source_path),
        "--all-source-queries",
        "--backend",
        "bytecode",
    ])

    assert status == 1
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "false.",
    ]


def test_cli_json_output_serializes_all_source_queries(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who). ?- parent(marge, Who).",
        "--all-source-queries",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 1
    assert payload[0]["success"] is True
    assert payload[0]["source_query_index"] == 0
    assert payload[0]["answers"][0]["bindings"]["Who"] == {
        "type": "atom",
        "value": "bart",
    }
    assert payload[1] == {
        "answer_count": 0,
        "answers": [],
        "source_query_index": 1,
        "success": False,
    }


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


def test_cli_rejects_all_source_queries_with_ad_hoc_query(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--all-source-queries",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--all-source-queries cannot be combined" in capsys.readouterr().err


def test_cli_json_output_serializes_validation_errors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--query",
        "parent(homer, Who)",
        "--format",
        "json",
    ])

    captured = capsys.readouterr()
    payload = json.loads(captured.err)

    assert status == 2
    assert captured.out == ""
    assert payload == {
        "error": {
            "message": "provide --source or at least one Prolog file",
            "type": "validation_error",
        },
        "success": False,
    }


def test_cli_json_output_serializes_parse_errors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main(["--format", "json", "--unknown"])

    captured = capsys.readouterr()
    payload = json.loads(captured.err)

    assert status == 2
    assert captured.out == ""
    assert payload["success"] is False
    assert payload["error"]["type"] == "parse_error"
    assert payload["error"]["message"] == "invalid command-line arguments"
    assert payload["error"]["errors"][0]["type"] == "unknown_flag"
    assert "--unknown" in payload["error"]["errors"][0]["message"]


def test_cli_jsonl_output_serializes_runtime_errors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "(",
        "--format",
        "jsonl",
    ])

    captured = capsys.readouterr()
    payload = json.loads(captured.err)

    assert status == 1
    assert captured.out == ""
    assert payload["success"] is False
    assert payload["error"]["type"] == "runtime_error"
    assert payload["error"]["message"]
