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
  Maps changed file paths to package names.

  A file belongs to a package if its path starts with the package's directory
  path relative to the repo root. Returns a `MapSet` of package names.

  ## Parameters

    - `changed_files` — list of file paths relative to the repo root
    - `packages` — list of package maps from discovery
    - `repo_root` — absolute path to the repo root

  ## Example

      iex> files = ["code/packages/python/logic-gates/lib/gates.py"]
      iex> packages = [%{name: "python/logic-gates", path: "/repo/code/packages/python/logic-gates"}]
      iex> BuildTool.GitDiff.map_files_to_packages(files, packages, "/repo")
      MapSet.new(["python/logic-gates"])
  """
  def map_files_to_packages(changed_files, packages, repo_root) do
    # Build relative path lookup for each package.
    pkg_paths =
      packages
      |> Enum.map(fn pkg ->
        rel_path = Path.relative_to(pkg.path, repo_root)
        # Normalize to forward slashes for consistent matching.
        rel_path = String.replace(rel_path, "\\", "/")
        {pkg.name, rel_path}
      end)

    changed_files
    |> Enum.reduce(MapSet.new(), fn file, acc ->
      # Normalize file path to forward slashes.
      file = String.replace(file, "\\", "/")

      case Enum.find(pkg_paths, fn {_name, rel_path} ->
             String.starts_with?(file, rel_path <> "/") or file == rel_path
           end) do
        {name, _} -> MapSet.put(acc, name)
        nil -> acc
      end
    end)
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
