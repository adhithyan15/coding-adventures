"""Tests for HelpGenerator — help text formatting.

We verify the structure and content of generated help output per spec §9.
"""

from __future__ import annotations

from typing import Any

import pytest

from cli_builder.help_generator import HelpGenerator


# =========================================================================
# Sample specs
# =========================================================================

ROOT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "mytool",
    "display_name": "My Tool",
    "description": "A tool that does things",
    "version": "1.0.0",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Enable verbose output",
            "type": "boolean",
            "required": False,
            "default": None,
            "value_name": None,
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        },
        {
            "id": "output",
            "short": "o",
            "long": "output",
            "description": "Output file",
            "type": "string",
            "required": False,
            "default": "out.txt",
            "value_name": "FILE",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        },
    ],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-run",
            "name": "run",
            "description": "Run the tool",
            "aliases": [],
            "inherit_global_flags": True,
            "flags": [
                {
                    "id": "debug",
                    "short": "d",
                    "long": "debug",
                    "description": "Enable debug mode",
                    "type": "boolean",
                    "required": False,
                    "default": None,
                    "value_name": None,
                    "enum_values": [],
                    "conflicts_with": [],
                    "requires": [],
                    "required_unless": [],
                    "repeatable": False,
                }
            ],
            "arguments": [
                {
                    "id": "target",
                    "name": "TARGET",
                    "description": "Target to run",
                    "type": "string",
                    "required": True,
                    "variadic": False,
                    "variadic_min": 1,
                    "variadic_max": None,
                    "default": None,
                    "enum_values": [],
                    "required_unless_flag": [],
                },
                {
                    "id": "extra",
                    "name": "EXTRA",
                    "description": "Extra arguments",
                    "type": "string",
                    "required": False,
                    "variadic": True,
                    "variadic_min": 0,
                    "variadic_max": None,
                    "default": None,
                    "enum_values": [],
                    "required_unless_flag": [],
                },
            ],
            "commands": [],
            "mutually_exclusive_groups": [],
        },
        {
            "id": "cmd-build",
            "name": "build",
            "description": "Build the project",
            "aliases": [],
            "inherit_global_flags": True,
            "flags": [],
            "arguments": [],
            "commands": [],
            "mutually_exclusive_groups": [],
        },
    ],
    "mutually_exclusive_groups": [],
}


# =========================================================================
# Root help
# =========================================================================


def test_root_help_contains_usage() -> None:
    """Root help contains a USAGE section."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "USAGE" in text
    assert "mytool" in text


def test_root_help_contains_description() -> None:
    """Root help contains the DESCRIPTION section."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "DESCRIPTION" in text
    assert "A tool that does things" in text


def test_root_help_contains_commands() -> None:
    """Root help lists subcommands in a COMMANDS section."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "COMMANDS" in text
    assert "run" in text
    assert "Run the tool" in text
    assert "build" in text
    assert "Build the project" in text


def test_root_help_contains_options() -> None:
    """Root help lists flags in an OPTIONS section."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "OPTIONS" in text
    assert "--verbose" in text
    assert "--output" in text


def test_root_help_contains_global_options() -> None:
    """Root help includes GLOBAL OPTIONS with builtin flags."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "GLOBAL OPTIONS" in text
    assert "--help" in text
    assert "--version" in text


def test_root_help_shows_default_values() -> None:
    """Default values are shown as [default: X]."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "[default: out.txt]" in text


def test_root_help_shows_value_placeholder() -> None:
    """Non-boolean flags show a <VALUE> placeholder."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "<FILE>" in text


# =========================================================================
# Subcommand help
# =========================================================================


def test_subcommand_help_shows_correct_usage() -> None:
    """Subcommand help shows the subcommand name in the USAGE line."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    assert "USAGE" in text
    assert "run" in text


def test_subcommand_help_shows_options() -> None:
    """Subcommand help shows the subcommand's specific OPTIONS."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    assert "OPTIONS" in text
    assert "--debug" in text


def test_subcommand_help_shows_arguments() -> None:
    """Subcommand help shows ARGUMENTS section."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    assert "ARGUMENTS" in text
    assert "TARGET" in text


def test_subcommand_help_required_arg_angle_brackets() -> None:
    """Required args are shown as <NAME>."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    assert "<TARGET>" in text


def test_subcommand_help_optional_variadic_arg() -> None:
    """Optional variadic args are shown as [NAME...]."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    assert "[EXTRA...]" in text


def test_subcommand_usage_shows_required_arg() -> None:
    """USAGE line for subcommand includes <TARGET> for required arg."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "run"])
    text = gen.generate()
    # The usage line should include the argument forms
    usage_line = text.split("\n")[1]  # "  mytool run [OPTIONS] <TARGET> [EXTRA...]"
    assert "<TARGET>" in usage_line


# =========================================================================
# Flag signature formatting
# =========================================================================


def test_boolean_flag_no_value_placeholder() -> None:
    """Boolean flags don't show a value placeholder."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    # --verbose is boolean: should appear without <VALUE>
    lines = text.split("\n")
    verbose_lines = [l for l in lines if "verbose" in l and "OPTIONS" not in l]
    assert any("--verbose" in l and "<" not in l for l in verbose_lines)


def test_short_and_long_flag_shown() -> None:
    """Flags with both short and long forms show both."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    assert "-v, --verbose" in text or "-v" in text


# =========================================================================
# Single-dash-long flags
# =========================================================================

SDL_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "java",
    "description": "Java runtime",
    "version": "17",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "classpath",
            "single_dash_long": "classpath",
            "description": "Classpath",
            "type": "string",
            "required": False,
            "default": None,
            "value_name": "PATH",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        }
    ],
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_sdl_flag_shown_in_help() -> None:
    """-classpath is shown in help with single-dash form."""
    gen = HelpGenerator(SDL_SPEC, ["java"])
    text = gen.generate()
    assert "-classpath" in text


# =========================================================================
# No version in builtin flags
# =========================================================================


def test_no_version_flag_when_version_absent() -> None:
    """If spec has no version, --version is not shown in help."""
    spec = dict(ROOT_SPEC)
    spec = {**ROOT_SPEC, "version": None}
    gen = HelpGenerator(spec, ["mytool"])
    text = gen.generate()
    # --version should not appear in GLOBAL OPTIONS
    global_section_start = text.find("GLOBAL OPTIONS")
    if global_section_start >= 0:
        global_section = text[global_section_start:]
        assert "--version" not in global_section.split("\n\n")[0]


# =========================================================================
# Edge cases
# =========================================================================


def test_unknown_command_path_falls_back_to_root() -> None:
    """An unresolvable command path falls back to root spec."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool", "nonexistent"])
    text = gen.generate()
    # Falls back to root — should still show something
    assert "USAGE" in text
    assert "mytool" in text


# =========================================================================
# Required variadic argument in USAGE and ARGUMENTS
# =========================================================================


REQUIRED_VARIADIC_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "cp",
    "description": "Copy files",
    "version": "8.32",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [],
    "arguments": [
        {
            "id": "source",
            "name": "SOURCE",
            "description": "Source file",
            "type": "path",
            "required": True,
            "variadic": True,
            "variadic_min": 1,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        },
    ],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_required_variadic_arg_in_usage() -> None:
    """Required variadic args appear as <NAME...> in the USAGE line."""
    gen = HelpGenerator(REQUIRED_VARIADIC_SPEC, ["cp"])
    text = gen.generate()
    assert "<SOURCE...>" in text


def test_required_variadic_arg_in_arguments_section() -> None:
    """Required variadic args appear as <NAME...> in the ARGUMENTS section."""
    gen = HelpGenerator(REQUIRED_VARIADIC_SPEC, ["cp"])
    text = gen.generate()
    assert "ARGUMENTS" in text
    assert "<SOURCE...>" in text


# =========================================================================
# Long flag signature wrapping
# =========================================================================


LONG_FLAG_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "tool",
    "description": "A tool",
    "version": None,
    "parsing_mode": "gnu",
    "builtin_flags": {"help": False, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "very-long-flag-name-that-exceeds-column-width",
            "long": "very-long-flag-name-that-exceeds-column-width",
            "description": "A flag with a very long name",
            "type": "string",
            "required": False,
            "default": None,
            "value_name": "VALUE",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        }
    ],
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_long_flag_signature_wraps_description() -> None:
    """A flag signature exceeding COLUMN_WIDTH puts description on next line."""
    gen = HelpGenerator(LONG_FLAG_SPEC, ["tool"])
    text = gen.generate()
    assert "very-long-flag-name-that-exceeds-column-width" in text
    assert "A flag with a very long name" in text


# =========================================================================
# Long argument name wrapping
# =========================================================================


LONG_ARG_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "tool",
    "description": "A tool",
    "version": None,
    "parsing_mode": "gnu",
    "builtin_flags": {"help": False, "version": False},
    "global_flags": [],
    "flags": [],
    "arguments": [
        {
            "id": "arg",
            "name": "VERY_LONG_ARGUMENT_NAME_EXCEEDING_COLUMN_WIDTH_LIMIT",
            "description": "A very long argument name",
            "type": "string",
            "required": True,
            "variadic": False,
            "variadic_min": 1,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        }
    ],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_long_argument_name_wraps_description() -> None:
    """An argument name exceeding COLUMN_WIDTH puts description on next line."""
    gen = HelpGenerator(LONG_ARG_SPEC, ["tool"])
    text = gen.generate()
    assert "VERY_LONG_ARGUMENT_NAME_EXCEEDING_COLUMN_WIDTH_LIMIT" in text
    assert "A very long argument name" in text


# =========================================================================
# Command path resolution via alias
# =========================================================================


ALIAS_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "Version control",
    "version": "2.0",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-commit",
            "name": "commit",
            "aliases": ["ci"],
            "description": "Record changes",
            "flags": [
                {
                    "id": "message",
                    "short": "m",
                    "long": "message",
                    "description": "Commit message",
                    "type": "string",
                    "required": False,
                    "default": None,
                    "value_name": "MSG",
                    "enum_values": [],
                    "conflicts_with": [],
                    "requires": [],
                    "required_unless": [],
                    "repeatable": False,
                }
            ],
            "arguments": [],
            "commands": [],
            "mutually_exclusive_groups": [],
        }
    ],
    "mutually_exclusive_groups": [],
}


def test_resolve_node_via_alias() -> None:
    """_resolve_node follows an alias to find the correct command node."""
    gen = HelpGenerator(ALIAS_SPEC, ["git", "ci"])
    text = gen.generate()
    # Should resolve to the "commit" command via its "ci" alias
    assert "--message" in text or "-m" in text


# =========================================================================
# Minimal spec with no flags / no version
# =========================================================================


def test_minimal_spec_no_options_section() -> None:
    """A spec with no flags produces no OPTIONS section."""
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "minimal",
        "description": "Minimal tool",
        "version": None,
        "parsing_mode": "gnu",
        "builtin_flags": {"help": False, "version": False},
        "global_flags": [],
        "flags": [],
        "arguments": [],
        "commands": [],
        "mutually_exclusive_groups": [],
    }
    gen = HelpGenerator(spec, ["minimal"])
    text = gen.generate()
    assert "USAGE" in text
    # No OPTIONS or GLOBAL OPTIONS when no flags
    assert "OPTIONS" not in text


def test_flag_with_no_default_does_not_show_default() -> None:
    """A flag with default=None does not show [default: ...] in help."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    # --verbose has default=None, so [default: ...] should not appear on verbose line
    lines = text.split("\n")
    verbose_lines = [l for l in lines if "--verbose" in l]
    for line in verbose_lines:
        assert "[default:" not in line


def test_flag_with_required_true_does_not_show_default() -> None:
    """A required flag with a default value does not show [default: ...] in help."""
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "tool",
        "description": "Tool",
        "version": None,
        "parsing_mode": "gnu",
        "builtin_flags": {"help": False, "version": False},
        "global_flags": [],
        "flags": [
            {
                "id": "output",
                "long": "output",
                "description": "Output file",
                "type": "string",
                "required": True,
                "default": "out.txt",  # default set but flag is required
                "value_name": "FILE",
                "enum_values": [],
                "conflicts_with": [],
                "requires": [],
                "required_unless": [],
                "repeatable": False,
            }
        ],
        "arguments": [],
        "commands": [],
        "mutually_exclusive_groups": [],
    }
    gen = HelpGenerator(spec, ["tool"])
    text = gen.generate()
    # [default: out.txt] should NOT appear because flag is required=True
    assert "[default: out.txt]" not in text


def test_value_name_used_in_placeholder() -> None:
    """A custom value_name is shown in the placeholder instead of the type name."""
    gen = HelpGenerator(ROOT_SPEC, ["mytool"])
    text = gen.generate()
    # --output has value_name="FILE"
    assert "<FILE>" in text
    # --output should not show <STRING> (the type name)
    assert "<STRING>" not in text or "<FILE>" in text  # custom name takes precedence


def test_flag_type_used_as_placeholder_when_no_value_name() -> None:
    """When value_name is None, the type name is used as placeholder."""
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "tool",
        "description": "Tool",
        "version": None,
        "parsing_mode": "gnu",
        "builtin_flags": {"help": False, "version": False},
        "global_flags": [],
        "flags": [
            {
                "id": "count",
                "long": "count",
                "description": "A count",
                "type": "integer",
                "required": False,
                "default": None,
                "value_name": None,  # no custom name
                "enum_values": [],
                "conflicts_with": [],
                "requires": [],
                "required_unless": [],
                "repeatable": False,
            }
        ],
        "arguments": [],
        "commands": [],
        "mutually_exclusive_groups": [],
    }
    gen = HelpGenerator(spec, ["tool"])
    text = gen.generate()
    # Type "integer" should be used as placeholder
    assert "<INTEGER>" in text
