"""Tests for the Prolog VM command-line interface."""

from __future__ import annotations

import io
import json
import sys
from pathlib import Path

import pytest

from prolog_vm_compiler.cli import main


def test_cli_dumps_capability_manifest_without_source(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main(["--dump-capabilities", "--format", "json"])

    payload = json.loads(capsys.readouterr().out)
    assert status == 0
    assert payload["success"] is True
    assert payload["mode"] == "capabilities"
    assert payload["status"] == "core-plus-stream-io"
    assert payload["dialects"] == ["iso", "swi"]
    assert payload["backends"] == ["structured", "bytecode"]
    assert payload["capabilities"][0]["specs"][0] == "PR00"
    assert payload["capabilities"][-1]["specs"][-1] == "PR79"


def test_cli_dump_capabilities_rejects_source_modes(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--dump-capabilities",
        "--source",
        "parent(homer, bart).",
    ])

    assert status == 2
    assert "--dump-capabilities cannot be combined with --source" in (
        capsys.readouterr().err
    )


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


def test_cli_reads_source_from_stdin_for_ad_hoc_queries(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        sys,
        "stdin",
        io.StringIO("parent(homer, bart). parent(homer, lisa)."),
    )

    status = main([
        "--source-stdin",
        "--query",
        "parent(homer, Who)",
        "--backend",
        "bytecode",
    ])

    assert status == 0
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "Who = lisa.",
    ]


def test_cli_reads_source_queries_from_stdin(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr(
        sys,
        "stdin",
        io.StringIO("parent(homer, bart). ?- parent(homer, Who)."),
    )

    status = main(["--source-stdin"])

    assert status == 0
    assert capsys.readouterr().out == "Who = bart.\n"


def test_cli_source_stdin_rejects_interactive_mode_without_reading_stdin(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source-stdin",
        "--interactive",
    ])

    assert status == 2
    assert "--source-stdin cannot be combined with --interactive" in (
        capsys.readouterr().err
    )


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


def test_cli_commit_requires_ad_hoc_query(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--commit",
    ])

    assert status == 2
    assert "--commit requires at least one --query" in capsys.readouterr().err


@pytest.mark.parametrize(
    "compile_only_flag",
    [
        "--check",
        "--dump-bytecode",
        "--dump-instructions",
        "--dump-source-metadata",
        "--list-source-queries",
    ],
)
def test_cli_limit_rejects_compile_only_modes(
    compile_only_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        compile_only_flag,
        "--limit",
        "1",
    ])

    assert status == 2
    assert f"--limit cannot be combined with {compile_only_flag}" in (
        capsys.readouterr().err
    )


@pytest.mark.parametrize(
    "compile_only_flag",
    [
        "--check",
        "--dump-bytecode",
        "--dump-instructions",
        "--dump-source-metadata",
        "--list-source-queries",
    ],
)
def test_cli_values_rejects_compile_only_modes(
    compile_only_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        compile_only_flag,
        "--values",
    ])

    assert status == 2
    assert f"--values cannot be combined with {compile_only_flag}" in (
        capsys.readouterr().err
    )


@pytest.mark.parametrize(
    "diagnostic_flag",
    [
        "--dump-bytecode",
        "--dump-instructions",
        "--dump-source-metadata",
        "--list-source-queries",
    ],
)
def test_cli_no_initialize_rejects_non_initializing_diagnostic_modes(
    diagnostic_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        ":- initialization(true). parent(homer, bart). ?- parent(homer, Who).",
        diagnostic_flag,
        "--no-initialize",
    ])

    assert status == 2
    assert f"--no-initialize cannot be combined with {diagnostic_flag}" in (
        capsys.readouterr().err
    )


@pytest.mark.parametrize(
    ("mode_flags", "conflicting_flag"),
    [
        (["--query", "parent(homer, Who)"], "--query"),
        (["--all-source-queries"], "--all-source-queries"),
        (["--check"], "--check"),
        (["--dump-bytecode"], "--dump-bytecode"),
        (["--dump-instructions"], "--dump-instructions"),
        (["--dump-source-metadata"], "--dump-source-metadata"),
        (["--list-source-queries"], "--list-source-queries"),
        (["--interactive"], "--interactive"),
    ],
)
def test_cli_source_query_index_rejects_non_source_query_modes(
    mode_flags: list[str],
    conflicting_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--source-query-index",
        "0",
        *mode_flags,
    ])

    assert status == 2
    assert (
        f"--source-query-index cannot be combined with {conflicting_flag}"
        in capsys.readouterr().err
    )


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


def test_cli_dump_bytecode_renders_disassembly(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--dump-bytecode",
    ])

    assert status == 0
    lines = capsys.readouterr().out.splitlines()
    assert lines[0] == "0000: EMIT_RELATION 0 ; parent/2"
    assert "EMIT_FACT" in lines[1]
    assert "EMIT_QUERY" in lines[2]
    assert lines[-1] == "0003: HALT"


def test_cli_dump_bytecode_json_reports_pools_and_lines(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--dump-bytecode",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload["success"] is True
    assert payload["mode"] == "bytecode"
    assert payload["instruction_count"] == 4
    assert payload["relation_pool_count"] == 1
    assert payload["fact_pool_count"] == 1
    assert payload["rule_pool_count"] == 0
    assert payload["query_pool_count"] == 1
    assert payload["lines"][0] == {
        "comment": "parent/2",
        "index": 0,
        "opcode": "EMIT_RELATION",
        "operand": 0,
    }
    assert payload["lines"][-1] == {
        "comment": None,
        "index": 3,
        "opcode": "HALT",
        "operand": None,
    }


def test_cli_dump_bytecode_rejects_execution_modes(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--dump-bytecode",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--dump-bytecode cannot be combined with --query" in (
        capsys.readouterr().err
    )


def test_cli_dump_instructions_renders_instruction_stream(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--dump-instructions",
    ])

    assert status == 0
    lines = capsys.readouterr().out.splitlines()
    assert lines[0] == "0000: DEF_REL parent/2"
    assert lines[1].startswith("0001: FACT parent(")
    assert lines[2].startswith("0002: QUERY parent(")
    assert " -> " in lines[2]


def test_cli_dump_instructions_json_reports_instruction_records(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who).",
        "--dump-instructions",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload["success"] is True
    assert payload["mode"] == "instructions"
    assert payload["instruction_count"] == 3
    assert payload["instructions"][0] == {
        "index": 0,
        "opcode": "DEF_REL",
        "text": "parent/2",
    }
    assert payload["instructions"][1]["opcode"] == "FACT"
    assert payload["instructions"][1]["text"].startswith("parent(")
    assert payload["instructions"][2]["opcode"] == "QUERY"
    assert " -> " in payload["instructions"][2]["text"]


def test_cli_dump_instructions_rejects_execution_modes(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--dump-instructions",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--dump-instructions cannot be combined with --query" in (
        capsys.readouterr().err
    )


def test_cli_dump_source_metadata_reports_query_counts(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        ":- initialization(true). parent(homer, bart). "
        "?- parent(homer, Who). ?- true.",
        "--dump-source-metadata",
    ])

    assert status == 0
    lines = capsys.readouterr().out.splitlines()
    assert lines[:5] == [
        "dialect: SWI-Prolog",
        "initialization queries: 1",
        "source queries: 2",
        "total queries: 3",
        "instructions: 5",
    ]
    assert lines[5:] == [
        "query 0 (vm query 1) variables: Who",
        "query 1 (vm query 2) variables: (none)",
    ]


def test_cli_dump_source_metadata_json_reports_source_queries(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        ":- initialization(true). parent(homer, bart). "
        "?- parent(homer, Who). ?- true.",
        "--dump-source-metadata",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 0
    assert payload == {
        "dialect": "swi",
        "dialect_display_name": "SWI-Prolog",
        "initialization_query_count": 1,
        "instruction_count": 5,
        "mode": "source_metadata",
        "query_count": 3,
        "source_queries": [
            {"index": 0, "variables": ["Who"], "vm_query_index": 1},
            {"index": 1, "variables": [], "vm_query_index": 2},
        ],
        "source_query_count": 2,
        "success": True,
    }


def test_cli_dump_source_metadata_rejects_execution_modes(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--dump-source-metadata",
        "--query",
        "parent(homer, Who)",
    ])

    assert status == 2
    assert "--dump-source-metadata cannot be combined with --query" in (
        capsys.readouterr().err
    )


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
        "query 0 (vm query 0): Who",
        "query 1 (vm query 1): (no variables)",
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
            {"index": 0, "variables": ["Who"], "vm_query_index": 0},
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


def test_cli_summary_reports_noninteractive_text_counts(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--query",
        "parent(homer, Who)",
        "--query",
        "parent(marge, Who)",
        "--summary",
    ])

    assert status == 1
    assert capsys.readouterr().out.splitlines() == [
        "Who = bart.",
        "false.",
        "summary: queries=2, succeeded=1, failed=1, answers=1.",
    ]


def test_cli_json_summary_wraps_query_results(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart). ?- parent(homer, Who). ?- parent(marge, Who).",
        "--all-source-queries",
        "--summary",
        "--format",
        "json",
    ])

    payload = json.loads(capsys.readouterr().out)

    assert status == 1
    assert payload["success"] is False
    assert payload["summary"] == {
        "answer_count": 1,
        "failed_query_count": 1,
        "mode": "summary",
        "query_count": 2,
        "succeeded_query_count": 1,
        "success": False,
    }
    assert payload["results"][0]["source_query_index"] == 0
    assert payload["results"][1]["source_query_index"] == 1


def test_cli_jsonl_summary_appends_summary_record(
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
        "--summary",
    ])

    records = [
        json.loads(line)
        for line in capsys.readouterr().out.splitlines()
    ]

    assert status == 1
    assert records[-1] == {
        "answer_count": 1,
        "failed_query_count": 1,
        "mode": "summary",
        "query_count": 2,
        "succeeded_query_count": 1,
        "success": False,
    }


def test_cli_summary_rejects_interactive(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--summary",
        "--interactive",
    ])

    assert status == 2
    assert "--summary cannot be combined with --interactive" in (
        capsys.readouterr().err
    )


def test_cli_json_format_rejects_interactive(
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        "--interactive",
        "--format",
        "json",
    ])

    assert status == 2
    assert "--format json cannot be combined with --interactive" in (
        capsys.readouterr().err
    )


@pytest.mark.parametrize(
    "dump_flag",
    ["--dump-bytecode", "--dump-instructions", "--dump-source-metadata"],
)
def test_cli_summary_rejects_compile_only_dump_modes(
    dump_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    status = main([
        "--source",
        "parent(homer, bart).",
        dump_flag,
        "--summary",
    ])

    assert status == 2
    assert f"--summary cannot be combined with {dump_flag}" in (
        capsys.readouterr().err
    )


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


def test_cli_interactive_project_file_graph_uses_query_module(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    family_path = tmp_path / "family.pl"
    family_path.write_text(
        ":- module(family, [ancestor/2]).\n"
        "ancestor(homer, bart).\n",
        encoding="utf-8",
    )
    app_path = tmp_path / "app.pl"
    app_path.write_text(
        ":- module(app, []).\n"
        ":- use_module(family, [ancestor/2]).\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(sys, "stdin", io.StringIO("ancestor(homer, Who)\nhalt.\n"))

    status = main([
        str(app_path),
        str(family_path),
        "--query-module",
        "app",
        "--interactive",
        "--backend",
        "bytecode",
    ])

    assert status == 0
    assert capsys.readouterr().out == "Who = bart.\n"


@pytest.mark.parametrize(
    "mode_flag",
    [
        "--check",
        "--dump-bytecode",
        "--dump-instructions",
        "--dump-source-metadata",
        "--list-source-queries",
        "--all-source-queries",
    ],
)
def test_cli_query_module_rejects_non_query_modes(
    tmp_path: Path,
    mode_flag: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    family_path = tmp_path / "family.pl"
    family_path.write_text("parent(homer, bart).\n", encoding="utf-8")
    app_path = tmp_path / "app.pl"
    app_path.write_text(":- module(app, []).\n", encoding="utf-8")

    status = main([
        str(app_path),
        str(family_path),
        "--query-module",
        "app",
        mode_flag,
    ])

    assert status == 2
    assert "--query-module requires --query or --interactive" in (
        capsys.readouterr().err
    )


def test_cli_query_module_rejects_non_project_inputs(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    source_path = tmp_path / "family.pl"
    source_path.write_text("parent(homer, bart).\n", encoding="utf-8")

    status = main([
        str(source_path),
        "--query",
        "parent(homer, Who)",
        "--query-module",
        "app",
    ])

    assert status == 2
    assert "--query-module requires a project file graph" in capsys.readouterr().err


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
