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
      group NAME:                            — begins a named pattern group

  Lines starting with `#` are comments. Blank lines are ignored.

  ## Pattern Groups

  A `group NAME:` section defines a named set of token patterns that are
  active together during context-sensitive lexing. The lexer maintains a
  stack of active groups and only tries patterns from the group on top of
  the stack. Skip patterns are global and always tried regardless of the
  active group.

  For example, an XML lexer defines a "tag" group with patterns for
  attribute names, equals signs, and attribute values. The callback pushes
  the "tag" group when `<` is matched and pops it when `>` is matched.
  """

  defstruct definitions: [],
            keywords: [],
            skip_definitions: [],
            reserved_keywords: [],
            mode: nil,
            escape_mode: nil,
            groups: %{}

  @type token_definition :: %{
          name: String.t(),
          pattern: String.t(),
          is_regex: boolean(),
          line_number: pos_integer(),
          alias: String.t() | nil
        }

  @typedoc """
  A named set of token definitions that are active together.

  When this group is at the top of the lexer's group stack, only these
  patterns are tried during token matching. The `name` is a lowercase
  identifier (e.g., "tag", "cdata") and `definitions` is an ordered list
  of token definitions (first-match-wins, just like the top-level list).
  """
  @type pattern_group :: %{
          name: String.t(),
          definitions: [token_definition()]
        }

  @type t :: %__MODULE__{
          definitions: [token_definition()],
          keywords: [String.t()],
          skip_definitions: [token_definition()],
          reserved_keywords: [String.t()],
          mode: String.t() | nil,
          escape_mode: String.t() | nil,
          groups: %{optional(String.t()) => pattern_group()}
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

            # Escapes directive — controls escape processing for STRING tokens.
            # "none" means the lexer strips quotes but leaves escape sequences
            # as raw text, deferring escape handling to the semantic layer.
            String.starts_with?(stripped, "escapes:") ->
              escape_mode = stripped |> String.replace_prefix("escapes:", "") |> String.trim()
              {:cont, %{acc | grammar: %{acc.grammar | escape_mode: escape_mode}}}

            # Group headers — "group NAME:" declares a named pattern group.
            # Group names must be lowercase identifiers matching [a-z_][a-z0-9_]*.
            # Reserved names (default, skip, keywords, reserved, errors) are
            # rejected to prevent confusion with built-in sections.
            String.starts_with?(stripped, "group ") and String.ends_with?(stripped, ":") ->
              group_name =
                stripped
                |> String.replace_prefix("group ", "")
                |> String.replace_suffix(":", "")
                |> String.trim()

              cond do
                group_name == "" ->
                  {:halt, {:error, "Line #{line_number}: Missing group name after 'group'"}}

                not Regex.match?(~r/^[a-z_][a-z0-9_]*$/, group_name) ->
                  {:halt,
                   {:error,
                    "Line #{line_number}: Invalid group name: #{inspect(group_name)} " <>
                      "(must be a lowercase identifier like 'tag' or 'cdata')"}}

                group_name in ~w(default skip keywords reserved errors) ->
                  reserved_names = Enum.join(Enum.sort(~w(default errors keywords reserved skip)), ", ")

                  {:halt,
                   {:error,
                    "Line #{line_number}: Reserved group name: #{inspect(group_name)} " <>
                      "(cannot use #{reserved_names})"}}

                Map.has_key?(acc.grammar.groups, group_name) ->
                  {:halt,
                   {:error, "Line #{line_number}: Duplicate group name: #{inspect(group_name)}"}}

                true ->
                  new_group = %{name: group_name, definitions: []}
                  grammar = %{acc.grammar | groups: Map.put(acc.grammar.groups, group_name, new_group)}
                  {:cont, %{acc | grammar: grammar, section: {:group, group_name}}}
              end

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

            # Inside a group section — parse as token definitions into the
            # named group. Uses the same definition parser as skip: and
            # errors: sections, but appends to the group's definition list.
            match?({:group, _}, acc.section) and
                (String.starts_with?(line, " ") or String.starts_with?(line, "\t")) ->
              {:group, current_group_name} = acc.section

              case parse_group_definition(stripped, line_number, current_group_name) do
                {:ok, defn} ->
                  old_group = acc.grammar.groups[current_group_name]
                  updated_group = %{old_group | definitions: old_group.definitions ++ [defn]}
                  grammar = %{acc.grammar | groups: Map.put(acc.grammar.groups, current_group_name, updated_group)}
                  {:cont, %{acc | grammar: grammar}}

                {:error, msg} ->
                  {:halt, {:error, "Line #{line_number}: #{msg}"}}
              end

            # Non-indented line exits any section
            true ->
              section =
                if acc.section in [:keywords, :reserved, :skip] or match?({:group, _}, acc.section),
                  do: :definitions,
                  else: acc.section

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

  When a definition has an alias, both the original name and the alias are
  included. Names from all pattern groups are also included, since group
  tokens can appear in parser grammars.
  """
  @spec token_names(t()) :: MapSet.t(String.t())
  def token_names(%__MODULE__{definitions: definitions, groups: groups}) do
    # Collect definitions from all groups into a single flat list,
    # then merge with top-level definitions.
    group_defs = groups |> Map.values() |> Enum.flat_map(& &1.definitions)
    all_defs = definitions ++ group_defs

    all_defs
    |> Enum.flat_map(fn defn ->
      case defn.alias do
        nil -> [defn.name]
        alias_name -> [defn.name, alias_name]
      end
    end)
    |> MapSet.new()
  end

  @doc """
  Return the set of token names as the parser will see them.

  For definitions with aliases, this returns the alias (not the definition
  name), because that is what the lexer emits and what the parser grammar
  references. For definitions without aliases, this returns the definition
  name. Includes names from all pattern groups.
  """
  @spec effective_token_names(t()) :: MapSet.t(String.t())
  def effective_token_names(%__MODULE__{definitions: definitions, groups: groups}) do
    group_defs = groups |> Map.values() |> Enum.flat_map(& &1.definitions)
    all_defs = definitions ++ group_defs

    all_defs
    |> Enum.map(fn defn ->
      case defn.alias do
        nil -> defn.name
        alias_name -> alias_name
      end
    end)
    |> MapSet.new()
  end

  # -- Private: parse a group definition line --------------------------------
  #
  # Group definitions use the same NAME = /pattern/ or NAME = "literal"
  # format as other sections. This helper validates the line format and
  # delegates to parse_definition/2 for the actual pattern parsing.
  # Provides group-specific error messages on failure.

  defp parse_group_definition(stripped, line_number, group_name) do
    if not String.contains?(stripped, "=") do
      {:error,
       "Expected token definition in group '#{group_name}' (NAME = pattern), got: #{inspect(stripped)}"}
    else
      eq_index = :binary.match(stripped, "=") |> elem(0)
      g_name = stripped |> binary_part(0, eq_index) |> String.trim()
      g_pattern = stripped |> binary_part(eq_index + 1, byte_size(stripped) - eq_index - 1) |> String.trim()

      cond do
        g_name == "" or g_pattern == "" ->
          {:error, "Incomplete definition in group '#{group_name}': #{inspect(stripped)}"}

        true ->
          parse_definition(stripped, line_number)
      end
    end
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
