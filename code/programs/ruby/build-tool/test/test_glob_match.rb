# frozen_string_literal: true

# test_glob_match.rb -- Tests for pure string-based glob pattern matching
# =======================================================================
#
# These tests verify that the glob matcher correctly handles all pattern
# types: **, *, ?, character classes, literal paths, and edge cases.
# The test cases are identical to the Go build tool's globmatch_test.go
# to ensure consistent behavior across implementations.

require_relative "test_helper"

class TestGlobMatch < Minitest::Test
  # -- Double-star (**) patterns -----------------------------------------------
  # ** matches zero or more complete path segments.

  def test_double_star_src_nested_py
    assert BuildTool::GlobMatch.match_path?("src/**/*.py", "src/foo.py")
  end

  def test_double_star_deep_nesting
    assert BuildTool::GlobMatch.match_path?("src/**/*.py", "src/a/b/c.py")
  end

  def test_double_star_two_levels
    assert BuildTool::GlobMatch.match_path?("src/**/*.py", "src/a/b.py")
  end

  def test_double_star_wrong_prefix
    refute BuildTool::GlobMatch.match_path?("src/**/*.py", "tests/foo.py")
  end

  def test_double_star_wrong_extension
    refute BuildTool::GlobMatch.match_path?("src/**/*.py", "src/foo.txt")
  end

  def test_leading_double_star_root_file
    assert BuildTool::GlobMatch.match_path?("**/*.py", "foo.py")
  end

  def test_leading_double_star_deep
    assert BuildTool::GlobMatch.match_path?("**/*.py", "a/b/c.py")
  end

  def test_leading_double_star_no_match
    refute BuildTool::GlobMatch.match_path?("**/*.py", "foo.txt")
  end

  def test_standalone_double_star_single_segment
    assert BuildTool::GlobMatch.match_path?("**", "anything")
  end

  def test_standalone_double_star_multi_segment
    # This is the key case that File.fnmatch(FNM_PATHNAME) gets wrong.
    # ** must match zero or more COMPLETE path segments.
    assert BuildTool::GlobMatch.match_path?("**", "a/b/c")
  end

  def test_standalone_double_star_empty
    # ** matches zero segments, so it matches the empty string.
    assert BuildTool::GlobMatch.match_path?("**", "")
  end

  def test_double_star_middle_test_prefix
    assert BuildTool::GlobMatch.match_path?("src/**/test_*.py", "src/test_foo.py")
  end

  def test_double_star_middle_deep_test_prefix
    assert BuildTool::GlobMatch.match_path?("src/**/test_*.py", "src/a/b/test_bar.py")
  end

  def test_double_star_middle_no_test_prefix
    refute BuildTool::GlobMatch.match_path?("src/**/test_*.py", "src/a/b/foo.py")
  end

  def test_trailing_double_star_matches_below
    assert BuildTool::GlobMatch.match_path?("src/**", "src/foo.py")
  end

  def test_trailing_double_star_deep_below
    assert BuildTool::GlobMatch.match_path?("src/**", "src/a/b/c.py")
  end

  def test_trailing_double_star_matches_zero_segments
    # src/** should match "src" because ** can match zero segments.
    # This is another case File.fnmatch gets wrong.
    assert BuildTool::GlobMatch.match_path?("src/**", "src")
  end

  def test_consecutive_double_stars_collapse
    # **/**/*.py is equivalent to **/*.py
    assert BuildTool::GlobMatch.match_path?("**/**/*.py", "a/b.py")
  end

  def test_triple_double_stars_collapse
    assert BuildTool::GlobMatch.match_path?("**/**/**", "x/y/z")
  end

  # -- Single-star (*) patterns ------------------------------------------------
  # * matches within a single path segment (no slashes).

  def test_star_matches_extension
    assert BuildTool::GlobMatch.match_path?("*.py", "foo.py")
  end

  def test_star_matches_different_name
    assert BuildTool::GlobMatch.match_path?("*.py", "bar.py")
  end

  def test_star_does_not_cross_slash
    # * should NOT match across directory boundaries.
    refute BuildTool::GlobMatch.match_path?("*.py", "dir/foo.py")
  end

  def test_star_toml
    assert BuildTool::GlobMatch.match_path?("*.toml", "pyproject.toml")
  end

  def test_star_in_directory
    assert BuildTool::GlobMatch.match_path?("src/*.py", "src/foo.py")
  end

  def test_star_in_directory_no_nesting
    refute BuildTool::GlobMatch.match_path?("src/*.py", "src/a/foo.py")
  end

  # -- Question mark (?) patterns ----------------------------------------------

  def test_question_mark_matches_one
    assert BuildTool::GlobMatch.match_path?("?.py", "a.py")
  end

  def test_question_mark_does_not_match_two
    refute BuildTool::GlobMatch.match_path?("?.py", "ab.py")
  end

  # -- Literal (no wildcards) --------------------------------------------------

  def test_literal_match
    assert BuildTool::GlobMatch.match_path?("pyproject.toml", "pyproject.toml")
  end

  def test_literal_no_match
    refute BuildTool::GlobMatch.match_path?("pyproject.toml", "other.toml")
  end

  def test_literal_path_match
    assert BuildTool::GlobMatch.match_path?("src/main.py", "src/main.py")
  end

  def test_literal_path_no_match
    refute BuildTool::GlobMatch.match_path?("src/main.py", "src/other.py")
  end

  # -- Character classes -------------------------------------------------------

  def test_character_class_c
    assert BuildTool::GlobMatch.match_path?("*.[ch]", "foo.c")
  end

  def test_character_class_h
    assert BuildTool::GlobMatch.match_path?("*.[ch]", "foo.h")
  end

  def test_character_class_no_match
    refute BuildTool::GlobMatch.match_path?("*.[ch]", "foo.py")
  end

  # -- Edge cases --------------------------------------------------------------

  def test_empty_matches_empty
    assert BuildTool::GlobMatch.match_path?("", "")
  end

  def test_empty_pattern_no_match
    refute BuildTool::GlobMatch.match_path?("", "a")
  end

  def test_empty_path_no_match
    refute BuildTool::GlobMatch.match_path?("a", "")
  end

  def test_star_needs_at_least_one_char
    # * in a segment requires at least one character. An empty segment
    # (which can't appear in a split path anyway) wouldn't match.
    refute BuildTool::GlobMatch.match_path?("*", "")
  end

  def test_double_star_matches_zero_segments
    assert BuildTool::GlobMatch.match_path?("**", "")
  end

  def test_literal_multi_segment
    assert BuildTool::GlobMatch.match_path?("a/b/c", "a/b/c")
  end

  def test_literal_multi_segment_mismatch
    refute BuildTool::GlobMatch.match_path?("a/b/c", "a/b/d")
  end

  def test_trailing_slash_normalized
    # Trailing slashes should be stripped before matching.
    assert BuildTool::GlobMatch.match_path?("src/", "src")
  end

  # -- Real Starlark BUILD patterns --------------------------------------------
  # These patterns come from actual BUILD files in the monorepo.

  def test_real_python_src
    assert BuildTool::GlobMatch.match_path?("src/**/*.py", "src/build_tool/cli.py")
  end

  def test_real_python_tests
    assert BuildTool::GlobMatch.match_path?("tests/**/*.py", "tests/test_hasher.py")
  end

  def test_real_elixir_src
    assert BuildTool::GlobMatch.match_path?("src/**/*.ex", "src/build_tool/glob_match.ex")
  end

  def test_real_ruby_lib
    assert BuildTool::GlobMatch.match_path?("lib/**/*.rb", "lib/build_tool/plan.rb")
  end

  # -- split_path tests --------------------------------------------------------

  def test_split_path_empty
    assert_equal [], BuildTool::GlobMatch.split_path("")
  end

  def test_split_path_single
    assert_equal ["a"], BuildTool::GlobMatch.split_path("a")
  end

  def test_split_path_multi
    assert_equal %w[a b c], BuildTool::GlobMatch.split_path("a/b/c")
  end

  def test_split_path_leading_trailing_slashes
    assert_equal %w[a b], BuildTool::GlobMatch.split_path("/a/b/")
  end

  def test_split_path_double_slashes
    assert_equal %w[a b], BuildTool::GlobMatch.split_path("a//b")
  end
end
