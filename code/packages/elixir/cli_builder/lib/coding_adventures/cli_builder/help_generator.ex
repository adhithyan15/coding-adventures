defmodule CodingAdventures.CliBuilder.HelpGenerator do
  @moduledoc """
  Generates human-readable help text from a loaded CLI spec.

  ## Format (§9)

  ```
  USAGE
    <name> [OPTIONS] [COMMAND] [ARGS...]

  DESCRIPTION
    <description>

  COMMANDS
    subcommand    Description of the subcommand.

  OPTIONS
    -s, --long-name <VALUE>    Description of the flag. [default: val]
    -b, --boolean              Boolean flag description.

  ARGUMENTS
    <ARG>      Description. Required.
    [ARG...]   Description. Optional, repeatable.

  GLOBAL OPTIONS
    -h, --help     Show this help message and exit.
    --version      Show version and exit.
  ```

  The module only produces a string; it does not print or exit. Printing and
  exiting are the caller's responsibility.

  ## Usage

      help_text = HelpGenerator.generate(spec, ["git", "remote"])
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Generate help text for the command identified by `command_path`.

  `spec` is a normalised spec map as returned by `SpecLoader.load!/1`.
  `command_path` is a list of command names from root to the target command
  (e.g. `["git"]` for root, `["git", "remote", "add"]` for a subcommand).

  Returns a formatted help string.
  """
  @spec generate(map(), [String.t()]) :: String.t()
  def generate(spec, command_path) do
    # Navigate to the command node for this path.
    # command_path[0] is the program name (root); remainder are subcommand names.
    {node, inherited_global_flags} = resolve_command_node(spec, command_path)

    # Build the usage synopsis.
    synopsis = build_synopsis(spec["name"], command_path, node)

    sections = []

    # USAGE section
    sections = sections ++ ["USAGE\n  #{synopsis}"]

    # DESCRIPTION
    description = Map.get(node, "description", spec["description"])
    sections = sections ++ ["DESCRIPTION\n  #{description}"]

    # COMMANDS (if this node has subcommands)
    commands = Map.get(node, "commands", [])
    sections =
      if Enum.empty?(commands) do
        sections
      else
        cmd_lines =
          commands
          |> Enum.map(fn cmd ->
            name = String.pad_trailing(cmd["name"], 18)
            "  #{name}#{cmd["description"]}"
          end)
          |> Enum.join("\n")

        sections ++ ["COMMANDS\n#{cmd_lines}"]
      end

    # OPTIONS (flags specific to this command node)
    node_flags = Map.get(node, "flags", [])
    sections =
      if Enum.empty?(node_flags) do
        sections
      else
        flag_lines = Enum.map_join(node_flags, "\n", &format_flag/1)
        sections ++ ["OPTIONS\n#{flag_lines}"]
      end

    # ARGUMENTS section
    arg_defs = Map.get(node, "arguments", [])
    sections =
      if Enum.empty?(arg_defs) do
        sections
      else
        arg_lines = Enum.map_join(arg_defs, "\n", &format_argument/1)
        sections ++ ["ARGUMENTS\n#{arg_lines}"]
      end

    # GLOBAL OPTIONS (shown separately)
    global_flags = inherited_global_flags
    builtin_flags = build_builtin_flags(spec)
    all_global = global_flags ++ builtin_flags

    sections =
      if Enum.empty?(all_global) do
        sections
      else
        global_lines = Enum.map_join(all_global, "\n", &format_flag/1)
        sections ++ ["GLOBAL OPTIONS\n#{global_lines}"]
      end

    Enum.join(sections, "\n\n")
  end

  # ---------------------------------------------------------------------------
  # Command node resolution
  # ---------------------------------------------------------------------------

  # Walk the command_path (skipping the program name at index 0) to find the
  # target command node. Returns {node_map, global_flags_inherited}.
  defp resolve_command_node(spec, command_path) do
    # path[0] is the program itself; subcommands start at index 1.
    subcommand_names = Enum.drop(command_path, 1)

    # The "root" node is the spec itself — it has the same shape as a command
    # node (flags, arguments, commands).
    root = %{
      "flags" => spec["flags"],
      "arguments" => spec["arguments"],
      "commands" => spec["commands"],
      "description" => spec["description"],
      "mutually_exclusive_groups" => spec["mutually_exclusive_groups"]
    }

    global_flags = spec["global_flags"]

    {node, globals} =
      Enum.reduce(subcommand_names, {root, global_flags}, fn name, {current, gf} ->
        cmds = Map.get(current, "commands", [])

        found =
          Enum.find(cmds, fn c ->
            c["name"] == name or name in c["aliases"]
          end)

        if found == nil do
          {current, gf}
        else
          # Inherit global flags only if the command says so.
          new_gf = if found["inherit_global_flags"] != false, do: gf, else: []
          {found, new_gf}
        end
      end)

    {node, globals}
  end

  # ---------------------------------------------------------------------------
  # Synopsis
  # ---------------------------------------------------------------------------

  defp build_synopsis(program_name, command_path, node) do
    # Build the command prefix: "prog subcmd1 subcmd2"
    prefix = Enum.join(command_path, " ")

    # If the node has subcommands, show [COMMAND]
    has_subcommands = not Enum.empty?(Map.get(node, "commands", []))
    has_flags = not Enum.empty?(Map.get(node, "flags", []))
    has_args = not Enum.empty?(Map.get(node, "arguments", []))

    opts_part = if has_flags, do: " [OPTIONS]", else: ""
    cmd_part = if has_subcommands, do: " [COMMAND]", else: ""

    args_part =
      if has_args do
        arg_strs =
          Enum.map(Map.get(node, "arguments", []), fn a ->
            format_arg_usage(a)
          end)

        " " <> Enum.join(arg_strs, " ")
      else
        ""
      end

    "#{prefix}#{opts_part}#{cmd_part}#{args_part}"
    |> String.replace("#{program_name} #{program_name}", program_name)
  end

  # Format a single argument in usage synopsis style.
  defp format_arg_usage(arg) do
    # Prefer display_name, fall back to name for backward compatibility.
    name = arg["display_name"] || arg["name"]
    required = arg["required"]
    variadic = arg["variadic"]

    cond do
      required and variadic -> "<#{name}>..."
      required -> "<#{name}>"
      variadic -> "[#{name}...]"
      true -> "[#{name}]"
    end
  end

  # ---------------------------------------------------------------------------
  # Flag formatting
  # ---------------------------------------------------------------------------

  # Format a single flag definition into an OPTIONS line.
  #
  # Examples:
  #   -l, --long-listing              Use long listing format.
  #   -o, --output <FILE>             Output file path. [default: out.txt]
  #       --verbose                   Enable verbose mode.
  #   -classpath <VALUE>              Classpath (single-dash-long)
  defp format_flag(flag) do
    short = flag["short"]
    long = flag["long"]
    sdl = flag["single_dash_long"]
    type = flag["type"]
    value_name = flag["value_name"] || (if type != "boolean", do: type |> String.upcase(), else: nil)
    default = flag["default"]
    required = flag["required"]
    description = flag["description"]

    # Build the flag syntax part.
    parts =
      [
        if(short, do: "-#{short}", else: nil),
        if(long, do: "--#{long}#{if value_name, do: " <#{value_name}>", else: ""}", else: nil),
        if(sdl, do: "-#{sdl}#{if value_name, do: " <#{value_name}>", else: ""}", else: nil)
      ]
      |> Enum.reject(&is_nil/1)

    flag_str = Enum.join(parts, ", ")

    # Pad the flag syntax to align descriptions.
    padded = String.pad_trailing(flag_str, 28)

    # Append default/required suffixes to the description.
    suffix =
      cond do
        default != nil and not required -> " [default: #{default}]"
        required -> " [required]"
        true -> ""
      end

    "  #{padded}#{description}#{suffix}"
  end

  # ---------------------------------------------------------------------------
  # Argument formatting
  # ---------------------------------------------------------------------------

  # Format a single argument definition into an ARGUMENTS line.
  defp format_argument(arg) do
    # Prefer display_name, fall back to name for backward compatibility.
    name = arg["display_name"] || arg["name"]
    required = arg["required"]
    variadic = arg["variadic"]
    description = arg["description"]

    display =
      cond do
        required and variadic -> "<#{name}>..."
        required -> "<#{name}>"
        variadic -> "[#{name}...]"
        true -> "[#{name}]"
      end

    padded = String.pad_trailing(display, 20)
    req_str = if required, do: " Required.", else: " Optional."
    variadic_str = if variadic, do: " Repeatable.", else: ""

    "  #{padded}#{description}.#{req_str}#{variadic_str}"
  end

  # ---------------------------------------------------------------------------
  # Builtin flags
  # ---------------------------------------------------------------------------

  # Produce synthetic flag definitions for --help and --version so they appear
  # in GLOBAL OPTIONS.
  defp build_builtin_flags(spec) do
    builtin = spec["builtin_flags"]

    help_flag =
      if Map.get(builtin, "help", true) do
        %{
          "id" => "help",
          "short" => "h",
          "long" => "help",
          "single_dash_long" => nil,
          "type" => "boolean",
          "description" => "Show this help message and exit",
          "required" => false,
          "default" => nil,
          "value_name" => nil
        }
      else
        nil
      end

    version_flag =
      if Map.get(builtin, "version", true) and spec["version"] != nil do
        %{
          "id" => "version",
          "short" => nil,
          "long" => "version",
          "single_dash_long" => nil,
          "type" => "boolean",
          "description" => "Show version and exit",
          "required" => false,
          "default" => nil,
          "value_name" => nil
        }
      else
        nil
      end

    [help_flag, version_flag] |> Enum.reject(&is_nil/1)
  end
end
