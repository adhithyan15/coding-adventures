"""
Tests for glob_match.py -- Pure String-Based Glob Pattern Matching
===================================================================

These tests verify the ``match_path()`` function which matches file paths
against glob patterns without any filesystem access. The function supports
``*`` (single segment), ``?`` (single character), ``[…]`` (character classes),
and ``**`` (recursive, zero or more segments).

Test organization
-----------------

Tests are grouped by the glob feature being tested:

1. **Literal matching** -- exact string comparison, no wildcards.
2. **Single-star (``*``)** -- matches within one path segment.
3. **Question mark (``?``)** -- matches exactly one character.
4. **Character classes (``[…]``)** -- set and range matching.
5. **Double-star (``**``)** -- recursive matching across segments.
6. **Edge cases** -- empty strings, slashes, consecutive ``**``.
7. **Real-world BUILD patterns** -- patterns that actually appear in
   Starlark BUILD files in this monorepo.
"""

from __future__ import annotations

from build_tool.glob_match import match_path

# =========================================================================
# 1. Literal matching -- no wildcards
# =========================================================================
#
# When a pattern contains no special characters, it must match the path
# exactly (segment by segment).


class TestLiteralMatching:
    """Patterns with no wildcards must match paths exactly."""

    def test_exact_match(self):
        """An exact path matches itself."""
        assert match_path("src/main.py", "src/main.py") is True

    def test_different_filename(self):
        """Different filenames don't match."""
        assert match_path("src/main.py", "src/other.py") is False

    def test_different_directory(self):
        """Different directory prefixes don't match."""
        assert match_path("src/main.py", "lib/main.py") is False

    def test_extra_depth(self):
        """Pattern is shorter than path -- no match."""
        assert match_path("src/main.py", "src/sub/main.py") is False

    def test_pattern_longer_than_path(self):
        """Pattern is deeper than path -- no match."""
        assert match_path("src/sub/main.py", "src/main.py") is False

    def test_single_segment(self):
        """Single segment pattern matches single segment path."""
        assert match_path("BUILD", "BUILD") is True

    def test_single_segment_mismatch(self):
        """Single segment pattern rejects different single segment."""
        assert match_path("BUILD", "Makefile") is False


# =========================================================================
# 2. Single-star (*) -- matches within one segment
# =========================================================================
#
# The ``*`` wildcard matches any sequence of non-``/`` characters within
# a single path segment. It never crosses directory boundaries.


class TestSingleStar:
    """The * wildcard matches any characters within one segment."""

    def test_star_extension(self):
        """``*.py`` matches any Python file in the same directory."""
        assert match_path("src/*.py", "src/main.py") is True

    def test_star_extension_nested(self):
        """``*.py`` does NOT match files in subdirectories."""
        assert match_path("src/*.py", "src/sub/main.py") is False

    def test_star_prefix(self):
        """``src/*`` matches any file directly under src/."""
        assert match_path("src/*", "src/anything.txt") is True

    def test_star_prefix_no_subdirs(self):
        """``src/*`` does not match files in subdirectories of src/."""
        assert match_path("src/*", "src/sub/file.txt") is False

    def test_star_in_middle(self):
        """``src/test_*.py`` matches files starting with test_."""
        assert match_path("src/test_*.py", "src/test_foo.py") is True
        assert match_path("src/test_*.py", "src/main.py") is False

    def test_multiple_stars_one_segment(self):
        """Multiple * in one segment: ``*_test_*.py``."""
        assert match_path("*_test_*.py", "foo_test_bar.py") is True
        assert match_path("*_test_*.py", "foobar.py") is False


# =========================================================================
# 3. Question mark (?) -- matches exactly one character
# =========================================================================
#
# The ``?`` wildcard matches exactly one non-``/`` character.


class TestQuestionMark:
    """The ? wildcard matches exactly one character."""

    def test_single_question_mark(self):
        """``?.py`` matches a one-character Python filename."""
        assert match_path("?.py", "a.py") is True

    def test_question_mark_too_many(self):
        """``?.py`` does not match a multi-character filename."""
        assert match_path("?.py", "ab.py") is False

    def test_multiple_question_marks(self):
        """``???.txt`` matches exactly three characters before .txt."""
        assert match_path("???.txt", "abc.txt") is True
        assert match_path("???.txt", "ab.txt") is False
        assert match_path("???.txt", "abcd.txt") is False

    def test_question_in_path(self):
        """Question mark works in directory names too."""
        assert match_path("src/?/main.py", "src/a/main.py") is True
        assert match_path("src/?/main.py", "src/ab/main.py") is False


# =========================================================================
# 4. Character classes ([…]) -- set and range matching
# =========================================================================
#
# Character classes like ``[abc]`` match one character from a set.
# ``[!abc]`` negates the set. ``[a-z]`` is a range.


class TestCharacterClasses:
    """Character classes [abc], [!abc], [a-z] match one character from a set."""

    def test_character_set(self):
        """``[abc].py`` matches a.py, b.py, c.py but not d.py."""
        assert match_path("[abc].py", "a.py") is True
        assert match_path("[abc].py", "d.py") is False

    def test_negated_set(self):
        """``[!abc].py`` matches d.py but not a.py."""
        assert match_path("[!abc].py", "d.py") is True
        assert match_path("[!abc].py", "a.py") is False

    def test_range(self):
        """``[0-9].txt`` matches digit filenames."""
        assert match_path("[0-9].txt", "5.txt") is True
        assert match_path("[0-9].txt", "a.txt") is False


# =========================================================================
# 5. Double-star (**) -- recursive matching
# =========================================================================
#
# The ``**`` pattern matches zero or more complete path segments. It must
# appear as an entire segment on its own (not combined with other characters
# like ``foo**``).


class TestDoubleStar:
    """The ** pattern matches zero or more path segments."""

    def test_leading_doublestar(self):
        """``**/*.py`` matches .py files at any depth."""
        assert match_path("**/*.py", "main.py") is True
        assert match_path("**/*.py", "src/main.py") is True
        assert match_path("**/*.py", "src/sub/deep/main.py") is True

    def test_trailing_doublestar(self):
        """``src/**`` matches anything under src/."""
        assert match_path("src/**", "src/a") is True
        assert match_path("src/**", "src/a/b") is True
        assert match_path("src/**", "src/a/b/c") is True

    def test_doublestar_zero_segments(self):
        """``src/**/*.py`` matches files directly under src/ (zero segments)."""
        assert match_path("src/**/*.py", "src/main.py") is True

    def test_doublestar_one_segment(self):
        """``src/**/*.py`` matches one directory level deep."""
        assert match_path("src/**/*.py", "src/sub/main.py") is True

    def test_doublestar_many_segments(self):
        """``src/**/*.py`` matches many levels deep."""
        assert match_path("src/**/*.py", "src/a/b/c/d/main.py") is True

    def test_doublestar_wrong_extension(self):
        """``**/*.py`` does NOT match non-.py files."""
        assert match_path("**/*.py", "src/main.rb") is False

    def test_middle_doublestar(self):
        """``src/**/test_*.py`` matches test files at any depth under src/."""
        assert match_path("src/**/test_*.py", "src/test_foo.py") is True
        assert match_path("src/**/test_*.py", "src/sub/test_bar.py") is True
        assert match_path("src/**/test_*.py", "src/a/b/test_baz.py") is True

    def test_doublestar_not_matching_wrong_prefix(self):
        """``src/**`` does NOT match paths not starting with src/."""
        assert match_path("src/**", "lib/a") is False

    def test_consecutive_doublestars(self):
        """``**/**/*.py`` collapses to the same as ``**/*.py``."""
        assert match_path("**/**/*.py", "main.py") is True
        assert match_path("**/**/*.py", "src/main.py") is True
        assert match_path("**/**/*.py", "a/b/c/main.py") is True

    def test_only_doublestar(self):
        """``**`` alone matches any path at any depth."""
        assert match_path("**", "anything") is True
        assert match_path("**", "a/b/c") is True


# =========================================================================
# 6. Edge cases
# =========================================================================
#
# These test unusual inputs: empty strings, leading/trailing slashes,
# double slashes, and other boundary conditions.


class TestEdgeCases:
    """Edge cases: empty strings, extra slashes, unusual paths."""

    def test_empty_pattern_empty_path(self):
        """Empty pattern matches empty path."""
        assert match_path("", "") is True

    def test_empty_pattern_nonempty_path(self):
        """Empty pattern does NOT match a non-empty path."""
        assert match_path("", "src/main.py") is False

    def test_nonempty_pattern_empty_path(self):
        """Non-empty pattern does NOT match an empty path."""
        assert match_path("src/*.py", "") is False

    def test_leading_slash_ignored(self):
        """Leading slashes are stripped (both are split on /)."""
        assert match_path("/src/main.py", "/src/main.py") is True

    def test_trailing_slash_ignored(self):
        """Trailing slashes are stripped."""
        assert match_path("src/", "src/") is True

    def test_double_slash_ignored(self):
        """Double slashes are collapsed (split filters empty strings)."""
        assert match_path("src//main.py", "src/main.py") is True

    def test_doublestar_with_empty_path(self):
        """``**`` matches an empty path (zero segments)."""
        assert match_path("**", "") is True

    def test_doublestar_with_only_star_suffix(self):
        """``**/BUILD`` matches BUILD at any depth."""
        assert match_path("**/BUILD", "BUILD") is True
        assert match_path("**/BUILD", "src/BUILD") is True
        assert match_path("**/BUILD", "a/b/c/BUILD") is True


# =========================================================================
# 7. Real-world BUILD patterns
# =========================================================================
#
# These patterns are taken from actual Starlark BUILD files in this monorepo.
# They verify that glob_match works for the patterns that matter most.


class TestRealWorldPatterns:
    """Patterns commonly found in Starlark BUILD files."""

    def test_python_src_pattern(self):
        """``src/**/*.py`` matches Python source files in src layout."""
        assert match_path("src/**/*.py", "src/build_tool/cli.py") is True
        assert match_path("src/**/*.py", "src/build_tool/__init__.py") is True

    def test_python_test_pattern(self):
        """``tests/**/*.py`` matches test files."""
        assert match_path("tests/**/*.py", "tests/test_cli.py") is True
        assert match_path("tests/**/*.py", "tests/fixtures/helper.py") is True

    def test_go_pattern(self):
        """``**/*.go`` matches Go files at any depth."""
        assert match_path("**/*.go", "main.go") is True
        assert match_path("**/*.go", "internal/plan/plan.go") is True

    def test_toml_pattern(self):
        """``*.toml`` matches TOML files in the root."""
        assert match_path("*.toml", "pyproject.toml") is True
        assert match_path("*.toml", "sub/pyproject.toml") is False

    def test_build_file_pattern(self):
        """``BUILD`` matches the BUILD file literally."""
        assert match_path("BUILD", "BUILD") is True
        assert match_path("BUILD", "BUILD_mac") is False
