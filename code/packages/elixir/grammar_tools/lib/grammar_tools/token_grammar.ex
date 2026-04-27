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
            layout_keywords: [],
            context_keywords: [],
            soft_keywords: [],
            mode: nil,
            escape_mode: nil,
            groups: %{},
            case_sensitive: true,
            error_definitions: [],
            version: 0,
            case_insensitive: false

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
          layout_keywords: [String.t()],
          context_keywords: [String.t()],
          soft_keywords: [String.t()],
          mode: String.t() | nil,
          escape_mode: String.t() | nil,
          groups: %{optional(String.t()) => pattern_group()},
          case_sensitive: boolean(),
          error_definitions: [token_definition()],
          version: non_neg_integer(),
          case_insensitive: boolean()
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
            # Blank lines
            stripped == "" ->
              {:cont, acc}

            # Magic comments — lines starting with "# @key value"
            # These configure grammar-level options such as version number
            # and case-insensitivity. Unknown keys are silently ignored for
            # forward compatibility.
            String.starts_with?(stripped, "#") and
                Regex.match?(~r/^#\s*@\w+/, stripped) ->
              case Regex.run(~r/^#\s*@(\w+)\s*(.*)$/, stripped) do
                [_, key, value] ->
                  grammar =
                    case key do
                      "version" ->
                        case Integer.parse(String.trim(value)) do
                          {n, _} -> %{acc.grammar | version: n}
                          :error -> acc.grammar
                        end

                      "case_insensitive" ->
                        ci = String.trim(value) == "true"
                        %{acc.grammar | case_insensitive: ci}

                      _ ->
                        # Unknown magic comment key — silently ignore
                        acc.grammar
                    end

                  {:cont, %{acc | grammar: grammar}}

                _ ->
                  {:cont, acc}
              end

            # Plain comments
            String.starts_with?(stripped, "#") ->
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

            # Case-sensitivity directive — controls whether the lexer matches
            # patterns case-sensitively. When false, the lexer lowercases the
            # source text before matching. Used by case-insensitive languages
            # like VHDL or SQL.
            String.starts_with?(stripped, "case_sensitive:") ->
              cs_value = stripped |> String.replace_prefix("case_sensitive:", "") |> String.trim() |> String.downcase()

              case cs_value do
                v when v in ["true", "false"] ->
                  {:cont, %{acc | grammar: %{acc.grammar | case_sensitive: v == "true"}}}

                _ ->
                  {:halt, {:error, "Line #{line_number}: Invalid value for 'case_sensitive:': #{inspect(cs_value)} (expected 'true' or 'false')"}}
              end

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

                group_name in ~w(default skip keywords reserved errors layout_keywords context_keywords soft_keywords) ->
                  reserved_names = Enum.join(Enum.sort(~w(context_keywords default errors keywords layout_keywords reserved skip soft_keywords)), ", ")

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

            stripped in ["errors:", "errors :"] ->
              {:cont, %{acc | section: :errors}}

            stripped in ["context_keywords:", "context_keywords :"] ->
              {:cont, %{acc | section: :context_keywords}}

            stripped in ["layout_keywords:", "layout_keywords :"] ->
              {:cont, %{acc | section: :layout_keywords}}

            stripped in ["soft_keywords:", "soft_keywords :"] ->
              {:cont, %{acc | section: :soft_keywords}}

            # Inside a section �� indented lines are section entries
            acc.section in [:keywords, :reserved, :context_keywords, :layout_keywords, :soft_keywords] and
                (String.starts_with?(line, " ") or String.starts_with?(line, "\t")) ->
              word = stripped

              case acc.section do
                :keywords ->
                  grammar = %{acc.grammar | keywords: acc.grammar.keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}

                :reserved ->
                  grammar = %{acc.grammar | reserved_keywords: acc.grammar.reserved_keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}

                :context_keywords ->
                  grammar = %{acc.grammar | context_keywords: acc.grammar.context_keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}

                :layout_keywords ->
                  grammar = %{acc.grammar | layout_keywords: acc.grammar.layout_keywords ++ [word]}
                  {:cont, %{acc | grammar: grammar}}

                :soft_keywords ->
                  grammar = %{acc.grammar | soft_keywords: acc.grammar.soft_keywords ++ [word]}
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

            # Inside errors section — parse as token definitions for error recovery.
            # Error patterns are tried as a fallback when no normal token matches.
            # Example: BAD_STRING for unclosed strings in CSS allows graceful degradation.
            acc.section == :errors and
                (String.starts_with?(line, " ") or String.starts_with?(line, "\t")) ->
              case parse_definition(stripped, line_number) do
                {:ok, defn} ->
                  grammar = %{acc.grammar | error_definitions: acc.grammar.error_definitions ++ [defn]}
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
                if acc.section in [:keywords, :reserved, :skip, :errors, :context_keywords, :layout_keywords, :soft_keywords] or match?({:group, _}, acc.section),
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

  @doc """
  Validate a parsed `TokenGrammar` for common problems.

  This is a *lint* pass, not a parse pass — the grammar has already been
  parsed successfully. We look for semantic issues that would cause problems
  downstream:

  - **Duplicate token names** — two definitions with the same name.
  - **Empty patterns** — should have been caught during parsing, but we
    double-check here.
  - **Invalid regex** — a pattern that `Regex.compile/1` cannot compile.
  - **Non-UPPER_CASE names** — token names should be UPPER_CASE by convention.
  - **Non-UPPER_CASE aliases** — alias names follow the same convention.
  - **Unknown mode** — only `"indentation"` is currently supported.
  - **Unknown escape_mode** — only `"none"` is currently supported.

  The same checks are applied to `skip_definitions` and `error_definitions`.

  Returns a list of issue strings. An empty list means no problems found.
  """
  @spec validate_token_grammar(t()) :: [String.t()]
  def validate_token_grammar(%__MODULE__{} = grammar) do
    issues = []

    # Validate regular definitions
    issues = issues ++ validate_definitions(grammar.definitions, "token")

    # Validate skip definitions
    issues = issues ++ validate_definitions(grammar.skip_definitions, "skip pattern")

    # Validate error definitions
    issues = issues ++ validate_definitions(grammar.error_definitions, "error pattern")

    # Validate mode — only "indentation" and "layout" are supported currently
    issues =
      if grammar.mode != nil and grammar.mode not in ["indentation", "layout"] do
        issues ++ ["Unknown lexer mode '#{grammar.mode}' (only 'indentation' and 'layout' are supported)"]
      else
        issues
      end

    issues =
      if grammar.mode == "layout" and grammar.layout_keywords == [] do
        issues ++ ["Layout mode requires a non-empty layout_keywords section"]
      else
        issues
      end

    # Validate escape_mode — only "none" is supported currently
    issues =
      if grammar.escape_mode != nil and grammar.escape_mode != "none" do
        issues ++ ["Unknown escape mode '#{grammar.escape_mode}' (only 'none' is supported)"]
      else
        issues
      end

    # Validate pattern groups — name format, emptiness, and definitions within
    issues =
      Enum.reduce(grammar.groups, issues, fn {group_name, group}, acc ->
        # Group name format check (parser rejects bad names, but belt-and-suspenders)
        acc =
          if not Regex.match?(~r/^[a-z_][a-z0-9_]*$/, group_name) do
            acc ++ ["Invalid group name '#{group_name}' (must be a lowercase identifier)"]
          else
            acc
          end

        # Empty group warning
        acc =
          if group.definitions == [] do
            acc ++ ["Empty pattern group '#{group_name}' (has no token definitions)"]
          else
            acc
          end

        # Validate definitions within the group
        acc ++ validate_definitions(group.definitions, "group '#{group_name}' token")
      end)

    issues
  end

  # -- Private: validate a list of token definitions -------------------------
  #
  # Shared logic for regular, skip, and error definitions. Returns a list of
  # issue strings. Checks: duplicate names, empty patterns, invalid regexes,
  # and naming conventions (UPPER_CASE names and aliases).

  defp validate_definitions(definitions, label) do
    {issues, _seen} =
      Enum.reduce(definitions, {[], %{}}, fn defn, {issues, seen} ->
        issues =
          # --- Duplicate name check ---
          if Map.has_key?(seen, defn.name) do
            first_line = seen[defn.name]
            issues ++
              [
                "Line #{defn.line_number}: Duplicate #{label} name '#{defn.name}' " <>
                  "(first defined on line #{first_line})"
              ]
          else
            issues
          end

        seen = Map.put_new(seen, defn.name, defn.line_number)

        # --- Empty pattern check ---
        issues =
          if defn.pattern == "" do
            issues ++ ["Line #{defn.line_number}: Empty pattern for #{label} '#{defn.name}'"]
          else
            issues
          end

        # --- Invalid regex check ---
        issues =
          if defn.is_regex do
            case Regex.compile(defn.pattern) do
              {:ok, _} ->
                issues

              {:error, {reason, _pos}} ->
                issues ++
                  [
                    "Line #{defn.line_number}: Invalid regex for #{label} '#{defn.name}': #{reason}"
                  ]
            end
          else
            issues
          end

        # --- Naming convention: UPPER_CASE ---
        issues =
          if defn.name != String.upcase(defn.name) do
            issues ++
              ["Line #{defn.line_number}: Token name '#{defn.name}' should be UPPER_CASE"]
          else
            issues
          end

        # --- Alias convention: UPPER_CASE ---
        issues =
          if defn.alias != nil and defn.alias != String.upcase(defn.alias) do
            issues ++
              [
                "Line #{defn.line_number}: Alias '#{defn.alias}' for token '#{defn.name}' " <>
                  "should be UPPER_CASE"
              ]
          else
            issues
          end

        {issues, seen}
      end)

    issues
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
    #
    # We must be careful not to confuse "->" inside a regex pattern with
    # the alias separator. Strategy: find the closing delimiter first
    # (/ for regex, " for literal), then check for -> in the remainder.
    {pattern_str, alias_name} = split_pattern_and_alias(rest)
    parse_pattern(name, pattern_str, line_number, alias_name)
  end

  # Find the closing delimiter of the pattern, then check for -> alias
  # in whatever follows. This avoids incorrectly splitting on -> that
  # appears inside a regex (e.g., /([^-]|-(?!->))+/).
  defp split_pattern_and_alias(rest) do
    cond do
      String.starts_with?(rest, "/") ->
        # Regex pattern — find the closing /
        # Start searching from index 1 (after the opening /)
        close_idx = find_closing_slash(rest, 1) || find_closing_slash_fallback(rest)

        case close_idx do
          nil ->
            # No closing slash found — return as-is, let parse_pattern error
            {rest, nil}

          close_idx ->
            pattern = String.slice(rest, 0, close_idx + 1)
            remainder = String.trim(String.slice(rest, (close_idx + 1)..-1//1))
            extract_alias(pattern, remainder)
        end

      String.starts_with?(rest, "\"") ->
        # Literal pattern — find the closing "
        case find_closing_quote(rest, 1) do
          nil ->
            {rest, nil}

          close_idx ->
            pattern = String.slice(rest, 0, close_idx + 1)
            remainder = String.trim(String.slice(rest, (close_idx + 1)..-1//1))
            extract_alias(pattern, remainder)
        end

      true ->
        # Neither regex nor literal — return as-is for error handling
        {rest, nil}
    end
  end

  # Find the index of the closing / in a regex pattern (skipping escaped ones).
  # Tracks bracket depth so that / inside [...] character classes is not
  # mistaken for the closing delimiter.
  defp find_closing_slash(str, idx, in_bracket \\ false)

  defp find_closing_slash(str, idx, _in_bracket) when idx >= byte_size(str), do: nil

  defp find_closing_slash(str, idx, in_bracket) do
    ch = binary_part(str, idx, 1)

    cond do
      ch == "\\" and idx + 1 < byte_size(str) ->
        # Escaped character — skip next
        find_closing_slash(str, idx + 2, in_bracket)

      ch == "[" and not in_bracket ->
        find_closing_slash(str, idx + 1, true)

      ch == "]" and in_bracket ->
        find_closing_slash(str, idx + 1, false)

      ch == "/" and not in_bracket ->
        idx

      true ->
        find_closing_slash(str, idx + 1, in_bracket)
    end
  end

  # Fallback version called when the bracket-aware scan returns nil.
  # Used by split_pattern_and_alias to handle patterns with unclosed brackets.
  defp find_closing_slash_fallback(str) do
    # Find the last / in the string as a best-effort parse
    case :binary.matches(str, "/") do
      [] -> nil
      matches ->
        {last_pos, _} = List.last(matches)
        if last_pos > 0, do: last_pos, else: nil
    end
  end

  # Find the index of the closing " in a literal pattern (skipping escaped ones)
  defp find_closing_quote(str, idx) when idx >= byte_size(str), do: nil

  defp find_closing_quote(str, idx) do
    ch = binary_part(str, idx, 1)

    cond do
      ch == "\\" and idx + 1 < byte_size(str) ->
        find_closing_quote(str, idx + 2)

      ch == "\"" ->
        idx

      true ->
        find_closing_quote(str, idx + 1)
    end
  end

  # Extract alias from remainder after closing delimiter
  defp extract_alias(pattern, remainder) do
    if String.starts_with?(remainder, "->") do
      alias_name = String.trim(String.slice(remainder, 2..-1//1))

      if alias_name == "" do
        {pattern, nil}
      else
        {pattern, alias_name}
      end
    else
      {pattern, nil}
    end
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
