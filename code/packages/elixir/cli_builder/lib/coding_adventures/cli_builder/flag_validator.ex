defmodule CodingAdventures.CliBuilder.FlagValidator do
  @moduledoc """
  Validates parsed flag values against the spec's constraint rules.

  ## Constraints Validated (§6.4.2)

  1. **`conflicts_with`** — two flags that list each other cannot both be present.
  2. **`requires` (transitive)** — if flag A is present and requires B (directly
     or transitively via the flag dependency graph G_flag), B must also be present.
  3. **Required flags** — flags with `required: true` must be present unless
     `required_unless` is satisfied.
  4. **Mutually exclusive groups** — at most one (or exactly one if `required: true`)
     flag in the group may be present.

  All errors are collected (not fail-fast) so the user gets a full picture.

  ## Connection to DirectedGraph

  The transitive-requires check uses
  `CodingAdventures.DirectedGraph.Graph.transitive_closure/2` to walk the flag
  dependency graph G_flag forward from each present flag, finding all transitively
  required flags in one BFS pass.
  """

  alias CodingAdventures.CliBuilder.ParseError
  alias CodingAdventures.DirectedGraph.Graph

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Validate a map of parsed flags against the active flag definitions.

  Returns a (possibly empty) list of `ParseError` structs.

  ## Parameters

  - `parsed_flags` — map of flag id -> value as produced by Phase 2 scanning.
  - `active_flags` — list of normalised flag definition maps in scope.
  - `exclusive_groups` — list of normalised mutually-exclusive group maps.
  - `command_path` — current command path, used as error context.

  ## Example

      iex> alias CodingAdventures.CliBuilder.FlagValidator
      iex> flags = [%{"id" => "verbose", "type" => "boolean", "required" => false,
      ...>            "conflicts_with" => [], "requires" => [], "required_unless" => []}]
      iex> FlagValidator.validate(%{"verbose" => true}, flags, [], ["prog"])
      []
  """
  @spec validate(map(), [map()], [map()], [String.t()]) :: [ParseError.t()]
  def validate(parsed_flags, active_flags, exclusive_groups, command_path) do
    # Build a lookup map from flag id -> flag def for quick access.
    flag_map = Map.new(active_flags, fn f -> {f["id"], f} end)

    # Build the flag dependency graph G_flag.
    g_flag = build_flag_graph(active_flags)

    # Collect all errors from each validation pass.
    conflicts_errors = check_conflicts(parsed_flags, flag_map, command_path)
    requires_errors = check_requires(parsed_flags, flag_map, g_flag, command_path)
    required_errors = check_required(parsed_flags, active_flags, command_path)
    group_errors = check_exclusive_groups(parsed_flags, exclusive_groups, flag_map, command_path)

    conflicts_errors ++ requires_errors ++ required_errors ++ group_errors
  end

  # ---------------------------------------------------------------------------
  # Conflict checking
  # ---------------------------------------------------------------------------

  # For every flag that is present and has a `conflicts_with` list, check that
  # none of the conflicting flags are also present.
  defp check_conflicts(parsed_flags, flag_map, command_path) do
    # Track which pairs we've already reported to avoid duplicate errors
    # (A conflicts_with B and B conflicts_with A would otherwise fire twice).
    Enum.reduce(parsed_flags, {[], MapSet.new()}, fn {flag_id, _}, {errors, seen} ->
      flag = Map.get(flag_map, flag_id)

      if flag == nil do
        {errors, seen}
      else
        Enum.reduce(Map.get(flag, "conflicts_with", []), {errors, seen}, fn other_id, {errs, s} ->
          pair = Enum.sort([flag_id, other_id])
          already_seen = MapSet.member?(s, pair)

          if Map.has_key?(parsed_flags, other_id) and not already_seen do
            other = Map.get(flag_map, other_id)
            label_self = flag_label(flag)
            label_other = if other, do: flag_label(other), else: inspect(other_id)

            err = %ParseError{
              error_type: "conflicting_flags",
              message: "#{label_self} and #{label_other} cannot be used together",
              suggestion: nil,
              context: command_path
            }

            {[err | errs], MapSet.put(s, pair)}
          else
            {errs, s}
          end
        end)
      end
    end)
    |> elem(0)
    |> Enum.reverse()
  end

  # ---------------------------------------------------------------------------
  # Requires (transitive) checking
  # ---------------------------------------------------------------------------

  # For every flag that is present, compute the full transitive closure of its
  # `requires` edges in G_flag. Any required flag that is absent is an error.
  defp check_requires(parsed_flags, flag_map, g_flag, command_path) do
    Enum.flat_map(parsed_flags, fn {flag_id, _} ->
      case Graph.transitive_closure(g_flag, flag_id) do
        {:error, _} ->
          # Flag not in graph (e.g. builtin help/version) — skip.
          []

        {:ok, required_set} ->
          Enum.flat_map(required_set, fn req_id ->
            if not Map.has_key?(parsed_flags, req_id) do
              flag = Map.get(flag_map, flag_id)
              req_flag = Map.get(flag_map, req_id)
              label_self = if flag, do: flag_label(flag), else: inspect(flag_id)
              label_req = if req_flag, do: flag_label(req_flag), else: inspect(req_id)

              [
                %ParseError{
                  error_type: "missing_dependency_flag",
                  message: "#{label_self} requires #{label_req}",
                  suggestion: nil,
                  context: command_path
                }
              ]
            else
              []
            end
          end)
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Required flags checking
  # ---------------------------------------------------------------------------

  # For every flag with `required: true`, it must be present unless at least one
  # flag in its `required_unless` list is present.
  defp check_required(parsed_flags, active_flags, command_path) do
    Enum.flat_map(active_flags, fn flag ->
      if flag["required"] and not Map.has_key?(parsed_flags, flag["id"]) do
        unless_flags = Map.get(flag, "required_unless", [])
        exempt = Enum.any?(unless_flags, &Map.has_key?(parsed_flags, &1))

        if exempt do
          []
        else
          [
            %ParseError{
              error_type: "missing_required_flag",
              message: "#{flag_label(flag)} is required",
              suggestion: nil,
              context: command_path
            }
          ]
        end
      else
        []
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Exclusive group checking
  # ---------------------------------------------------------------------------

  defp check_exclusive_groups(parsed_flags, exclusive_groups, _flag_map, command_path) do
    Enum.flat_map(exclusive_groups, fn group ->
      present = Enum.filter(group["flag_ids"], &Map.has_key?(parsed_flags, &1))

      cond do
        length(present) > 1 ->
          [
            %ParseError{
              error_type: "exclusive_group_violation",
              message:
                "Only one of #{Enum.join(present, ", ")} may be used at a time",
              suggestion: nil,
              context: command_path
            }
          ]

        group["required"] and length(present) == 0 ->
          [
            %ParseError{
              error_type: "missing_exclusive_group",
              message:
                "One of #{Enum.join(group["flag_ids"], ", ")} is required",
              suggestion: nil,
              context: command_path
            }
          ]

        true ->
          []
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Flag dependency graph construction
  # ---------------------------------------------------------------------------

  # Build a directed graph where an edge A → B means "flag A requires flag B".
  # Used for transitive requires checking.
  defp build_flag_graph(active_flags) do
    g = Graph.new()

    g =
      Enum.reduce(active_flags, g, fn flag, acc ->
        {:ok, acc} = Graph.add_node(acc, flag["id"])
        acc
      end)

    Enum.reduce(active_flags, g, fn flag, acc ->
      Enum.reduce(Map.get(flag, "requires", []), acc, fn req_id, a ->
        case Graph.add_edge(a, flag["id"], req_id) do
          {:ok, a2} -> a2
          {:error, _} -> a
        end
      end)
    end)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Produce a human-readable label for a flag, e.g. "-l/--long-listing".
  defp flag_label(flag) do
    parts =
      [
        if(flag["short"], do: "-#{flag["short"]}", else: nil),
        if(flag["long"], do: "--#{flag["long"]}", else: nil),
        if(flag["single_dash_long"], do: "-#{flag["single_dash_long"]}", else: nil)
      ]
      |> Enum.reject(&is_nil/1)

    Enum.join(parts, "/")
  end
end
