defmodule CodingAdventures.GrammarTools.TokenGrammar do
  @moduledoc """
  Parses and validates `.tokens` files.

  A `.tokens` file is a declarative description of the lexical grammar of a
  language. It lists every token the lexer should recognize, in priority order
  (first match wins), along with optional sections for keywords, reserved
  words, skip patterns, and lexer mode configuration.

  ## File Format

  Each non-blank, non-comment line has one of these forms:

      TOKEN_NAME = /regex_pattern/           — regex-based token
      TOKEN_NAME = "literal_string"          — literal-string token
      TOKEN_NAME = /regex/ -> ALIAS          — emits token type ALIAS
      TOKEN_NAME = "literal" -> ALIAS        — same for literals
      mode: indentation                      — sets the lexer mode
      keywords:                              — begins keywords section
      reserved:                              — begins reserved keywords section
      skip:                                  — begins skip patterns section

  Lines starting with `#` are comments. Blank lines are ignored.
  """

  defstruct definitions: [],
            keywords: [],
            skip_definitions: [],
            reserved_keywords: [],
            mode: nil

  @type token_definition :: %{
          name: String.t(),
          pattern: String.t(),
          is_regex: boolean(),
          line_number: pos_integer(),
          alias: String.t() | nil
        }

  @type t :: %__MODULE__{
          definitions: [token_definition()],
          keywords: [String.t()],
          skip_definitions: [token_definition()],
          reserved_keywords: [String.t()],
          mode: String.t() | nil
        }

  @doc """
  Parse the text of a `.tokens` file into a `TokenGrammar` struct.

  Returns `{:ok, grammar}` on success, `{:error, message}` on failure.
  """
  @spec parse(String.t()) :: {:ok, t()} | {:error, String.t()}
  def parse(source) do
    lines = String.split(source, "\n")

    result =
      lines
      |> Enum.with_index(1)
      |> Enum.reduce_while(
        %{grammar: %__MODULE__{}, section: :definitions},
        fn {raw_line, line_number}, acc ->
          line = String.trim_trailing(raw_line)
          stripped = String.trim(line)

          cond do
            # Blank lines and comments
            stripped == "" or String.starts_with?(stripped, "#") ->
              {:cont, acc}

            # Mode directive
            String.starts_with?(stripped, "mode:") ->
              mode = stripped |> String.replace_prefix("mode:", "") |> String.trim()
              {:cont, %{acc | grammar: %{acc.grammar | mode: mode}}}

            # Section headers
            stripped in ["keywords:", "keywords :"] ->
              {:cont, %{acc | section: :keywords}}

            stripped in ["reserved:", "reserved :"] ->
              {:cont, %{acc | section: :reserved}}

            stripped in ["skip:", "skip :"] ->
              {:cont, %{acc | section: :skip}}

            # Inside a section — indented lines are section entries
            acc.section in [:keywords, :reserved] and
                (String.starts_with?(line, " ") or String.starts_with?(line, "\t")) ->
              word = stripped

              case acc.section do
                :keywords ->
                  grammar = %{acc.grammar | keywords: acc.grammar.keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}

                :reserved ->
                  grammar = %{acc.grammar | reserved_keywords: acc.grammar.reserved_keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}
              end

            # Inside skip section — parse as token definitions into skip list
            acc.section == :skip and
                (String.starts_with?(line, " ") or String.starts_with?(line, "\t")) ->
              case parse_definition(stripped, line_number) do
                {:ok, defn} ->
                  grammar = %{acc.grammar | skip_definitions: acc.grammar.skip_definitions ++ [defn]}
                  {:cont, %{acc | grammar: grammar}}

                {:error, msg} ->
                  {:halt, {:error, "Line #{line_number}: #{msg}"}}
              end

            # Non-indented line exits any section
            true ->
              section =
                if acc.section in [:keywords, :reserved, :skip], do: :definitions, else: acc.section

              acc = %{acc | section: section}

              case parse_definition(stripped, line_number) do
                {:ok, defn} ->
                  grammar = %{acc.grammar | definitions: acc.grammar.definitions ++ [defn]}
                  {:cont, %{acc | grammar: grammar}}

                {:error, msg} ->
                  {:halt, {:error, "Line #{line_number}: #{msg}"}}
              end
          end
        end
      )

    case result do
      {:error, msg} -> {:error, msg}
      %{grammar: grammar} -> {:ok, grammar}
    end
  end

  @doc """
  Return the set of all defined token names.
  """
  @spec token_names(t()) :: MapSet.t(String.t())
  def token_names(%__MODULE__{definitions: definitions}) do
    definitions
    |> Enum.map(& &1.name)
    |> MapSet.new()
  end

  # -- Private: parse a single token definition line -------------------------

  defp parse_definition(line, line_number) do
    case String.split(line, "=", parts: 2) do
      [name_part, pattern_part] ->
        name = String.trim(name_part)
        rest = String.trim(pattern_part)

        if name == "" do
          {:error, "Missing token name before '='"}
        else
          parse_pattern_with_alias(name, rest, line_number)
        end

      _ ->
        {:error, "Expected token definition (NAME = pattern), got: #{inspect(line)}"}
    end
  end

  defp parse_pattern_with_alias(name, rest, line_number) do
    # Check for alias: pattern -> ALIAS
    {pattern_str, alias_name} =
      case String.split(rest, "->", parts: 2) do
        [pat, ali] -> {String.trim(pat), String.trim(ali)}
        _ -> {rest, nil}
      end

    parse_pattern(name, pattern_str, line_number, alias_name)
  end

  defp parse_pattern(name, pattern_str, line_number, alias_name) do
    cond do
      String.starts_with?(pattern_str, "/") and String.ends_with?(pattern_str, "/") ->
        body = String.slice(pattern_str, 1..-2//1)

        if body == "" do
          {:error, "Empty regex pattern for token '#{name}'"}
        else
          {:ok,
           %{
             name: name,
             pattern: body,
             is_regex: true,
             line_number: line_number,
             alias: alias_name
           }}
        end

      String.starts_with?(pattern_str, "\"") and String.ends_with?(pattern_str, "\"") ->
        body = String.slice(pattern_str, 1..-2//1)

        if body == "" do
          {:error, "Empty literal pattern for token '#{name}'"}
        else
          {:ok,
           %{
             name: name,
             pattern: body,
             is_regex: false,
             line_number: line_number,
             alias: alias_name
           }}
        end

      true ->
        {:error,
         "Pattern for token '#{name}' must be /regex/ or \"literal\", got: #{inspect(pattern_str)}"}
    end
  end
end
