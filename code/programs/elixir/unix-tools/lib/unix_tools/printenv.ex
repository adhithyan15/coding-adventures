defmodule UnixTools.Printenv do
  @moduledoc """
  printenv -- print environment variables.

  ## What This Program Does

  This is a reimplementation of the GNU `printenv` utility in Elixir. It
  prints the values of the specified environment variables. If no variables
  are specified, it prints all environment variables as NAME=VALUE pairs.

  ## How printenv Works

      printenv              =>   all variables, one per line (NAME=VALUE)
      printenv HOME         =>   just the value of $HOME
      printenv HOME PATH    =>   values of $HOME and $PATH, one per line

  ## Exit Status

  printenv exits with status 0 if all specified variables are found, or
  with status 1 if any are not found. When printing all variables (no
  arguments), it always exits 0.

  ## printenv vs env

  Both `printenv` and `env` can display environment variables, but they
  differ in purpose:

  - `printenv` is for *reading* variables. It can query specific ones.
  - `env` is for *modifying* the environment before running a command.

  ## NUL Termination (-0)

  With `-0`, output lines are terminated with NUL instead of newline.
  This is useful when piping to `xargs -0`.
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

    case Parser.parse(spec_path, ["printenv" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        null_terminated = !!flags["null"]
        terminator = if null_terminated, do: <<0>>, else: "\n"

        variables = normalize_variables(arguments["variables"])

        if variables == [] do
          # No specific variables: print all as NAME=VALUE pairs, sorted.
          System.get_env()
          |> Enum.sort_by(fn {key, _} -> key end)
          |> Enum.each(fn {key, value} ->
            IO.write("#{key}=#{value}#{terminator}")
          end)
        else
          # Specific variables: print just their values.
          all_found =
            Enum.reduce(variables, true, fn var_name, acc ->
              case System.get_env(var_name) do
                nil ->
                  false

                value ->
                  IO.write(value <> terminator)
                  acc
              end
            end)

          unless all_found do
            System.halt(1)
          end
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "printenv: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_variables(nil), do: []
  defp normalize_variables(vars) when is_list(vars), do: vars
  defp normalize_variables(var) when is_binary(var), do: [var]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "printenv.json"),
        else: nil
      ),
      "printenv.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "printenv.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find printenv.json spec file"
  end
end
