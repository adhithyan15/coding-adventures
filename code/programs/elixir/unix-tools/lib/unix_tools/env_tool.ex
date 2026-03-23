defmodule UnixTools.EnvTool do
  @moduledoc """
  env -- run a program in a modified environment.

  ## What This Program Does

  This is a reimplementation of the GNU `env` utility in Elixir. It prints
  the current environment or runs a command with a modified environment.

  ## How env Works

  With no arguments, env prints all environment variables:

      env                     =>   HOME=/Users/alice
                                   PATH=/usr/bin:...
                                   SHELL=/bin/zsh

  With arguments, env can modify the environment before running a command:

      env FOO=bar bash        =>   runs bash with FOO=bar added
      env -i bash             =>   runs bash with an empty environment
      env -u HOME bash        =>   runs bash without the HOME variable

  ## Modifier Flags

  | Flag | Effect                                          |
  |------|-------------------------------------------------|
  | -i   | Start with a completely empty environment        |
  | -u   | Remove (unset) a specific variable              |
  | -0   | End output lines with NUL instead of newline     |
  | -C   | Change working directory before running command   |

  ## Argument Parsing

  env uses a unique argument style. Arguments that contain `=` are treated
  as environment variable assignments. The first argument without `=` starts
  the command:

      env A=1 B=2 my_command arg1 arg2
      ^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^
      assignments  command + its args

  ## Implementation Approach

  1. `parse_assignments/1` separates NAME=VALUE pairs from the command.
  2. `build_environment/3` constructs the final environment map.
  3. `format_environment/2` formats env vars for display.
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

    case Parser.parse(spec_path, ["env" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          ignore_env: !!flags["ignore_environment"],
          unset_vars: normalize_list(flags["unset"]),
          null_terminator: !!flags["null"],
          chdir: flags["chdir"]
        }

        raw_args = normalize_list(arguments["assignments_and_command"])
        {assignments, command_with_args} = parse_assignments(raw_args)

        run(assignments, command_with_args, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "env: #{err.message}")
        end)

        System.halt(125)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Separate NAME=VALUE assignments from the command and its arguments.

  Arguments containing `=` (before the first non-assignment) are treated
  as environment variable assignments. The first argument without `=`
  starts the command.

  ## Examples

      iex> UnixTools.EnvTool.parse_assignments(["A=1", "B=2", "echo", "hello"])
      {[{"A", "1"}], ["echo", "hello"]}

      iex> UnixTools.EnvTool.parse_assignments(["echo", "A=1"])
      {[], ["echo", "A=1"]}

      iex> UnixTools.EnvTool.parse_assignments(["FOO=bar"])
      {[{"FOO", "bar"}], []}
  """
  def parse_assignments(args) do
    {assignments, cmd_parts, _done} =
      Enum.reduce(args, {[], [], false}, fn arg, {assigns, cmd, done} ->
        if done do
          {assigns, cmd ++ [arg], true}
        else
          case parse_single_assignment(arg) do
            {:ok, key, value} ->
              {assigns ++ [{key, value}], cmd, false}

            :not_assignment ->
              {assigns, cmd ++ [arg], true}
          end
        end
      end)

    {assignments, cmd_parts}
  end

  @doc """
  Try to parse a single string as a NAME=VALUE assignment.

  Returns `{:ok, name, value}` if the string contains `=` and the
  part before `=` looks like a valid environment variable name.
  Returns `:not_assignment` otherwise.

  ## Rules for Valid Variable Names

  - Must start with a letter or underscore
  - Can contain letters, digits, and underscores
  - The `=` must not be the first character

  ## Examples

      iex> UnixTools.EnvTool.parse_single_assignment("HOME=/Users/alice")
      {:ok, "HOME", "/Users/alice"}

      iex> UnixTools.EnvTool.parse_single_assignment("echo")
      :not_assignment

      iex> UnixTools.EnvTool.parse_single_assignment("FOO=")
      {:ok, "FOO", ""}
  """
  def parse_single_assignment(arg) do
    case String.split(arg, "=", parts: 2) do
      [key, value] when key != "" ->
        if Regex.match?(~r/^[A-Za-z_][A-Za-z0-9_]*$/, key) do
          {:ok, key, value}
        else
          :not_assignment
        end

      _ ->
        :not_assignment
    end
  end

  @doc """
  Build the final environment map from the current environment,
  assignments, and options.

  ## Steps

  1. Start with current environment (or empty if -i is set).
  2. Remove any variables specified by -u.
  3. Apply NAME=VALUE assignments.

  ## Examples

      iex> current = %{"HOME" => "/home/user", "PATH" => "/usr/bin"}
      iex> UnixTools.EnvTool.build_environment(current, [{"FOO", "bar"}], %{unset_vars: ["PATH"]})
      %{"HOME" => "/home/user", "FOO" => "bar"}
  """
  def build_environment(current_env, assignments, opts) do
    # Step 1: Start with current or empty environment
    base =
      if opts[:ignore_env] do
        %{}
      else
        current_env
      end

    # Step 2: Remove unset variables
    unset_list = opts[:unset_vars] || []

    filtered =
      Enum.reduce(unset_list, base, fn var_name, env_map ->
        Map.delete(env_map, var_name)
      end)

    # Step 3: Apply assignments
    Enum.reduce(assignments, filtered, fn {key, value}, env_map ->
      Map.put(env_map, key, value)
    end)
  end

  @doc """
  Format environment variables for display.

  Each variable is printed as NAME=VALUE, separated by either a
  newline (default) or NUL character (with -0 flag).

  ## Examples

      iex> UnixTools.EnvTool.format_environment(%{"A" => "1", "B" => "2"}, %{})
      "A=1\\nB=2"

      iex> UnixTools.EnvTool.format_environment(%{"A" => "1"}, %{null_terminator: true})
      "A=1\\0"
  """
  def format_environment(env_map, opts) do
    separator = if opts[:null_terminator], do: "\0", else: "\n"

    env_map
    |> Enum.sort_by(fn {key, _value} -> key end)
    |> Enum.map(fn {key, value} -> "#{key}=#{value}" end)
    |> Enum.join(separator)
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(assignments, command_with_args, opts) do
    current_env = System.get_env()
    final_env = build_environment(current_env, assignments, opts)

    case command_with_args do
      [] ->
        # No command: print the environment
        output = format_environment(final_env, opts)
        if output != "", do: IO.write(output <> if(opts[:null_terminator], do: "", else: "\n"))

      [cmd | cmd_args] ->
        # Run the command with the modified environment
        env_list = Enum.map(final_env, fn {k, v} -> {String.to_charlist(k), String.to_charlist(v)} end)

        cmd_opts = [
          env: env_list,
          stderr_to_stdout: true
        ]

        cmd_opts =
          if opts[:chdir] do
            Keyword.put(cmd_opts, :cd, opts[:chdir])
          else
            cmd_opts
          end

        try do
          {output, exit_code} = System.cmd(cmd, cmd_args, cmd_opts)
          if output != "", do: IO.write(output)
          if exit_code != 0, do: System.halt(exit_code)
        rescue
          err ->
            IO.puts(:stderr, "env: '#{cmd}': #{Exception.message(err)}")
            System.halt(127)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_list(nil), do: []
  defp normalize_list(val) when is_binary(val), do: [val]
  defp normalize_list(val) when is_list(val), do: val

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "env.json"),
        else: nil
      ),
      "env.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "env.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find env.json spec file"
  end
end
