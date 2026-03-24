defmodule BuildTool.Hasher do
  @moduledoc """
  Computes SHA256 hashes for package source files and their dependencies.

  ## Why hashing?

  The core of incremental builds is change detection. If nothing changed
  in a package's source files, there is no reason to rebuild it. We detect
  changes by computing a SHA256 hash of all relevant source files and
  comparing it against the cached hash from the last build.

  ## How hashing works

  The hashing algorithm is deterministic — given the same files with the
  same contents, it always produces the same hash. Here is the procedure:

    1. Collect all source files in the package directory, filtered by the
       language's relevant extensions. Always include BUILD files.
    2. Sort the file list lexicographically by relative path. This ensures
       that file ordering does not affect the hash.
    3. SHA256-hash each file's contents individually.
    4. Concatenate all individual hashes into one string.
    5. SHA256-hash that concatenated string to produce the final hash.

  This two-level hashing means:
    - Reordering files doesn't change the hash (we sort first).
    - Adding or removing a file changes the hash.
    - Modifying any file's contents changes the hash.

  ## Dependency hashing

  A package should be rebuilt if any of its transitive dependencies changed.
  `hash_deps/3` takes a package's dependency information and produces a single
  hash representing the state of all its dependencies.

  ## Elixir implementation note

  We use `:crypto.hash(:sha256, data)` from Erlang's built-in crypto module.
  This is the same OpenSSL-backed implementation that powers BEAM's SSL/TLS
  stack — fast, well-tested, and available without any external dependencies.
  """

  alias BuildTool.{DirectedGraph, GlobMatch}

  # ---------------------------------------------------------------------------
  # Source file extensions by language
  # ---------------------------------------------------------------------------
  #
  # Each language has a set of file extensions that matter for change detection.
  # If any file with these extensions changes, the package needs rebuilding.
  # Extensions that don't affect build output (like .md, .txt) are excluded
  # to avoid unnecessary rebuilds.

  @source_extensions %{
    "python" => MapSet.new([".py", ".toml", ".cfg"]),
    "ruby" => MapSet.new([".rb", ".gemspec"]),
    "go" => MapSet.new([".go"]),
    "rust" => MapSet.new([".rs", ".toml"]),
    "typescript" => MapSet.new([".ts", ".tsx", ".js", ".jsx", ".json"]),
    "elixir" => MapSet.new([".ex", ".exs"]),
    "perl" => MapSet.new([".pl", ".pm", ".t", ".xs"])
  }

  # ---------------------------------------------------------------------------
  # Special filenames by language
  # ---------------------------------------------------------------------------
  #
  # Certain filenames should always be included regardless of their extension.
  # These are configuration files that affect build behavior.

  @special_filenames %{
    "python" => MapSet.new(),
    "ruby" => MapSet.new(["Gemfile", "Rakefile"]),
    "go" => MapSet.new(["go.mod", "go.sum"]),
    "rust" => MapSet.new(["Cargo.lock"]),
    "typescript" => MapSet.new(["package-lock.json", "tsconfig.json"]),
    "elixir" => MapSet.new(["mix.lock"]),
    "perl" => MapSet.new(["Makefile.PL", "Build.PL", "cpanfile", "MANIFEST", "META.json", "META.yml"])
  }

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Computes a SHA256 hash representing all source files in the package.

  The hash changes if any source file is added, removed, or modified.
  If the package has no source files, we hash the empty string for
  consistency — every package gets a hash, even empty ones.

  ## Parameters

    - `package` — a package map with `:path` and `:language` keys

  ## Example

      iex> hash = BuildTool.Hasher.hash_package(%{path: "/repo/pkg", language: "python"})
      iex> String.length(hash)
      64  # SHA256 hex digest is always 64 characters
  """
  def hash_package(package) do
    # When a package has declared_srcs (from a Starlark BUILD file), we hash
    # ONLY those declared files — this is strict mode. When declared_srcs is
    # empty or absent (shell BUILD files), we fall back to extension-based
    # collection. This mirrors the Go implementation's HashPackage function.
    declared_srcs = Map.get(package, :declared_srcs, [])

    files =
      if declared_srcs != [] do
        resolve_declared_srcs(package, declared_srcs)
      else
        collect_source_files(package)
      end

    if files == [] do
      # No source files — hash the empty string.
      hash_string("")
    else
      # Hash each file individually, concatenate all hashes, hash again.
      # This two-level scheme means the final hash changes if any file
      # changes, is added, or is removed.
      file_hashes =
        Enum.map(files, fn f ->
          case hash_file(f) do
            {:ok, h} -> h
            {:error, _} -> "error-reading-file"
          end
        end)

      combined = Enum.join(file_hashes, "")
      hash_string(combined)
    end
  end

  @doc """
  Computes a SHA256 hash of all transitive dependency hashes.

  If any transitive dependency's source files changed, this hash will
  change too, triggering a rebuild of the dependent package. This is
  how we propagate changes through the dependency tree.

  In our graph convention:
    - Edge A -> B means "B depends on A"
    - So B's dependencies are found by following reverse edges (predecessors)

  We walk backwards from the package through all transitive predecessors
  and hash their package hashes together.

  ## Parameters

    - `package_name` — the name of the package
    - `graph` — the dependency graph
    - `package_hashes` — map of package name to its hash

  ## Example

      iex> hash = BuildTool.Hasher.hash_deps("python/arithmetic", graph, hashes)
      iex> String.length(hash)
      64
  """
  def hash_deps(package_name, graph, package_hashes) do
    if not DirectedGraph.has_node?(graph, package_name) do
      hash_string("")
    else
      # Collect all transitive dependencies (packages this one depends on).
      transitive_deps = DirectedGraph.transitive_predecessors(graph, package_name)

      if MapSet.size(transitive_deps) == 0 do
        hash_string("")
      else
        # Sort for determinism, concatenate hashes, hash again.
        combined =
          transitive_deps
          |> MapSet.to_list()
          |> Enum.sort()
          |> Enum.map(fn dep -> Map.get(package_hashes, dep, "") end)
          |> Enum.join("")

        hash_string(combined)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Declared source resolution (Starlark strict mode)
  # ---------------------------------------------------------------------------
  #
  # When a package has declared_srcs from a Starlark BUILD file, we resolve
  # those glob patterns into actual file paths. This is "strict mode" — only
  # files matching the declared patterns (plus BUILD files) are included.
  #
  # The algorithm:
  #   1. Always include BUILD files (the build definition itself).
  #   2. Walk the package directory recursively.
  #   3. For each file, compute its path relative to the package root.
  #   4. Check if it matches any declared src pattern using GlobMatch.
  #   5. Sort by relative path and deduplicate.
  #
  # We use walk_files + GlobMatch instead of Path.wildcard because
  # Path.wildcard does NOT support ** the way build systems expect.

  defp resolve_declared_srcs(package, declared_srcs) do
    # Step 1: Always include BUILD files.
    build_files =
      ["BUILD", "BUILD_mac", "BUILD_linux", "BUILD_windows"]
      |> Enum.map(&Path.join(package.path, &1))
      |> Enum.filter(&file_exists?/1)

    # Step 2: Walk the package directory and match against declared patterns.
    matched_files =
      package.path
      |> walk_files([])
      |> Enum.filter(fn path ->
        rel = Path.relative_to(path, package.path)
        # Normalize to forward slashes for pattern matching.
        rel = String.replace(rel, "\\", "/")

        Enum.any?(declared_srcs, fn pattern ->
          GlobMatch.match_path?(pattern, rel)
        end)
      end)

    # Step 3: Combine, deduplicate, and sort by relative path.
    (build_files ++ matched_files)
    |> Enum.uniq()
    |> Enum.sort_by(fn path ->
      Path.relative_to(path, package.path)
    end)
  end

  defp file_exists?(path) do
    case File.stat(path) do
      {:ok, %File.Stat{type: :regular}} -> true
      _ -> false
    end
  end

  # ---------------------------------------------------------------------------
  # File collection (legacy extension-based mode)
  # ---------------------------------------------------------------------------
  #
  # collectSourceFiles walks the package directory and returns all source
  # files relevant to the package's language. Files are sorted by their
  # relative path for deterministic hashing.
  #
  # The collection rules:
  #   - BUILD, BUILD_mac, BUILD_linux are always included.
  #   - Files matching the language's extensions are included.
  #   - Special filenames (go.mod, Gemfile, etc.) are included.
  #   - Everything else is ignored.

  defp collect_source_files(package) do
    extensions = Map.get(@source_extensions, package.language, MapSet.new())
    specials = Map.get(@special_filenames, package.language, MapSet.new())

    package.path
    |> walk_files([])
    |> Enum.filter(fn path ->
      name = Path.basename(path)
      ext = Path.extname(name)

      # Always include BUILD files — they define how the package is built.
      name in ["BUILD", "BUILD_mac", "BUILD_linux"] or
        MapSet.member?(extensions, ext) or
        MapSet.member?(specials, name)
    end)
    |> Enum.sort_by(fn path ->
      # Sort by relative path for determinism. Two developers with different
      # absolute paths to the repo should get the same hash.
      Path.relative_to(path, package.path)
    end)
  end

  # Recursively walks a directory collecting all regular files.
  defp walk_files(dir, acc) do
    case File.ls(dir) do
      {:ok, entries} ->
        Enum.reduce(entries, acc, fn entry, files ->
          full_path = Path.join(dir, entry)

          case File.stat(full_path) do
            {:ok, %File.Stat{type: :regular}} ->
              [full_path | files]

            {:ok, %File.Stat{type: :directory}} ->
              walk_files(full_path, files)

            _ ->
              files
          end
        end)

      {:error, _} ->
        acc
    end
  end

  # ---------------------------------------------------------------------------
  # Hashing helpers
  # ---------------------------------------------------------------------------

  @doc """
  Computes the SHA256 hex digest of a single file's contents.

  We read the entire file into memory. For very large files, a streaming
  approach would be more memory-efficient, but packages in this monorepo
  are small enough that this is not a concern.

  ## Example

      iex> {:ok, hash} = BuildTool.Hasher.hash_file("/path/to/file.py")
      iex> String.length(hash)
      64
  """
  def hash_file(path) do
    case File.read(path) do
      {:ok, data} ->
        hash = :crypto.hash(:sha256, data) |> Base.encode16(case: :lower)
        {:ok, hash}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Computes the SHA256 hex digest of a string.
  """
  def hash_string(str) do
    :crypto.hash(:sha256, str) |> Base.encode16(case: :lower)
  end
end
