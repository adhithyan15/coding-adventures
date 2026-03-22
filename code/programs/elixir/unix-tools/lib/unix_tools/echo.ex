defmodule UnixTools.Echo do
  @moduledoc """
  echo -- display a line of text.

  ## What This Program Does

  This is a reimplementation of the `echo` utility in Elixir. It writes
  its arguments to standard output, separated by spaces, followed by a
  newline (unless `-n` is specified).

  ## How echo Works

  At its core, echo is simple:

      echo hello world    =>    "hello world\\n"

  The arguments are joined with spaces and a newline is appended.
  Three flags modify this behavior:

  - `-n`: Suppress the trailing newline. Useful when building prompts
          or composing output with other commands.

  - `-e`: Enable interpretation of backslash escapes. Without this flag,
          `\\n` is printed literally as two characters. With `-e`, it
          becomes an actual newline character.

  - `-E`: Disable escape interpretation (the default). This exists so
          you can explicitly override a previous `-e` in an alias or
          script.

  ## Backslash Escape Table

  When `-e` is active, the following escape sequences are interpreted:

      Escape    Meaning              ASCII Code
      ------    -------              ----------
      \\\\        Backslash            0x5C
      \\a        Alert (bell)         0x07
      \\b        Backspace            0x08
      \\f        Form feed            0x0C
      \\n        Newline              0x0A
      \\r        Carriage return      0x0D
      \\t        Horizontal tab       0x09
      \\0NNN     Octal value          (up to 3 digits)
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  ## How It Works

  1. Parse arguments with CLI Builder.
  2. Handle --help and --version.
  3. Join positional arguments with spaces.
  4. If -e is set, interpret escape sequences.
  5. If -n is NOT set, append a newline.
  6. Write to stdout.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["echo" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        # -----------------------------------------------------------------------
        # Business logic: join arguments, apply flags, output.
        # -----------------------------------------------------------------------

        # The "strings" argument is variadic, so it comes as a list.
        # If no arguments were given, it will be an empty list or nil.
        strings = arguments["strings"] || []

        # Normalize to a list if it's a single string.
        strings = if is_list(strings), do: strings, else: [strings]

        # Join all arguments with a single space, just like the real echo.
        output = Enum.join(strings, " ")

        # If -e is set (and -E is not), interpret backslash escapes.
        output =
          if flags["enable_escapes"] do
            interpret_escapes(output)
          else
            output
          end

        # Write the output, with or without trailing newline.
        if flags["no_newline"] do
          IO.write(output)
        else
          IO.puts(output)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "echo: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Escape Sequence Processing
  # ---------------------------------------------------------------------------

  @doc """
  Interpret backslash escape sequences in a string.

  This function processes escape sequences by walking through the string
  character by character. When a backslash is found, the next character
  determines which escape to emit.

  ## Algorithm

  We use a recursive approach with an accumulator, which is idiomatic
  Elixir. The function pattern-matches on the head of the binary to
  identify escape sequences.

  The base case is an empty string, which returns the accumulated output.
  Each recursive case handles one escape sequence or one literal character.
  """
  def interpret_escapes(input) do
    do_interpret_escapes(input, [])
    |> Enum.reverse()
    |> IO.iodata_to_binary()
  end

  # Base case: empty string, return accumulator.
  defp do_interpret_escapes("", acc), do: acc

  # Backslash followed by a recognized escape character.
  defp do_interpret_escapes(<<?\\, ?\\, rest::binary>>, acc),
    do: do_interpret_escapes(rest, ["\\" | acc])

  defp do_interpret_escapes(<<?\\, ?a, rest::binary>>, acc),
    do: do_interpret_escapes(rest, [<<7>> | acc])

  defp do_interpret_escapes(<<?\\, ?b, rest::binary>>, acc),
    do: do_interpret_escapes(rest, [<<8>> | acc])

  defp do_interpret_escapes(<<?\\, ?f, rest::binary>>, acc),
    do: do_interpret_escapes(rest, [<<12>> | acc])

  defp do_interpret_escapes(<<?\\, ?n, rest::binary>>, acc),
    do: do_interpret_escapes(rest, ["\n" | acc])

  defp do_interpret_escapes(<<?\\, ?r, rest::binary>>, acc),
    do: do_interpret_escapes(rest, ["\r" | acc])

  defp do_interpret_escapes(<<?\\, ?t, rest::binary>>, acc),
    do: do_interpret_escapes(rest, ["\t" | acc])

  # Octal escape: \0 followed by up to 3 octal digits.
  defp do_interpret_escapes(<<?\\, ?0, rest::binary>>, acc) do
    {octal_digits, remaining} = consume_octal(rest, 3, [])

    char =
      if octal_digits == [] do
        # \0 with no digits = null character
        <<0>>
      else
        value =
          octal_digits
          |> Enum.reverse()
          |> List.to_string()
          |> String.to_integer(8)

        <<value>>
      end

    do_interpret_escapes(remaining, [char | acc])
  end

  # Backslash followed by an unrecognized character: emit both literally.
  defp do_interpret_escapes(<<?\\, char, rest::binary>>, acc),
    do: do_interpret_escapes(rest, [<<char>>, "\\" | acc])

  # Regular character: emit as-is.
  defp do_interpret_escapes(<<char, rest::binary>>, acc),
    do: do_interpret_escapes(rest, [<<char>> | acc])

  # ---------------------------------------------------------------------------
  # Helper: consume octal digits
  # ---------------------------------------------------------------------------

  @doc false
  defp consume_octal(<<digit, rest::binary>>, remaining, acc)
       when remaining > 0 and digit >= ?0 and digit <= ?7 do
    consume_octal(rest, remaining - 1, [<<digit>> | acc])
  end

  defp consume_octal(input, _remaining, acc), do: {acc, input}

  # ---------------------------------------------------------------------------
  # Helpers: spec file resolution
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "echo.json"),
        else: nil
      ),
      "echo.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "echo.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find echo.json spec file"
  end
end
