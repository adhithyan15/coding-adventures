defmodule BuildTool.Plan do
  @moduledoc """
  Serialization and deserialization for build plans.

  ## Why build plans?

  A build plan captures the results of the build tool's discovery,
  dependency resolution, and change detection steps as a JSON file.
  This enables CI to compute the plan once in a fast "detect" job and
  share it across build jobs on multiple platforms — eliminating
  redundant computation.

  Consider a CI pipeline with three stages:

      ┌──────────┐    ┌──────────┐    ┌──────────┐
      │  detect   │───>│  build   │───>│  report  │
      │ (1 job)   │    │ (N jobs)  │    │ (1 job)  │
      └──────────┘    └──────────┘    └──────────┘

  The "detect" job runs the build tool with `--emit-plan`, producing a
  JSON file listing which packages need building. The "build" jobs each
  read this plan with `--plan-file` and execute their slice of the work.
  No job re-runs discovery or git diff — the plan is the single source
  of truth.

  ## Schema versioning

  The plan uses a simple integer version scheme (`schema_version` field).
  Readers MUST reject plans with a version higher than what they support,
  falling back to the normal discovery flow. Writers always stamp the
  current version. This forward-incompatibility rule means:

    - Old tools can read plans from old or same-version tools.
    - Old tools reject plans from newer tools (safe fallback).
    - New tools can read plans from older tools (backward compatible).

  ## Path conventions

  All paths in the plan use forward slashes (`/`) regardless of platform.
  On write, paths are normalized. On read, consumers should convert back
  to platform-native separators if needed.

  ## JSON structure

  The plan is a single JSON object with these top-level keys:

      {
        "schema_version": 1,
        "diff_base": "origin/main",
        "force": false,
        "affected_packages": ["python/logic-gates", "go/directed-graph"],
        "packages": [...],
        "dependency_edges": [["python/logic-gates", "python/arithmetic"]],
        "languages_needed": {"python": true, "go": true}
      }

  See `code/specs/build-plan-v1.md` for the full specification.

  This module is a direct port of the Go implementation at
  `code/programs/go/build-tool/internal/plan/plan.go`.
  """

  # ---------------------------------------------------------------------------
  # Schema version
  # ---------------------------------------------------------------------------
  #
  # This constant is the version that this implementation reads and writes.
  # Plans with a higher version are rejected. Bump this when the plan
  # format changes in a backward-incompatible way.

  @current_schema_version 1

  @doc "Returns the current schema version supported by this implementation."
  def current_schema_version, do: @current_schema_version

  # ---------------------------------------------------------------------------
  # Struct definition
  # ---------------------------------------------------------------------------
  #
  # The BuildPlan struct mirrors the Go BuildPlan type field-for-field.
  # Each field is documented inline.

  defstruct [
    # Integer identifying the plan format. Readers MUST reject plans
    # with a version higher than @current_schema_version.
    schema_version: @current_schema_version,

    # Git ref used for change detection (informational).
    diff_base: "origin/main",

    # Whether --force was set. When true, all packages should be rebuilt.
    force: false,

    # List of qualified package names that need building.
    # Semantics:
    #   nil   → rebuild all (force mode or git diff unavailable)
    #   []    → nothing changed, build nothing
    #   [a,b] → only these packages need building
    affected_packages: nil,

    # ALL discovered packages, not just affected ones.
    # The executor needs the full list for dep-skipped detection.
    packages: [],

    # Directed edges [from, to] where from→to means
    # "to depends on from" (from must be built before to).
    dependency_edges: [],

    # Map of language name → boolean indicating whether that language's
    # toolchain is needed for this build.
    languages_needed: %{}
  ]

  # ---------------------------------------------------------------------------
  # PackageEntry
  # ---------------------------------------------------------------------------
  #
  # A single package in the build plan. Mirrors Go's PackageEntry struct.

  defmodule PackageEntry do
    @moduledoc """
    A single package entry in a serialized build plan.

    ## Fields

      - `name` — qualified package name: `"language/package-name"`
      - `rel_path` — repo-root-relative path, always forward slashes
      - `language` — the package's programming language
      - `build_commands` — shell commands to execute for building/testing
      - `is_starlark` — whether the BUILD file uses Starlark syntax
      - `declared_srcs` — glob patterns from the Starlark srcs field
      - `declared_deps` — qualified names from the Starlark deps field
    """
    defstruct name: "",
              rel_path: "",
              language: "",
              build_commands: [],
              is_starlark: false,
              declared_srcs: [],
              declared_deps: []
  end

  # ---------------------------------------------------------------------------
  # Write
  # ---------------------------------------------------------------------------

  @doc """
  Serializes a build plan to a JSON file at the given path.

  Always stamps `schema_version` to the current version before writing.
  Uses atomic write (write to temp file, then rename) to avoid partial
  writes on crash.

  ## Parameters

    - `plan` — a `%BuildTool.Plan{}` struct
    - `path` — file path to write to

  ## Returns

    - `:ok` on success
    - `{:error, reason}` on failure

  ## Example

      plan = %BuildTool.Plan{
        diff_base: "origin/main",
        force: false,
        affected_packages: ["python/logic-gates"],
        packages: [%BuildTool.Plan.PackageEntry{name: "python/logic-gates", ...}],
        dependency_edges: [],
        languages_needed: %{"python" => true}
      }
      :ok = BuildTool.Plan.write_plan(plan, "/tmp/build-plan.json")
  """
  def write_plan(plan, path) do
    # Always stamp current schema version.
    plan = %{plan | schema_version: @current_schema_version}

    json_map = %{
      "schema_version" => plan.schema_version,
      "diff_base" => plan.diff_base,
      "force" => plan.force,
      "affected_packages" => plan.affected_packages,
      "packages" => Enum.map(plan.packages, &package_entry_to_map/1),
      "dependency_edges" => plan.dependency_edges,
      "languages_needed" => plan.languages_needed
    }

    case Jason.encode(json_map, pretty: true) do
      {:ok, data} ->
        # Atomic write: write to temp file, then rename.
        tmp_path = path <> ".tmp"

        with :ok <- File.write(tmp_path, data),
             :ok <- File.rename(tmp_path, path) do
          :ok
        else
          {:error, reason} ->
            # Clean up temp file on failure.
            File.rm(tmp_path)
            {:error, reason}
        end

      {:error, reason} ->
        {:error, {:json_encode, reason}}
    end
  end

  # ---------------------------------------------------------------------------
  # Read
  # ---------------------------------------------------------------------------

  @doc """
  Deserializes a build plan from a JSON file.

  Returns an error if the file is missing, unparseable, or has a
  `schema_version` higher than what this implementation supports.

  ## Parameters

    - `path` — file path to read from

  ## Returns

    - `{:ok, %BuildTool.Plan{}}` on success
    - `{:error, reason}` on failure

  ## Example

      {:ok, plan} = BuildTool.Plan.read_plan("/tmp/build-plan.json")
      plan.affected_packages  #=> ["python/logic-gates"]
  """
  def read_plan(path) do
    with {:ok, data} <- File.read(path),
         {:ok, json} <- Jason.decode(data) do
      version = Map.get(json, "schema_version", 0)

      if version > @current_schema_version do
        {:error,
         "unsupported build plan version #{version} " <>
           "(this tool supports up to #{@current_schema_version})"}
      else
        plan = %__MODULE__{
          schema_version: version,
          diff_base: Map.get(json, "diff_base", ""),
          force: Map.get(json, "force", false),
          affected_packages: Map.get(json, "affected_packages"),
          packages: parse_packages(Map.get(json, "packages", [])),
          dependency_edges: parse_edges(Map.get(json, "dependency_edges", [])),
          languages_needed: Map.get(json, "languages_needed", %{})
        }

        {:ok, plan}
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Serialization helpers
  # ---------------------------------------------------------------------------

  defp package_entry_to_map(%PackageEntry{} = entry) do
    map = %{
      "name" => entry.name,
      "rel_path" => entry.rel_path,
      "language" => entry.language,
      "build_commands" => entry.build_commands,
      "is_starlark" => entry.is_starlark
    }

    # Only include declared_srcs and declared_deps if non-empty,
    # matching Go's `omitempty` behavior.
    map =
      if entry.declared_srcs != [] do
        Map.put(map, "declared_srcs", entry.declared_srcs)
      else
        map
      end

    if entry.declared_deps != [] do
      Map.put(map, "declared_deps", entry.declared_deps)
    else
      map
    end
  end

  # ---------------------------------------------------------------------------
  # Deserialization helpers
  # ---------------------------------------------------------------------------

  defp parse_packages(raw) when is_list(raw) do
    Enum.map(raw, fn entry when is_map(entry) ->
      %PackageEntry{
        name: Map.get(entry, "name", ""),
        rel_path: Map.get(entry, "rel_path", ""),
        language: Map.get(entry, "language", ""),
        build_commands: Map.get(entry, "build_commands", []),
        is_starlark: Map.get(entry, "is_starlark", false),
        declared_srcs: Map.get(entry, "declared_srcs", []),
        declared_deps: Map.get(entry, "declared_deps", [])
      }
    end)
  end

  defp parse_packages(_), do: []

  defp parse_edges(raw) when is_list(raw) do
    Enum.map(raw, fn
      [from, to] when is_binary(from) and is_binary(to) -> [from, to]
      _ -> nil
    end)
    |> Enum.reject(&is_nil/1)
  end

  defp parse_edges(_), do: []
end
