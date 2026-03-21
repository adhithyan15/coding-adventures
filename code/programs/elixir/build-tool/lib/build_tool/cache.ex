defmodule BuildTool.Cache do
  @moduledoc """
  Manages a JSON-based build cache file (`.build-cache.json`) that records
  the state of each package after its last build.

  ## Why caching?

  Without caching, every "build" would rebuild every package — even those
  whose source files haven't changed. This is wasteful for large monorepos.
  The cache records the SHA256 hash of each package's source files and
  dependencies at build time. On the next build, we compare current hashes
  against cached hashes to determine which packages actually need rebuilding.

  ## OTP pattern: Agent

  This module uses Elixir's `Agent` — a simple wrapper around a GenServer
  that holds state in a single process. The Agent pattern is ideal here
  because:

    - The cache is a shared mutable map that multiple build processes
      read from and write to.
    - Agent provides concurrent-safe reads and writes without manual locking.
    - The API is simple: `get`, `update`, `get_and_update`.

  In Go, the equivalent is a `sync.Mutex`-protected struct. In Elixir,
  the Agent process's mailbox serializes all access automatically.

  ## Cache format

  The cache file is a JSON object mapping package names to cache entries:

      {
        "python/logic-gates": {
          "package_hash": "abc123...",
          "deps_hash": "def456...",
          "last_built": "2024-01-15T10:30:00Z",
          "status": "success"
        }
      }

  ## Atomic writes

  To prevent corruption if the process is interrupted mid-write, we write
  to a temporary file first, then atomically rename it. On POSIX systems,
  `File.rename/2` is atomic within the same filesystem.
  """

  use Agent

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Starts a new Cache agent with an empty entries map.

  Returns `{:ok, pid}` on success.

  ## Example

      iex> {:ok, cache} = BuildTool.Cache.start_link()
      iex> is_pid(cache)
      true
  """
  def start_link do
    Agent.start_link(fn -> %{} end)
  end

  @doc """
  Loads cache entries from a JSON file.

  If the file doesn't exist or is malformed, we start with an empty cache —
  no error is raised. A missing cache simply means everything gets rebuilt,
  which is the safe default.

  ## Parameters

    - `agent` — the cache agent pid
    - `path` — absolute path to the cache JSON file

  ## Example

      iex> BuildTool.Cache.load(cache, "/repo/.build-cache.json")
      :ok
  """
  def load(agent, path) do
    entries =
      case File.read(path) do
        {:ok, data} ->
          case Jason.decode(data) do
            {:ok, map} when is_map(map) -> map
            _ -> %{}
          end

        {:error, _} ->
          %{}
      end

    Agent.update(agent, fn _state -> entries end)
  end

  @doc """
  Saves cache entries to a JSON file with atomic write.

  The atomicity guarantee: we write to `path.tmp` first, then rename.
  If the process crashes during the write, the original cache file is
  untouched. If it crashes during the rename, the temporary file may
  be left behind, but no data is lost.

  Returns `:ok` on success, `{:error, reason}` on failure.

  ## Example

      iex> BuildTool.Cache.save(cache, "/repo/.build-cache.json")
      :ok
  """
  def save(agent, path) do
    entries = Agent.get(agent, & &1)

    # Sort entries by key for deterministic output.
    sorted =
      entries
      |> Enum.sort_by(fn {key, _val} -> key end)
      |> Enum.into(%{})

    case Jason.encode(sorted, pretty: true) do
      {:ok, data} ->
        tmp_path = path <> ".tmp"

        with :ok <- File.write(tmp_path, data <> "\n") do
          File.rename(tmp_path, path)
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Determines if a package needs rebuilding.

  A package needs rebuilding if any of these conditions hold:

    1. It's not in the cache (never built before).
    2. Its source hash changed (files were modified).
    3. Its dependency hash changed (a dependency was modified).
    4. Its last build failed.

  This is the decision function at the heart of incremental builds.

  ## Parameters

    - `agent` — the cache agent pid
    - `name` — the package name
    - `pkg_hash` — current SHA256 hash of the package's source files
    - `dep_hash` — current SHA256 hash of the package's dependency hashes

  ## Example

      iex> BuildTool.Cache.needs_build?(cache, "python/logic-gates", "abc123", "def456")
      true
  """
  def needs_build?(agent, name, pkg_hash, dep_hash) do
    Agent.get(agent, fn entries ->
      case Map.get(entries, name) do
        nil ->
          true

        entry ->
          entry["status"] == "failed" or
            entry["package_hash"] != pkg_hash or
            entry["deps_hash"] != dep_hash
      end
    end)
  end

  @doc """
  Records a build result in the cache.

  ## Parameters

    - `agent` — the cache agent pid
    - `name` — the package name
    - `pkg_hash` — SHA256 hash of source files at build time
    - `dep_hash` — SHA256 hash of dependency hashes at build time
    - `status` — `"success"` or `"failed"`

  ## Example

      iex> BuildTool.Cache.record(cache, "python/logic-gates", "abc", "def", "success")
      :ok
  """
  def record(agent, name, pkg_hash, dep_hash, status) do
    now = DateTime.utc_now() |> DateTime.to_iso8601()

    Agent.update(agent, fn entries ->
      Map.put(entries, name, %{
        "package_hash" => pkg_hash,
        "deps_hash" => dep_hash,
        "last_built" => now,
        "status" => status
      })
    end)
  end

  @doc """
  Returns a copy of all cache entries (for inspection/testing).
  """
  def entries(agent) do
    Agent.get(agent, & &1)
  end
end
