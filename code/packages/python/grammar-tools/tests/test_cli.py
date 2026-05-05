"""Tests for the grammar-tools command-line compiler."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from grammar_tools.cli import main
from grammar_tools.parser_grammar import ParserGrammar
from grammar_tools.token_grammar import TokenGrammar


def _exec_generated(path: Path, name: str) -> object:
    namespace: dict[str, Any] = {}
    exec(path.read_text(encoding="utf-8"), namespace)
    return namespace[name]


def test_compile_tokens_writes_python_module(tmp_path: Path) -> None:
    source = tmp_path / "mini.tokens"
    output = tmp_path / "mini_tokens.py"
    source.write_text(
        """
NAME = /[a-z]+/
EQUALS = "="

keywords:
  begin
""".strip(),
        encoding="utf-8",
    )

    result = main(["compile-tokens", str(source), "-o", str(output)])

    loaded = _exec_generated(output, "TOKEN_GRAMMAR")
    assert result == 0
    assert isinstance(loaded, TokenGrammar)
    assert loaded.token_names() == {"NAME", "EQUALS"}
    assert loaded.keywords == ["begin"]


def test_compile_grammar_writes_python_module(tmp_path: Path) -> None:
    source = tmp_path / "mini.grammar"
    output = tmp_path / "mini_grammar.py"
    source.write_text("program = NAME ;", encoding="utf-8")

    result = main(["compile-grammar", str(source), "-o", str(output)])

    loaded = _exec_generated(output, "PARSER_GRAMMAR")
    assert result == 0
    assert isinstance(loaded, ParserGrammar)
    assert [rule.name for rule in loaded.rules] == ["program"]


def test_compile_tokens_writes_stdout(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    source = tmp_path / "mini.tokens"
    source.write_text("NAME = /[a-z]+/", encoding="utf-8")

    result = main(["compile-tokens", str(source)])

    captured = capsys.readouterr()
    assert result == 0
    assert "TOKEN_GRAMMAR = TokenGrammar(" in captured.out
    assert captured.err == ""


def test_command_help_is_generated_by_cli_builder(
    capsys: pytest.CaptureFixture[str],
) -> None:
    result = main(["compile-tokens", "--help"])

    captured = capsys.readouterr()
    assert result == 0
    assert "compile-tokens" in captured.out
    assert "Output Python module path" in captured.out
    assert captured.err == ""


def test_root_invocation_requires_command(capsys: pytest.CaptureFixture[str]) -> None:
    result = main([])

    captured = capsys.readouterr()
    assert result == 2
    assert captured.out == ""
    assert "expected a command" in captured.err


def test_compile_grammar_reports_parse_errors(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    source = tmp_path / "broken.grammar"
    source.write_text("program = NAME", encoding="utf-8")

    result = main(["compile-grammar", str(source)])

    captured = capsys.readouterr()
    assert result == 1
    assert captured.out == ""
    assert "Expected SEMI" in captured.err
