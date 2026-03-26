"""Tests for FlagValidator — flag constraint checking.

Covers conflicts_with deduplication, transitive requires chains, required
flags, required_unless exemption, exclusive groups (violation and missing),
and the _flag_display helper.
"""

from __future__ import annotations

from typing import Any

import pytest

from cli_builder.flag_validator import FlagValidator


# =========================================================================
# Helpers
# =========================================================================


def _make_bool_flag(fid: str, short: str | None = None, long: str | None = None,
                    sdl: str | None = None, conflicts_with: list[str] | None = None,
                    requires: list[str] | None = None,
                    required: bool = False,
                    required_unless: list[str] | None = None) -> dict[str, Any]:
    f: dict[str, Any] = {
        "id": fid,
        "description": f"Flag {fid}",
        "type": "boolean",
        "required": required,
        "conflicts_with": conflicts_with or [],
        "requires": requires or [],
        "required_unless": required_unless or [],
        "repeatable": False,
        "default": None,
        "value_name": None,
        "enum_values": [],
    }
    if short:
        f["short"] = short
    if long:
        f["long"] = long
    if sdl:
        f["single_dash_long"] = sdl
    return f


# =========================================================================
# conflicts_with
# =========================================================================


def test_no_conflicts_no_errors() -> None:
    """Flags with no conflicts_with produce no errors."""
    flags = [
        _make_bool_flag("verbose", short="v", long="verbose"),
        _make_bool_flag("debug", short="d", long="debug"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"verbose": True, "debug": True})
    assert errors == []


def test_conflict_detected() -> None:
    """Two conflicting flags produce a conflicting_flags error."""
    flags = [
        _make_bool_flag("extended", short="E", long="extended-regexp",
                        conflicts_with=["fixed"]),
        _make_bool_flag("fixed", short="F", long="fixed-strings",
                        conflicts_with=["extended"]),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"extended": True, "fixed": True})
    assert any(e.error_type == "conflicting_flags" for e in errors)


def test_conflict_not_reported_twice() -> None:
    """A bidirectional conflict is only reported once (pair deduplication)."""
    flags = [
        _make_bool_flag("a", long="flag-a", conflicts_with=["b"]),
        _make_bool_flag("b", long="flag-b", conflicts_with=["a"]),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"a": True, "b": True})
    conflict_errors = [e for e in errors if e.error_type == "conflicting_flags"]
    assert len(conflict_errors) == 1


def test_conflict_not_present_no_error() -> None:
    """Conflicting flag not used: no error."""
    flags = [
        _make_bool_flag("a", long="flag-a", conflicts_with=["b"]),
        _make_bool_flag("b", long="flag-b"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"a": True, "b": False})
    assert errors == []


# =========================================================================
# requires (transitive)
# =========================================================================


def test_direct_requires_satisfied() -> None:
    """Flag A requires B: no error when both present."""
    flags = [
        _make_bool_flag("human", short="h", long="human-readable",
                        requires=["long"]),
        _make_bool_flag("long", short="l", long="long-listing"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"human": True, "long": True})
    assert errors == []


def test_direct_requires_missing() -> None:
    """Flag A requires B: error when B is absent."""
    flags = [
        _make_bool_flag("human", short="h", long="human-readable",
                        requires=["long"]),
        _make_bool_flag("long", short="l", long="long-listing"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"human": True, "long": False})
    assert any(e.error_type == "missing_dependency_flag" for e in errors)


def test_transitive_requires_chain() -> None:
    """A requires B, B requires C: using A without C raises an error."""
    flags = [
        _make_bool_flag("a", long="flag-a", requires=["b"]),
        _make_bool_flag("b", long="flag-b", requires=["c"]),
        _make_bool_flag("c", long="flag-c"),
    ]
    validator = FlagValidator(flags, [])
    # a is present, b is present but c is missing
    errors = validator.validate({"a": True, "b": True, "c": False})
    assert any(e.error_type == "missing_dependency_flag" for e in errors)


def test_transitive_requires_all_present_no_error() -> None:
    """A requires B requires C: all present → no error."""
    flags = [
        _make_bool_flag("a", long="flag-a", requires=["b"]),
        _make_bool_flag("b", long="flag-b", requires=["c"]),
        _make_bool_flag("c", long="flag-c"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"a": True, "b": True, "c": True})
    assert errors == []


def test_requires_only_checked_when_flag_present() -> None:
    """A requires B: no error if A is absent (not present)."""
    flags = [
        _make_bool_flag("a", long="flag-a", requires=["b"]),
        _make_bool_flag("b", long="flag-b"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"a": False, "b": False})
    assert errors == []


# =========================================================================
# required flags
# =========================================================================


def test_required_flag_present_no_error() -> None:
    """Required flag that is present produces no error."""
    flags = [_make_bool_flag("output", long="output", required=True)]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"output": True})
    assert errors == []


def test_required_flag_absent_error() -> None:
    """Required flag that is absent produces missing_required_flag error."""
    flags = [_make_bool_flag("output", long="output", required=True)]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"output": False})
    assert any(e.error_type == "missing_required_flag" for e in errors)


def test_required_flag_none_is_absent() -> None:
    """A required flag with value None is treated as absent."""
    flags = [
        {
            "id": "output",
            "long": "output",
            "description": "Output file",
            "type": "string",
            "required": True,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "default": None,
            "value_name": None,
            "enum_values": [],
        }
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"output": None})
    assert any(e.error_type == "missing_required_flag" for e in errors)


def test_required_unless_exempts_flag() -> None:
    """required_unless: flag is not required when the exempting flag is present."""
    flags = [
        {
            "id": "output",
            "long": "output",
            "description": "Output",
            "type": "string",
            "required": True,
            "conflicts_with": [],
            "requires": [],
            "required_unless": ["dry-run"],
            "repeatable": False,
            "default": None,
            "value_name": None,
            "enum_values": [],
        },
        _make_bool_flag("dry-run", long="dry-run"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"output": None, "dry-run": True})
    assert not any(e.error_type == "missing_required_flag" for e in errors)


def test_required_unless_not_triggered_when_exempting_false() -> None:
    """required_unless: flag IS required when the exempting flag is False."""
    flags = [
        {
            "id": "output",
            "long": "output",
            "description": "Output",
            "type": "string",
            "required": True,
            "conflicts_with": [],
            "requires": [],
            "required_unless": ["dry-run"],
            "repeatable": False,
            "default": None,
            "value_name": None,
            "enum_values": [],
        },
        _make_bool_flag("dry-run", long="dry-run"),
    ]
    validator = FlagValidator(flags, [])
    errors = validator.validate({"output": None, "dry-run": False})
    assert any(e.error_type == "missing_required_flag" for e in errors)


# =========================================================================
# mutually_exclusive_groups
# =========================================================================


def test_exclusive_group_one_flag_ok() -> None:
    """Exclusive group with one flag used: no error."""
    flags = [
        _make_bool_flag("extended", short="E", long="extended-regexp"),
        _make_bool_flag("fixed", short="F", long="fixed-strings"),
        _make_bool_flag("perl", short="P", long="perl-regexp"),
    ]
    groups = [
        {"id": "regexp-engine", "flag_ids": ["extended", "fixed", "perl"], "required": False}
    ]
    validator = FlagValidator(flags, groups)
    errors = validator.validate({"extended": True, "fixed": False, "perl": False})
    assert errors == []


def test_exclusive_group_two_flags_error() -> None:
    """Exclusive group with two flags used: exclusive_group_violation error."""
    flags = [
        _make_bool_flag("extended", short="E", long="extended-regexp"),
        _make_bool_flag("fixed", short="F", long="fixed-strings"),
    ]
    groups = [
        {"id": "regexp-engine", "flag_ids": ["extended", "fixed"], "required": False}
    ]
    validator = FlagValidator(flags, groups)
    errors = validator.validate({"extended": True, "fixed": True})
    assert any(e.error_type == "exclusive_group_violation" for e in errors)


def test_exclusive_group_required_none_present_error() -> None:
    """Required exclusive group with no flags used: missing_exclusive_group error."""
    flags = [
        _make_bool_flag("json", long="json"),
        _make_bool_flag("xml", long="xml"),
    ]
    groups = [
        {"id": "output-format", "flag_ids": ["json", "xml"], "required": True}
    ]
    validator = FlagValidator(flags, groups)
    errors = validator.validate({"json": False, "xml": False})
    assert any(e.error_type == "missing_exclusive_group" for e in errors)


def test_exclusive_group_required_one_present_ok() -> None:
    """Required exclusive group with one flag used: no error."""
    flags = [
        _make_bool_flag("json", long="json"),
        _make_bool_flag("xml", long="xml"),
    ]
    groups = [
        {"id": "output-format", "flag_ids": ["json", "xml"], "required": True}
    ]
    validator = FlagValidator(flags, groups)
    errors = validator.validate({"json": True, "xml": False})
    assert errors == []


# =========================================================================
# _flag_display helper
# =========================================================================


def test_flag_display_short_and_long() -> None:
    """_flag_display produces -s/--long form."""
    flags = [_make_bool_flag("verbose", short="v", long="verbose")]
    validator = FlagValidator(flags, [])
    display = validator._flag_display("verbose")
    assert "-v" in display
    assert "--verbose" in display


def test_flag_display_long_only() -> None:
    """_flag_display with long-only flag produces --long form."""
    flags = [_make_bool_flag("verbose", long="verbose")]
    validator = FlagValidator(flags, [])
    display = validator._flag_display("verbose")
    assert display == "--verbose"


def test_flag_display_short_only() -> None:
    """_flag_display with short-only flag produces -s form."""
    flags = [_make_bool_flag("v", short="v")]
    validator = FlagValidator(flags, [])
    display = validator._flag_display("v")
    assert display == "-v"


def test_flag_display_single_dash_long_only() -> None:
    """_flag_display with single_dash_long only produces -name form."""
    flags = [_make_bool_flag("classpath", sdl="classpath")]
    validator = FlagValidator(flags, [])
    display = validator._flag_display("classpath")
    assert display == "-classpath"


def test_flag_display_unknown_id_fallback() -> None:
    """_flag_display for unknown flag id falls back to --id form."""
    flags = [_make_bool_flag("verbose", long="verbose")]
    validator = FlagValidator(flags, [])
    display = validator._flag_display("nonexistent-flag")
    assert display == "--nonexistent-flag"


def test_validate_with_context() -> None:
    """Errors from validate() carry the provided context."""
    flags = [_make_bool_flag("output", long="output", required=True)]
    validator = FlagValidator(flags, [])
    ctx = ["myapp", "run"]
    errors = validator.validate({"output": False}, context=ctx)
    for e in errors:
        assert e.context == ctx


def test_validate_empty_flags() -> None:
    """Validating with no flags and no constraints produces no errors."""
    validator = FlagValidator([], [])
    errors = validator.validate({})
    assert errors == []
