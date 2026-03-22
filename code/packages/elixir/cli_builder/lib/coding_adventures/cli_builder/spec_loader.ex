defmodule CodingAdventures.CliBuilder.SpecLoader do
  @moduledoc """
  Reads, validates, and normalises a CLI Builder JSON specification file.

  ## Responsibilities

  1. **Read** the UTF-8 JSON file from disk using `Jason`.
  2. **Validate** top-level structure, required fields, type constraints, and
     cross-reference integrity (§6.4.3 of the spec).
  3. **Detect cycles** in each scope's flag dependency graph (G_flag) using
     `CodingAdventures.DirectedGraph.Graph.has_cycle?/1`.
  4. **Normalise** the resulting map so that optional fields always have
     sensible default values, making downstream modules simpler.

  ## Spec Errors

  Any structural or semantic problem raises `CodingAdventures.CliBuilder.SpecError`.
  These are programmer errors (the spec is wrong) rather than user errors
  (the user typed a bad command), so we fail loudly at load time.

  ## Usage

      spec = CodingAdventures.CliBuilder.SpecLoader.load!("path/to/cli.json")
  """

  alias CodingAdventures.CliBuilder.SpecError
  alias CodingAdventures.DirectedGraph.Graph

  @supported_versions ["1.0"]
  @valid_parsing_modes ~w[gnu posix subcommand_first traditional]
  @valid_types ~w[boolean string integer float path file directory enum]

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Load and validate a CLI spec from a JSON file.

  Returns a normalised map on success. Raises `SpecError` on any validation
  failure.

  ## Parameters

  - `path` — file system path to the JSON spec file.

  ## Raises

  - `SpecError` — if the file cannot be read, is not valid JSON, or violates
    any of the structural rules in §6.4.3.

  ## Example

      iex> spec = CodingAdventures.CliBuilder.SpecLoader.load!("echo.json")
      iex> spec["name"]
      "echo"
  """
  @spec load!(String.t()) :: map()
  def load!(path) do
    raw =
      case File.read(path) do
        {:ok, content} -> content
        {:error, reason} -> raise SpecError, message: "Cannot read spec file #{path}: #{reason}"
      end

    decoded =
      case Jason.decode(raw) do
        {:ok, map} -> map
        {:error, err} -> raise SpecError, message: "Invalid JSON in #{path}: #{Exception.message(err)}"
      end

    validate_and_normalise(decoded)
  end

  @doc """
  Load and validate a CLI spec from a JSON string (for testing without files).

  Returns a normalised map on success. Raises `SpecError` on validation failure.
  """
  @spec load_from_string!(String.t()) :: map()
  def load_from_string!(json) do
    decoded =
      case Jason.decode(json) do
        {:ok, map} -> map
        {:error, err} -> raise SpecError, message: "Invalid JSON: #{Exception.message(err)}"
      end

    validate_and_normalise(decoded)
  end

  # ---------------------------------------------------------------------------
  # Validation and normalisation
  # ---------------------------------------------------------------------------

  defp validate_and_normalise(raw) do
    # Step 1 — check spec version
    version = Map.get(raw, "cli_builder_spec_version")

    unless version in @supported_versions do
      raise SpecError,
        message:
          "Unsupported cli_builder_spec_version: #{inspect(version)}. " <>
            "Supported: #{Enum.join(@supported_versions, ", ")}"
    end

    # Step 2 — required top-level fields
    require_field!(raw, "name", "string")
    require_field!(raw, "description", "string")

    parsing_mode = Map.get(raw, "parsing_mode", "gnu")

    unless parsing_mode in @valid_parsing_modes do
      raise SpecError,
        message:
          "Invalid parsing_mode: #{inspect(parsing_mode)}. " <>
            "Must be one of: #{Enum.join(@valid_parsing_modes, ", ")}"
    end

    # Step 3 — normalise all sections
    global_flags = normalise_flags(Map.get(raw, "global_flags", []), "global_flags")
    root_flags = normalise_flags(Map.get(raw, "flags", []), "root flags")
    root_args = normalise_arguments(Map.get(raw, "arguments", []), "root arguments")
    commands = normalise_commands(Map.get(raw, "commands", []), [Map.get(raw, "name")])
    excl_groups = normalise_exclusive_groups(Map.get(raw, "mutually_exclusive_groups", []))

    # Collect all flag IDs in root scope (global + root-specific)
    root_flag_ids = flag_ids(global_flags) ++ flag_ids(root_flags)

    # Step 4 — validate cross-references in root scope
    validate_flag_refs!(root_flags, root_flag_ids, "root")
    validate_exclusive_group_refs!(excl_groups, root_flag_ids, "root")
    validate_at_most_one_variadic!(root_args, "root")

    # Step 5 — cycle detection in root G_flag
    check_flag_dependency_cycles!(global_flags ++ root_flags, root_flag_ids, "root")

    # Step 6 — recursively validate commands
    validate_commands_recursive!(commands, global_flags)

    # Step 7 — build the normalised spec map
    builtin = Map.get(raw, "builtin_flags", %{"help" => true, "version" => true})

    %{
      "cli_builder_spec_version" => version,
      "name" => Map.get(raw, "name"),
      "display_name" => Map.get(raw, "display_name", Map.get(raw, "name")),
      "description" => Map.get(raw, "description"),
      "version" => Map.get(raw, "version"),
      "parsing_mode" => parsing_mode,
      "builtin_flags" => %{
        "help" => Map.get(builtin, "help", true),
        "version" => Map.get(builtin, "version", true)
      },
      "global_flags" => global_flags,
      "flags" => root_flags,
      "arguments" => root_args,
      "commands" => commands,
      "mutually_exclusive_groups" => excl_groups
    }
  end

  # ---------------------------------------------------------------------------
  # Normalisation helpers
  # ---------------------------------------------------------------------------

  # Normalise a list of flag definitions, filling in all optional fields with
  # their default values.
  defp normalise_flags(flags, scope_name) when is_list(flags) do
    Enum.with_index(flags)
    |> Enum.map(fn {flag, idx} ->
      unless is_map(flag), do: raise(SpecError, message: "Flag at index #{idx} in #{scope_name} is not an object")
      require_field!(flag, "id", "string", "flag ##{idx} in #{scope_name}")
      require_field!(flag, "description", "string", "flag #{inspect(Map.get(flag, "id"))} in #{scope_name}")
      require_field_in!(flag, "type", @valid_types, "flag #{inspect(Map.get(flag, "id"))} in #{scope_name}")

      # At least one of short, long, single_dash_long must be present
      has_handle =
        Map.has_key?(flag, "short") or
          Map.has_key?(flag, "long") or
          Map.has_key?(flag, "single_dash_long")

      unless has_handle do
        raise SpecError,
          message:
            "Flag #{inspect(Map.get(flag, "id"))} in #{scope_name} must have at least one of: short, long, single_dash_long"
      end

      # enum_values required when type is "enum"
      if Map.get(flag, "type") == "enum" do
        ev = Map.get(flag, "enum_values", [])
        if not is_list(ev) or Enum.empty?(ev) do
          raise SpecError,
            message:
              "Flag #{inspect(Map.get(flag, "id"))} has type \"enum\" but enum_values is empty or missing"
        end
      end

      %{
        "id" => Map.get(flag, "id"),
        "short" => Map.get(flag, "short"),
        "long" => Map.get(flag, "long"),
        "single_dash_long" => Map.get(flag, "single_dash_long"),
        "description" => Map.get(flag, "description"),
        "type" => Map.get(flag, "type"),
        "required" => Map.get(flag, "required", false),
        "default" => Map.get(flag, "default"),
        "value_name" => Map.get(flag, "value_name"),
        "enum_values" => Map.get(flag, "enum_values", []),
        "conflicts_with" => Map.get(flag, "conflicts_with", []),
        "requires" => Map.get(flag, "requires", []),
        "required_unless" => Map.get(flag, "required_unless", []),
        "repeatable" => Map.get(flag, "repeatable", false)
      }
    end)
  end

  defp normalise_flags(_flags, scope_name) do
    raise SpecError, message: "flags in #{scope_name} must be an array"
  end

  defp normalise_arguments(args, scope_name) when is_list(args) do
    Enum.with_index(args)
    |> Enum.map(fn {arg, idx} ->
      unless is_map(arg), do: raise(SpecError, message: "Argument at index #{idx} in #{scope_name} is not an object")
      require_field!(arg, "id", "string", "argument ##{idx} in #{scope_name}")
      # Accept display_name (preferred) or name (backward compatibility).
      unless Map.has_key?(arg, "display_name") or Map.has_key?(arg, "name") do
        raise SpecError, message: "argument #{inspect(Map.get(arg, "id"))} in #{scope_name} is missing required field \"display_name\""
      end
      require_field!(arg, "description", "string", "argument #{inspect(Map.get(arg, "id"))} in #{scope_name}")
      require_field_in!(arg, "type", @valid_types, "argument #{inspect(Map.get(arg, "id"))} in #{scope_name}")

      required = Map.get(arg, "required", true)
      variadic = Map.get(arg, "variadic", false)
      variadic_min_default = if required, do: 1, else: 0
      display_name = Map.get(arg, "display_name", Map.get(arg, "name"))

      %{
        "id" => Map.get(arg, "id"),
        "display_name" => display_name,
        "description" => Map.get(arg, "description"),
        "type" => Map.get(arg, "type"),
        "required" => required,
        "variadic" => variadic,
        "variadic_min" => Map.get(arg, "variadic_min", variadic_min_default),
        "variadic_max" => Map.get(arg, "variadic_max"),
        "default" => Map.get(arg, "default"),
        "enum_values" => Map.get(arg, "enum_values", []),
        "required_unless_flag" => Map.get(arg, "required_unless_flag", [])
      }
    end)
  end

  defp normalise_arguments(_args, scope_name) do
    raise SpecError, message: "arguments in #{scope_name} must be an array"
  end

  defp normalise_commands(commands, parent_path) when is_list(commands) do
    Enum.with_index(commands)
    |> Enum.map(fn {cmd, idx} ->
      unless is_map(cmd), do: raise(SpecError, message: "Command at index #{idx} under #{inspect(parent_path)} is not an object")
      require_field!(cmd, "id", "string", "command ##{idx} under #{inspect(parent_path)}")
      require_field!(cmd, "name", "string", "command #{inspect(Map.get(cmd, "id"))}")
      require_field!(cmd, "description", "string", "command #{inspect(Map.get(cmd, "id"))}")

      cmd_name = Map.get(cmd, "name")
      path = parent_path ++ [cmd_name]

      flags = normalise_flags(Map.get(cmd, "flags", []), "command #{cmd_name}")
      args = normalise_arguments(Map.get(cmd, "arguments", []), "command #{cmd_name}")
      subcmds = normalise_commands(Map.get(cmd, "commands", []), path)
      excl = normalise_exclusive_groups(Map.get(cmd, "mutually_exclusive_groups", []))

      %{
        "id" => Map.get(cmd, "id"),
        "name" => cmd_name,
        "aliases" => Map.get(cmd, "aliases", []),
        "description" => Map.get(cmd, "description"),
        "inherit_global_flags" => Map.get(cmd, "inherit_global_flags", true),
        "flags" => flags,
        "arguments" => args,
        "commands" => subcmds,
        "mutually_exclusive_groups" => excl
      }
    end)
  end

  defp normalise_commands(_cmds, parent_path) do
    raise SpecError, message: "commands under #{inspect(parent_path)} must be an array"
  end

  defp normalise_exclusive_groups(groups) when is_list(groups) do
    Enum.with_index(groups)
    |> Enum.map(fn {g, idx} ->
      unless is_map(g), do: raise(SpecError, message: "Exclusive group at index #{idx} is not an object")
      require_field!(g, "id", "string", "exclusive group ##{idx}")

      flag_ids = Map.get(g, "flag_ids", [])
      unless is_list(flag_ids) and length(flag_ids) >= 2 do
        raise SpecError,
          message: "Exclusive group #{inspect(Map.get(g, "id"))} must have at least 2 flag_ids"
      end

      %{
        "id" => Map.get(g, "id"),
        "flag_ids" => flag_ids,
        "required" => Map.get(g, "required", false)
      }
    end)
  end

  defp normalise_exclusive_groups(_groups) do
    raise SpecError, message: "mutually_exclusive_groups must be an array"
  end

  # ---------------------------------------------------------------------------
  # Cross-reference validation
  # ---------------------------------------------------------------------------

  # Validate that conflicts_with and requires references point to known flag IDs
  # in the same scope or in global_flags.
  defp validate_flag_refs!(flags, all_ids_in_scope, scope_name) do
    Enum.each(flags, fn flag ->
      Enum.each(Map.get(flag, "conflicts_with", []), fn ref ->
        unless ref in all_ids_in_scope do
          raise SpecError,
            message:
              "Flag #{inspect(flag["id"])} in #{scope_name} has conflicts_with reference " <>
                "to unknown flag #{inspect(ref)}"
        end
      end)

      Enum.each(Map.get(flag, "requires", []), fn ref ->
        unless ref in all_ids_in_scope do
          raise SpecError,
            message:
              "Flag #{inspect(flag["id"])} in #{scope_name} has requires reference " <>
                "to unknown flag #{inspect(ref)}"
        end
      end)
    end)
  end

  defp validate_exclusive_group_refs!(groups, flag_ids, scope_name) do
    Enum.each(groups, fn group ->
      Enum.each(group["flag_ids"], fn ref ->
        unless ref in flag_ids do
          raise SpecError,
            message:
              "Exclusive group #{inspect(group["id"])} in #{scope_name} references " <>
                "unknown flag #{inspect(ref)}"
        end
      end)
    end)
  end

  defp validate_at_most_one_variadic!(args, scope_name) do
    variadic_count = Enum.count(args, fn a -> a["variadic"] end)

    if variadic_count > 1 do
      raise SpecError,
        message: "Scope #{scope_name} has #{variadic_count} variadic arguments; at most 1 is allowed"
    end
  end

  # ---------------------------------------------------------------------------
  # Flag dependency cycle detection
  # ---------------------------------------------------------------------------

  # Build a directed graph for the `requires` edges among all flags in a scope
  # and check it for cycles.  A cycle like "A requires B requires A" is a spec
  # error because it can never be satisfied.
  defp check_flag_dependency_cycles!(flags, _all_ids, scope_name) do
    g = Graph.new()

    # Add all flag nodes first so isolated nodes are represented.
    g =
      Enum.reduce(flags, g, fn flag, acc ->
        {:ok, acc} = Graph.add_node(acc, flag["id"])
        acc
      end)

    # Add directed edges for each requires relationship.
    g =
      Enum.reduce(flags, g, fn flag, acc ->
        Enum.reduce(Map.get(flag, "requires", []), acc, fn req_id, a ->
          case Graph.add_edge(a, flag["id"], req_id) do
            {:ok, a2} -> a2
            {:error, _} -> a
          end
        end)
      end)

    if Graph.has_cycle?(g) do
      # Find the cycle for a helpful error message.
      # We'll report a generic message; detailed cycle path requires
      # topological_sort which returns the error with the cycle info.
      case Graph.topological_sort(g) do
        {:error, err} ->
          raise SpecError,
            message:
              "Circular requires dependency in #{scope_name}: #{err.message}"

        {:ok, _} ->
          :ok
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Recursive command validation
  # ---------------------------------------------------------------------------

  defp validate_commands_recursive!(commands, global_flags) do
    # Check for duplicate command IDs and names among siblings.
    ids = Enum.map(commands, & &1["id"])
    names = Enum.flat_map(commands, fn c -> [c["name"] | c["aliases"]] end)

    if length(ids) != length(Enum.uniq(ids)) do
      raise SpecError, message: "Duplicate command IDs among siblings: #{inspect(ids)}"
    end

    if length(names) != length(Enum.uniq(names)) do
      raise SpecError, message: "Duplicate command names/aliases among siblings: #{inspect(names)}"
    end

    Enum.each(commands, fn cmd ->
      scope_name = "command #{inspect(cmd["name"])}"
      global_ids = flag_ids(global_flags)
      cmd_flag_ids = flag_ids(cmd["flags"])
      all_ids = global_ids ++ cmd_flag_ids

      validate_flag_refs!(cmd["flags"], all_ids, scope_name)
      validate_exclusive_group_refs!(cmd["mutually_exclusive_groups"], all_ids, scope_name)
      validate_at_most_one_variadic!(cmd["arguments"], scope_name)

      # Combine global + command flags for cycle check
      check_flag_dependency_cycles!(global_flags ++ cmd["flags"], all_ids, scope_name)

      # Recurse into sub-commands
      validate_commands_recursive!(cmd["commands"], global_flags)
    end)
  end

  # ---------------------------------------------------------------------------
  # Small helpers
  # ---------------------------------------------------------------------------

  defp flag_ids(flags), do: Enum.map(flags, & &1["id"])

  defp require_field!(map, key, type, context \\ "spec") do
    val = Map.get(map, key)

    if val == nil do
      raise SpecError, message: "Missing required field #{inspect(key)} in #{context}"
    end

    case type do
      "string" ->
        unless is_binary(val) do
          raise SpecError,
            message: "Field #{inspect(key)} in #{context} must be a string, got #{inspect(val)}"
        end

      _ ->
        :ok
    end
  end

  defp require_field_in!(map, key, valid_values, context) do
    val = Map.get(map, key)

    if val == nil do
      raise SpecError, message: "Missing required field #{inspect(key)} in #{context}"
    end

    unless val in valid_values do
      raise SpecError,
        message:
          "Field #{inspect(key)} in #{context} must be one of #{inspect(valid_values)}, got #{inspect(val)}"
    end
  end
end
