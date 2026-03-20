"""Tests for the grammar-tools CLI validation tool.

These tests verify that ``python -m grammar_tools`` correctly validates
``.tokens`` and ``.grammar`` files, catching typos and inconsistencies.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from grammar_tools.__main__ import (
    main,
    validate_command,
    validate_grammar_only,
    validate_tokens_only,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write_temp(content: str, suffix: str) -> str:
    """Write content to a temp file and return its path."""
    f = tempfile.NamedTemporaryFile(mode="w", suffix=suffix, delete=False)
    f.write(content)
    f.close()
    return f.name


# A minimal valid .tokens file for testing
VALID_TOKENS = """\
NUMBER = /[0-9]+/
PLUS = "+"
STAR = "*"
LPAREN = "("
RPAREN = ")"
"""

# A minimal valid .grammar file that uses the above tokens
VALID_GRAMMAR = """\
expr = term { PLUS term } ;
term = NUMBER | LPAREN expr RPAREN ;
"""


# ---------------------------------------------------------------------------
# validate command tests (both files)
# ---------------------------------------------------------------------------


class TestValidateCommand:
    """Tests for the full validate command (tokens + grammar)."""

    def test_valid_pair(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A valid tokens/grammar pair should return 0."""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        grammar_path = _write_temp(VALID_GRAMMAR, ".grammar")
        result = validate_command(tokens_path, grammar_path)
        assert result == 0
        captured = capsys.readouterr()
        assert "All checks passed" in captured.out

    def test_missing_tokens_file(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A missing .tokens file should return 1."""
        grammar_path = _write_temp(VALID_GRAMMAR, ".grammar")
        result = validate_command("/nonexistent.tokens", grammar_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "File not found" in captured.out

    def test_missing_grammar_file(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A missing .grammar file should return 1."""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        result = validate_command(tokens_path, "/nonexistent.grammar")
        assert result == 1
        captured = capsys.readouterr()
        assert "File not found" in captured.out

    def test_invalid_tokens_syntax(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A .tokens file with syntax errors should return 1."""
        tokens_path = _write_temp("NOT VALID TOKENS !!!", ".tokens")
        grammar_path = _write_temp(VALID_GRAMMAR, ".grammar")
        result = validate_command(tokens_path, grammar_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "PARSE ERROR" in captured.out

    def test_invalid_grammar_syntax(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A .grammar file with syntax errors should return 1."""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        grammar_path = _write_temp("not a valid grammar !!!", ".grammar")
        result = validate_command(tokens_path, grammar_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "PARSE ERROR" in captured.out

    def test_typo_in_token_reference(
        self, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """A grammar referencing a mistyped token should report an error."""
        grammar_with_typo = """\
expr = term { PLUSS term } ;
term = NUMBER ;
"""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        grammar_path = _write_temp(grammar_with_typo, ".grammar")
        result = validate_command(tokens_path, grammar_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "PLUSS" in captured.out

    def test_typo_in_rule_reference(
        self, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """A grammar referencing a mistyped rule should report an error."""
        grammar_with_typo = """\
expr = trm { PLUS trm } ;
trm = NUMBER ;
"""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        grammar_path = _write_temp(grammar_with_typo, ".grammar")
        # 'trm' is defined (second rule), so no undefined reference here.
        # But let's do an actual undefined reference:
        grammar_with_undef = """\
expr = terrm { PLUS terrm } ;
"""
        grammar_path2 = _write_temp(grammar_with_undef, ".grammar")
        result = validate_command(tokens_path, grammar_path2)
        assert result == 1
        captured = capsys.readouterr()
        assert "terrm" in captured.out

    def test_unused_token_warning_does_not_fail(
        self, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Unused token warnings should NOT cause a non-zero exit code.

        Warnings are informational — only errors (missing references, typos)
        should cause failure. An unused token might be intentional.
        """
        # STAR is defined in tokens but not used in grammar
        grammar_no_star = """\
expr = NUMBER { PLUS NUMBER } ;
"""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        grammar_path = _write_temp(grammar_no_star, ".grammar")
        result = validate_command(tokens_path, grammar_path)
        assert result == 0  # Warnings don't fail
        captured = capsys.readouterr()
        assert "STAR" in captured.out
        assert "never used" in captured.out
        assert "All checks passed" in captured.out


# ---------------------------------------------------------------------------
# validate-tokens tests
# ---------------------------------------------------------------------------


class TestValidateTokensOnly:
    """Tests for validating just a .tokens file."""

    def test_valid_tokens(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A valid .tokens file should return 0."""
        tokens_path = _write_temp(VALID_TOKENS, ".tokens")
        result = validate_tokens_only(tokens_path)
        assert result == 0
        captured = capsys.readouterr()
        assert "All checks passed" in captured.out

    def test_missing_file(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A missing file should return 1."""
        result = validate_tokens_only("/nonexistent.tokens")
        assert result == 1

    def test_parse_error(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A file with parse errors should return 1."""
        tokens_path = _write_temp("BROKEN !!!", ".tokens")
        result = validate_tokens_only(tokens_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "PARSE ERROR" in captured.out

    def test_duplicate_token(self, capsys: pytest.CaptureFixture[str]) -> None:
        """Duplicate token names should be reported."""
        tokens_with_dup = """\
NUMBER = /[0-9]+/
NUMBER = /[0-9]+/
"""
        tokens_path = _write_temp(tokens_with_dup, ".tokens")
        result = validate_tokens_only(tokens_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "Duplicate" in captured.out


# ---------------------------------------------------------------------------
# validate-grammar tests
# ---------------------------------------------------------------------------


class TestValidateGrammarOnly:
    """Tests for validating just a .grammar file."""

    def test_valid_grammar(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A valid .grammar file should return 0."""
        grammar_path = _write_temp(VALID_GRAMMAR, ".grammar")
        result = validate_grammar_only(grammar_path)
        assert result == 0
        captured = capsys.readouterr()
        assert "All checks passed" in captured.out

    def test_missing_file(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A missing file should return 1."""
        result = validate_grammar_only("/nonexistent.grammar")
        assert result == 1

    def test_parse_error(self, capsys: pytest.CaptureFixture[str]) -> None:
        """A file with parse errors should return 1."""
        grammar_path = _write_temp("broken !!!", ".grammar")
        result = validate_grammar_only(grammar_path)
        assert result == 1

    def test_undefined_rule(self, capsys: pytest.CaptureFixture[str]) -> None:
        """An undefined rule reference should be reported."""
        grammar = "expr = undefined_rule ;\n"
        grammar_path = _write_temp(grammar, ".grammar")
        result = validate_grammar_only(grammar_path)
        assert result == 1
        captured = capsys.readouterr()
        assert "undefined_rule" in captured.out


# ---------------------------------------------------------------------------
# main() dispatch tests
# ---------------------------------------------------------------------------


class TestMain:
    """Tests for the main() CLI dispatch function."""

    def test_help(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """--help should print usage and return 0."""
        monkeypatch.setattr("sys.argv", ["grammar_tools", "--help"])
        result = main()
        assert result == 0
        captured = capsys.readouterr()
        assert "Usage" in captured.out

    def test_no_args(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """No arguments should print usage and return 0."""
        monkeypatch.setattr("sys.argv", ["grammar_tools"])
        result = main()
        assert result == 0

    def test_unknown_command(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """An unknown command should return 2."""
        monkeypatch.setattr("sys.argv", ["grammar_tools", "unknown"])
        result = main()
        assert result == 2
        captured = capsys.readouterr()
        assert "Unknown command" in captured.out

    def test_validate_wrong_args(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """validate with wrong number of args should return 2."""
        monkeypatch.setattr("sys.argv", ["grammar_tools", "validate", "one"])
        result = main()
        assert result == 2

    def test_validate_tokens_wrong_args(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """validate-tokens with no file should return 2."""
        monkeypatch.setattr("sys.argv", ["grammar_tools", "validate-tokens"])
        result = main()
        assert result == 2

    def test_validate_grammar_wrong_args(self, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch) -> None:
        """validate-grammar with no file should return 2."""
        monkeypatch.setattr("sys.argv", ["grammar_tools", "validate-grammar"])
        result = main()
        assert result == 2
