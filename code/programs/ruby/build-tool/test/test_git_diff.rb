# frozen_string_literal: true

# test_git_diff.rb -- Tests for git-based change detection
# ========================================================
#
# These tests verify the map_files_to_packages function, which is the
# core logic of git diff-based change detection. We test shell BUILD
# packages, Starlark strict filtering, mixed packages, and edge cases.
#
# We do NOT test get_changed_files here because it shells out to git.
# Integration testing of git diff is left to the CI pipeline.

require_relative "test_helper"

class TestGitDiff < Minitest::Test
  include TestHelper

  # -- Helper to create a simple package struct --------------------------------
  #
  # The Package Data.define doesn't have is_starlark/declared_srcs fields,
  # so we create a test double that responds to those methods.

  # SimplePackage is a test stand-in for packages with Starlark metadata.
  # We use Struct here because Package = Data.define doesn't include
  # is_starlark and declared_srcs fields.
  SimplePackage = Struct.new(:name, :path, :build_commands, :language,
                             :is_starlark, :declared_srcs, keyword_init: true) do
    def initialize(name:, path:, build_commands: [], language: "python",
                   is_starlark: false, declared_srcs: [])
      super
    end
  end

  # -- Shell BUILD packages (legacy behavior) ----------------------------------

  def test_shell_build_any_file_triggers
    # Shell BUILD packages: any file under the package triggers rebuild.
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/src/main.py"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_shell_build_readme_triggers
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/README.md"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_shell_build_changelog_triggers
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/CHANGELOG.md"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_shell_build_other_package_no_trigger
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/bar/src/main.py"],
      packages, Pathname("/repo")
    )

    refute changed["python/foo"]
  end

  # -- Starlark strict filtering -----------------------------------------------

  def test_starlark_source_file_matches
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py", "tests/**/*.py", "pyproject.toml"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/src/main.py"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_starlark_nested_source_matches
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/src/a/b.py"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_starlark_test_file_matches
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py", "tests/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/tests/test_main.py"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_starlark_literal_pattern_matches
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["pyproject.toml"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/pyproject.toml"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_starlark_readme_does_not_trigger
    # This is the key behavior: README.md edits should NOT trigger
    # rebuilds for Starlark packages with declared srcs.
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py", "tests/**/*.py", "pyproject.toml"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/README.md"],
      packages, Pathname("/repo")
    )

    refute changed["python/foo"]
  end

  def test_starlark_changelog_does_not_trigger
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/CHANGELOG.md"],
      packages, Pathname("/repo")
    )

    refute changed["python/foo"]
  end

  def test_starlark_docs_does_not_trigger
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/docs/guide.md"],
      packages, Pathname("/repo")
    )

    refute changed["python/foo"]
  end

  def test_starlark_build_file_always_triggers
    # BUILD file changes always trigger, even in strict mode.
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/BUILD"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  def test_starlark_build_linux_always_triggers
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foo/BUILD_linux"],
      packages, Pathname("/repo")
    )

    assert changed["python/foo"]
  end

  # -- Starlark without declared srcs (fallback) --------------------------------

  def test_starlark_no_declared_srcs_any_file_triggers
    # Starlark package but with empty DeclaredSrcs: falls back to any-file.
    packages = [
      SimplePackage.new(
        name: "go/bar",
        path: Pathname("/repo/code/packages/go/bar"),
        language: "go",
        is_starlark: true,
        declared_srcs: []
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/go/bar/README.md"],
      packages, Pathname("/repo")
    )

    assert changed["go/bar"]
  end

  # -- Mixed packages ----------------------------------------------------------

  def test_mixed_strict_and_loose
    packages = [
      SimplePackage.new(
        name: "python/strict",
        path: Pathname("/repo/code/packages/python/strict"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      ),
      SimplePackage.new(
        name: "python/loose",
        path: Pathname("/repo/code/packages/python/loose"),
        language: "python"
      )
    ]

    # README in strict package: no trigger.
    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/strict/README.md"],
      packages, Pathname("/repo")
    )
    refute changed["python/strict"]

    # README in loose package: triggers.
    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/loose/README.md"],
      packages, Pathname("/repo")
    )
    assert changed["python/loose"]
  end

  # -- Multiple files ----------------------------------------------------------

  def test_multiple_files_multiple_packages
    packages = [
      SimplePackage.new(
        name: "python/a",
        path: Pathname("/repo/code/packages/python/a"),
        language: "python",
        is_starlark: true,
        declared_srcs: ["src/**/*.py"]
      ),
      SimplePackage.new(
        name: "ruby/b",
        path: Pathname("/repo/code/packages/ruby/b"),
        language: "ruby"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      [
        "code/packages/python/a/src/foo.py",
        "code/packages/ruby/b/lib/bar.rb",
        "code/packages/python/a/README.md"  # should NOT trigger
      ],
      packages, Pathname("/repo")
    )

    assert changed["python/a"], "python/a should be changed (src/foo.py matches)"
    assert changed["ruby/b"], "ruby/b should be changed (shell BUILD, any file)"
  end

  # -- Edge cases --------------------------------------------------------------

  def test_no_changed_files
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages([], packages, Pathname("/repo"))
    assert_empty changed
  end

  def test_file_outside_any_package
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["README.md", ".github/workflows/ci.yml"],
      packages, Pathname("/repo")
    )

    assert_empty changed
  end

  def test_prefix_collision_avoided
    # "foobar" should NOT match package "foo".
    packages = [
      SimplePackage.new(
        name: "python/foo",
        path: Pathname("/repo/code/packages/python/foo"),
        language: "python"
      )
    ]

    changed = BuildTool::GitDiff.map_files_to_packages(
      ["code/packages/python/foobar/src/main.py"],
      packages, Pathname("/repo")
    )

    refute changed["python/foo"]
  end
end
