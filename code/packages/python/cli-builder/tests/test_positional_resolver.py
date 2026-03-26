"""Tests for PositionalResolver and coerce_value.

Covers all type coercions (float, path, file, directory, enum, boolean, string),
the last-wins variadic algorithm edge cases, required_unless_flag exemption,
variadic_max enforcement, and defaults filling.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import Any

import pytest

from cli_builder.errors import ParseError
from cli_builder.positional_resolver import PositionalResolver, coerce_value


# =========================================================================
# coerce_value — type coercions
# =========================================================================


def test_coerce_boolean_true_values() -> None:
    """Boolean type accepts 'true', '1', 'yes' as True."""
    for raw in ("true", "True", "TRUE", "1", "yes", "Yes", "YES"):
        val, err = coerce_value(raw, "boolean", [], [], "flag")
        assert err is None, f"Expected no error for '{raw}'"
        assert val is True, f"Expected True for '{raw}'"


def test_coerce_boolean_false_values() -> None:
    """Boolean type treats any other value as False."""
    val, err = coerce_value("false", "boolean", [], [], "flag")
    assert err is None
    assert val is False


def test_coerce_integer_valid() -> None:
    """Integer type coerces '42' to int 42."""
    val, err = coerce_value("42", "integer", [], [], "count")
    assert err is None
    assert val == 42
    assert isinstance(val, int)


def test_coerce_integer_negative() -> None:
    """Integer type coerces '-7' to int -7."""
    val, err = coerce_value("-7", "integer", [], [], "count")
    assert err is None
    assert val == -7


def test_coerce_integer_invalid() -> None:
    """Integer type returns ParseError for non-numeric string."""
    val, err = coerce_value("abc", "integer", [], [], "count")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_value"
    assert "count" in err.message
    assert "abc" in err.message


def test_coerce_float_valid() -> None:
    """Float type coerces '3.14' to float."""
    val, err = coerce_value("3.14", "float", [], [], "ratio")
    assert err is None
    assert abs(val - 3.14) < 1e-9
    assert isinstance(val, float)


def test_coerce_float_integer_string() -> None:
    """Float type coerces '42' to float 42.0."""
    val, err = coerce_value("42", "float", [], [], "ratio")
    assert err is None
    assert val == 42.0


def test_coerce_float_negative() -> None:
    """Float type coerces '-1.5' to float -1.5."""
    val, err = coerce_value("-1.5", "float", [], [], "ratio")
    assert err is None
    assert val == -1.5


def test_coerce_float_invalid() -> None:
    """Float type returns ParseError for non-numeric string."""
    val, err = coerce_value("not-a-float", "float", [], [], "ratio")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_value"
    assert "ratio" in err.message


def test_coerce_string_valid() -> None:
    """String type returns the raw string unchanged."""
    val, err = coerce_value("hello", "string", [], [], "name")
    assert err is None
    assert val == "hello"


def test_coerce_string_empty_fails() -> None:
    """String type returns ParseError for empty string."""
    val, err = coerce_value("", "string", [], [], "name")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_value"
    assert "name" in err.message


def test_coerce_path_valid() -> None:
    """Path type returns the raw string (no filesystem check)."""
    val, err = coerce_value("/nonexistent/path/to/file.txt", "path", [], [], "src")
    assert err is None
    assert val == "/nonexistent/path/to/file.txt"


def test_coerce_path_relative() -> None:
    """Path type accepts relative paths too."""
    val, err = coerce_value("./relative.txt", "path", [], [], "src")
    assert err is None
    assert val == "./relative.txt"


def test_coerce_file_existing_file() -> None:
    """File type accepts a path that is an existing regular file."""
    with tempfile.NamedTemporaryFile(delete=False) as f:
        tmp_path = f.name
    try:
        val, err = coerce_value(tmp_path, "file", [], [], "input")
        assert err is None
        assert val == tmp_path
    finally:
        os.unlink(tmp_path)


def test_coerce_file_nonexistent() -> None:
    """File type returns ParseError for a non-existent path."""
    val, err = coerce_value("/definitely/does/not/exist.txt", "file", [], [], "input")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_value"
    assert "input" in err.message


def test_coerce_file_directory_not_file() -> None:
    """File type returns ParseError when path is a directory, not a file."""
    with tempfile.TemporaryDirectory() as tmpdir:
        val, err = coerce_value(tmpdir, "file", [], [], "input")
        assert val is None
        assert err is not None
        assert err.error_type == "invalid_value"


def test_coerce_directory_existing() -> None:
    """Directory type accepts a path that is an existing directory."""
    with tempfile.TemporaryDirectory() as tmpdir:
        val, err = coerce_value(tmpdir, "directory", [], [], "dest")
        assert err is None
        assert val == tmpdir


def test_coerce_directory_nonexistent() -> None:
    """Directory type returns ParseError for a non-existent path."""
    val, err = coerce_value("/definitely/does/not/exist_dir", "directory", [], [], "dest")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_value"
    assert "dest" in err.message


def test_coerce_directory_file_not_dir() -> None:
    """Directory type returns ParseError when path is a file, not a directory."""
    with tempfile.NamedTemporaryFile(delete=False) as f:
        tmp_path = f.name
    try:
        val, err = coerce_value(tmp_path, "directory", [], [], "dest")
        assert val is None
        assert err is not None
        assert err.error_type == "invalid_value"
    finally:
        os.unlink(tmp_path)


def test_coerce_enum_valid() -> None:
    """Enum type accepts a value that is in enum_values."""
    val, err = coerce_value("json", "enum", ["json", "csv", "xml"], [], "format")
    assert err is None
    assert val == "json"


def test_coerce_enum_invalid() -> None:
    """Enum type returns ParseError for a value not in enum_values."""
    val, err = coerce_value("yaml", "enum", ["json", "csv", "xml"], [], "format")
    assert val is None
    assert err is not None
    assert err.error_type == "invalid_enum_value"
    assert "yaml" in err.message
    assert "json" in err.message


def test_coerce_unknown_type_passthrough() -> None:
    """Unknown type passes the raw value through (defensive default)."""
    val, err = coerce_value("anything", "nonexistent_type", [], [], "thing")
    assert err is None
    assert val == "anything"


def test_coerce_error_carries_context() -> None:
    """ParseError from coerce_value carries the provided context."""
    ctx = ["myapp", "run"]
    _, err = coerce_value("bad", "integer", [], ctx, "count")
    assert err is not None
    assert err.context == ctx


# =========================================================================
# PositionalResolver — fixed (non-variadic) arguments
# =========================================================================

def _make_string_arg(arg_id: str, name: str, required: bool = True) -> dict[str, Any]:
    return {
        "id": arg_id,
        "name": name,
        "description": f"The {name}",
        "type": "string",
        "required": required,
        "variadic": False,
        "variadic_min": 1 if required else 0,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": [],
    }


def test_fixed_one_arg_ok() -> None:
    """Single required arg: one token maps correctly."""
    resolver = PositionalResolver([_make_string_arg("src", "SOURCE")])
    result, errors = resolver.resolve(["file.txt"], {})
    assert errors == []
    assert result["src"] == "file.txt"


def test_fixed_two_args_ok() -> None:
    """Two required args assigned left-to-right."""
    resolver = PositionalResolver([
        _make_string_arg("src", "SOURCE"),
        _make_string_arg("dst", "DEST"),
    ])
    result, errors = resolver.resolve(["a.txt", "b.txt"], {})
    assert errors == []
    assert result["src"] == "a.txt"
    assert result["dst"] == "b.txt"


def test_fixed_too_many_args() -> None:
    """Extra tokens beyond defined args produce too_many_arguments error."""
    resolver = PositionalResolver([_make_string_arg("src", "SOURCE")])
    result, errors = resolver.resolve(["a.txt", "b.txt", "c.txt"], {})
    assert any(e.error_type == "too_many_arguments" for e in errors)


def test_fixed_missing_required_arg() -> None:
    """Missing required arg produces missing_required_argument error."""
    resolver = PositionalResolver([_make_string_arg("src", "SOURCE")])
    result, errors = resolver.resolve([], {})
    assert any(e.error_type == "missing_required_argument" for e in errors)


def test_fixed_optional_arg_absent_ok() -> None:
    """Optional arg that is absent produces no error; default is None."""
    resolver = PositionalResolver([_make_string_arg("src", "SOURCE", required=False)])
    result, errors = resolver.resolve([], {})
    assert errors == []
    assert result["src"] is None


def test_fixed_arg_coerce_error() -> None:
    """Coerce error for a fixed arg is captured in errors."""
    arg = {
        "id": "count",
        "name": "COUNT",
        "description": "A number",
        "type": "integer",
        "required": True,
        "variadic": False,
        "variadic_min": 1,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": [],
    }
    resolver = PositionalResolver([arg])
    result, errors = resolver.resolve(["not-a-number"], {})
    assert any(e.error_type == "invalid_value" for e in errors)


def test_no_arg_defs_with_tokens_error() -> None:
    """No argument definitions but tokens provided → too_many_arguments."""
    resolver = PositionalResolver([])
    result, errors = resolver.resolve(["extra"], {})
    assert any(e.error_type == "too_many_arguments" for e in errors)


def test_no_arg_defs_no_tokens_ok() -> None:
    """No argument definitions and no tokens → no error."""
    resolver = PositionalResolver([])
    result, errors = resolver.resolve([], {})
    assert errors == []
    assert result == {}


# =========================================================================
# PositionalResolver — required_unless_flag exemption
# =========================================================================


def test_required_unless_flag_exempts_arg() -> None:
    """Argument is not required when the exempting flag is present."""
    arg = {
        "id": "src",
        "name": "SOURCE",
        "description": "Source",
        "type": "string",
        "required": True,
        "variadic": False,
        "variadic_min": 1,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": ["dry-run"],
    }
    resolver = PositionalResolver([arg])
    # "dry-run" flag is present (truthy) — arg should be optional
    result, errors = resolver.resolve([], {"dry-run": True})
    assert errors == []


def test_required_unless_flag_not_triggered_when_flag_absent() -> None:
    """Argument IS required when the exempting flag is absent (False/None)."""
    arg = {
        "id": "src",
        "name": "SOURCE",
        "description": "Source",
        "type": "string",
        "required": True,
        "variadic": False,
        "variadic_min": 1,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": ["dry-run"],
    }
    resolver = PositionalResolver([arg])
    result, errors = resolver.resolve([], {"dry-run": False})
    assert any(e.error_type == "missing_required_argument" for e in errors)


def test_required_unless_flag_not_triggered_when_flag_none() -> None:
    """Argument IS required when the exempting flag value is None."""
    arg = {
        "id": "src",
        "name": "SOURCE",
        "description": "Source",
        "type": "string",
        "required": True,
        "variadic": False,
        "variadic_min": 1,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": ["output"],
    }
    resolver = PositionalResolver([arg])
    result, errors = resolver.resolve([], {"output": None})
    assert any(e.error_type == "missing_required_argument" for e in errors)


# =========================================================================
# PositionalResolver — variadic cases
# =========================================================================


def _make_variadic_arg(
    arg_id: str,
    name: str,
    v_min: int = 0,
    v_max: int | None = None,
    required: bool = False,
) -> dict[str, Any]:
    return {
        "id": arg_id,
        "name": name,
        "description": f"The {name}",
        "type": "string",
        "required": required,
        "variadic": True,
        "variadic_min": v_min,
        "variadic_max": v_max,
        "default": None,
        "enum_values": [],
        "required_unless_flag": [],
    }


def test_variadic_absorbs_middle_tokens() -> None:
    """Last-wins: leading before variadic, trailing after variadic."""
    # [leading: src] [variadic: files] [trailing: dest]
    defs = [
        _make_string_arg("leading", "LEADING"),
        _make_variadic_arg("files", "FILES"),
        _make_string_arg("trailing", "TRAILING"),
    ]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve(["L", "a", "b", "T"], {})
    assert errors == []
    assert result["leading"] == "L"
    assert result["files"] == ["a", "b"]
    assert result["trailing"] == "T"


def test_variadic_only_min_one_ok() -> None:
    """Variadic with v_min=1 succeeds when at least one token present."""
    defs = [_make_variadic_arg("files", "FILES", v_min=1, required=True)]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve(["a.txt"], {})
    assert errors == []
    assert result["files"] == ["a.txt"]


def test_variadic_too_few_tokens() -> None:
    """Variadic with v_min=2 produces too_few_arguments when only one token given."""
    defs = [_make_variadic_arg("files", "FILES", v_min=2, required=True)]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve(["a.txt"], {})
    assert any(e.error_type == "too_few_arguments" for e in errors)


def test_variadic_max_exceeded() -> None:
    """Variadic with v_max=2 produces too_many_arguments when three tokens given."""
    defs = [_make_variadic_arg("files", "FILES", v_min=1, v_max=2, required=True)]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve(["a", "b", "c"], {})
    assert any(e.error_type == "too_many_arguments" for e in errors)


def test_variadic_max_exact_ok() -> None:
    """Variadic at exact v_max produces no error."""
    defs = [_make_variadic_arg("files", "FILES", v_min=1, v_max=2, required=True)]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve(["a", "b"], {})
    assert errors == []
    assert result["files"] == ["a", "b"]


def test_variadic_zero_tokens_with_min_zero() -> None:
    """Variadic with v_min=0 succeeds when no tokens given."""
    defs = [_make_variadic_arg("files", "FILES", v_min=0, required=False)]
    resolver = PositionalResolver(defs)
    result, errors = resolver.resolve([], {})
    assert errors == []
    assert result["files"] == []


def test_variadic_missing_trailing_required() -> None:
    """Trailing required arg after variadic produces missing_required_argument."""
    defs = [
        _make_variadic_arg("files", "FILES", v_min=0, required=False),
        _make_string_arg("dest", "DEST", required=True),
    ]
    resolver = PositionalResolver(defs)
    # Only one token: goes to trailing "dest", but variadic gets nothing.
    # With zero tokens: both missing
    result, errors = resolver.resolve([], {})
    assert any(e.error_type == "missing_required_argument" for e in errors)


def test_variadic_coerce_error_captured() -> None:
    """Coerce error in variadic tokens is captured in errors list."""
    arg = {
        "id": "nums",
        "name": "NUMS",
        "description": "Numbers",
        "type": "integer",
        "required": False,
        "variadic": True,
        "variadic_min": 0,
        "variadic_max": None,
        "default": None,
        "enum_values": [],
        "required_unless_flag": [],
    }
    resolver = PositionalResolver([arg])
    result, errors = resolver.resolve(["1", "bad", "3"], {})
    assert any(e.error_type == "invalid_value" for e in errors)


def test_default_fills_absent_optional_arg() -> None:
    """Absent optional arg is filled with its default value."""
    arg = {
        "id": "format",
        "name": "FORMAT",
        "description": "Format",
        "type": "string",
        "required": False,
        "variadic": False,
        "variadic_min": 0,
        "variadic_max": None,
        "default": "json",
        "enum_values": [],
        "required_unless_flag": [],
    }
    resolver = PositionalResolver([arg])
    result, errors = resolver.resolve([], {})
    assert errors == []
    assert result["format"] == "json"


def test_context_passed_to_errors() -> None:
    """Errors from resolve() carry the provided context."""
    resolver = PositionalResolver([_make_string_arg("src", "SOURCE")])
    ctx = ["myapp", "copy"]
    result, errors = resolver.resolve([], {}, context=ctx)
    assert errors
    for e in errors:
        assert e.context == ctx
