defmodule UnixTools.Xargs do
  @moduledoc """
  xargs -- build and execute command lines from standard input.

  ## What This Program Does

  This is a reimplementation of the GNU `xargs` utility in Elixir. It reads
  items from standard input (or a file), splits them into arguments, and
  passes those arguments to a command.

  ## How xargs Works

  At its simplest:

      echo "a b c" | xargs echo   =>   echo a b c   =>   "a b c"

  xargs reads whitespace-delimited items from stdin and appends them as
  arguments to the specified command (default: `/bin/echo`).

  ## Splitting Modes

  | Mode       | Flag | Delimiter                                   |
  |------------|------|---------------------------------------------|
  | Default    | -    | Whitespace (spaces, tabs, newlines)         |
  | Null       | -0   | NUL character (\\0)                         |
  | Custom     | -d   | User-specified single character              |

  ## Batching

  - `-n MAX_ARGS`: Pass at most MAX_ARGS arguments per command invocation.
  - Without -n: All arguments are passed in a single invocation.

  ## Replacement Mode (-I)

  With `-I {}`, each input item is substituted into the command template:

      echo -e "a\\nb" | xargs -I {} echo "item: {}"
      =>  echo "item: a"
      =>  echo "item: b"

  In replacement mode, each input item triggers a separate command.

  ## Other Flags

  - `-t` (verbose): Print the command to stderr before executing.
  - `-r` (no-run-if-empty): Don't run the command if input is empty.

  ## Implementation Approach

  1. `split_input/2` splits raw input into items based on the delimiter mode.
  2. `build_commands/3` groups items into command invocations.
  3. `execute_command/2` runs each command via `System.cmd/3`.
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

    case Parser.parse(spec_path, ["xargs" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          null_delimiter: !!flags["null"],
          delimiter: flags["delimiter"],
          max_args: flags["max_args"],
          replace_str: flags["replace"],
          verbose: !!flags["verbose"],
          no_run_if_empty: !!flags["no_run_if_empty"],
          eof_str: flags["eof"],
          arg_file: flags["arg_file"]
        }

        command_parts = normalize_command(arguments["command"])

        run(command_parts, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "xargs: #{err.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Input Splitting
  # ---------------------------------------------------------------------------

  @doc """
  Split raw input into individual items based on the delimiter mode.

  ## Delimiter Modes

  | Mode       | Behavior                                    |
  |------------|---------------------------------------------|
  | null (-0)  | Split on NUL (\\0) characters               |
  | custom (-d)| Split on the specified character             |
  | default    | Split on whitespace, respecting quotes       |

  ## Examples

      iex> UnixTools.Xargs.split_input("a b c", %{})
      ["a", "b", "c"]

      iex> UnixTools.Xargs.split_input("a\\0b\\0c", %{null_delimiter: true})
      ["a", "b", "c"]

      iex> UnixTools.Xargs.split_input("a,b,c", %{delimiter: ","})
      ["a", "b", "c"]
  """
  def split_input(input, opts) do
    items =
      cond do
        opts[:null_delimiter] ->
          String.split(input, "\0", trim: true)

        opts[:delimiter] ->
          String.split(input, opts[:delimiter], trim: true)

        true ->
          split_whitespace_with_quotes(input)
      end

    # Apply EOF string: stop processing at the eof marker
    items =
      if opts[:eof_str] do
        Enum.take_while(items, fn item -> item != opts[:eof_str] end)
      else
        items
      end

    items
  end

  @doc """
  Split input by whitespace, respecting single and double quotes.

  Quoted strings are kept as single items. Backslash escapes work inside
  double quotes.

  ## Examples

      iex> UnixTools.Xargs.split_whitespace_with_quotes(~s|hello "world peace" test|)
      ["hello", "world peace", "test"]

      iex> UnixTools.Xargs.split_whitespace_with_quotes("a  b  c")
      ["a", "b", "c"]
  """
  def split_whitespace_with_quotes(input) do
    # Simple state machine: iterate through chars, tracking quote state
    {items, current, _in_quote} =
      input
      |> String.graphemes()
      |> Enum.reduce({[], "", nil}, fn char, {items_acc, current_item, quote_char} ->
        cond do
          # Inside a quote — look for the closing quote
          quote_char != nil and char == quote_char ->
            {items_acc, current_item, nil}

          quote_char != nil ->
            {items_acc, current_item <> char, quote_char}

          # Start a quote
          char == "\"" or char == "'" ->
            {items_acc, current_item, char}

          # Whitespace outside quotes — emit current item
          char in [" ", "\t", "\n", "\r"] ->
            if current_item == "" do
              {items_acc, "", nil}
            else
              {items_acc ++ [current_item], "", nil}
            end

          # Regular character
          true ->
            {items_acc, current_item <> char, nil}
        end
      end)

    # Don't forget the last item
    if current == "" do
      items
    else
      items ++ [current]
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Command Building
  # ---------------------------------------------------------------------------

  @doc """
  Build a list of command invocations from items and options.

  Returns a list of `{command, args}` tuples ready for execution.

  ## Batching Rules

  | Mode       | Behavior                                         |
  |------------|--------------------------------------------------|
  | -I str     | One command per item, replacing str in template    |
  | -n N       | At most N items per command                        |
  | default    | All items in one command                           |

  ## Examples

      iex> UnixTools.Xargs.build_commands(["echo"], ["a", "b", "c"], %{})
      [{"echo", ["a", "b", "c"]}]

      iex> UnixTools.Xargs.build_commands(["echo"], ["a", "b", "c"], %{max_args: 2})
      [{"echo", ["a", "b"]}, {"echo", ["c"]}]
  """
  def build_commands(command_parts, items, opts) do
    {cmd, base_args} = split_command(command_parts)

    cond do
      # Replacement mode: one command per item, substituting the placeholder
      opts[:replace_str] ->
        placeholder = opts[:replace_str]
        Enum.map(items, fn item ->
          replaced_args = Enum.map(base_args, fn arg ->
            String.replace(arg, placeholder, item)
          end)
          {cmd, replaced_args}
        end)

      # Batch mode: split items into chunks of max_args
      opts[:max_args] ->
        items
        |> Enum.chunk_every(opts[:max_args])
        |> Enum.map(fn chunk -> {cmd, base_args ++ chunk} end)

      # Default: all items in one command
      true ->
        [{cmd, base_args ++ items}]
    end
  end

  @doc """
  Execute a single command and return its result.

  Uses `System.cmd/3` which runs the command in a subprocess.
  Returns `{output, exit_code}`.

  ## Verbose Mode

  When verbose (-t) is set, the full command is printed to stderr
  before execution.
  """
  def execute_command({cmd, cmd_args}, opts) do
    if opts[:verbose] do
      full_cmd = Enum.join([cmd | cmd_args], " ")
      IO.puts(:stderr, full_cmd)
    end

    try do
      {output, exit_code} = System.cmd(cmd, cmd_args, stderr_to_stdout: true)
      {output, exit_code}
    rescue
      err ->
        IO.puts(:stderr, "xargs: #{cmd}: #{Exception.message(err)}")
        {"", 127}
    end
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(command_parts, opts) do
    raw_input = read_input(opts)
    items = split_input(raw_input, opts)

    # -r flag: don't run if input is empty
    if opts[:no_run_if_empty] and items == [] do
      :ok
    else
      commands = build_commands(command_parts, items, opts)

      exit_code =
        Enum.reduce(commands, 0, fn cmd_tuple, max_exit ->
          {output, code} = execute_command(cmd_tuple, opts)
          if output != "", do: IO.write(output)
          max(max_exit, code)
        end)

      if exit_code != 0, do: System.halt(exit_code)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_input(opts) do
    if opts[:arg_file] do
      case File.read(opts[:arg_file]) do
        {:ok, content} -> content
        {:error, reason} ->
          IO.puts(:stderr, "xargs: #{opts[:arg_file]}: #{:file.format_error(reason)}")
          System.halt(1)
          ""
      end
    else
      case IO.read(:stdio, :eof) do
        {:error, _} -> ""
        :eof -> ""
        data -> data
      end
    end
  end

  defp normalize_command(nil), do: ["echo"]
  defp normalize_command(cmd) when is_binary(cmd), do: [cmd]
  defp normalize_command(cmd) when is_list(cmd) and length(cmd) == 0, do: ["echo"]
  defp normalize_command(cmd) when is_list(cmd), do: cmd

  defp split_command([cmd | cmd_args]), do: {cmd, cmd_args}
  defp split_command([]), do: {"echo", []}

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "xargs.json"),
        else: nil
      ),
      "xargs.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "xargs.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find xargs.json spec file"
  end
end
