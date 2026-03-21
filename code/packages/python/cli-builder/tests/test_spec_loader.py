"""Tests for SpecLoader — spec file validation and normalization.

We embed minimal JSON specs as strings in temporary files. Each test
exercises one validation rule from spec §6.4.3.
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
from typing import Any

import pytest

from cli_builder.errors import SpecError
from cli_builder.spec_loader import SpecLoader


# =========================================================================
# Helpers
# =========================================================================


def make_spec_file(spec: dict[str, Any]) -> Path:
    """Write a spec dict to a temporary JSON file and return the path."""
    f = tempfile.NamedTemporaryFile(
        mode="w",
        suffix=".json",
        delete=False,
        encoding="utf-8",
    )
    json.dump(spec, f)
    f.close()
    return Path(f.name)


MINIMAL_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "description": "Display a line of text",
    "version": "8.32",
    "flags": [],
    "arguments": [
        {
            "id": "string",
            "name": "STRING",
            "description": "Text to display",
            "type": "string",
            "required": False,
            "variadic": True,
            "variadic_min": 0,
        }
    ],
    "commands": [],
}

GIT_LIKE_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The fast version control system",
    "version": "2.40.0",
    "parsing_mode": "gnu",
    "global_flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Be verbose",
            "type": "boolean",
        }
    ],
    "flags": [],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-remote",
            "name": "remote",
            "description": "Manage set of tracked repositories",
            "flags": [],
            "arguments": [],
            "commands": [
                {
                    "id": "cmd-remote-add",
                    "name": "add",
                    "description": "Add a remote",
                    "flags": [],
                    "arguments": [
                        {
                            "id": "name",
                            "name": "NAME",
                            "description": "Remote name",
                            "type": "string",
                        },
                        {
                            "id": "url",
                            "name": "URL",
                            "description": "Remote URL",
                            "type": "string",
                        },
                    ],
                }
            ],
        }
    ],
}


# =========================================================================
# Happy path
# =========================================================================


def test_minimal_spec_loads() -> None:
    """A minimal echo-like spec loads without errors."""
    path = make_spec_file(MINIMAL_SPEC)
    spec = SpecLoader(path).load()
    assert spec["name"] == "echo"
    assert spec["parsing_mode"] == "gnu"  # default filled in
    assert spec["builtin_flags"] == {"help": True, "version": True}  # defaults


def test_complex_spec_loads() -> None:
    """A git-like spec with nested commands and global flags loads successfully."""
    path = make_spec_file(GIT_LIKE_SPEC)
    spec = SpecLoader(path).load()
    assert spec["name"] == "git"
    assert len(spec["global_flags"]) == 1
    assert spec["commands"][0]["name"] == "remote"


def test_defaults_are_filled() -> None:
    """Defaults for optional fields are injected during load."""
    path = make_spec_file(MINIMAL_SPEC)
    spec = SpecLoader(path).load()
    assert spec["display_name"] == "echo"
    assert spec["global_flags"] == []
    assert spec["mutually_exclusive_groups"] == []


# =========================================================================
# Version validation
# =========================================================================


def test_missing_version_field_raises() -> None:
    """Missing cli_builder_spec_version raises SpecError."""
    bad = dict(MINIMAL_SPEC)
    del bad["cli_builder_spec_version"]
    path = make_spec_file(bad)
    with pytest.raises(SpecError, match="cli_builder_spec_version"):
        SpecLoader(path).load()


def test_wrong_version_raises() -> None:
    """An unsupported spec version raises SpecError."""
    bad = dict(MINIMAL_SPEC)
    bad["cli_builder_spec_version"] = "2.0"
    path = make_spec_file(bad)
    with pytest.raises(SpecError, match="Unsupported spec version"):
        SpecLoader(path).load()


# =========================================================================
# Required field validation
# =========================================================================


def test_missing_name_raises() -> None:
    """Missing 'name' raises SpecError."""
    bad = {k: v for k, v in MINIMAL_SPEC.items() if k != "name"}
    path = make_spec_file(bad)
    with pytest.raises(SpecError, match="'name'"):
        SpecLoader(path).load()


def test_missing_description_raises() -> None:
    """Missing 'description' raises SpecError."""
    bad = {k: v for k, v in MINIMAL_SPEC.items() if k != "description"}
    path = make_spec_file(bad)
    with pytest.raises(SpecError, match="'description'"):
        SpecLoader(path).load()


# =========================================================================
# Flag validation
# =========================================================================


def test_duplicate_flag_ids_raise() -> None:
    """Duplicate flag IDs in the same scope raise SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Be verbose",
            "type": "boolean",
        },
        {
            "id": "verbose",  # duplicate
            "short": "q",
            "long": "quiet",
            "description": "Be quiet",
            "type": "boolean",
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Duplicate flag id 'verbose'"):
        SpecLoader(path).load()


def test_flag_without_name_form_raises() -> None:
    """A flag with no short/long/single_dash_long raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "verbose",
            "description": "Be verbose",
            "type": "boolean",
            # No short, long, or single_dash_long!
        }
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="must have at least one of"):
        SpecLoader(path).load()


def test_enum_without_enum_values_raises() -> None:
    """Type 'enum' without enum_values raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "format",
            "long": "format",
            "description": "Output format",
            "type": "enum",
            # Missing enum_values!
        }
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="enum_values"):
        SpecLoader(path).load()


def test_unknown_conflicts_with_raises() -> None:
    """A flag referencing an unknown ID in conflicts_with raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Be verbose",
            "type": "boolean",
            "conflicts_with": ["nonexistent-flag"],
        }
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="unknown flag 'nonexistent-flag'"):
        SpecLoader(path).load()


def test_unknown_requires_raises() -> None:
    """A flag referencing an unknown ID in requires raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "human-readable",
            "short": "h",
            "long": "human-readable",
            "description": "Human-readable sizes",
            "type": "boolean",
            "requires": ["nonexistent-flag"],
        }
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="unknown flag"):
        SpecLoader(path).load()


# =========================================================================
# Circular requires detection
# =========================================================================


def test_circular_requires_raises() -> None:
    """A → B → A circular requires cycle raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "flag-a",
            "long": "flag-a",
            "description": "Flag A",
            "type": "boolean",
            "requires": ["flag-b"],
        },
        {
            "id": "flag-b",
            "long": "flag-b",
            "description": "Flag B",
            "type": "boolean",
            "requires": ["flag-a"],
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Circular"):
        SpecLoader(path).load()


def test_three_way_cycle_raises() -> None:
    """A → B → C → A three-way cycle raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "a",
            "long": "flag-a",
            "description": "A",
            "type": "boolean",
            "requires": ["b"],
        },
        {
            "id": "b",
            "long": "flag-b",
            "description": "B",
            "type": "boolean",
            "requires": ["c"],
        },
        {
            "id": "c",
            "long": "flag-c",
            "description": "C",
            "type": "boolean",
            "requires": ["a"],
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Circular"):
        SpecLoader(path).load()


def test_non_circular_requires_ok() -> None:
    """A → B → C without a cycle loads successfully."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "a",
            "long": "flag-a",
            "description": "A",
            "type": "boolean",
            "requires": ["b"],
        },
        {
            "id": "b",
            "long": "flag-b",
            "description": "B",
            "type": "boolean",
            "requires": ["c"],
        },
        {
            "id": "c",
            "long": "flag-c",
            "description": "C",
            "type": "boolean",
        },
    ]
    path = make_spec_file(spec)
    SpecLoader(path).load()  # Should not raise


# =========================================================================
# Argument validation
# =========================================================================


def test_multiple_variadic_raises() -> None:
    """Two variadic arguments in the same scope raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["arguments"] = [
        {
            "id": "source",
            "name": "SOURCE",
            "description": "Source",
            "type": "path",
            "variadic": True,
        },
        {
            "id": "dest",
            "name": "DEST",
            "description": "Destination",
            "type": "path",
            "variadic": True,  # Second variadic!
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="more than one variadic"):
        SpecLoader(path).load()


def test_duplicate_argument_ids_raise() -> None:
    """Duplicate argument IDs raise SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["arguments"] = [
        {
            "id": "file",
            "name": "FILE",
            "description": "Input",
            "type": "path",
        },
        {
            "id": "file",  # duplicate
            "name": "DEST",
            "description": "Output",
            "type": "path",
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Duplicate argument id"):
        SpecLoader(path).load()


# =========================================================================
# File I/O errors
# =========================================================================


def test_nonexistent_file_raises() -> None:
    """A nonexistent spec file raises SpecError."""
    with pytest.raises(SpecError, match="Cannot read"):
        SpecLoader("/nonexistent/path/spec.json").load()


def test_invalid_json_raises() -> None:
    """A file with invalid JSON raises SpecError."""
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False
    ) as f:
        f.write("{not valid json")
    with pytest.raises(SpecError, match="not valid JSON"):
        SpecLoader(f.name).load()


def test_non_object_json_raises() -> None:
    """A JSON file that is not an object raises SpecError."""
    path = make_spec_file([1, 2, 3])  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="JSON object"):
        SpecLoader(path).load()


# =========================================================================
# Exclusive group validation
# =========================================================================


def test_exclusive_group_unknown_flag_raises() -> None:
    """An exclusive group referencing an unknown flag ID raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["flags"] = [
        {
            "id": "extended",
            "short": "E",
            "long": "extended-regexp",
            "description": "Extended regexp",
            "type": "boolean",
        }
    ]
    spec["mutually_exclusive_groups"] = [
        {
            "id": "regexp-type",
            "flag_ids": ["extended", "nonexistent"],
        }
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="unknown flag"):
        SpecLoader(path).load()


# =========================================================================
# Parsing mode validation
# =========================================================================


def test_invalid_parsing_mode_raises() -> None:
    """An invalid parsing_mode value raises SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["parsing_mode"] = "weird_mode"
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Invalid parsing_mode"):
        SpecLoader(path).load()


# =========================================================================
# Command validation
# =========================================================================


def test_duplicate_command_names_raise() -> None:
    """Duplicate command names at the same level raise SpecError."""
    spec = dict(MINIMAL_SPEC)
    spec["commands"] = [
        {
            "id": "cmd-add-1",
            "name": "add",
            "description": "Add something",
        },
        {
            "id": "cmd-add-2",
            "name": "add",  # duplicate
            "description": "Also add",
        },
    ]
    path = make_spec_file(spec)
    with pytest.raises(SpecError, match="Duplicate command"):
        SpecLoader(path).load()
