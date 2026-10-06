defmodule BuildTool.Hasher do
  @moduledoc """
  Computes SHA256 hashes for package source files and their dependencies.

  ## Why hashing?

  The core of incremental builds is change detection. If nothing changed
  in a package's source files, there is no reason to rebuild it. We detect
  changes by computing a SHA256 hash of all relevant source files and
  comparing it against the cached hash from the last build.

  ## How hashing works

  Package source hashing follows the language-neutral Hashing v1 stream:

    1. Select package-local inputs with the checked, immutable source registry.
    2. Sort normalized repository-relative UTF-8 paths by raw bytes.
    3. Frame each path and exact content with unsigned 64-bit big-endian lengths.
    4. SHA256-hash the concatenated frames, including an empty stream for an
       empty source set.

  Unlike concatenated per-file digests, these frames distinguish both path and
  file boundaries. No host timestamp, absolute path, or locale enters the hash.

  ## Dependency hashing

  A package should be rebuilt if any of its transitive dependencies changed.
  `hash_deps/3` takes a package's dependency information and produces a single
  hash representing the state of all its dependencies.

  ## Elixir implementation note

  We use `:crypto.hash(:sha256, data)` from Erlang's built-in crypto module.
  This is the same OpenSSL-backed implementation that powers BEAM's SSL/TLS
  stack — fast, well-tested, and available without any external dependencies.
  """

  alias BuildTool.{DirectedGraph, SourceInputRegistry}

  @max_source_candidates 100_000
  @max_source_depth 64
  @max_source_bytes 268_435_456
  @read_chunk_bytes 65_536

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
    language = package.language
    entry = SourceInputRegistry.language_entry!(language)

    package_root =
      SourceInputRegistry.validate_package_root!(Map.get(package, :package_root), language, entry)

    declared_srcs = Map.get(package, :declared_srcs, [])

    requested_mode =
      if Map.get(package, :is_starlark, false) or declared_srcs != [],
        do: "declared_sources",
        else: "extension"

    {mode, compiled_srcs} = SourceInputRegistry.compile_mode!(requested_mode, declared_srcs)
    files = collect_source_files(package.path, entry, package_root, mode, compiled_srcs)

    files
    |> Enum.reduce(:crypto.hash_init(:sha256), fn {relative, absolute, stat}, hash ->
      path = if package_root == nil, do: relative, else: package_root <> "/" <> relative

      hash =
        :crypto.hash_update(hash, [
          <<byte_size(path)::unsigned-big-integer-size(64)>>,
          path,
          <<stat.size::unsigned-big-integer-size(64)>>
        ])

      hash_open_file(hash, absolute, stat)
    end)
    |> :crypto.hash_final()
    |> Base.encode16(case: :lower)
  end

  defdelegate source_input_registry(), to: SourceInputRegistry, as: :registry
  defdelegate source_input_registry_snapshot(), to: SourceInputRegistry, as: :snapshot_bytes
  defdelegate source_input_registry_digest(), to: SourceInputRegistry, as: :digest
  defdelegate collect_candidates(options), to: SourceInputRegistry

  defp hash_open_file(hash, absolute, expected) do
    {:ok, device} = File.open(absolute, [:read, :raw, :binary])

    try do
      verify_open_file!(device, expected)
      {hashed, size} = hash_file_chunks(device, hash, 0)

      if size != expected.size do
        raise ArgumentError, "source file size changed while hashing"
      end

      verify_open_file!(device, expected)
      hashed
    after
      File.close(device)
    end
  end

  defp hash_file_chunks(device, hash, size) do
    case IO.binread(device, @read_chunk_bytes) do
      :eof ->
        {hash, size}

      {:error, reason} ->
        raise File.Error, reason: reason, action: "read", path: "source"

      bytes ->
        next_size = size + byte_size(bytes)

        if next_size > @max_source_bytes do
          raise ArgumentError, "source byte limit exceeded"
        end

        hash_file_chunks(device, :crypto.hash_update(hash, bytes), next_size)
    end
  end

  defp verify_open_file!(device, expected) do
    {:ok, info} = :file.read_file_info(device)
    actual = File.Stat.from_record(info)

    unless actual.type == :regular and actual.size == expected.size and
             actual.major_device == expected.major_device and
             actual.minor_device == expected.minor_device and actual.inode == expected.inode do
      raise ArgumentError, "source file identity changed while hashing"
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

  # Discover immediate children before reading bytes. Exact generated names
  # are pruned before descent, while `File.lstat!` keeps link-shaped children
  # inert. Stable retained-handle proof is separately owned; this stage only
  # removes the old follow-links collector and its silent read sentinel.
  defp collect_source_files(root, entry, package_root, mode, compiled_srcs) do
    {files, _count, _bytes} =
      walk_files(root, "", [], 0, 0, 0, entry, package_root, mode, compiled_srcs)

    Enum.sort_by(files, fn {relative, _absolute, _stat} -> relative end)
  end

  defp walk_files(
         dir,
         prefix,
         files,
         count,
         bytes,
         depth,
         entry,
         package_root,
         mode,
         compiled_srcs
       ) do
    if depth > @max_source_depth do
      raise ArgumentError, "source depth limit exceeded"
    end

    names = File.ls!(dir)

    if count + length(names) > @max_source_candidates do
      raise ArgumentError, "source candidate limit exceeded"
    end

    names
    |> Enum.sort()
    |> Enum.reduce({files, count, bytes}, fn name, {found, seen, total_bytes} ->
      relative = if prefix == "", do: name, else: prefix <> "/" <> name
      absolute = Path.join(dir, name)
      next_count = seen + 1
      stat = File.lstat!(absolute)

      case stat.type do
        :directory ->
          if name in SourceInputRegistry.registry()["universal_inputs"][
               "generated_directory_components"
             ] do
            {found, next_count, total_bytes}
          else
            walk_files(
              absolute,
              relative,
              found,
              next_count,
              total_bytes,
              depth + 1,
              entry,
              package_root,
              mode,
              compiled_srcs
            )
          end

        :regular ->
          if SourceInputRegistry.selected_path?(
               entry,
               package_root,
               mode,
               compiled_srcs,
               relative
             ) do
            next_bytes = total_bytes + stat.size

            if next_bytes > @max_source_bytes do
              raise ArgumentError, "source byte limit exceeded"
            end

            {[{relative, absolute, stat} | found], next_count, next_bytes}
          else
            {found, next_count, total_bytes}
          end

        _ ->
          {found, next_count, total_bytes}
      end
    end)
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
