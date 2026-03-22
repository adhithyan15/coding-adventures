"""Tests for the standalone validate_spec() and validate_spec_string() functions.

=== What are we testing? ===

The ``validate_spec()`` and ``validate_spec_string()`` functions wrap
``SpecLoader.load()`` in a try/except and return a ``ValidationResult``
instead of raising. We need to verify that:

1. Valid specs produce ``ValidationResult(valid=True, errors=[])``.
2. Every category of invalid spec produces ``valid=False`` with a
   descriptive error message.
3. Non-JSON and missing-file errors are also caught gracefully.

=== Test strategy ===

We embed minimal spec dicts inline and serialize them to JSON strings via
``validate_spec_string()``. For file-based tests (nonexistent file), we use
``validate_spec()`` directly. Each test exercises one validation rule.

=== Relationship to test_spec_loader.py ===

``test_spec_loader.py`` tests the ``SpecLoader`` class directly and asserts
that ``SpecError`` is raised. These tests verify the *wrapper* functions —
that exceptions become ``ValidationResult`` entries, not uncaught crashes.
"""

from __future__ import annotations

import json
import tempfile
from typing import Any

from cli_builder import ValidationResult, validate_spec, validate_spec_string

# =========================================================================
# Helpers
# =========================================================================


def _minimal_spec(**overrides: Any) -> dict[str, Any]:
    """Return a minimal valid spec dict, with optional field overrides.

    This is the smallest spec that passes all validation rules. Override
    specific fields to introduce targeted errors.

    Example::

        # Valid spec
        spec = _minimal_spec()

        # Spec missing the 'name' field
        spec = _minimal_spec(name=None)  # then pop None keys
    """
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "testapp",
        "description": "A test application",
        "version": "1.0.0",
        "flags": [],
        "arguments": [],
        "commands": [],
    }
    spec.update(overrides)
    return spec


def _validate(spec: dict[str, Any]) -> ValidationResult:
    """Convenience: serialize a spec dict and validate it as a string."""
    return validate_spec_string(json.dumps(spec))


# =========================================================================
# Valid spec
# =========================================================================


class TestValidSpec:
    """A well-formed spec should produce valid=True with no errors."""

    def test_minimal_valid_spec(self) -> None:
        """The simplest possible valid spec passes validation."""
        result = _validate(_minimal_spec())
        assert result.valid is True
        assert result.errors == []

    def test_valid_spec_with_flags_and_arguments(self) -> None:
        """A spec with flags and arguments still validates cleanly."""
        spec = _minimal_spec(
            flags=[
                {
                    "id": "verbose",
                    "long": "--verbose",
                    "short": "-v",
                    "description": "Enable verbose output",
                    "type": "boolean",
                }
            ],
            arguments=[
                {
                    "id": "file",
                    "name": "FILE",
                    "description": "Input file",
                    "type": "string",
                }
            ],
        )
        result = _validate(spec)
        assert result.valid is True
        assert result.errors == []


# =========================================================================
# Missing or invalid spec version
# =========================================================================


class TestSpecVersion:
    """The cli_builder_spec_version field is mandatory and must be "1.0"."""

    def test_missing_spec_version(self) -> None:
        """Omitting cli_builder_spec_version produces a descriptive error.

        The version field acts as a format discriminant. Without it, the
        loader cannot determine which validation rules to apply.
        """
        spec = _minimal_spec()
        del spec["cli_builder_spec_version"]
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "cli_builder_spec_version" in result.errors[0]

    def test_unsupported_spec_version(self) -> None:
        """A version string other than "1.0" is rejected.

        Future versions of CLI Builder may support "2.0", but this
        implementation only understands "1.0".
        """
        spec = _minimal_spec(cli_builder_spec_version="2.0")
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "2.0" in result.errors[0]
        assert "Unsupported" in result.errors[0]


# =========================================================================
# Missing required top-level fields
# =========================================================================


class TestRequiredFields:
    """The 'name' and 'description' fields are required at the top level."""

    def test_missing_name(self) -> None:
        """A spec without 'name' is invalid."""
        spec = _minimal_spec()
        del spec["name"]
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "name" in result.errors[0]

    def test_missing_description(self) -> None:
        """A spec without 'description' is invalid."""
        spec = _minimal_spec()
        del spec["description"]
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "description" in result.errors[0]

    def test_empty_name(self) -> None:
        """An empty string for 'name' is treated as missing.

        The SpecLoader uses ``spec.get("name")`` which is falsy for "".
        """
        spec = _minimal_spec(name="")
        result = _validate(spec)

        assert result.valid is False
        assert "name" in result.errors[0]


# =========================================================================
# Invalid flag definitions
# =========================================================================


class TestInvalidFlags:
    """Flags must have valid types and at least one name form."""

    def test_invalid_flag_type(self) -> None:
        """A flag with an unrecognized type value is rejected.

        Valid types are: boolean, string, integer, float, path, file,
        directory, enum. Anything else is a spec error.
        """
        spec = _minimal_spec(
            flags=[
                {
                    "id": "output",
                    "long": "--output",
                    "description": "Output file",
                    "type": "url",  # not a valid type
                }
            ],
        )
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "url" in result.errors[0]
        error_lower = result.errors[0].lower()
        assert "invalid type" in error_lower or "invalid" in error_lower

    def test_flag_with_no_name_form(self) -> None:
        """A flag must have at least one of short, long, or single_dash_long.

        Without any of these, there is no way for the user to specify the
        flag on the command line. The spec is logically incomplete.
        """
        spec = _minimal_spec(
            flags=[
                {
                    "id": "orphan",
                    "description": "A flag with no name",
                    "type": "boolean",
                    # No short, long, or single_dash_long
                }
            ],
        )
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "short" in result.errors[0] or "long" in result.errors[0]


# =========================================================================
# Circular requires dependency
# =========================================================================


class TestCircularRequires:
    """Flags that mutually require each other create a logical contradiction."""

    def test_circular_requires(self) -> None:
        """Two flags that require each other form a cycle.

        If flag A requires B and flag B requires A, there is no valid
        invocation that satisfies both constraints. This is detected via
        directed graph cycle detection (Kahn's algorithm).

        Truth table for the constraint "A requires B AND B requires A":

            A present | B present | Valid?
            ----------|-----------|-------
            No        | No        | Yes (vacuously — neither used)
            No        | Yes       | No (B requires A, but A absent)
            Yes       | No        | No (A requires B, but B absent)
            Yes       | Yes       | Yes (both present)

        The only valid states are "both absent" and "both present". But if
        both must always appear together, they should be a single flag. The
        cycle indicates a design mistake.
        """
        spec = _minimal_spec(
            flags=[
                {
                    "id": "alpha",
                    "long": "--alpha",
                    "description": "Flag alpha",
                    "type": "boolean",
                    "requires": ["beta"],
                },
                {
                    "id": "beta",
                    "long": "--beta",
                    "description": "Flag beta",
                    "type": "boolean",
                    "requires": ["alpha"],
                },
            ],
        )
        result = _validate(spec)

        assert result.valid is False
        assert len(result.errors) == 1
        assert "ircular" in result.errors[0] or "cycle" in result.errors[0].lower()


# =========================================================================
# Invalid JSON
# =========================================================================


class TestInvalidJson:
    """Malformed JSON should be caught and reported, not crash."""

    def test_invalid_json_string(self) -> None:
        """A string that is not valid JSON produces valid=False.

        Common causes: trailing commas, single quotes, unquoted keys.
        """
        result = validate_spec_string("{not valid json!}")

        assert result.valid is False
        assert len(result.errors) == 1
        assert "JSON" in result.errors[0] or "json" in result.errors[0].lower()

    def test_json_array_instead_of_object(self) -> None:
        """A JSON array at the top level is not a valid spec.

        The spec must be a JSON object (dict), not an array (list).
        """
        result = validate_spec_string("[1, 2, 3]")

        assert result.valid is False
        assert len(result.errors) == 1

    def test_empty_string(self) -> None:
        """An empty string is not valid JSON."""
        result = validate_spec_string("")

        assert result.valid is False
        assert len(result.errors) == 1


# =========================================================================
# Nonexistent file
# =========================================================================


class TestNonexistentFile:
    """Pointing validate_spec() at a missing file should not raise."""

    def test_nonexistent_file(self) -> None:
        """A path that does not exist produces valid=False.

        The error message should mention the file path so the user knows
        which file was not found.
        """
        result = validate_spec("/tmp/this_file_does_not_exist_cli_builder_test.json")

        assert result.valid is False
        assert len(result.errors) == 1
        assert "this_file_does_not_exist" in result.errors[0]


# =========================================================================
# validate_spec() with a real file
# =========================================================================


class TestValidateSpecFile:
    """Test validate_spec() reading from an actual file on disk."""

    def test_valid_file(self) -> None:
        """A valid spec file on disk produces valid=True."""
        spec = _minimal_spec()
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as f:
            json.dump(spec, f)
            tmp_path = f.name

        result = validate_spec(tmp_path)
        assert result.valid is True
        assert result.errors == []

    def test_invalid_file(self) -> None:
        """An invalid spec file on disk produces valid=False."""
        spec = _minimal_spec()
        del spec["name"]
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as f:
            json.dump(spec, f)
            tmp_path = f.name

        result = validate_spec(tmp_path)
        assert result.valid is False
        assert "name" in result.errors[0]


# =========================================================================
# ValidationResult dataclass behavior
# =========================================================================


class TestValidationResult:
    """Verify the ValidationResult dataclass itself behaves correctly."""

    def test_default_errors_is_empty_list(self) -> None:
        """The errors field defaults to an empty list.

        This ensures ``ValidationResult(valid=True)`` works without
        explicitly passing ``errors=[]``.
        """
        result = ValidationResult(valid=True)
        assert result.errors == []

    def test_equality(self) -> None:
        """Two ValidationResults with the same fields are equal.

        Dataclass equality is structural — it compares field values,
        not object identity.
        """
        a = ValidationResult(valid=False, errors=["missing name"])
        b = ValidationResult(valid=False, errors=["missing name"])
        assert a == b

    def test_errors_are_independent(self) -> None:
        """Each ValidationResult has its own errors list.

        Mutable default fields in dataclasses can be shared across instances
        if implemented incorrectly. The ``field(default_factory=list)`` pattern
        prevents this.
        """
        a = ValidationResult(valid=True)
        b = ValidationResult(valid=True)
        a.errors.append("oops")
        assert b.errors == []
