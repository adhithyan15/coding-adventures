"""Tests for TokenClassifier — token classification and disambiguation.

Each test exercises one rule from spec §5.2 (longest-match-first).
We build a small set of synthetic flags to cover all classification paths.
"""

from __future__ import annotations

from typing import Any

import pytest

from cli_builder.token_classifier import TokenClassifier


# =========================================================================
# Shared flag definitions for tests
# =========================================================================

BOOLEAN_FLAGS: list[dict[str, Any]] = [
    {
        "id": "long-listing",
        "short": "l",
        "long": "long-listing",
        "description": "Long listing format",
        "type": "boolean",
    },
    {
        "id": "all",
        "short": "a",
        "long": "all",
        "description": "Show all files",
        "type": "boolean",
    },
    {
        "id": "human-readable",
        "short": "h",
        "long": "human-readable",
        "description": "Human-readable sizes",
        "type": "boolean",
    },
    {
        "id": "verbose",
        "short": "v",
        "long": "verbose",
        "description": "Verbose output",
        "type": "boolean",
    },
]

VALUE_FLAGS: list[dict[str, Any]] = [
    {
        "id": "output",
        "short": "o",
        "long": "output",
        "description": "Output file",
        "type": "string",
        "value_name": "FILE",
    },
    {
        "id": "file",
        "short": "f",
        "long": "file",
        "description": "Input file",
        "type": "path",
    },
    {
        "id": "count",
        "short": "n",
        "long": "count",
        "description": "Count",
        "type": "integer",
    },
]

SDL_FLAGS: list[dict[str, Any]] = [
    {
        "id": "classpath",
        "single_dash_long": "classpath",
        "description": "Class path",
        "type": "string",
    },
    {
        "id": "cp",
        "single_dash_long": "cp",
        "description": "Classpath shorthand",
        "type": "string",
    },
]

ALL_FLAGS = BOOLEAN_FLAGS + VALUE_FLAGS + SDL_FLAGS


# =========================================================================
# Fixtures
# =========================================================================


@pytest.fixture()
def classifier() -> TokenClassifier:
    """A classifier with all test flags active."""
    return TokenClassifier(ALL_FLAGS)


@pytest.fixture()
def bool_classifier() -> TokenClassifier:
    """A classifier with only boolean flags."""
    return TokenClassifier(BOOLEAN_FLAGS)


@pytest.fixture()
def value_classifier() -> TokenClassifier:
    """A classifier with boolean + value flags."""
    return TokenClassifier(BOOLEAN_FLAGS + VALUE_FLAGS)


# =========================================================================
# End-of-flags
# =========================================================================


def test_double_dash_is_end_of_flags(classifier: TokenClassifier) -> None:
    """-- is classified as end_of_flags."""
    result = classifier.classify("--")
    assert result["type"] == "end_of_flags"


# =========================================================================
# Long flags (-- prefix)
# =========================================================================


def test_long_boolean_flag(classifier: TokenClassifier) -> None:
    """--verbose is classified as long_flag."""
    result = classifier.classify("--verbose")
    assert result["type"] == "long_flag"
    assert result["name"] == "verbose"
    assert result["flag_def"]["id"] == "verbose"


def test_long_flag_with_inline_value(classifier: TokenClassifier) -> None:
    """--output=file.txt is classified as long_flag_with_value."""
    result = classifier.classify("--output=file.txt")
    assert result["type"] == "long_flag_with_value"
    assert result["name"] == "output"
    assert result["value"] == "file.txt"
    assert result["flag_def"]["id"] == "output"


def test_unknown_long_flag(classifier: TokenClassifier) -> None:
    """--unknown is classified as unknown_flag."""
    result = classifier.classify("--unknown")
    assert result["type"] == "unknown_flag"
    assert result["token"] == "--unknown"


def test_unknown_long_flag_with_value(classifier: TokenClassifier) -> None:
    """--unknown=value is classified as unknown_flag."""
    result = classifier.classify("--unknown=value")
    assert result["type"] == "unknown_flag"


# =========================================================================
# Single-dash-long flags
# =========================================================================


def test_single_dash_long_match(classifier: TokenClassifier) -> None:
    """-classpath matches single_dash_long (longest match wins over stacking)."""
    result = classifier.classify("-classpath")
    assert result["type"] == "single_dash_long"
    assert result["name"] == "classpath"
    assert result["flag_def"]["id"] == "classpath"


def test_single_dash_long_short_form(classifier: TokenClassifier) -> None:
    """-cp matches single_dash_long 'cp' (not -c stacked with -p)."""
    result = classifier.classify("-cp")
    assert result["type"] == "single_dash_long"
    assert result["name"] == "cp"


# =========================================================================
# Short flags (single char)
# =========================================================================


def test_short_boolean_flag(bool_classifier: TokenClassifier) -> None:
    """-l is classified as short_flag (boolean)."""
    result = bool_classifier.classify("-l")
    assert result["type"] == "short_flag"
    assert result["char"] == "l"
    assert result["flag_def"]["id"] == "long-listing"


def test_short_value_flag(value_classifier: TokenClassifier) -> None:
    """-f alone (no inline value) → short_flag (value follows as next token)."""
    result = value_classifier.classify("-f")
    assert result["type"] == "short_flag"
    assert result["char"] == "f"
    assert result["flag_def"]["type"] == "path"


def test_short_flag_with_inline_value(value_classifier: TokenClassifier) -> None:
    """-ffile.txt is classified as short_flag_with_value."""
    result = value_classifier.classify("-ffile.txt")
    assert result["type"] == "short_flag_with_value"
    assert result["char"] == "f"
    assert result["value"] == "file.txt"


def test_short_flag_with_inline_value_output(value_classifier: TokenClassifier) -> None:
    """-ooutput.txt is short_flag_with_value for -o."""
    result = value_classifier.classify("-ooutput.txt")
    assert result["type"] == "short_flag_with_value"
    assert result["char"] == "o"
    assert result["value"] == "output.txt"


# =========================================================================
# Stacked flags
# =========================================================================


def test_stacked_boolean_flags(bool_classifier: TokenClassifier) -> None:
    """-lah is stacked boolean flags (l, a, h)."""
    result = bool_classifier.classify("-lah")
    assert result["type"] == "stacked_flags"
    assert result["chars"] == ["l", "a", "h"]
    assert len(result["flag_defs"]) == 3


def test_stacked_two_booleans(bool_classifier: TokenClassifier) -> None:
    """-la is stacked flags [l, a]."""
    result = bool_classifier.classify("-la")
    assert result["type"] == "stacked_flags"
    assert set(result["chars"]) == {"l", "a"}


def test_stacked_with_trailing_value(value_classifier: TokenClassifier) -> None:
    """-lf followed by a non-boolean: l is boolean, f takes remaining chars."""
    # -lf where f is non-boolean and there's no remainder: result is stacked
    # with the non-boolean last char needing a value from the next token.
    result = value_classifier.classify("-lf")
    # l is boolean, f is non-boolean at end with no inline value.
    # The stacked result should have both chars.
    assert result["type"] == "stacked_flags"
    assert "l" in result["chars"]
    assert "f" in result["chars"]
    assert result["trailing_value"] is None


def test_stacked_non_boolean_with_inline_value(value_classifier: TokenClassifier) -> None:
    """-lfmyfile.txt: l is boolean, f is non-boolean with inline 'myfile.txt'."""
    result = value_classifier.classify("-lfmyfile.txt")
    assert result["type"] == "stacked_flags"
    assert result["chars"] == ["l", "f"]
    assert result["trailing_value"] == "myfile.txt"


# =========================================================================
# Positional
# =========================================================================


def test_bare_dash_is_positional(classifier: TokenClassifier) -> None:
    """- (bare dash) is always positional."""
    result = classifier.classify("-")
    assert result["type"] == "positional"
    assert result["value"] == "-"


def test_plain_string_is_positional(classifier: TokenClassifier) -> None:
    """hello is classified as positional."""
    result = classifier.classify("hello")
    assert result["type"] == "positional"
    assert result["value"] == "hello"


def test_path_positional(classifier: TokenClassifier) -> None:
    """/tmp/file.txt is positional."""
    result = classifier.classify("/tmp/file.txt")
    assert result["type"] == "positional"


def test_empty_string_is_positional(classifier: TokenClassifier) -> None:
    """Empty string is classified as positional."""
    result = classifier.classify("")
    assert result["type"] == "positional"


# =========================================================================
# Unknown flags
# =========================================================================


def test_unknown_short_flag(bool_classifier: TokenClassifier) -> None:
    """-z (not in active flags) is unknown_flag."""
    result = bool_classifier.classify("-z")
    assert result["type"] == "unknown_flag"


def test_stacking_unknown_char_is_unknown(bool_classifier: TokenClassifier) -> None:
    """-lXh where X is unknown results in unknown_flag."""
    result = bool_classifier.classify("-lXh")
    assert result["type"] == "unknown_flag"


# =========================================================================
# Edge cases
# =========================================================================


def test_long_flag_empty_value(value_classifier: TokenClassifier) -> None:
    """--output= with empty value is long_flag_with_value with value=''."""
    result = value_classifier.classify("--output=")
    assert result["type"] == "long_flag_with_value"
    assert result["value"] == ""


def test_empty_classifier_no_flags() -> None:
    """Classifier with no flags treats all dash-prefixed tokens as unknown."""
    c = TokenClassifier([])
    assert c.classify("--verbose")["type"] == "unknown_flag"
    assert c.classify("-v")["type"] == "unknown_flag"
    assert c.classify("hello")["type"] == "positional"
    assert c.classify("--")["type"] == "end_of_flags"


# =========================================================================
# Additional stacking edge cases
# =========================================================================


def test_stacked_single_non_boolean_no_inline_value_is_stacked(
    value_classifier: TokenClassifier,
) -> None:
    """A multi-char token starting with a non-boolean flag yields stacked_flags."""
    # -la where 'l' is boolean and 'a' is also boolean — already tested.
    # Test the path where suffix starts with a non-boolean flag that is NOT alone.
    # -ol where 'o' is non-boolean: remainder 'l' is the inline value.
    result = value_classifier.classify("-ol")
    # 'o' is non-boolean, 'l' is the remainder (inline value)
    assert result["type"] == "short_flag_with_value"
    assert result["char"] == "o"
    assert result["value"] == "l"


def test_stacked_three_booleans(bool_classifier: TokenClassifier) -> None:
    """-vla is stacked with three boolean flags."""
    result = bool_classifier.classify("-vla")
    assert result["type"] == "stacked_flags"
    assert set(result["chars"]) == {"v", "l", "a"}
    assert result["trailing_value"] is None


def test_stacked_bool_then_nonbool_at_end_no_inline_is_stacked(
    value_classifier: TokenClassifier,
) -> None:
    """-lo where 'l' is boolean and 'o' is non-boolean at end: stacked, trailing_value=None."""
    result = value_classifier.classify("-lo")
    assert result["type"] == "stacked_flags"
    assert "l" in result["chars"]
    assert "o" in result["chars"]
    assert result["trailing_value"] is None


def test_stacked_single_non_boolean_alone_falls_back_to_short_flag(
    value_classifier: TokenClassifier,
) -> None:
    """Single non-boolean flag in stacked path (len=1): falls back to short_flag."""
    # When we call _classify_stacked with a single char that is non-boolean,
    # and trailing_value is None, the result should be short_flag, not stacked.
    # This is exercised via the suffix len==1 path.
    result = value_classifier.classify("-o")
    assert result["type"] == "short_flag"
    assert result["char"] == "o"


def test_unknown_first_char_in_multi_char_suffix(
    bool_classifier: TokenClassifier,
) -> None:
    """-Xl where X is unknown falls back to unknown_flag via _classify_stacked."""
    result = bool_classifier.classify("-Xl")
    assert result["type"] == "unknown_flag"


def test_single_dash_long_not_matching_falls_to_short_path(
    classifier: TokenClassifier,
) -> None:
    """-unknown where suffix is not in sdl and first char is unknown → unknown_flag."""
    result = classifier.classify("-zzz")
    assert result["type"] == "unknown_flag"
