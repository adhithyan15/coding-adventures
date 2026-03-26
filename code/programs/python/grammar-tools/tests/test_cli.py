"""Tests for the grammar-tools CLI program."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

import pytest

# Allow importing main.py from the parent directory.
sys.path.insert(0, str(Path(__file__).parent.parent))

from main import (  # noqa: E402
    ROOT,
    dispatch,
    validate_command,
    validate_grammar_only,
    validate_tokens_only,
)

GRAMMARS_DIR = ROOT / "code" / "grammars"


# ---------------------------------------------------------------------------
# validate_command
# ---------------------------------------------------------------------------


class TestValidateCommand:
    def test_succeeds_on_json_pair(self) -> None:
        tokens = GRAMMARS_DIR / "json.tokens"
        grammar = GRAMMARS_DIR / "json.grammar"
        if tokens.exists() and grammar.exists():
            assert validate_command(str(tokens), str(grammar)) == 0

    def test_succeeds_on_lisp_pair(self) -> None:
        tokens = GRAMMARS_DIR / "lisp.tokens"
        grammar = GRAMMARS_DIR / "lisp.grammar"
        if tokens.exists() and grammar.exists():
            assert validate_command(str(tokens), str(grammar)) == 0

    def test_returns_1_on_missing_tokens(self) -> None:
        assert validate_command("/nonexistent/x.tokens", "any.grammar") == 1

    def test_returns_1_on_missing_grammar(self) -> None:
        tokens = GRAMMARS_DIR / "json.tokens"
        if tokens.exists():
            assert validate_command(str(tokens), "/nonexistent/x.grammar") == 1


# ---------------------------------------------------------------------------
# validate_tokens_only
# ---------------------------------------------------------------------------


class TestValidateTokensOnly:
    def test_succeeds_on_json_tokens(self) -> None:
        tokens = GRAMMARS_DIR / "json.tokens"
        if tokens.exists():
            assert validate_tokens_only(str(tokens)) == 0

    def test_succeeds_on_python_tokens(self) -> None:
        tokens = GRAMMARS_DIR / "python.tokens"
        if tokens.exists():
            assert validate_tokens_only(str(tokens)) == 0

    def test_returns_1_on_missing_file(self) -> None:
        assert validate_tokens_only("/nonexistent/x.tokens") == 1

    def test_bad_content_does_not_crash(self) -> None:
        with tempfile.NamedTemporaryFile(
            suffix=".tokens", mode="w", delete=False
        ) as f:
            f.write("BAD =\n")
            path = f.name
        try:
            result = validate_tokens_only(path)
            assert result in (0, 1)
        finally:
            os.unlink(path)


# ---------------------------------------------------------------------------
# validate_grammar_only
# ---------------------------------------------------------------------------


class TestValidateGrammarOnly:
    def test_succeeds_on_json_grammar(self) -> None:
        grammar = GRAMMARS_DIR / "json.grammar"
        if grammar.exists():
            assert validate_grammar_only(str(grammar)) == 0

    def test_returns_1_on_missing_file(self) -> None:
        assert validate_grammar_only("/nonexistent/x.grammar") == 1


# ---------------------------------------------------------------------------
# dispatch
# ---------------------------------------------------------------------------


class TestDispatch:
    def test_unknown_command_returns_2(self) -> None:
        assert dispatch("unknown", []) == 2

    def test_validate_wrong_file_count_returns_2(self) -> None:
        assert dispatch("validate", ["only-one.tokens"]) == 2

    def test_validate_tokens_no_files_returns_2(self) -> None:
        assert dispatch("validate-tokens", []) == 2

    def test_validate_grammar_no_files_returns_2(self) -> None:
        assert dispatch("validate-grammar", []) == 2

    def test_validate_dispatches_correctly(self) -> None:
        tokens = GRAMMARS_DIR / "json.tokens"
        grammar = GRAMMARS_DIR / "json.grammar"
        if tokens.exists() and grammar.exists():
            assert dispatch("validate", [str(tokens), str(grammar)]) == 0

    def test_validate_tokens_dispatches_correctly(self) -> None:
        tokens = GRAMMARS_DIR / "json.tokens"
        if tokens.exists():
            assert dispatch("validate-tokens", [str(tokens)]) == 0

    def test_validate_grammar_dispatches_correctly(self) -> None:
        grammar = GRAMMARS_DIR / "json.grammar"
        if grammar.exists():
            assert dispatch("validate-grammar", [str(grammar)]) == 0
