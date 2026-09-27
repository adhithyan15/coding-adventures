defmodule BuildTool.GitDiffTest do
  use ExUnit.Case, async: true

  alias BuildTool.GitDiff
  alias BuildTool.GlobMatch.PatternError

  setup do
    repo_root = Path.join(System.tmp_dir!(), "build_tool_gitdiff_test")
    package_root = Path.join([repo_root, "code", "packages", "python", "demo"])

    package = %{
      name: "python/demo",
      path: package_root,
      language: "python",
      is_starlark: true,
      declared_srcs: ["src/*.py", "[z-a].py"]
    }

    {:ok, repo_root: repo_root, package: package}
  end

  test "validates every declared source before an earlier pattern can match", context do
    assert_raise PatternError, "ambiguous or descending character class in glob pattern", fn ->
      GitDiff.map_files_to_packages(
        ["code/packages/python/demo/src/main.py"],
        [context.package],
        context.repo_root
      )
    end
  end

  test "validates every declared source before the BUILD fast path", context do
    assert_raise PatternError, "ambiguous or descending character class in glob pattern", fn ->
      GitDiff.map_files_to_packages(
        ["code/packages/python/demo/BUILD"],
        [context.package],
        context.repo_root
      )
    end
  end

  test "validates every declared source before inspecting changed files", context do
    assert_raise PatternError, "ambiguous or descending character class in glob pattern", fn ->
      GitDiff.map_files_to_packages([], [context.package], context.repo_root)
    end
  end
end
