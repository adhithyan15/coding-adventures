"""Tests for error types: ParseError, ParseErrors, SpecError.

Covers formatting paths, multiple-error collection, and base class hierarchy.
"""

from __future__ import annotations

import pytest

from cli_builder.errors import CliBuilderError, ParseError, ParseErrors, SpecError


# =========================================================================
# SpecError
# =========================================================================


def test_spec_error_str() -> None:
    """SpecError.__str__ includes the prefix and the message."""
    err = SpecError("Bad spec field")
    assert "CliBuilder spec error" in str(err)
    assert "Bad spec field" in str(err)


def test_spec_error_message_attribute() -> None:
    """SpecError.message attribute holds the raw message."""
    err = SpecError("something wrong")
    assert err.message == "something wrong"


def test_spec_error_is_cli_builder_error() -> None:
    """SpecError inherits from CliBuilderError."""
    err = SpecError("x")
    assert isinstance(err, CliBuilderError)


# =========================================================================
# ParseError.format()
# =========================================================================


def test_parse_error_format_minimal() -> None:
    """ParseError with only error_type and message formats correctly."""
    err = ParseError(
        error_type="unknown_flag",
        message="Unknown flag '--foo'",
    )
    formatted = err.format()
    assert "unknown_flag" in formatted
    assert "Unknown flag '--foo'" in formatted
    # No suggestion or context lines
    assert "Did you mean" not in formatted
    assert "Context" not in formatted


def test_parse_error_format_with_suggestion() -> None:
    """ParseError with a suggestion includes 'Did you mean' line."""
    err = ParseError(
        error_type="unknown_flag",
        message="Unknown flag '--verbos'",
        suggestion="--verbose",
    )
    formatted = err.format()
    assert "Did you mean: --verbose" in formatted


def test_parse_error_format_with_context() -> None:
    """ParseError with context includes a 'Context:' line."""
    err = ParseError(
        error_type="missing_required_argument",
        message="Missing required argument: <FILE>",
        context=["git", "commit"],
    )
    formatted = err.format()
    assert "Context: git commit" in formatted


def test_parse_error_format_with_suggestion_and_context() -> None:
    """ParseError with both suggestion and context includes both lines."""
    err = ParseError(
        error_type="unknown_flag",
        message="Unknown flag '--verbos'",
        suggestion="--verbose",
        context=["myapp", "run"],
    )
    formatted = err.format()
    assert "Did you mean: --verbose" in formatted
    assert "Context: myapp run" in formatted


# =========================================================================
# ParseErrors
# =========================================================================


def test_parse_errors_is_cli_builder_error() -> None:
    """ParseErrors inherits from CliBuilderError."""
    errs = ParseErrors([ParseError(error_type="x", message="y")])
    assert isinstance(errs, CliBuilderError)


def test_parse_errors_str_single() -> None:
    """ParseErrors.__str__ with one error returns its formatted form."""
    errs = ParseErrors(
        [ParseError(error_type="unknown_flag", message="Unknown '--foo'")]
    )
    text = str(errs)
    assert "unknown_flag" in text
    assert "Unknown '--foo'" in text


def test_parse_errors_str_multiple() -> None:
    """ParseErrors.__str__ with multiple errors separates them with blank lines."""
    errs = ParseErrors(
        [
            ParseError(error_type="unknown_flag", message="Unknown '--foo'"),
            ParseError(error_type="missing_required_flag", message="--bar is required"),
        ]
    )
    text = str(errs)
    # Both errors should appear
    assert "unknown_flag" in text
    assert "missing_required_flag" in text
    # Separated by blank line
    assert "\n\n" in text


def test_parse_errors_errors_attribute() -> None:
    """ParseErrors.errors attribute holds the list of ParseError objects."""
    e1 = ParseError(error_type="a", message="msg a")
    e2 = ParseError(error_type="b", message="msg b")
    exc = ParseErrors([e1, e2])
    assert exc.errors == [e1, e2]
