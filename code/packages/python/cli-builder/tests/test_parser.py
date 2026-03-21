"""Tests for Parser — the full three-phase parse engine.

We embed JSON specs as strings (written to temp files) and test the complete
parse flow from argv to ParseResult/HelpResult/VersionResult/ParseErrors.

Each test corresponds to one of the representative scenarios from the spec.
"""

from __future__ import annotations

import json
import tempfile
from typing import Any

import pytest

from cli_builder.errors import ParseErrors
from cli_builder.parser import Parser, _levenshtein
from cli_builder.types import HelpResult, ParseResult, VersionResult


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
# Spec definitions
# =========================================================================

ECHO_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "echo",
    "description": "Display a line of text",
    "version": "8.32",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "no-newline",
            "short": "n",
            "long": "no-newline",
            "description": "Do not output trailing newline",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "enable-escapes",
            "short": "e",
            "long": "enable-escapes",
            "description": "Enable backslash escapes",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": ["disable-escapes"],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "disable-escapes",
            "short": "E",
            "long": "disable-escapes",
            "description": "Disable backslash escapes",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": ["enable-escapes"],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
    ],
    "arguments": [
        {
            "id": "string",
            "name": "STRING",
            "description": "Text to display",
            "type": "string",
            "required": False,
            "variadic": True,
            "variadic_min": 0,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        }
    ],
    "commands": [],
    "mutually_exclusive_groups": [],
}

LS_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "ls",
    "description": "List directory contents",
    "version": "8.32",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "long-listing",
            "short": "l",
            "long": "long-listing",
            "description": "Use long listing format",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "all",
            "short": "a",
            "long": "all",
            "description": "Show hidden files",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "human-readable",
            "short": "h",
            "long": "human-readable",
            "description": "Human-readable sizes",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": ["long-listing"],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
    ],
    "arguments": [
        {
            "id": "path",
            "name": "PATH",
            "description": "Directory or file to list",
            "type": "string",
            "required": False,
            "variadic": True,
            "variadic_min": 0,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        }
    ],
    "commands": [],
    "mutually_exclusive_groups": [],
}

CP_SPEC: dict[str, Any] = {
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
            "description": "Source file(s)",
            "type": "string",
            "required": True,
            "variadic": True,
            "variadic_min": 1,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        },
        {
            "id": "dest",
            "name": "DEST",
            "description": "Destination",
            "type": "string",
            "required": True,
            "variadic": False,
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

GREP_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "grep",
    "description": "Search text patterns",
    "version": "3.7",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [],
    "flags": [
        {
            "id": "extended-regexp",
            "short": "E",
            "long": "extended-regexp",
            "description": "Use extended regular expressions",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "fixed-strings",
            "short": "F",
            "long": "fixed-strings",
            "description": "Use fixed strings",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "perl-regexp",
            "short": "P",
            "long": "perl-regexp",
            "description": "Use Perl-compatible regexps",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
    ],
    "arguments": [
        {
            "id": "pattern",
            "name": "PATTERN",
            "description": "Pattern to search for",
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
            "id": "file",
            "name": "FILE",
            "description": "File to search",
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
    "mutually_exclusive_groups": [
        {
            "id": "regexp-engine",
            "flag_ids": ["extended-regexp", "fixed-strings", "perl-regexp"],
            "required": False,
        }
    ],
}

TAR_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "tar",
    "description": "Archive files",
    "version": "1.34",
    "parsing_mode": "traditional",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "extract",
            "short": "x",
            "long": "extract",
            "description": "Extract files from archive",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Verbose output",
            "type": "boolean",
            "required": False,
            "default": None,
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
            "value_name": None,
            "enum_values": [],
        },
        {
            "id": "file",
            "short": "f",
            "long": "file",
            "description": "Archive file",
            "type": "string",
            "required": False,
            "default": None,
            "value_name": "ARCHIVE",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        },
    ],
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}

JAVA_SPEC: dict[str, Any] = {
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
    "arguments": [
        {
            "id": "main-class",
            "name": "MAIN_CLASS",
            "description": "Main class to run",
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

GIT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "git",
    "description": "The fast version control system",
    "version": "2.40.0",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": True},
    "global_flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Be verbose",
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
    "flags": [],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-remote",
            "name": "remote",
            "description": "Manage set of tracked repositories",
            "aliases": [],
            "inherit_global_flags": True,
            "flags": [],
            "arguments": [],
            "mutually_exclusive_groups": [],
            "commands": [
                {
                    "id": "cmd-remote-add",
                    "name": "add",
                    "description": "Add a named remote",
                    "aliases": ["a"],
                    "inherit_global_flags": True,
                    "flags": [],
                    "arguments": [
                        {
                            "id": "name",
                            "name": "NAME",
                            "description": "Remote name",
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
                            "id": "url",
                            "name": "URL",
                            "description": "Remote URL",
                            "type": "string",
                            "required": True,
                            "variadic": False,
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
            ],
        }
    ],
    "mutually_exclusive_groups": [],
}


# =========================================================================
# Test 1: echo hello world — variadic args, all flags default
# =========================================================================


def test_echo_hello_world() -> None:
    """echo hello world: variadic args parsed, all flags default to False."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "hello", "world"]).parse()
    assert isinstance(result, ParseResult)
    assert result.program == "echo"
    assert result.command_path == ["echo"]
    assert result.flags["no-newline"] is False
    assert result.flags["enable-escapes"] is False
    assert result.flags["disable-escapes"] is False
    assert result.arguments["string"] == ["hello", "world"]


def test_echo_no_args() -> None:
    """echo with no args: empty variadic list."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo"]).parse()
    assert isinstance(result, ParseResult)
    assert result.arguments["string"] == []


def test_echo_flag() -> None:
    """echo -n sets no-newline flag."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "-n", "hello"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["no-newline"] is True
    assert result.arguments["string"] == ["hello"]


# =========================================================================
# Test 2: ls -lah /tmp — stacked flags
# =========================================================================


def test_ls_stacked_flags() -> None:
    """ls -lah /tmp: stacked booleans l+a+h and positional /tmp."""
    path = make_spec_file(LS_SPEC)
    result = Parser(path, ["ls", "-lah", "/tmp"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["long-listing"] is True
    assert result.flags["all"] is True
    assert result.flags["human-readable"] is True
    assert result.arguments["path"] == ["/tmp"]


def test_ls_no_flags() -> None:
    """ls with no flags: all flags default to False."""
    path = make_spec_file(LS_SPEC)
    result = Parser(path, ["ls"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["long-listing"] is False
    assert result.flags["all"] is False
    assert result.flags["human-readable"] is False


# =========================================================================
# Test 3: ls -h — missing dependency flag error
# =========================================================================


def test_ls_h_without_l_raises() -> None:
    """ls -h without -l raises ParseErrors: missing_dependency_flag."""
    path = make_spec_file(LS_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["ls", "-h"]).parse()
    errors = exc_info.value.errors
    types = [e.error_type for e in errors]
    assert "missing_dependency_flag" in types


def test_ls_h_with_l_ok() -> None:
    """ls -lh works because h requires l and l is present."""
    path = make_spec_file(LS_SPEC)
    result = Parser(path, ["ls", "-lh"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["long-listing"] is True
    assert result.flags["human-readable"] is True


# =========================================================================
# Test 4: cp a.txt b.txt /dest — variadic + trailing positional
# =========================================================================


def test_cp_variadic_with_trailing() -> None:
    """cp a.txt b.txt /dest: source=[a.txt, b.txt], dest=/dest."""
    path = make_spec_file(CP_SPEC)
    result = Parser(path, ["cp", "a.txt", "b.txt", "/dest"]).parse()
    assert isinstance(result, ParseResult)
    assert result.arguments["source"] == ["a.txt", "b.txt"]
    assert result.arguments["dest"] == "/dest"


def test_cp_single_source() -> None:
    """cp a.txt /dest: source=[a.txt], dest=/dest."""
    path = make_spec_file(CP_SPEC)
    result = Parser(path, ["cp", "a.txt", "/dest"]).parse()
    assert isinstance(result, ParseResult)
    assert result.arguments["source"] == ["a.txt"]
    assert result.arguments["dest"] == "/dest"


def test_cp_missing_dest_raises() -> None:
    """cp with only one arg (no dest): missing_required_argument."""
    path = make_spec_file(CP_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["cp", "a.txt"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "missing_required_argument" in types or "too_few_arguments" in types


# =========================================================================
# Test 5: grep -E pattern file.txt — exclusive group, one flag used
# =========================================================================


def test_grep_extended_regexp() -> None:
    """grep -E pattern file.txt: extended-regexp=True, pattern set."""
    path = make_spec_file(GREP_SPEC)
    result = Parser(path, ["grep", "-E", "pattern", "file.txt"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["extended-regexp"] is True
    assert result.flags["fixed-strings"] is False
    assert result.arguments["pattern"] == "pattern"
    assert result.arguments["file"] == ["file.txt"]


# =========================================================================
# Test 6: grep -E -F pattern — exclusive group violation
# =========================================================================


def test_grep_two_exclusive_flags_raises() -> None:
    """grep -E -F pattern raises ParseErrors: exclusive_group_violation."""
    path = make_spec_file(GREP_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["grep", "-E", "-F", "pattern"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "exclusive_group_violation" in types


# =========================================================================
# Test 7: tar xvf archive.tar — traditional mode
# =========================================================================


def test_tar_traditional_mode() -> None:
    """tar xvf archive.tar: traditional mode parses xvf as stacked flags."""
    path = make_spec_file(TAR_SPEC)
    result = Parser(path, ["tar", "xvf", "archive.tar"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["extract"] is True
    assert result.flags["verbose"] is True
    # 'f' is a non-boolean flag; 'archive.tar' should be its value
    assert result.flags["file"] == "archive.tar"


def test_tar_with_leading_dash_still_works() -> None:
    """tar -xvf archive.tar also works (standard mode fallback)."""
    path = make_spec_file(TAR_SPEC)
    result = Parser(path, ["tar", "-xvf", "archive.tar"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["extract"] is True
    assert result.flags["verbose"] is True


# =========================================================================
# Test 8: java -classpath . Main — single_dash_long
# =========================================================================


def test_java_classpath() -> None:
    """java -classpath . Main: single_dash_long flag matched."""
    path = make_spec_file(JAVA_SPEC)
    result = Parser(path, ["java", "-classpath", ".", "Main"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["classpath"] == "."
    assert result.arguments["main-class"] == "Main"


# =========================================================================
# Test 9: git remote add origin https://... — deep command routing
# =========================================================================


def test_git_remote_add_routing() -> None:
    """git remote add origin https://... routes to command_path=['git','remote','add']."""
    path = make_spec_file(GIT_SPEC)
    result = Parser(
        path, ["git", "remote", "add", "origin", "https://example.com"]
    ).parse()
    assert isinstance(result, ParseResult)
    assert result.command_path == ["git", "remote", "add"]
    assert result.arguments["name"] == "origin"
    assert result.arguments["url"] == "https://example.com"


def test_git_remote_add_via_alias() -> None:
    """git remote a (alias for add) resolves to command_path containing 'add'."""
    path = make_spec_file(GIT_SPEC)
    result = Parser(
        path, ["git", "remote", "a", "origin", "https://example.com"]
    ).parse()
    assert isinstance(result, ParseResult)
    assert result.command_path == ["git", "remote", "add"]


# =========================================================================
# Help and version
# =========================================================================


def test_help_flag_returns_help_result() -> None:
    """--help returns a HelpResult."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "--help"]).parse()
    assert isinstance(result, HelpResult)
    assert "USAGE" in result.text


def test_short_help_flag() -> None:
    """-h returns a HelpResult."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "-h"]).parse()
    assert isinstance(result, HelpResult)


def test_version_flag_returns_version_result() -> None:
    """--version returns VersionResult with version string."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "--version"]).parse()
    assert isinstance(result, VersionResult)
    assert result.version == "8.32"


def test_subcommand_help() -> None:
    """git remote add --help returns help for the add subcommand."""
    path = make_spec_file(GIT_SPEC)
    result = Parser(path, ["git", "remote", "add", "--help"]).parse()
    assert isinstance(result, HelpResult)
    assert "add" in result.command_path


# =========================================================================
# Unknown flag errors
# =========================================================================


def test_unknown_flag_raises() -> None:
    """An unknown flag raises ParseErrors: unknown_flag."""
    path = make_spec_file(ECHO_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["echo", "--unknown-flag"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "unknown_flag" in types


def test_unknown_flag_suggests_similar() -> None:
    """An unknown flag close to a known one includes a suggestion."""
    path = make_spec_file(ECHO_SPEC)
    try:
        Parser(path, ["echo", "--no-newlin"]).parse()  # typo
    except ParseErrors as e:
        errors_with_suggestions = [err for err in e.errors if err.suggestion]
        # Should have a suggestion for 'no-newline'
        assert any(errors_with_suggestions)


# =========================================================================
# End-of-flags behavior
# =========================================================================


def test_double_dash_makes_all_remaining_positional() -> None:
    """After --, all tokens are positional even if they look like flags."""
    path = make_spec_file(ECHO_SPEC)
    result = Parser(path, ["echo", "--", "--no-newline"]).parse()
    assert isinstance(result, ParseResult)
    # --no-newline after -- should be a positional, not a flag
    assert result.flags["no-newline"] is False
    assert "--no-newline" in result.arguments["string"]


# =========================================================================
# Type coercion
# =========================================================================

INTEGER_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test app",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "count",
            "short": "n",
            "long": "count",
            "description": "Count",
            "type": "integer",
            "required": False,
            "default": None,
            "value_name": "N",
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


def test_integer_flag_coerced() -> None:
    """--count=5 results in integer 5, not string '5'."""
    path = make_spec_file(INTEGER_SPEC)
    result = Parser(path, ["myapp", "--count=5"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["count"] == 5
    assert isinstance(result.flags["count"], int)


def test_invalid_integer_raises() -> None:
    """--count=abc raises ParseErrors: invalid_value."""
    path = make_spec_file(INTEGER_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "--count=abc"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_value" in types


# =========================================================================
# POSIX mode
# =========================================================================

POSIX_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "posixapp",
    "description": "Test POSIX mode",
    "parsing_mode": "posix",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Verbose",
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
            "id": "input",
            "name": "INPUT",
            "description": "Input",
            "type": "string",
            "required": False,
            "variadic": True,
            "variadic_min": 0,
            "variadic_max": None,
            "default": None,
            "enum_values": [],
            "required_unless_flag": [],
        }
    ],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_posix_mode_flag_before_positional() -> None:
    """POSIX mode: -v before positional works."""
    path = make_spec_file(POSIX_SPEC)
    result = Parser(path, ["posixapp", "-v", "file.txt"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["verbose"] is True


def test_posix_mode_flag_after_positional_treated_as_positional() -> None:
    """POSIX mode: first non-flag ends flag scanning; --verbose after is positional."""
    path = make_spec_file(POSIX_SPEC)
    result = Parser(path, ["posixapp", "file.txt", "--verbose"]).parse()
    assert isinstance(result, ParseResult)
    # --verbose after the first positional should be treated as positional
    assert result.flags["verbose"] is False
    assert "--verbose" in result.arguments["input"]


# =========================================================================
# Repeatable flags
# =========================================================================

REPEATABLE_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test repeatable",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "define",
            "short": "D",
            "long": "define",
            "description": "Define a value",
            "type": "string",
            "required": False,
            "default": None,
            "value_name": "KEY",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": True,
        }
    ],
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_repeatable_flag_accumulates() -> None:
    """-D foo -D bar results in define=['foo', 'bar']."""
    path = make_spec_file(REPEATABLE_SPEC)
    result = Parser(path, ["myapp", "-D", "foo", "-D", "bar"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["define"] == ["foo", "bar"]


# =========================================================================
# Duplicate non-repeatable flag
# =========================================================================


def test_duplicate_flag_raises() -> None:
    """Repeating a non-repeatable flag raises ParseErrors: duplicate_flag."""
    path = make_spec_file(ECHO_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["echo", "-n", "-n"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "duplicate_flag" in types


# =========================================================================
# ParseErrors formatting
# =========================================================================


def test_parse_errors_str_format() -> None:
    """ParseErrors __str__ returns formatted error messages."""
    path = make_spec_file(ECHO_SPEC)
    try:
        Parser(path, ["echo", "--bogus"]).parse()
    except ParseErrors as e:
        text = str(e)
        assert "unknown_flag" in text or "error" in text.lower()


# =========================================================================
# Levenshtein distance
# =========================================================================


def test_levenshtein_identical() -> None:
    """Same string has distance 0."""
    assert _levenshtein("hello", "hello") == 0


def test_levenshtein_empty_strings() -> None:
    """Empty vs non-empty = length of non-empty."""
    assert _levenshtein("", "abc") == 3
    assert _levenshtein("abc", "") == 3


def test_levenshtein_one_edit() -> None:
    """One substitution = distance 1."""
    assert _levenshtein("kitten", "sitten") == 1


def test_levenshtein_two_edits() -> None:
    """Two edits."""
    assert _levenshtein("verbose", "verbos") == 1  # deletion
    assert _levenshtein("output", "outpu") == 1


def test_levenshtein_classical() -> None:
    """Classical example: kitten → sitting = 3."""
    assert _levenshtein("kitten", "sitting") == 3


# =========================================================================
# Empty argv
# =========================================================================


def test_empty_argv_raises() -> None:
    """Empty argv raises ParseErrors."""
    path = make_spec_file(ECHO_SPEC)
    with pytest.raises(ParseErrors):
        Parser(path, []).parse()


# =========================================================================
# Float flag type
# =========================================================================

FLOAT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test float",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "ratio",
            "long": "ratio",
            "description": "A float ratio",
            "type": "float",
            "required": False,
            "default": None,
            "value_name": "RATIO",
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


def test_float_flag_coerced() -> None:
    """--ratio=3.14 results in float 3.14."""
    path = make_spec_file(FLOAT_SPEC)
    result = Parser(path, ["myapp", "--ratio=3.14"]).parse()
    assert isinstance(result, ParseResult)
    assert abs(result.flags["ratio"] - 3.14) < 1e-9
    assert isinstance(result.flags["ratio"], float)


def test_invalid_float_raises() -> None:
    """--ratio=abc raises ParseErrors: invalid_value."""
    path = make_spec_file(FLOAT_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "--ratio=abc"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_value" in types


# =========================================================================
# Enum flag type
# =========================================================================

ENUM_FLAG_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test enum",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "format",
            "long": "format",
            "description": "Output format",
            "type": "enum",
            "enum_values": ["json", "csv", "xml"],
            "required": False,
            "default": None,
            "value_name": "FORMAT",
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


def test_enum_flag_valid_value() -> None:
    """--format=json is accepted and returned as string."""
    path = make_spec_file(ENUM_FLAG_SPEC)
    result = Parser(path, ["myapp", "--format=json"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["format"] == "json"


def test_enum_flag_invalid_value_raises() -> None:
    """--format=yaml raises ParseErrors: invalid_enum_value."""
    path = make_spec_file(ENUM_FLAG_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "--format=yaml"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_enum_value" in types


# =========================================================================
# Subcommand_first parsing mode
# =========================================================================

SUBCMD_FIRST_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Subcommand-first app",
    "parsing_mode": "subcommand_first",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-run",
            "name": "run",
            "description": "Run something",
            "aliases": [],
            "inherit_global_flags": True,
            "flags": [],
            "arguments": [],
            "commands": [],
            "mutually_exclusive_groups": [],
        }
    ],
    "mutually_exclusive_groups": [],
}


def test_subcommand_first_valid_command_routes() -> None:
    """In subcommand_first mode, a valid first token routes to the subcommand."""
    path = make_spec_file(SUBCMD_FIRST_SPEC)
    result = Parser(path, ["myapp", "run"]).parse()
    assert isinstance(result, ParseResult)
    assert result.command_path == ["myapp", "run"]


def test_subcommand_first_unknown_command_raises() -> None:
    """In subcommand_first mode, an unknown first non-flag token raises ParseErrors."""
    path = make_spec_file(SUBCMD_FIRST_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "notacommand"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "unknown_command" in types


def test_subcommand_first_unknown_command_suggests() -> None:
    """In subcommand_first mode, a close-match token includes a suggestion."""
    path = make_spec_file(SUBCMD_FIRST_SPEC)
    try:
        Parser(path, ["myapp", "ru"]).parse()  # close to "run"
    except ParseErrors as e:
        suggestions = [err.suggestion for err in e.errors if err.suggestion]
        # Should suggest "run"
        assert any("run" in (s or "") for s in suggestions)


# =========================================================================
# User-defined -h flag does not trigger builtin help
# =========================================================================

USER_H_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "App with custom -h",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "human-readable",
            "short": "h",
            "long": "human-readable",
            "description": "Human-readable output",
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
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_user_defined_short_h_does_not_trigger_help() -> None:
    """-h does not trigger builtin help when user has a -h flag."""
    path = make_spec_file(USER_H_SPEC)
    result = Parser(path, ["myapp", "-h"]).parse()
    # Should be ParseResult (not HelpResult) because user defined -h
    assert isinstance(result, ParseResult)
    assert result.flags["human-readable"] is True


def test_double_dash_help_still_works_with_user_short_h() -> None:
    """--help still triggers help even when user defines -h."""
    path = make_spec_file(USER_H_SPEC)
    result = Parser(path, ["myapp", "--help"]).parse()
    assert isinstance(result, HelpResult)


# =========================================================================
# inherit_global_flags = False
# =========================================================================

NO_INHERIT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "App",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Verbose",
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
    "flags": [],
    "arguments": [],
    "commands": [
        {
            "id": "cmd-run",
            "name": "run",
            "description": "Run",
            "aliases": [],
            "inherit_global_flags": False,  # does NOT inherit globals
            "flags": [],
            "arguments": [],
            "commands": [],
            "mutually_exclusive_groups": [],
        }
    ],
    "mutually_exclusive_groups": [],
}


def test_inherit_global_flags_false_excludes_globals() -> None:
    """When inherit_global_flags=False, global flags are not active in subcommand."""
    path = make_spec_file(NO_INHERIT_SPEC)
    # Passing --verbose to a command that does not inherit globals should raise unknown_flag
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "run", "--verbose"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "unknown_flag" in types


# =========================================================================
# Traditional mode — fallback to positional when token doesn't match flags
# =========================================================================


def test_traditional_mode_unknown_chars_become_positional() -> None:
    """In traditional mode, first token with unknown chars falls back to positional."""
    # TAR_SPEC has no argument defs, so a positional would trigger too_many_arguments.
    # We need a spec with an argument to accept the fallback positional.
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "tar",
        "description": "Archive",
        "version": "1.34",
        "parsing_mode": "traditional",
        "builtin_flags": {"help": True, "version": False},
        "global_flags": [],
        "flags": [
            {
                "id": "extract",
                "short": "x",
                "long": "extract",
                "description": "Extract",
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
                "id": "archive",
                "name": "ARCHIVE",
                "description": "Archive file",
                "type": "string",
                "required": False,
                "variadic": False,
                "variadic_min": 0,
                "variadic_max": None,
                "default": None,
                "enum_values": [],
                "required_unless_flag": [],
            }
        ],
        "commands": [],
        "mutually_exclusive_groups": [],
    }
    path = make_spec_file(spec)
    # "Zarchive.tar" — 'Z' is not a known short flag → falls through to positional
    result = Parser(path, ["tar", "Zarchive.tar"]).parse()
    assert isinstance(result, ParseResult)
    assert result.arguments["archive"] == "Zarchive.tar"
    assert result.flags["extract"] is False


# =========================================================================
# Traditional mode — non-boolean flag in middle of stack (error path)
# =========================================================================


def test_traditional_mode_nonbool_not_last_errors() -> None:
    """In traditional mode, non-boolean flag not at end of stack raises error."""
    spec: dict[str, Any] = {
        "cli_builder_spec_version": "1.0",
        "name": "tar",
        "description": "Archive",
        "version": "1.34",
        "parsing_mode": "traditional",
        "builtin_flags": {"help": True, "version": False},
        "global_flags": [],
        "flags": [
            {
                "id": "file",
                "short": "f",
                "long": "file",
                "description": "Archive file",
                "type": "string",
                "required": False,
                "default": None,
                "value_name": "ARCHIVE",
                "enum_values": [],
                "conflicts_with": [],
                "requires": [],
                "required_unless": [],
                "repeatable": False,
            },
            {
                "id": "extract",
                "short": "x",
                "long": "extract",
                "description": "Extract",
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
        ],
        "arguments": [],
        "commands": [],
        "mutually_exclusive_groups": [],
    }
    path = make_spec_file(spec)
    # "fx" — 'f' is non-boolean in the middle, 'x' follows → error
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["tar", "fx", "archive.tar"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "missing_flag_value" in types


# =========================================================================
# Short flag with inline value — coerce error
# =========================================================================

SHORT_INTEGER_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "count",
            "short": "n",
            "long": "count",
            "description": "Count",
            "type": "integer",
            "required": False,
            "default": None,
            "value_name": "N",
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


def test_short_flag_with_inline_value_coerce_error() -> None:
    """-nabc (inline value 'abc' for integer flag) raises ParseErrors: invalid_value."""
    path = make_spec_file(SHORT_INTEGER_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "-nabc"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_value" in types


# =========================================================================
# Stacked flags with inline trailing value — coerce error
# =========================================================================

STACKED_INTEGER_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "verbose",
            "short": "v",
            "long": "verbose",
            "description": "Verbose",
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
            "id": "count",
            "short": "n",
            "long": "count",
            "description": "Count",
            "type": "integer",
            "required": False,
            "default": None,
            "value_name": "N",
            "enum_values": [],
            "conflicts_with": [],
            "requires": [],
            "required_unless": [],
            "repeatable": False,
        },
    ],
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_stacked_with_trailing_value_coerce_error() -> None:
    """-vnabc (stacked: v boolean, n integer with inline 'abc') raises invalid_value."""
    path = make_spec_file(STACKED_INTEGER_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "-vnabc"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_value" in types


# =========================================================================
# Boolean flag default value
# =========================================================================

BOOL_DEFAULT_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test bool default",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "enabled",
            "long": "enabled",
            "description": "Enabled by default",
            "type": "boolean",
            "required": False,
            "default": True,  # non-None default for boolean
            "value_name": None,
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


def test_boolean_flag_with_non_none_default() -> None:
    """A boolean flag with default=True gets that default when absent."""
    path = make_spec_file(BOOL_DEFAULT_SPEC)
    result = Parser(path, ["myapp"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["enabled"] is True


# =========================================================================
# Help before routing errors (quick_help path)
# =========================================================================


def test_help_shown_even_with_routing_error() -> None:
    """--help is returned even when there is also a routing error (quick-help path)."""
    # subcommand_first mode: "badcmd" triggers unknown_command routing error,
    # but --help appears after it and short-circuits before that error propagates.
    path = make_spec_file(SUBCMD_FIRST_SPEC)
    result = Parser(path, ["myapp", "badcmd", "--help"]).parse()
    assert isinstance(result, HelpResult)


# =========================================================================
# Value flag — value is next token (FLAG_VALUE mode)
# =========================================================================


def test_value_flag_next_token_coerced() -> None:
    """-n 5 (separate token) is parsed as count=5."""
    path = make_spec_file(SHORT_INTEGER_SPEC)
    result = Parser(path, ["myapp", "-n", "5"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["count"] == 5


def test_value_flag_next_token_coerce_error() -> None:
    """-n abc (separate token, invalid integer) raises ParseErrors."""
    path = make_spec_file(SHORT_INTEGER_SPEC)
    with pytest.raises(ParseErrors) as exc_info:
        Parser(path, ["myapp", "-n", "abc"]).parse()
    types = [e.error_type for e in exc_info.value.errors]
    assert "invalid_value" in types


# =========================================================================
# Single-dash-long boolean flag
# =========================================================================

SDL_BOOLEAN_SPEC: dict[str, Any] = {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "Test SDL boolean",
    "parsing_mode": "gnu",
    "builtin_flags": {"help": True, "version": False},
    "global_flags": [],
    "flags": [
        {
            "id": "verbose",
            "single_dash_long": "verbose",
            "description": "Verbose mode",
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
    "arguments": [],
    "commands": [],
    "mutually_exclusive_groups": [],
}


def test_single_dash_long_boolean_flag() -> None:
    """-verbose (single_dash_long boolean) sets the flag to True."""
    path = make_spec_file(SDL_BOOLEAN_SPEC)
    result = Parser(path, ["myapp", "-verbose"]).parse()
    assert isinstance(result, ParseResult)
    assert result.flags["verbose"] is True


# =========================================================================
# _fuzzy_suggest — no candidates within threshold
# =========================================================================


def test_fuzzy_suggest_no_close_match() -> None:
    """A very different token gets no suggestion (distance > 2)."""
    from cli_builder.parser import _fuzzy_suggest
    suggestion = _fuzzy_suggest("zzzzzzz", ["verbose", "output", "debug"])
    assert suggestion is None


def test_fuzzy_suggest_exact_match() -> None:
    """An exact match returns that candidate."""
    from cli_builder.parser import _fuzzy_suggest
    suggestion = _fuzzy_suggest("verbose", ["verbose", "output"])
    assert suggestion == "verbose"
