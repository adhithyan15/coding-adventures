defmodule UnixTools.Chown do
  @moduledoc """
  chown -- change file owner and group.

  ## What This Program Does

  This is a reimplementation of the GNU `chown` utility in Elixir. It changes
  the owner and/or group of files and directories.

  ## How chown Works

  At its simplest:

      chown alice file.txt          =>   change owner to alice
      chown alice:staff file.txt    =>   change owner to alice, group to staff
      chown :staff file.txt         =>   change group only (to staff)
      chown alice: file.txt         =>   change owner to alice, group to alice's default

  ## Owner:Group Parsing

  The first argument specifies the new owner and/or group. It can take
  several forms:

  | Format        | Owner    | Group          |
  |---------------|----------|----------------|
  | `OWNER`       | OWNER    | (unchanged)    |
  | `OWNER:GROUP` | OWNER    | GROUP          |
  | `OWNER:`      | OWNER    | OWNER's default|
  | `:GROUP`      | (none)   | GROUP          |
  | `OWNER.GROUP` | OWNER    | GROUP          |

  ## Flags

  | Flag | Effect                                          |
  |------|-------------------------------------------------|
  | -R   | Recursively change owner/group for directories  |
  | -v   | Verbose: print every file processed             |
  | -c   | Changes: print only when a change is made        |
  | -f   | Silent: suppress error messages                  |
  | -h   | Don't follow symlinks (change link itself)       |

  ## Implementation Note

  Since Elixir's File module doesn't provide a direct `chown` function,
  we delegate to the system `chown` command via `System.cmd/3`. This is
  the same approach used by many scripting language implementations.

  ## Implementation Approach

  1. `parse_owner_group/1` parses the OWNER[:GROUP] specification.
  2. `build_chown_args/3` constructs the arguments for the system chown command.
  3. `execute_chown/3` runs the system chown on each file.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["chown" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          recursive: !!flags["recursive"],
          verbose: !!flags["verbose"],
          changes: !!flags["changes"],
          silent: !!flags["silent"],
          no_dereference: !!flags["no_dereference"],
          reference: flags["reference"]
        }

        owner_group_str = arguments["owner_group"]
        file_list = normalize_files(arguments["files"])

        run(owner_group_str, file_list, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "chown: #{err.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Owner:Group Parsing
  # ---------------------------------------------------------------------------

  @doc """
  Parse an OWNER[:GROUP] specification into its components.

  Returns `{owner, group}` where either can be `nil` if not specified.

  ## Parsing Rules

  The separator can be either `:` or `.` (for backward compatibility).

  | Input          | Owner    | Group    |
  |----------------|----------|----------|
  | "alice"        | "alice"  | nil      |
  | "alice:staff"  | "alice"  | "staff"  |
  | "alice:"       | "alice"  | ""       |
  | ":staff"       | nil      | "staff"  |
  | "alice.staff"  | "alice"  | "staff"  |

  Note: `""` for group (from "alice:") means "use the owner's default group."

  ## Examples

      iex> UnixTools.Chown.parse_owner_group("alice")
      {"alice", nil}

      iex> UnixTools.Chown.parse_owner_group("alice:staff")
      {"alice", "staff"}

      iex> UnixTools.Chown.parse_owner_group(":staff")
      {nil, "staff"}

      iex> UnixTools.Chown.parse_owner_group("alice:")
      {"alice", ""}
  """
  def parse_owner_group(spec) do
    # Try colon first, then dot
    separator =
      cond do
        String.contains?(spec, ":") -> ":"
        String.contains?(spec, ".") -> "."
        true -> nil
      end

    case separator do
      nil ->
        # Just an owner, no group
        {spec, nil}

      sep ->
        case String.split(spec, sep, parts: 2) do
          ["", group] ->
            {nil, group}

          [owner, group] ->
            {owner, group}
        end
    end
  end

  @doc """
  Build the arguments for the system chown command.

  Constructs the proper chown invocation based on the parsed owner/group
  and options.

  ## Examples

      iex> UnixTools.Chown.build_chown_args({"alice", "staff"}, ["file.txt"], %{})
      ["alice:staff", "file.txt"]

      iex> UnixTools.Chown.build_chown_args({"alice", nil}, ["file.txt"], %{recursive: true})
      ["-R", "alice", "file.txt"]
  """
  def build_chown_args({owner, group}, file_list, opts) do
    # Build the owner:group spec
    ownership_spec =
      case {owner, group} do
        {nil, grp} -> ":#{grp}"
        {own, nil} -> own
        {own, grp} -> "#{own}:#{grp}"
      end

    # Build flags
    flag_args =
      []
      |> maybe_add_flag(opts[:recursive], "-R")
      |> maybe_add_flag(opts[:verbose], "-v")
      |> maybe_add_flag(opts[:changes], "-c" )
      |> maybe_add_flag(opts[:silent], "-f")
      |> maybe_add_flag(opts[:no_dereference], "-h")

    flag_args ++ [ownership_spec | file_list]
  end

  @doc """
  Execute the chown command on the specified files.

  Uses `System.cmd/3` to delegate to the operating system's chown command.
  Returns `{output, exit_code}`.
  """
  def execute_chown(owner_group, file_list, opts) do
    chown_args = build_chown_args(owner_group, file_list, opts)

    try do
      {output, exit_code} = System.cmd("chown", chown_args, stderr_to_stdout: true)
      {output, exit_code}
    rescue
      err ->
        {"chown: #{Exception.message(err)}\n", 1}
    end
  end

  @doc """
  Get file ownership information (owner and group) for a file.

  Returns `{:ok, %{uid: uid, gid: gid}}` or `{:error, reason}`.

  This is used for verbose/changes output to show the old owner/group.
  """
  def get_file_info(file_path) do
    case File.stat(file_path) do
      {:ok, stat} ->
        {:ok, %{uid: stat.uid, gid: stat.gid, type: stat.type}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(owner_group_str, file_list, opts) do
    owner_group_spec =
      if opts[:reference] do
        case get_file_info(opts[:reference]) do
          {:ok, info} ->
            {to_string(info.uid), to_string(info.gid)}

          {:error, reason} ->
            IO.puts(:stderr, "chown: cannot stat '#{opts[:reference]}': #{:file.format_error(reason)}")
            System.halt(1)
        end
      else
        parse_owner_group(owner_group_str)
      end

    {output, exit_code} = execute_chown(owner_group_spec, file_list, opts)

    if output != "" and not opts[:silent] do
      IO.write(output)
    end

    if exit_code != 0, do: System.halt(exit_code)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp maybe_add_flag(flag_list, true, flag), do: flag_list ++ [flag]
  defp maybe_add_flag(flag_list, _, _flag), do: flag_list

  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]
  defp normalize_files(nil), do: []

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "chown.json"),
        else: nil
      ),
      "chown.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "chown.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find chown.json spec file"
  end
end
