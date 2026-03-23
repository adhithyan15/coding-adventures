defmodule BuildTool.GlobMatchTest do
  use ExUnit.Case, async: true

  alias BuildTool.GlobMatch

  # ===========================================================================
  # Double-star (**) patterns
  # ===========================================================================
  #
  # ** matches zero or more complete path segments. It is the key wildcard
  # for recursive directory matching in build systems.

  describe "double-star (**) patterns" do
    test "src/**/*.py matches file one level deep" do
      assert GlobMatch.match_path?("src/**/*.py", "src/foo.py")
    end

    test "src/**/*.py matches file multiple levels deep" do
      assert GlobMatch.match_path?("src/**/*.py", "src/a/b/c.py")
    end

    test "src/**/*.py matches file two levels deep" do
      assert GlobMatch.match_path?("src/**/*.py", "src/a/b.py")
    end

    test "src/**/*.py does NOT match file outside src/" do
      refute GlobMatch.match_path?("src/**/*.py", "tests/foo.py")
    end

    test "src/**/*.py does NOT match non-.py file" do
      refute GlobMatch.match_path?("src/**/*.py", "src/foo.txt")
    end

    test "**/*.py matches file at top level" do
      assert GlobMatch.match_path?("**/*.py", "foo.py")
    end

    test "**/*.py matches file deeply nested" do
      assert GlobMatch.match_path?("**/*.py", "a/b/c.py")
    end

    test "**/*.py does NOT match non-.py file" do
      refute GlobMatch.match_path?("**/*.py", "foo.txt")
    end

    test "** matches any single-segment path" do
      assert GlobMatch.match_path?("**", "anything")
    end

    test "** matches any multi-segment path" do
      assert GlobMatch.match_path?("**", "a/b/c")
    end

    test "** matches empty path (zero segments)" do
      assert GlobMatch.match_path?("**", "")
    end

    test "src/**/test_*.py matches file directly under src/" do
      assert GlobMatch.match_path?("src/**/test_*.py", "src/test_foo.py")
    end

    test "src/**/test_*.py matches file deeply nested" do
      assert GlobMatch.match_path?("src/**/test_*.py", "src/a/b/test_bar.py")
    end

    test "src/**/test_*.py does NOT match non-test file" do
      refute GlobMatch.match_path?("src/**/test_*.py", "src/a/b/foo.py")
    end

    test "src/** matches everything under src/" do
      assert GlobMatch.match_path?("src/**", "src/foo.py")
      assert GlobMatch.match_path?("src/**", "src/a/b/c.py")
    end

    test "src/** matches src itself (zero segments)" do
      assert GlobMatch.match_path?("src/**", "src")
    end

    test "consecutive ** segments collapse to one" do
      assert GlobMatch.match_path?("**/**/*.py", "a/b.py")
      assert GlobMatch.match_path?("**/**/**", "x/y/z")
    end
  end

  # ===========================================================================
  # Single-star (*) patterns
  # ===========================================================================
  #
  # * matches within a single path segment only. It does NOT cross
  # directory boundaries.

  describe "single-star (*) patterns" do
    test "*.py matches simple filename" do
      assert GlobMatch.match_path?("*.py", "foo.py")
      assert GlobMatch.match_path?("*.py", "bar.py")
    end

    test "*.py does NOT match path with directory" do
      refute GlobMatch.match_path?("*.py", "dir/foo.py")
    end

    test "*.toml matches config file" do
      assert GlobMatch.match_path?("*.toml", "pyproject.toml")
    end

    test "src/*.py matches file directly under src/" do
      assert GlobMatch.match_path?("src/*.py", "src/foo.py")
    end

    test "src/*.py does NOT match file in subdirectory" do
      refute GlobMatch.match_path?("src/*.py", "src/a/foo.py")
    end
  end

  # ===========================================================================
  # Question mark (?) patterns
  # ===========================================================================

  describe "question mark (?) patterns" do
    test "?.py matches single character before extension" do
      assert GlobMatch.match_path?("?.py", "a.py")
    end

    test "?.py does NOT match two characters before extension" do
      refute GlobMatch.match_path?("?.py", "ab.py")
    end
  end

  # ===========================================================================
  # Literal (no wildcards) patterns
  # ===========================================================================

  describe "literal patterns" do
    test "exact filename match" do
      assert GlobMatch.match_path?("pyproject.toml", "pyproject.toml")
    end

    test "exact filename does NOT match different file" do
      refute GlobMatch.match_path?("pyproject.toml", "other.toml")
    end

    test "exact path match" do
      assert GlobMatch.match_path?("src/main.py", "src/main.py")
    end

    test "exact path does NOT match different file in same dir" do
      refute GlobMatch.match_path?("src/main.py", "src/other.py")
    end
  end

  # ===========================================================================
  # Edge cases
  # ===========================================================================

  describe "edge cases" do
    test "empty pattern matches empty path" do
      assert GlobMatch.match_path?("", "")
    end

    test "empty pattern does NOT match non-empty path" do
      refute GlobMatch.match_path?("", "a")
    end

    test "non-empty pattern does NOT match empty path" do
      refute GlobMatch.match_path?("a", "")
    end

    test "* does NOT match empty path" do
      refute GlobMatch.match_path?("*", "")
    end

    test "** matches empty path" do
      assert GlobMatch.match_path?("**", "")
    end

    test "multi-segment literal match" do
      assert GlobMatch.match_path?("a/b/c", "a/b/c")
    end

    test "multi-segment literal mismatch" do
      refute GlobMatch.match_path?("a/b/c", "a/b/d")
    end

    test "trailing slashes are normalized" do
      assert GlobMatch.match_path?("src/", "src")
    end
  end

  # ===========================================================================
  # Real-world Starlark BUILD file patterns
  # ===========================================================================
  #
  # These patterns come from actual BUILD files in the monorepo.

  describe "real-world patterns from Starlark BUILD files" do
    test "src/**/*.py matches Python source" do
      assert GlobMatch.match_path?("src/**/*.py", "src/build_tool/cli.py")
    end

    test "tests/**/*.py matches Python test" do
      assert GlobMatch.match_path?("tests/**/*.py", "tests/test_hasher.py")
    end

    test "src/**/*.ex matches Elixir source" do
      assert GlobMatch.match_path?("src/**/*.ex", "src/build_tool/glob_match.ex")
    end

    test "lib/**/*.rb matches Ruby source" do
      assert GlobMatch.match_path?("lib/**/*.rb", "lib/build_tool/plan.rb")
    end

    test "lib/**/*.ex matches Elixir lib source" do
      assert GlobMatch.match_path?("lib/**/*.ex", "lib/build_tool/hasher.ex")
    end

    test "test/**/*.exs matches Elixir test" do
      assert GlobMatch.match_path?("test/**/*.exs", "test/glob_match_test.exs")
    end
  end

  # ===========================================================================
  # split_path/1
  # ===========================================================================

  describe "split_path/1" do
    test "empty string returns empty list" do
      assert GlobMatch.split_path("") == []
    end

    test "single segment" do
      assert GlobMatch.split_path("a") == ["a"]
    end

    test "multiple segments" do
      assert GlobMatch.split_path("a/b/c") == ["a", "b", "c"]
    end

    test "leading and trailing slashes are ignored" do
      assert length(GlobMatch.split_path("/a/b/")) == 2
    end

    test "double slashes are collapsed" do
      assert length(GlobMatch.split_path("a//b")) == 2
    end
  end
end
