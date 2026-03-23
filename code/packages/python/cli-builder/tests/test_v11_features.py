"""Tests for CLI Builder v1.1 features.

This module covers the four backwards-compatible features introduced in v1.1:

1. **Count type** — ``"type": "count"`` flags that increment on each occurrence.
2. **Enum optional values** — ``default_when_present`` for enum flags used
   without a value (``--color`` instead of ``--color=always``).
3. **Flag presence detection** — ``explicit_flags`` list in ParseResult.
4. **int64 range validation** — Integer values outside [-2^63, 2^63-1]
   produce ``invalid_value`` errors.
"""

from __future__ import annotations

import json
import tempfile
from typing import Any

import pytest

from cli_builder.errors import ParseErrors, SpecError
from cli_builder.parser import Parser
from cli_builder.positional_resolver import coerce_value
from cli_builder.spec_loader import SpecLoader
from cli_builder.token_classifier import TokenClassifier
from cli_builder.types import ParseResult

# =========================================================================
# Helpers
# =========================================================================


def make_spec_file(spec: dict[str, Any]) -> str:
    """Write a spec dict to a temp file and return the path string."""
    f = tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8"
    )
    json.dump(spec, f)
    f.close()
    return f.name


# =========================================================================
# Shared spec definitions
# =========================================================================

# A spec with a count-type flag (verbose) and a boolean flag (quiet).
COUNT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "App with count flag",
    "version": "1.0.0",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Increase verbosity",
            "type": "count",
        },
        {
            "id": "quiet",
            "short": "q",
            "long": "quiet",
            "description": "Suppress output",
            "type": "boolean",
        },
    ],
    "arguments": [],
    "commands": [],
}

# A spec with an enum flag that has default_when_present.
ENUM_DWP_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "App with enum optional value",
    "version": "1.0.0",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "color",
            "long": "color",
            "description": "Colorize output",
            "type": "enum",
            "enum_values": ["always", "never", "auto"],
            "default_when_present": "always",
            "default": "auto",
        },
    ],
    "arguments": [
        {
            "id": "file",
            "display_name": "FILE",
            "description": "Input file",
            "type": "string",
            "required": False,
        },
    ],
    "commands": [],
}

# A spec with an integer argument for int64 range testing.
INT64_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "App with integer arg",
    "version": "1.0.0",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "count",
            "long": "count",
            "short": "c",
            "description": "Number of items",
            "type": "integer",
        },
    ],
    "arguments": [
        {
            "id": "num",
            "display_name": "NUM",
            "description": "An integer argument",
            "type": "integer",
            "required": False,
        },
    ],
    "commands": [],
}


# =========================================================================
# Feature 1: Count type
# =========================================================================


class TestCountType:
    """Tests for the ``count`` flag type."""

    def test_count_absent_defaults_to_zero(self):
        """A count flag not present on the command line defaults to 0."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 0

    def test_count_single_long_flag(self):
        """``--verbose`` sets the count to 1."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "--verbose"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 1

    def test_count_double_long_flag(self):
        """``--verbose --verbose`` sets the count to 2."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(
            spec_path, ["myapp", "--verbose", "--verbose"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 2

    def test_count_single_short_flag(self):
        """``-v`` sets the count to 1."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "-v"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 1

    def test_count_stacked_short_flags(self):
        """``-vvv`` sets the count to 3 (each character increments)."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "-vvv"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 3

    def test_count_mixed_with_boolean_stacked(self):
        """``-vqv`` increments verbose twice and sets quiet to True."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "-vqv"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 2
        assert result.flags["quiet"] is True

    def test_count_mixed_long_and_short(self):
        """``--verbose -v`` gives count of 2."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(
            spec_path, ["myapp", "--verbose", "-v"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 2

    def test_count_no_duplicate_error(self):
        """Count flags should NOT produce duplicate_flag errors."""
        spec_path = make_spec_file(COUNT_SPEC)
        # This should not raise — count flags are inherently repeatable.
        result = Parser(
            spec_path, ["myapp", "-v", "-v", "-v"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] == 3

    def test_count_type_in_spec_validation(self):
        """The spec loader should accept ``"count"`` as a valid type."""
        spec_path = make_spec_file(COUNT_SPEC)
        spec = SpecLoader(spec_path).load()
        verbose_flag = next(
            f for f in spec["flags"] if f["id"] == "verbose"
        )
        assert verbose_flag["type"] == "count"

    def test_count_type_token_classifier_stacking(self):
        """Count flags should be stackable like boolean flags."""
        flags = [
            {"id": "v", "short": "v", "type": "count"},
            {"id": "q", "short": "q", "type": "boolean"},
        ]
        classifier = TokenClassifier(flags)

        # -vvv should produce stacked_flags, not unknown_flag
        event = classifier.classify("-vvv")
        assert event["type"] == "stacked_flags"
        assert event["chars"] == ["v", "v", "v"]

        # -vqv should also work
        event = classifier.classify("-vqv")
        assert event["type"] == "stacked_flags"
        assert event["chars"] == ["v", "q", "v"]


# =========================================================================
# Feature 2: Enum optional values (default_when_present)
# =========================================================================


class TestDefaultWhenPresent:
    """Tests for the ``default_when_present`` field on enum flags."""

    def test_flag_absent_uses_default(self):
        """When the flag is not present, the regular default is used."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(spec_path, ["myapp"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "auto"

    def test_flag_with_equals_value(self):
        """``--color=never`` uses the explicit value."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(
            spec_path, ["myapp", "--color=never"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "never"

    def test_flag_without_value_uses_default_when_present(self):
        """``--color`` alone uses ``default_when_present`` ("always")."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(spec_path, ["myapp", "--color"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "always"

    def test_flag_followed_by_valid_enum_value(self):
        """``--color never`` consumes "never" as the value."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(
            spec_path, ["myapp", "--color", "never"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "never"

    def test_flag_followed_by_non_enum_token(self):
        """``--color file.txt`` uses default_when_present; file.txt is positional."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(
            spec_path, ["myapp", "--color", "file.txt"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "always"
        assert result.arguments["file"] == "file.txt"

    def test_flag_at_end_of_argv_uses_default_when_present(self):
        """``--color`` at the end of argv uses default_when_present."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(spec_path, ["myapp", "--color"]).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["color"] == "always"

    def test_spec_validation_dwp_not_in_enum_values(self):
        """default_when_present must be in enum_values."""
        bad_spec = {
            "cli_builder_spec_version": "1.0",
            "name": "myapp",
            "description": "Bad spec",
            "flags": [
                {
                    "id": "color",
                    "long": "color",
                    "description": "Color mode",
                    "type": "enum",
                    "enum_values": ["always", "never"],
                    "default_when_present": "auto",
                },
            ],
        }
        spec_path = make_spec_file(bad_spec)
        with pytest.raises(SpecError, match="default_when_present"):
            SpecLoader(spec_path).load()

    def test_spec_validation_dwp_on_non_enum_type(self):
        """default_when_present is only valid for enum flags."""
        bad_spec = {
            "cli_builder_spec_version": "1.0",
            "name": "myapp",
            "description": "Bad spec",
            "flags": [
                {
                    "id": "output",
                    "long": "output",
                    "description": "Output file",
                    "type": "string",
                    "default_when_present": "stdout",
                },
            ],
        }
        spec_path = make_spec_file(bad_spec)
        with pytest.raises(SpecError, match="only 'enum'"):
            SpecLoader(spec_path).load()

    def test_help_shows_optional_value_syntax(self):
        """Help text should show ``[=VALUE]`` for enum flags with dwp."""
        from cli_builder.help_generator import HelpGenerator

        spec_path = make_spec_file(ENUM_DWP_SPEC)
        spec = SpecLoader(spec_path).load()
        gen = HelpGenerator(spec, ["myapp"])
        text = gen.generate()
        assert "[=ENUM]" in text or "[=enum]" in text.lower()


# =========================================================================
# Feature 3: Flag presence detection (explicit_flags)
# =========================================================================


class TestExplicitFlags:
    """Tests for the ``explicit_flags`` field in ParseResult."""

    def test_no_flags_explicit_flags_empty(self):
        """When no flags are passed, explicit_flags is empty."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp"]).parse()
        assert isinstance(result, ParseResult)
        assert result.explicit_flags == []

    def test_boolean_flag_tracked(self):
        """A boolean flag appears in explicit_flags."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "--quiet"]).parse()
        assert isinstance(result, ParseResult)
        assert "quiet" in result.explicit_flags

    def test_count_flag_tracked_per_occurrence(self):
        """Each occurrence of a count flag adds its ID."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "-vvv"]).parse()
        assert isinstance(result, ParseResult)
        assert result.explicit_flags.count("verbose") == 3

    def test_value_flag_tracked(self):
        """A value-taking flag appears in explicit_flags."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(
            spec_path, ["myapp", "--color=never"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert "color" in result.explicit_flags

    def test_absent_flag_not_in_explicit_flags(self):
        """A flag with a default but not on argv is NOT in explicit_flags."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(spec_path, ["myapp"]).parse()
        assert isinstance(result, ParseResult)
        # color has default "auto" but was not explicitly passed
        assert "color" not in result.explicit_flags

    def test_multiple_different_flags_tracked(self):
        """Multiple different flags all appear in explicit_flags."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(
            spec_path, ["myapp", "-v", "--quiet"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert "verbose" in result.explicit_flags
        assert "quiet" in result.explicit_flags

    def test_stacked_flags_each_tracked(self):
        """Stacked flags each get their own explicit_flags entry."""
        spec_path = make_spec_file(COUNT_SPEC)
        result = Parser(spec_path, ["myapp", "-vq"]).parse()
        assert isinstance(result, ParseResult)
        assert "verbose" in result.explicit_flags
        assert "quiet" in result.explicit_flags

    def test_dwp_flag_tracked(self):
        """An enum flag with default_when_present is tracked."""
        spec_path = make_spec_file(ENUM_DWP_SPEC)
        result = Parser(spec_path, ["myapp", "--color"]).parse()
        assert isinstance(result, ParseResult)
        assert "color" in result.explicit_flags


# =========================================================================
# Feature 4: int64 range validation
# =========================================================================


class TestInt64RangeValidation:
    """Tests for integer values outside the int64 range."""

    INT64_MIN = -(2**63)
    INT64_MAX = 2**63 - 1

    def test_valid_integer_in_range(self):
        """Normal integers within range are accepted."""
        spec_path = make_spec_file(INT64_SPEC)
        result = Parser(
            spec_path, ["myapp", "--count", "42"]
        ).parse()
        assert isinstance(result, ParseResult)
        assert result.flags["count"] == 42

    def test_int64_max_accepted(self):
        """The maximum int64 value is accepted."""
        val, err = coerce_value(
            str(self.INT64_MAX), "integer", [], [], "test"
        )
        assert err is None
        assert val == self.INT64_MAX

    def test_int64_min_accepted(self):
        """The minimum int64 value is accepted."""
        val, err = coerce_value(
            str(self.INT64_MIN), "integer", [], [], "test"
        )
        assert err is None
        assert val == self.INT64_MIN

    def test_above_int64_max_rejected(self):
        """A value above INT64_MAX produces an error."""
        over = str(self.INT64_MAX + 1)
        val, err = coerce_value(over, "integer", [], [], "test")
        assert err is not None
        assert err.error_type == "invalid_value"
        assert "int64" in err.message.lower() or "range" in err.message

    def test_below_int64_min_rejected(self):
        """A value below INT64_MIN produces an error."""
        under = str(self.INT64_MIN - 1)
        val, err = coerce_value(under, "integer", [], [], "test")
        assert err is not None
        assert err.error_type == "invalid_value"

    def test_very_large_positive_rejected(self):
        """A very large positive integer is rejected."""
        huge = str(2**100)
        val, err = coerce_value(huge, "integer", [], [], "test")
        assert err is not None
        assert err.error_type == "invalid_value"

    def test_very_large_negative_rejected(self):
        """A very large negative integer is rejected."""
        huge = str(-(2**100))
        val, err = coerce_value(huge, "integer", [], [], "test")
        assert err is not None
        assert err.error_type == "invalid_value"

    def test_integer_argument_range_check(self):
        """Integer positional arguments also get range checked."""
        spec_path = make_spec_file(INT64_SPEC)
        huge = str(2**100)
        with pytest.raises(ParseErrors) as exc_info:
            Parser(spec_path, ["myapp", huge]).parse()
        errors = exc_info.value.errors
        assert any(e.error_type == "invalid_value" for e in errors)

    def test_integer_flag_range_check(self):
        """Integer flags also get range checked."""
        spec_path = make_spec_file(INT64_SPEC)
        huge = str(2**100)
        with pytest.raises(ParseErrors) as exc_info:
            Parser(spec_path, ["myapp", "--count", huge]).parse()
        errors = exc_info.value.errors
        assert any(e.error_type == "invalid_value" for e in errors)

    def test_zero_accepted(self):
        """Zero is a valid integer."""
        val, err = coerce_value("0", "integer", [], [], "test")
        assert err is None
        assert val == 0

    def test_negative_one_accepted(self):
        """Negative one is a valid integer."""
        val, err = coerce_value("-1", "integer", [], [], "test")
        assert err is None
        assert val == -1

    def test_not_a_number_still_gives_invalid_value(self):
        """Non-numeric strings still give invalid_value error."""
        val, err = coerce_value("abc", "integer", [], [], "test")
        assert err is not None
        assert err.error_type == "invalid_value"
