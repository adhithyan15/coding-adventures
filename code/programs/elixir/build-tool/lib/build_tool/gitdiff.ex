defmodule BuildTool.GitDiff do
  @moduledoc """
  Git-based change detection for the build tool.

  Instead of maintaining a cache file, this module uses `git diff` to determine
  which files changed between the current branch and a base ref (typically
  `origin/main`). Changed files are mapped to packages, then the dependency
  graph's `affected_nodes/2` finds everything that needs rebuilding.

  This is the DEFAULT change detection mode. Git is the source of truth.

  ## How it works

  The detection pipeline has two stages:

    1. **Get changed files** — run `git diff --name-only <base>...HEAD` to get
       a list of file paths that changed since the branch diverged from the base.
       Uses three-dot diff (merge base) which shows changes specific to the
       current branch. Falls back to two-dot diff if three-dot fails.

    2. **Map files to packages** — for each changed file, check if its path
       starts with any package's directory path. If so, that package has changed.

  ## Three-dot vs two-dot diff

  Git's `A...B` (three-dot) diff compares B against the merge base of A and B.
  This shows only the changes made on the current branch, not changes made to
  the base since we branched. This is exactly what CI systems need.

  If three-dot fails (e.g., the base ref doesn't exist), we fall back to
  `A..B` (two-dot), which compares A directly to B. This may include changes
  from both sides but is still useful.

  ## Example

      iex> files = BuildTool.GitDiff.get_changed_files("/repo", "origin/main")
      ["code/packages/python/logic-gates/lib/gates.py", "README.md"]

      iex> changed_pkgs = BuildTool.GitDiff.map_files_to_packages(files, packages, "/repo")
      %MapSet<["python/logic-gates"]>
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Runs `git diff --name-only <base>...HEAD` and returns the list of changed
  file paths relative to the repo root.

  Uses three-dot diff first (merge base). Falls back to two-dot diff if
  three-dot fails. Returns an empty list if both fail.
  """
  def get_changed_files(repo_root, diff_base) do
    # Try three-dot diff first (merge base).
    case System.cmd("git", ["diff", "--name-only", diff_base <> "...HEAD"],
           cd: repo_root,
           stderr_to_stdout: true
         ) do
      {output, 0} ->
        parse_diff_output(output)

      _ ->
        # Fallback: two-dot diff.
        case System.cmd("git", ["diff", "--name-only", diff_base, "HEAD"],
               cd: repo_root,
               stderr_to_stdout: true
             ) do
          {output, 0} -> parse_diff_output(output)
          _ -> []
        end
    end
  end

  @doc """
  Maps changed file paths to package names, with strict Starlark filtering.

  For **shell BUILD packages** (or Starlark packages without declared srcs),
  a file belongs to a package if its path starts with the package's directory
  path. Any file change triggers a rebuild — this is the legacy behavior.

  For **Starlark BUILD packages** with declared srcs, we apply strict filtering:
  only trigger a rebuild if the changed file matches one of the declared source
  patterns (or is a BUILD file itself). This means editing README.md or
  CHANGELOG.md in a Starlark package does NOT trigger a rebuild.

  This strict filtering is the key optimization that makes Starlark BUILD files
  worthwhile: by declaring exactly which files matter for the build, we avoid
  rebuilding packages when only documentation changes.

  ## Parameters

    - `changed_files` — list of file paths relative to the repo root
    - `packages` — list of package maps from discovery
    - `repo_root` — absolute path to the repo root

  ## Package map fields used

    - `:name` — qualified package name (e.g., `"python/logic-gates"`)
    - `:path` — absolute path on disk
    - `:is_starlark` — (optional) boolean, true if BUILD file uses Starlark
    - `:declared_srcs` — (optional) list of glob patterns from Starlark srcs

  ## Example

      iex> files = ["code/packages/python/logic-gates/lib/gates.py"]
      iex> packages = [%{name: "python/logic-gates", path: "/repo/code/packages/python/logic-gates"}]
      iex> BuildTool.GitDiff.map_files_to_packages(files, packages, "/repo")
      MapSet.new(["python/logic-gates"])
  """
  def map_files_to_packages(changed_files, packages, repo_root) do
    # Build relative path lookup with Starlark metadata for each package.
    #
    # Each entry is a tuple of:
    #   {name, rel_path, is_starlark, declared_srcs}
    #
    # The is_starlark and declared_srcs fields are extracted from the package
    # map using Map.get with defaults — older package maps that predate
    # Starlark support won't have these keys, and that's fine.
    pkg_infos =
      packages
      |> Enum.map(fn pkg ->
        rel_path = Path.relative_to(pkg.path, repo_root)
        # Normalize to forward slashes for consistent matching.
        rel_path = String.replace(rel_path, "\\", "/")

        is_starlark = Map.get(pkg, :is_starlark, false)
        declared_srcs = Map.get(pkg, :declared_srcs, [])

        {pkg.name, rel_path, is_starlark, declared_srcs}
      end)

    changed_files
    |> Enum.reduce(MapSet.new(), fn file, acc ->
      # Normalize file path to forward slashes.
      file = String.replace(file, "\\", "/")

      case find_owning_package(file, pkg_infos) do
        nil ->
          # File doesn't belong to any package — skip it.
          acc

        {name, rel_path, is_starlark, declared_srcs} ->
          if should_trigger_rebuild?(file, rel_path, is_starlark, declared_srcs) do
            MapSet.put(acc, name)
          else
            acc
          end
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Strict Starlark filtering helpers
  # ---------------------------------------------------------------------------
  #
  # The rebuild decision tree:
  #
  #   1. Is the package a Starlark package with declared srcs?
  #      NO  → rebuild (legacy: any file change triggers rebuild)
  #      YES → continue to step 2
  #
  #   2. Is the changed file a BUILD file?
  #      YES → rebuild (build definition changed)
  #      NO  → continue to step 3
  #
  #   3. Does the changed file match any declared src pattern?
  #      YES → rebuild
  #      NO  → skip (e.g., README.md, CHANGELOG.md)

  defp find_owning_package(file, pkg_infos) do
    Enum.find(pkg_infos, fn {_name, rel_path, _is_starlark, _srcs} ->
      String.starts_with?(file, rel_path <> "/") or file == rel_path
    end)
  end

  defp should_trigger_rebuild?(file, rel_path, is_starlark, declared_srcs) do
    if not is_starlark or declared_srcs == [] do
      # Shell BUILD or no declared srcs: any file triggers rebuild.
      true
    else
      # Starlark package with declared srcs: strict filtering.
      # Get the file's path relative to the package directory.
      rel_to_package = String.trim_leading(file, rel_path <> "/")

      # BUILD file changes always trigger a rebuild — the build
      # definition itself changed.
      base = Path.basename(rel_to_package)

      if base == "BUILD" or String.starts_with?(base, "BUILD_") do
        true
      else
        # Check if the file matches any declared source pattern.
        Enum.any?(declared_srcs, fn pattern ->
          BuildTool.GlobMatch.match_path?(pattern, rel_to_package)
        end)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp parse_diff_output(output) do
    output
    |> String.trim()
    |> String.split("\n")
    |> Enum.map(&String.trim/1)
    |> Enum.filter(&(&1 != ""))
  end
end
