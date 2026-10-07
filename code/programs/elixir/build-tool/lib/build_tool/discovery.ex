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

  We infer a package's language from its directory path. Only the exact
  bucket immediately below "packages" or "programs" counts as a language.
  Package names use "{language}/{dirname}"; programs preserve an additional
  "programs/" identity segment.

  ## The Package struct

  Each discovered package is represented as a map with four fields:

      %{
        name: "python/logic-gates",       # qualified name
        path: "/repo/code/packages/...",   # absolute path on disk
        build_commands: ["python -m pip install", "pytest"],  # lines from BUILD
        language: "python"                 # inferred language
      }
  """

  defmodule DuplicatePackageIdentityError do
    @moduledoc """
    A stable discovery error for two physical roots with one graph identity.

    The paths are relative to the configured code root's parent, never host
    checkout paths. The CLI catches only this typed error and returns status 2.
    """

    defexception [:package, :paths, :message, code: "DUPLICATE_PACKAGE_IDENTITY"]
  end

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
               "dist-newstyle",
               "build",
               "target",
               ".claude",
               "specs",
               ".dart_tool",
               ".build",
               ".gradle",
               "gradle-build",
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
    "dotnet",
    "ocaml",
    "c",
    "cpp",
    "dart",
    "java",
    "kotlin",
    "mosaic",
    "starlark",
    "twig"
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
    discover_packages(root, current_os())
  end

  @doc """
  Discovers packages using an explicit platform selector. The native entry
  point above delegates here; fixture consumers can exercise each platform
  without changing the host OS or the directory walk.
  """
  def discover_packages(root, os) do
    packages =
      root
      |> walk_dirs([], os)
      |> Enum.sort_by(&{&1.name, &1.path})

    # Discovery must not silently choose one of two roots with the same graph
    # identity. Group after sorting so both the first reported collision and
    # the diagnostic's path order are independent of filesystem walk order.
    duplicate =
      packages
      |> Enum.chunk_by(& &1.name)
      |> Enum.find(&(length(&1) > 1))

    case duplicate do
      nil ->
        packages

      group ->
        name = hd(group).name

        paths =
          group
          |> Enum.map(&repository_package_path(root, &1.path))
          |> Enum.sort()

        raise DuplicatePackageIdentityError,
          package: name,
          paths: paths,
          message: "DUPLICATE_PACKAGE_IDENTITY: package=#{name} paths=#{Enum.join(paths, ",")}"
    end
  end

  # A root supplied as /checkout/code yields code/packages/... regardless of
  # checkout location. This representation is diagnostic data, not a host path.
  defp repository_package_path(root, package_path) do
    Path.join(Path.basename(root), Path.relative_to(package_path, root))
    |> String.replace("\\", "/")
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

  The exact component after the last `packages` or `programs` boundary is
  the sole language candidate. A later `go` in `packages/custom/go` does
  not turn the unknown `custom` bucket into Go.

  ## Example

      iex> BuildTool.Discovery.infer_language("/repo/code/packages/python/logic-gates")
      "python"
      iex> BuildTool.Discovery.infer_language("/some/random/path")
      "unknown"
  """
  def infer_language(path) do
    case package_boundary(path) do
      {_kind, bucket} when bucket in @known_languages -> bucket
      _ -> "unknown"
    end
  end

  @doc """
  Builds a qualified package name like "python/logic-gates" from the
  language and the directory's basename. Program roots keep `programs/`.

  ## Example

      iex> BuildTool.Discovery.infer_package_name("/repo/code/packages/python/logic-gates", "python")
      "python/logic-gates"
  """
  def infer_package_name(path, language) do
    case package_boundary(path) do
      {"programs", _bucket} -> language <> "/programs/" <> Path.basename(path)
      _ -> language <> "/" <> Path.basename(path)
    end
  end

  # Search boundaries from the root and retain the last complete pair. This
  # mirrors the canonical path-boundary rule while refusing a language word
  # from a later basename or an unrelated parent directory.
  defp package_boundary(path) do
    path
    |> String.replace("\\", "/")
    |> String.split("/")
    |> Enum.chunk_every(2, 1, :discard)
    |> Enum.reduce(nil, fn
      [kind, bucket], _previous when kind in ["packages", "programs"] -> {kind, bucket}
      _pair, previous -> previous
    end)
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
    3. `BUILD_windows` on Windows
    4. `BUILD_mac_and_linux` on macOS or Linux
    5. `BUILD` (cross-platform fallback)
    6. `nil` if the canonical BUILD file is absent

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
    canonical_file = Path.join(directory, "BUILD")

    # A variant is only an alternate recipe for an established package. Test
    # exact canonical membership before considering host-specific overrides,
    # or the same tree becomes a different package graph on each platform.
    if not file_exists?(canonical_file) do
      nil
    else
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

        # Step 3: Fall back to the membership-establishing canonical BUILD.
        true ->
          canonical_file
      end
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

  defp walk_dirs(directory, packages, os) do
    dir_name = Path.basename(directory)

    if MapSet.member?(@skip_dirs, dir_name) do
      packages
    else
      case get_build_file_for_platform(directory, os) do
        nil ->
          # Not a package — list all subdirectories and recurse into each one.
          case File.ls(directory) do
            {:ok, entries} ->
              entries
              |> Enum.sort()
              |> Enum.reduce(packages, fn entry, acc ->
                subdir = Path.join(directory, entry)

                if File.dir?(subdir) do
                  walk_dirs(subdir, acc, os)
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

          build_content =
            case File.read(build_file) do
              {:ok, content} -> content
              {:error, _reason} -> ""
            end

          language = infer_language(directory)
          name = infer_package_name(directory, language)

          package = %{
            name: name,
            path: directory,
            build_commands: commands,
            build_content: build_content,
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
    # File.stat alone accepts a wrong-case basename on case-insensitive hosts.
    # Check the directory entry before stat so package membership and override
    # selection use the same exact BUILD names on every platform.
    with {:ok, entries} <- File.ls(Path.dirname(path)),
         true <- Enum.member?(entries, Path.basename(path)),
         {:ok, %File.Stat{type: :regular}} <- File.stat(path) do
      true
    else
      _ -> false
    end
  end

  defp current_os do
    case :os.type() do
      {:unix, :darwin} -> :darwin
      {:unix, :linux} -> :linux
      {:win32, _} -> :windows
      _ -> :unknown
    end
  end
end
