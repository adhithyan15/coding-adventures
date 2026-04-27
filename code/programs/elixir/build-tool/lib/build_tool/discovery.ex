defmodule BuildTool.Discovery do
  @moduledoc """
  Package discovery walks a monorepo directory tree to find packages.

  ## How package discovery works

  A monorepo can contain hundreds of packages across multiple languages. The
  build system discovers them by recursively walking the directory tree and
  looking for BUILD files. Any directory containing a BUILD file is a package.

  The walk is recursive. Starting from the root:

    1. If the current directory's name is in the skip list, ignore it entirely.
    2. If the current directory has a BUILD file, it is a package. Register it
       and stop — we don't recurse into packages.
    3. Otherwise, list all subdirectories and recurse into each one.

  This is the same approach used by Bazel, Buck, and Pants. No configuration
  files are needed to route the walk — the presence of a BUILD file is
  sufficient to identify a package.

  ## Skip list

  Certain directories are known to never contain packages: `.git`, `.venv`,
  `node_modules`, `__pycache__`, etc. The skip list prevents the walker from
  descending into these directories, keeping discovery fast even in large
  repos with deep dependency trees.

  ## Platform-specific BUILD files

  On macOS, if `BUILD_mac` exists in a directory, we use it instead of BUILD.
  On Linux, `BUILD_linux` takes precedence. This allows platform-specific build
  commands (e.g., different compiler flags or test runners).

  ## Language inference

  We infer a package's language from its directory path. If the path contains
  "python", "ruby", "go", "rust", "typescript", "elixir", or "lua" as a component
  under "packages" or "programs", that is the language. The package name is
  "{language}/{dirname}", e.g., "python/logic-gates" or "go/directed-graph".

  ## The Package struct

  Each discovered package is represented as a map with four fields:

      %{
        name: "python/logic-gates",       # qualified name
        path: "/repo/code/packages/...",   # absolute path on disk
        build_commands: ["python -m pip install", "pytest"],  # lines from BUILD
        language: "python"                 # inferred language
      }
  """

  # ---------------------------------------------------------------------------
  # Skip list
  # ---------------------------------------------------------------------------
  #
  # Directories that should never be traversed during package discovery.
  # These are known to contain non-source files (caches, dependencies,
  # build artifacts) that would waste time to scan and could never contain
  # valid packages.

  @skip_dirs MapSet.new([
    ".git",
    ".hg",
    ".svn",
    ".venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "target",
    ".claude",
    "Pods",
    "_build",
    "deps",
    "coverage"
  ])

  # ---------------------------------------------------------------------------
  # Known languages
  # ---------------------------------------------------------------------------

  @known_languages [
    "python",
    "ruby",
    "go",
    "rust",
    "typescript",
    "elixir",
    "lua",
    "perl",
    "swift",
    "haskell",
    "wasm",
    "csharp",
    "fsharp",
    "dotnet"
  ]

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Recursively walks the directory tree starting from `root`, collecting
  packages with BUILD files. Returns a list of package maps sorted by
  package name for deterministic output.

  This is the main entry point for the discovery module. The `root`
  parameter should typically be the "code/" directory inside the repo.

  ## Example

      iex> packages = BuildTool.Discovery.discover_packages("/repo/code")
      iex> Enum.map(packages, & &1.name)
      ["elixir/progress-bar", "go/directed-graph", "python/logic-gates"]
  """
  def discover_packages(root) do
    root
    |> walk_dirs([])
    |> Enum.sort_by(& &1.name)
  end

  @doc """
  Reads a file and returns non-blank, non-comment lines.

  Blank lines and lines starting with '#' are stripped out. Leading and
  trailing whitespace is removed from each line. If the file does not
  exist or is unreadable, an empty list is returned — a missing file
  simply means "nothing to see here".

  This is exported for use by the resolver (to read go.mod, etc.).

  ## Example

      iex> BuildTool.Discovery.read_lines("/path/to/BUILD")
      ["python -m pip install -e .", "pytest"]
  """
  def read_lines(filepath) do
    case File.read(filepath) do
      {:ok, data} ->
        data
        |> String.split("\n")
        |> Enum.map(&String.trim/1)
        |> Enum.filter(fn line -> line != "" and not String.starts_with?(line, "#") end)

      {:error, _} ->
        []
    end
  end

  # ---------------------------------------------------------------------------
  # Language inference
  # ---------------------------------------------------------------------------

  @doc """
  Inspects the directory path to determine the programming language.

  We look for known language names ("python", "ruby", "go", "rust",
  "typescript", "elixir") as path components. For example,
  "/repo/code/packages/python/logic-gates" yields "python".

  ## Example

      iex> BuildTool.Discovery.infer_language("/repo/code/packages/python/logic-gates")
      "python"
      iex> BuildTool.Discovery.infer_language("/some/random/path")
      "unknown"
  """
  def infer_language(path) do
    # Normalize path separators to forward slashes for consistent parsing.
    parts =
      path
      |> String.replace("\\", "/")
      |> String.split("/")

    Enum.find(@known_languages, "unknown", fn lang ->
      lang in parts
    end)
  end

  @doc """
  Builds a qualified package name like "python/logic-gates" from the
  language and the directory's basename.

  ## Example

      iex> BuildTool.Discovery.infer_package_name("/repo/code/packages/python/logic-gates", "python")
      "python/logic-gates"
  """
  def infer_package_name(path, language) do
    language <> "/" <> Path.basename(path)
  end

  # ---------------------------------------------------------------------------
  # BUILD file selection
  # ---------------------------------------------------------------------------

  @doc """
  Returns the path to the appropriate BUILD file for the current platform,
  or nil if none exists.

  Priority:
    1. `BUILD_mac` on macOS (Darwin)
    2. `BUILD_linux` on Linux
    3. `BUILD` (cross-platform fallback)
    4. `nil` if no BUILD file exists

  ## Example

      iex> BuildTool.Discovery.get_build_file("/repo/code/packages/python/logic-gates")
      "/repo/code/packages/python/logic-gates/BUILD"
  """
  def get_build_file(directory) do
    get_build_file_for_platform(directory, current_os())
  end

  @doc """
  Like `get_build_file/1` but accepts an explicit OS name. This is useful
  for testing platform-specific behavior without running on that platform.

  The `os` parameter should be `:darwin`, `:linux`, or `:windows`.

  Priority (most specific wins):
    1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
    2. Shared: BUILD_mac_and_linux (macOS or Linux — for Unix-like systems)
    3. Generic: BUILD (all platforms)
    4. nil if no BUILD file exists

  ## Example

      iex> BuildTool.Discovery.get_build_file_for_platform("/some/dir", :darwin)
      # Returns path to BUILD_mac if it exists, else BUILD_mac_and_linux, else BUILD, else nil
  """
  def get_build_file_for_platform(directory, os) do
    # Step 1: Check for the most specific platform file.
    platform_file =
      case os do
        :darwin -> Path.join(directory, "BUILD_mac")
        :linux -> Path.join(directory, "BUILD_linux")
        :windows -> Path.join(directory, "BUILD_windows")
        _ -> nil
      end

    cond do
      platform_file != nil and file_exists?(platform_file) ->
        platform_file

      # Step 2: Check for the shared Unix file (macOS + Linux).
      os in [:darwin, :linux] and
          file_exists?(Path.join(directory, "BUILD_mac_and_linux")) ->
        Path.join(directory, "BUILD_mac_and_linux")

      # Step 3: Fall back to the generic BUILD file.
      file_exists?(Path.join(directory, "BUILD")) ->
        Path.join(directory, "BUILD")

      true ->
        nil
    end
  end

  # ---------------------------------------------------------------------------
  # Directory walking
  # ---------------------------------------------------------------------------
  #
  # walkDirs recursively descends into subdirectories, collecting packages
  # that have BUILD files. This is the heart of the discovery algorithm.
  #
  # The walk uses the skip list to avoid descending into directories that
  # are known to contain non-source files.
  #
  # The recursion stops at BUILD files: once we find a package, we don't
  # look inside it for sub-packages. This keeps the model simple — a
  # package is a leaf in the directory tree.

  defp walk_dirs(directory, packages) do
    dir_name = Path.basename(directory)

    if MapSet.member?(@skip_dirs, dir_name) do
      packages
    else
      case get_build_file(directory) do
        nil ->
          # Not a package — list all subdirectories and recurse into each one.
          case File.ls(directory) do
            {:ok, entries} ->
              entries
              |> Enum.sort()
              |> Enum.reduce(packages, fn entry, acc ->
                subdir = Path.join(directory, entry)

                if File.dir?(subdir) do
                  walk_dirs(subdir, acc)
                else
                  acc
                end
              end)

            {:error, _} ->
              packages
          end

        build_file ->
          # This directory is a package. Read the BUILD commands and register it.
          commands = read_lines(build_file)
          language = infer_language(directory)
          name = infer_package_name(directory, language)

          package = %{
            name: name,
            path: directory,
            build_commands: commands,
            language: language
          }

          [package | packages]
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp file_exists?(path) do
    case File.stat(path) do
      {:ok, %File.Stat{type: :regular}} -> true
      _ -> false
    end
  end

  defp current_os do
    case :os.type() do
      {:unix, :darwin} -> :darwin
      {:unix, :linux} -> :linux
      {:win32, _} -> :win32
      _ -> :unknown
    end
  end
end
