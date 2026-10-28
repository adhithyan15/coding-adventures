defmodule BuildTool.GraphDiff.GraphInput do
  @moduledoc """
  Already-materialized input for the process-free graph operation.

  Every edge is ordered as `[prerequisite, dependent]`.
  """

  @enforce_keys [:packages, :edges]
  defstruct [:packages, :edges]

  @type t :: %__MODULE__{packages: [String.t()], edges: [[String.t()]]}
end

defmodule BuildTool.GraphDiff.Package do
  @moduledoc "One package declaration for process-free diff selection."

  @enforce_keys [:name, :rel_path, :source_mode]
  defstruct [:name, :rel_path, :source_mode, source_globs: []]

  @type t :: %__MODULE__{
          name: String.t(),
          rel_path: String.t(),
          source_mode: String.t(),
          source_globs: [String.t()]
        }
end

defmodule BuildTool.GraphDiff.DiffInput do
  @moduledoc """
  Already-materialized input for process-free diff selection.

  `boundary` is inert decoded data. Supplying it grants no checkout, Git, or
  filesystem authority to the evaluator.
  """

  @enforce_keys [
    :packages,
    :edges,
    :forced_packages,
    :unknown_path_policy,
    :changed_paths
  ]
  defstruct [
    :packages,
    :edges,
    :forced_packages,
    :unknown_path_policy,
    :changed_paths,
    :boundary_sha256,
    :boundary
  ]

  @type t :: %__MODULE__{
          packages: [BuildTool.GraphDiff.Package.t()],
          edges: [[String.t()]],
          forced_packages: [String.t()],
          unknown_path_policy: String.t(),
          changed_paths: [String.t()],
          boundary_sha256: String.t() | nil,
          boundary: map() | nil
        }
end

defmodule BuildTool.GraphDiff do
  @moduledoc """
  Pure, bounded graph and diff-selection operations.

  The functions in this module accept only already-materialized values. They
  never discover a checkout, invoke Git, open a file, inspect the environment,
  launch a process, consult a clock, use randomness, or access the network.

  A successful graph result contains canonical prerequisite-first levels. Diff
  selection validates the entire request and preflights the operation-wide
  Unicode-scalar match budget before the first glob match. Every failure is a
  stable `{:error, code}` value with no partial graph or package selection.
  """

  alias BuildTool.GlobMatch
  alias BuildTool.GlobMatch.PatternError
  alias BuildTool.GraphDiff.DiffInput
  alias BuildTool.GraphDiff.GraphInput
  alias BuildTool.GraphDiff.Package
  alias BuildTool.TrackedArtifactUnicode17, as: Unicode

  @max_packages 4_096
  @max_edges 16_384
  @max_source_globs 256
  @max_path_scalars 512
  @max_name_scalars 240
  @max_match_work 50_000_000
  @build_fronts MapSet.new([
                  "BUILD",
                  "BUILD_windows",
                  "BUILD_mac",
                  "BUILD_linux",
                  "BUILD_mac_and_linux"
                ])
  @boundary_domain "coding-adventures/build-tool-repository-source-input-boundary/v1\0"
  @boundary_origins MapSet.new([
                      "c",
                      "cpp",
                      "csharp",
                      "dart",
                      "dotnet",
                      "elixir",
                      "fsharp",
                      "go",
                      "haskell",
                      "java",
                      "kotlin",
                      "lua",
                      "mosaic",
                      "ocaml",
                      "perl",
                      "python",
                      "repository",
                      "ruby",
                      "rust",
                      "starlark",
                      "swift",
                      "twig",
                      "typescript",
                      "wasm"
                    ])
  @reserved_names MapSet.new(
                    ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"] ++
                      Enum.flat_map(1..9, fn number -> ["COM#{number}", "LPT#{number}"] end) ++
                      ["COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³"]
                  )

  @typedoc "A stable all-or-error operation result."
  @type result(value) :: {:ok, value} | {:error, String.t()}

  @doc """
  Validates and evaluates one bounded dependency graph.
  """
  @spec evaluate_graph(GraphInput.t()) :: result(map())
  def evaluate_graph(%GraphInput{packages: packages, edges: edges}) do
    with {:ok, graph} <- validate_graph(packages, edges),
         {:ok, levels} <- graph_levels(graph, "GRAPH_CYCLE") do
      {:ok, %{"edges" => graph.edges, "levels" => levels}}
    end
  end

  def evaluate_graph(_input), do: {:error, "GRAPH_INPUT_INVALID"}

  @doc """
  Validates and evaluates one bounded diff-selection snapshot.

  The optional repository source-input boundary must be supplied as decoded,
  inert data together with its exact domain-separated digest.
  """
  @spec evaluate_diff_selection(DiffInput.t()) :: result(map())
  def evaluate_diff_selection(%DiffInput{} = input) do
    with {:ok, packages} <- validate_package_list(input.packages),
         {:ok, graph} <- validate_graph(Enum.map(packages, & &1.name), input.edges),
         {:ok, _levels} <- graph_levels(graph, "DIFF_EDGE_CYCLE"),
         {:ok, packages} <- validate_diff_input(input, packages, graph),
         {:ok, boundary_consumers} <- boundary_reverse_index(input, packages),
         :ok <- preflight_match_work(packages, input.changed_paths) do
      select_diff(input, packages, graph, boundary_consumers)
    end
  end

  def evaluate_diff_selection(_input), do: {:error, "DIFF_INPUT_INVALID"}

  defp validate_package_list(packages)
       when is_list(packages) and length(packages) <= @max_packages do
    if Enum.all?(packages, &match?(%Package{}, &1)) do
      {:ok, packages}
    else
      {:error, "DIFF_PACKAGE_INVALID"}
    end
  end

  defp validate_package_list(_packages), do: {:error, "DIFF_PACKAGE_INVALID"}

  defp validate_graph(packages, edges) do
    cond do
      not is_list(packages) or length(packages) > @max_packages ->
        {:error, "GRAPH_PACKAGE_LIMIT_EXCEEDED"}

      not is_list(edges) or length(edges) > @max_edges ->
        {:error, "GRAPH_EDGE_LIMIT_EXCEEDED"}

      not Enum.all?(packages, &valid_package_name?/1) ->
        {:error, "GRAPH_PACKAGE_INVALID"}

      MapSet.size(MapSet.new(packages)) != length(packages) ->
        {:error, "GRAPH_PACKAGE_DUPLICATE"}

      true ->
        build_graph(packages, edges)
    end
  end

  defp build_graph(packages, edges) do
    names = MapSet.new(packages)

    with :ok <- validate_edges(edges, names) do
      empty = Map.new(packages, &{&1, []})

      {dependents, prerequisites} =
        Enum.reduce(edges, {empty, empty}, fn [prerequisite, dependent], {forward, reverse} ->
          {
            Map.update!(forward, prerequisite, &[dependent | &1]),
            Map.update!(reverse, dependent, &[prerequisite | &1])
          }
        end)

      {:ok,
       %{
         names: names,
         edges: Enum.sort(edges),
         dependents: Map.new(dependents, fn {name, values} -> {name, Enum.sort(values)} end),
         prerequisites: Map.new(prerequisites, fn {name, values} -> {name, Enum.sort(values)} end)
       }}
    end
  end

  defp validate_edges(edges, names) do
    cond do
      not Enum.all?(edges, &valid_edge_shape?/1) ->
        {:error, "GRAPH_EDGE_UNKNOWN"}

      Enum.any?(edges, fn [left, right] ->
        not MapSet.member?(names, left) or not MapSet.member?(names, right)
      end) ->
        {:error, "GRAPH_EDGE_UNKNOWN"}

      Enum.any?(edges, fn [left, right] -> left == right end) ->
        {:error, "GRAPH_EDGE_SELF"}

      MapSet.size(MapSet.new(edges)) != length(edges) ->
        {:error, "GRAPH_EDGE_DUPLICATE"}

      true ->
        :ok
    end
  end

  defp valid_edge_shape?([left, right]), do: is_binary(left) and is_binary(right)
  defp valid_edge_shape?(_edge), do: false

  defp graph_levels(graph, cycle_code) do
    indegrees = Map.new(graph.prerequisites, fn {name, values} -> {name, length(values)} end)

    ready =
      indegrees
      |> Enum.filter(fn {_name, degree} -> degree == 0 end)
      |> Enum.map(&elem(&1, 0))
      |> Enum.sort()

    do_graph_levels(graph, indegrees, ready, [], 0, cycle_code)
  end

  defp do_graph_levels(graph, _indegrees, [], levels, visited, cycle_code) do
    if visited == MapSet.size(graph.names) do
      {:ok, Enum.reverse(levels)}
    else
      {:error, cycle_code}
    end
  end

  defp do_graph_levels(graph, indegrees, ready, levels, visited, cycle_code) do
    {next_indegrees, next_ready} =
      Enum.reduce(ready, {indegrees, []}, fn name, {degrees, newly_ready} ->
        Enum.reduce(graph.dependents[name], {degrees, newly_ready}, fn dependent,
                                                                       {inner_degrees,
                                                                        inner_ready} ->
          degree = inner_degrees[dependent] - 1

          {
            Map.put(inner_degrees, dependent, degree),
            if(degree == 0, do: [dependent | inner_ready], else: inner_ready)
          }
        end)
      end)

    do_graph_levels(
      graph,
      next_indegrees,
      Enum.sort(next_ready),
      [ready | levels],
      visited + length(ready),
      cycle_code
    )
  end

  defp validate_diff_input(input, packages, graph) do
    with {:ok, packages} <- validate_packages(packages),
         :ok <- validate_policy(input.unknown_path_policy),
         :ok <- validate_unique_package_names(input.forced_packages),
         :ok <- validate_changed_paths(input.changed_paths),
         :ok <- validate_forced_packages(input.forced_packages, graph.names) do
      {:ok, packages}
    end
  end

  defp validate_packages(packages) do
    with {:ok, validated} <- validate_each_package(packages, []),
         :ok <- validate_root_identities(validated) do
      {:ok, validated}
    end
  end

  defp validate_each_package([], validated), do: {:ok, Enum.reverse(validated)}

  defp validate_each_package([package | rest], validated) do
    with :ok <- validate_package_path(package.rel_path),
         :ok <- validate_source_mode(package.source_mode),
         {:ok, compiled_globs} <- validate_source_globs(package.source_mode, package.source_globs) do
      validate_each_package(
        rest,
        [
          %{
            name: package.name,
            rel_path: package.rel_path,
            source_mode: package.source_mode,
            source_globs: package.source_globs,
            compiled_globs: compiled_globs
          }
          | validated
        ]
      )
    end
  end

  defp validate_package_path(path) do
    if valid_portable_path?(path), do: :ok, else: {:error, "DIFF_PATH_INVALID"}
  end

  defp validate_source_mode(mode) when mode in ["package_prefix", "strict_globs"], do: :ok
  defp validate_source_mode(_mode), do: {:error, "DIFF_SOURCE_MODE_INVALID"}

  defp validate_source_globs(mode, globs)
       when is_list(globs) and length(globs) <= @max_source_globs do
    cond do
      MapSet.size(MapSet.new(globs)) != length(globs) ->
        {:error, "DIFF_GLOB_INVALID"}

      mode != "strict_globs" and globs != [] ->
        {:error, "DIFF_GLOB_INVALID"}

      not Enum.all?(globs, &valid_portable_glob?/1) ->
        {:error, "DIFF_GLOB_INVALID"}

      true ->
        compile_globs(globs)
    end
  end

  defp validate_source_globs(_mode, _globs), do: {:error, "DIFF_GLOB_INVALID"}

  defp compile_globs(globs) do
    try do
      {:ok, GlobMatch.compile_patterns!(globs)}
    rescue
      PatternError -> {:error, "DIFF_GLOB_INVALID"}
    end
  end

  defp validate_root_identities(packages) do
    identities =
      packages
      |> Enum.map(fn package -> package.rel_path |> Unicode.nfc() |> Unicode.casefold() end)
      |> Enum.sort()

    if identities
       |> Enum.chunk_every(2, 1, :discard)
       |> Enum.any?(fn [left, right] ->
         left == right or String.starts_with?(right, left <> "/")
       end) do
      {:error, "DIFF_PATH_INVALID"}
    else
      :ok
    end
  end

  defp validate_policy(policy) when policy in ["all", "error"], do: :ok
  defp validate_policy(_policy), do: {:error, "DIFF_POLICY_INVALID"}

  defp validate_unique_package_names(values)
       when is_list(values) and length(values) <= @max_packages do
    if Enum.all?(values, &valid_package_name?/1) and
         MapSet.size(MapSet.new(values)) == length(values) do
      :ok
    else
      {:error, "DIFF_FORCED_PACKAGE_INVALID"}
    end
  end

  defp validate_unique_package_names(_values), do: {:error, "DIFF_FORCED_PACKAGE_INVALID"}

  defp validate_changed_paths(paths) when is_list(paths) and length(paths) <= @max_packages do
    if Enum.all?(paths, &valid_portable_path?/1) and
         MapSet.size(MapSet.new(paths)) == length(paths) do
      :ok
    else
      {:error, "DIFF_PATH_INVALID"}
    end
  end

  defp validate_changed_paths(_paths), do: {:error, "DIFF_PATH_INVALID"}

  defp validate_forced_packages(forced, names) do
    if Enum.all?(forced, &MapSet.member?(names, &1)) do
      :ok
    else
      {:error, "DIFF_FORCED_PACKAGE_UNKNOWN"}
    end
  end

  defp boundary_reverse_index(%DiffInput{boundary_sha256: nil, boundary: nil}, _packages),
    do: {:ok, %{}}

  defp boundary_reverse_index(%DiffInput{boundary_sha256: nil}, _packages),
    do: {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"}

  defp boundary_reverse_index(%DiffInput{} = input, packages) do
    with :ok <- validate_digest_shape(input.boundary_sha256),
         {:ok, canonical} <- encode_boundary(input.boundary),
         :ok <- verify_boundary_digest(input.boundary_sha256, canonical),
         :ok <- validate_boundary(input.boundary) do
      {:ok, build_boundary_reverse_index(input.boundary, packages)}
    end
  end

  defp validate_digest_shape(digest) do
    if valid_string?(digest) and Regex.match?(~r/\A[0-9a-f]{64}\z/, digest) do
      :ok
    else
      {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"}
    end
  end

  defp encode_boundary(boundary) when is_map(boundary) do
    try do
      {:ok, boundary |> canonical_json() |> IO.iodata_to_binary()}
    rescue
      _error -> {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"}
    end
  end

  defp encode_boundary(_boundary), do: {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"}

  defp verify_boundary_digest(expected, canonical) do
    framed =
      @boundary_domain <> <<byte_size(canonical)::unsigned-big-integer-size(64)>> <> canonical

    actual = :crypto.hash(:sha256, framed) |> Base.encode16(case: :lower)

    if actual == expected, do: :ok, else: {:error, "DIFF_BOUNDARY_DIGEST_MISMATCH"}
  end

  defp validate_boundary(
         %{
           "schema_version" => 1,
           "language_source_input_registry_sha256" => registry_digest,
           "boundaries" => boundaries
         } = boundary
       )
       when map_size(boundary) == 3 and is_list(boundaries) and length(boundaries) in 1..256 do
    with :ok <- validate_digest_shape_for_boundary(registry_digest),
         true <- Enum.all?(boundaries, &valid_boundary_rule?/1) do
      :ok
    else
      _value -> {:error, "DIFF_BOUNDARY_INVALID"}
    end
  end

  defp validate_boundary(_boundary), do: {:error, "DIFF_BOUNDARY_INVALID"}

  defp validate_digest_shape_for_boundary(value) do
    if valid_string?(value) and Regex.match?(~r/\A[0-9a-f]{64}\z/, value), do: :ok, else: :error
  end

  defp valid_boundary_rule?(
         %{
           "id" => id,
           "input_origin" => origin,
           "applies_to" => applies_to,
           "inputs" => inputs,
           "reason" => reason,
           "owner" => owner
         } = rule
       )
       when map_size(rule) == 6 do
    valid_slug?(id, 120) and valid_slug?(owner, 120) and MapSet.member?(@boundary_origins, origin) and
      valid_string_length?(reason, 1, 512) and valid_applicability?(applies_to) and
      is_list(inputs) and length(inputs) in 1..64 and Enum.all?(inputs, &valid_boundary_input?/1)
  end

  defp valid_boundary_rule?(_rule), do: false

  defp valid_applicability?(
         %{
           "exact_roots" => exact,
           "descendant_roots" => descendants,
           "excluded_roots" => excluded
         } = applicability
       )
       when map_size(applicability) == 3 do
    valid_unique_paths?(exact, 4_096) and valid_unique_paths?(descendants, 64) and
      valid_unique_paths?(excluded, 4_096) and (exact != [] or descendants != [])
  end

  defp valid_applicability?(_applicability), do: false

  defp valid_unique_paths?(values, maximum) when is_list(values) and length(values) <= maximum do
    Enum.all?(values, &valid_portable_path?/1) and
      MapSet.size(MapSet.new(values)) == length(values)
  end

  defp valid_unique_paths?(_values, _maximum), do: false

  defp valid_boundary_input?(%{"path" => path, "role" => role} = input) do
    base_valid =
      role in ["cross_package_exact", "generated_pruning_exception", "shared_ancestor"] and
        valid_portable_path?(path)

    case {role, input} do
      {"generated_pruning_exception", %{"generated_component" => component}}
      when map_size(input) == 3 ->
        base_valid and valid_generated_component?(component)

      {other, value} when other != "generated_pruning_exception" and map_size(value) == 2 ->
        base_valid

      _other ->
        false
    end
  end

  defp valid_boundary_input?(_input), do: false

  defp valid_generated_component?(value) do
    valid_string_length?(value, 1, 128) and
      not String.contains?(value, ["/", "\\", "<", ">", ":", "\"", "|", "?", "*"]) and
      Enum.all?(String.to_charlist(value), &(&1 >= 32))
  end

  defp build_boundary_reverse_index(boundary, packages) do
    Enum.reduce(packages, %{}, fn package, index ->
      Enum.reduce(boundary["boundaries"], index, fn rule, inner_index ->
        if boundary_applies?(rule["applies_to"], package.rel_path) do
          Enum.reduce(rule["inputs"], inner_index, fn input, result ->
            Map.update(
              result,
              input["path"],
              MapSet.new([package.name]),
              &MapSet.put(&1, package.name)
            )
          end)
        else
          inner_index
        end
      end)
    end)
  end

  defp boundary_applies?(applicability, root) do
    root in applicability["exact_roots"] or
      (root not in applicability["excluded_roots"] and
         Enum.any?(applicability["descendant_roots"], &String.starts_with?(root, &1 <> "/")))
  end

  defp preflight_match_work(packages, paths) do
    Enum.reduce_while(packages, 0, fn package, total ->
      if package.source_mode == "strict_globs" do
        factor = Enum.sum(Enum.map(package.source_globs, &(scalar_length(&1) + 1)))
        charge_package_paths(package, paths, factor, total)
      else
        {:cont, total}
      end
    end)
    |> case do
      :over_limit -> {:error, "DIFF_MATCH_LIMIT_EXCEEDED"}
      _total -> :ok
    end
  end

  defp charge_package_paths(package, paths, factor, initial) do
    Enum.reduce_while(paths, initial, fn path, total ->
      if inside?(path, package.rel_path) do
        relative = relative_path(path, package.rel_path)

        if build_front?(relative) do
          {:cont, total}
        else
          charge = factor * (scalar_length(relative) + 1)

          if charge > @max_match_work - total do
            {:halt, :over_limit}
          else
            {:cont, total + charge}
          end
        end
      else
        {:cont, total}
      end
    end)
    |> case do
      :over_limit -> {:halt, :over_limit}
      total -> {:cont, total}
    end
  end

  defp select_diff(input, packages, graph, boundary_consumers) do
    initial = {MapSet.new(input.forced_packages), false}

    {selected, unknown} =
      Enum.reduce(input.changed_paths, initial, fn path, {changed, prior_unknown} ->
        boundary_matches = Map.get(boundary_consumers, path, MapSet.new())
        {package_matches, known_package_path} = select_package_path(path, packages)

        {
          changed |> MapSet.union(boundary_matches) |> MapSet.union(package_matches),
          prior_unknown or (MapSet.size(boundary_matches) == 0 and not known_package_path)
        }
      end)

    cond do
      unknown and input.unknown_path_policy == "error" ->
        {:error, "DIFF_UNKNOWN_PATH"}

      true ->
        changed = if unknown, do: graph.names, else: selected
        affected = closure(changed, graph.dependents)
        prerequisites = affected |> closure(graph.prerequisites) |> MapSet.difference(affected)

        {:ok,
         %{
           "changed_packages" => sorted_set(changed),
           "affected_packages" => sorted_set(affected),
           "prerequisite_packages" => sorted_set(prerequisites)
         }}
    end
  end

  defp select_package_path(path, packages) do
    Enum.reduce(packages, {MapSet.new(), false}, fn package, {matches, known} ->
      if inside?(path, package.rel_path) do
        relative = relative_path(path, package.rel_path)

        selected =
          package.source_mode == "package_prefix" or build_front?(relative) or
            GlobMatch.match_any_compiled_path?(package.compiled_globs, relative)

        {if(selected, do: MapSet.put(matches, package.name), else: matches), true}
      else
        {matches, known}
      end
    end)
  end

  defp closure(seeds, adjacency) do
    do_closure(seeds, MapSet.to_list(seeds), adjacency)
  end

  defp do_closure(seen, [], _adjacency), do: seen

  defp do_closure(seen, [name | pending], adjacency) do
    additions = adjacency |> Map.fetch!(name) |> MapSet.new() |> MapSet.difference(seen)
    do_closure(MapSet.union(seen, additions), MapSet.to_list(additions) ++ pending, adjacency)
  end

  defp sorted_set(set), do: set |> MapSet.to_list() |> Enum.sort()

  defp inside?(path, root), do: path == root or String.starts_with?(path, root <> "/")

  defp relative_path(path, root) do
    if path == root, do: "", else: String.replace_prefix(path, root <> "/", "")
  end

  defp build_front?(path) do
    path |> String.split("/") |> List.last() |> then(&MapSet.member?(@build_fronts, &1))
  end

  defp valid_package_name?(value) do
    valid_string?(value) and scalar_length(value) <= @max_name_scalars and
      Regex.match?(~r/\A[a-z0-9][a-z0-9._-]*(\/[a-z0-9][a-z0-9._-]*)+\z/, value)
  end

  defp valid_portable_path?(value) do
    valid_portable_prefix?(value, ["<", ">", ":", "\"", "|", "?", "*"]) and
      Enum.all?(String.split(value, "/", trim: false), &valid_path_segment?/1)
  end

  defp valid_portable_glob?(value) do
    valid_portable_prefix?(value, ["<", ">", ":", "\"", "|", "?"]) and
      Enum.all?(String.split(value, "/", trim: false), fn segment ->
        segment != "" and segment not in [".", ".."] and
          not String.ends_with?(segment, [" ", "."]) and
          (String.contains?(segment, ["*", "[", "]", "{", "}"]) or not reserved?(segment))
      end)
  end

  defp valid_portable_prefix?(value, forbidden) do
    valid_string?(value) and value != "" and scalar_length(value) <= @max_path_scalars and
      Unicode.nfc(value) == value and not String.starts_with?(value, "/") and
      not Regex.match?(~r/\A[A-Za-z]:/, value) and not String.contains?(value, "\\") and
      not String.contains?(value, "//") and not String.contains?(value, forbidden) and
      Enum.all?(String.to_charlist(value), &(&1 >= 32))
  end

  defp valid_path_segment?(segment) do
    segment != "" and segment not in [".", ".."] and not String.ends_with?(segment, [" ", "."]) and
      not reserved?(segment)
  end

  defp reserved?(segment) do
    base = segment |> String.split(".", parts: 2) |> hd() |> Unicode.full_uppercase()
    MapSet.member?(@reserved_names, base)
  end

  defp valid_slug?(value, maximum) do
    valid_string_length?(value, 1, maximum) and
      Regex.match?(~r/\A[a-z0-9]+(?:-[a-z0-9]+)*\z/, value)
  end

  defp valid_string_length?(value, minimum, maximum) do
    valid_string?(value) and scalar_length(value) in minimum..maximum
  end

  defp valid_string?(value), do: is_binary(value) and String.valid?(value)
  defp scalar_length(value), do: value |> String.to_charlist() |> length()

  defp canonical_json(value) when is_map(value) do
    encoded =
      value
      |> Enum.sort_by(fn {key, _item} -> key end)
      |> Enum.map(fn {key, item} -> [Jason.encode!(key), ":", canonical_json(item)] end)
      |> Enum.intersperse(",")

    ["{", encoded, "}"]
  end

  defp canonical_json(value) when is_list(value) do
    ["[", value |> Enum.map(&canonical_json/1) |> Enum.intersperse(","), "]"]
  end

  defp canonical_json(value), do: Jason.encode!(value)
end
