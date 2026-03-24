defmodule CodingAdventures.CliBuilder.Parser do
  @moduledoc """
  Main entry point for CLI Builder argument parsing.

  ## Overview

  `Parser.parse/2` orchestrates the three-phase parsing algorithm described in
  §6 of the spec:

  1. **Phase 1 — Routing** (§6.2): traverse the command routing graph G_cmd to
     find the deepest matching command node. The `DirectedGraph` is used to build
     G_cmd from the spec's `commands` hierarchy.

  2. **Phase 2 — Scanning** (§6.3): walk every non-routing token and classify it
     with `TokenClassifier`. A `ModalStateMachine` tracks the current parse mode
     (`SCANNING`, `FLAG_VALUE`, `END_OF_FLAGS`). Collect flag values and
     positional tokens.

  3. **Phase 3 — Validation** (§6.4): resolve positional tokens to argument slots
     with `PositionalResolver`; validate flag constraints with `FlagValidator`.

  ## Return values

  - `{:ok, %ParseResult{}}` — successful parse
  - `{:ok, %HelpResult{}}` — `--help` or `-h` was seen
  - `{:ok, %VersionResult{}}` — `--version` was seen
  - `{:error, %ParseErrors{}}` — one or more errors

  ## Parsing mode

  The `parsing_mode` field in the spec controls how positional tokens and flags
  interact:

  - `"gnu"` (default) — flags may appear anywhere; `--` ends flag scanning.
  - `"posix"` — the first non-flag, non-subcommand token ends flag scanning.
  - `"subcommand_first"` — the first token is always a subcommand.
  - `"traditional"` — `argv[1]` may be dash-less stacked flags (tar-style).
  """

  alias CodingAdventures.CliBuilder.{
    ParseError,
    ParseErrors,
    ParseResult,
    HelpResult,
    VersionResult,
    SpecLoader,
    TokenClassifier,
    PositionalResolver,
    FlagValidator,
    HelpGenerator
  }

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Parse `argv` against the spec at `spec_file_path`.

  `argv` is the list of command-line arguments as strings, typically
  `System.argv()`. Do NOT include `argv[0]` (the program name); this function
  reads the program name from the spec's `"name"` field. If `argv` does
  include `argv[0]` as its first element and it matches the spec name, it is
  automatically stripped.

  Returns one of:
  - `{:ok, %ParseResult{}}` — successful parse
  - `{:ok, %HelpResult{}}` — help was requested
  - `{:ok, %VersionResult{}}` — version was requested
  - `{:error, %ParseErrors{}}` — parse failure

  ## Example

      iex> alias CodingAdventures.CliBuilder.Parser
      iex> {:ok, result} = Parser.parse_string(echo_spec_json, ["hello", "world"])
      iex> result.arguments["string"]
      ["hello", "world"]
  """
  @spec parse(String.t(), [String.t()]) ::
          {:ok, ParseResult.t() | HelpResult.t() | VersionResult.t()}
          | {:error, ParseErrors.t()}
  def parse(spec_file_path, argv) do
    spec = SpecLoader.load!(spec_file_path)
    do_parse(spec, argv)
  end

  @doc """
  Parse `argv` against a spec supplied as a JSON string (useful for testing
  without files on disk).
  """
  @spec parse_string(String.t(), [String.t()]) ::
          {:ok, ParseResult.t() | HelpResult.t() | VersionResult.t()}
          | {:error, ParseErrors.t()}
  def parse_string(spec_json, argv) do
    spec = SpecLoader.load_from_string!(spec_json)
    do_parse(spec, argv)
  end

  # ---------------------------------------------------------------------------
  # Core parsing pipeline
  # ---------------------------------------------------------------------------

  defp do_parse(spec, argv) do
    program_name = spec["name"]

    # Strip argv[0] if it matches the program name (common when using System.argv()).
    argv =
      case argv do
        [^program_name | rest] -> rest
        other -> other
      end

    # -------------------------------------------------------------------------
    # Phase 1 — Routing
    # -------------------------------------------------------------------------
    # Walk argv and navigate the command graph until we hit a non-subcommand
    # token or run out of tokens.  Flags are skipped during routing.
    # -------------------------------------------------------------------------

    {command_path, remaining_argv, routing_errors} =
      phase1_routing(spec, program_name, argv)

    # If routing produced errors (e.g. unknown subcommand), surface them.
    # We still attempt Phase 2 from where routing left off so we can report
    # additional errors.

    # -------------------------------------------------------------------------
    # Phase 2 — Scanning
    # -------------------------------------------------------------------------
    # Build the active flag set, then walk every token, classifying and
    # collecting flag values and positionals.
    # -------------------------------------------------------------------------

    active_flags = build_active_flags(spec, command_path)
    parsing_mode = spec["parsing_mode"]

    # Apply traditional-mode pre-processing (tar-style: argv[1] without a dash
    # is treated as stacked flags).
    remaining_argv =
      if parsing_mode == "traditional" do
        apply_traditional_mode(remaining_argv, active_flags)
      else
        remaining_argv
      end

    scan_result =
      phase2_scan(remaining_argv, active_flags, parsing_mode, command_path)

    case scan_result do
      {:help, path} ->
        text = HelpGenerator.generate(spec, path)
        {:ok, %HelpResult{text: text, command_path: path}}

      {:version} ->
        {:ok, %VersionResult{version: spec["version"]}}

      {:done, parsed_flags, positional_tokens, scan_errors, explicit_flags} ->
        # -------------------------------------------------------------------------
        # Phase 3 — Validation
        # -------------------------------------------------------------------------

        # Resolve the current command node's argument defs.
        node = resolve_command_node(spec, command_path)
        arg_defs = Map.get(node, "arguments", [])
        excl_groups = Map.get(node, "mutually_exclusive_groups", [])

        # Positional resolution
        {arg_assignments, pos_errors} =
          case PositionalResolver.resolve(positional_tokens, arg_defs, parsed_flags, command_path) do
            {:ok, assignments} -> {assignments, []}
            {:error, errors} -> {%{}, errors}
          end

        # Fill in default values for all flags not present in parsed_flags.
        flags_with_defaults = apply_flag_defaults(parsed_flags, active_flags)

        # Enum value validation
        enum_errors = validate_enum_values(flags_with_defaults, active_flags, command_path)

        # Flag constraint validation.
        # IMPORTANT: pass the original parsed_flags (before defaults are filled
        # in) so that conflict/requires checks only fire for flags the user
        # actually specified.  If we passed flags_with_defaults, every boolean
        # flag that defaulted to false would appear "present", causing false
        # conflicts_with violations (e.g. -e and -E both appearing as false).
        constraint_errors =
          FlagValidator.validate(parsed_flags, active_flags, excl_groups, command_path)

        all_errors =
          routing_errors ++
            scan_errors ++
            pos_errors ++
            enum_errors ++
            constraint_errors

        if Enum.empty?(all_errors) do
          {:ok,
           %ParseResult{
             program: program_name,
             command_path: command_path,
             flags: flags_with_defaults,
             arguments: arg_assignments,
             explicit_flags: explicit_flags
           }}
        else
          {:error,
           %ParseErrors{
             errors: all_errors,
             message: Enum.map_join(all_errors, "\n", & &1.message)
           }}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Phase 1 — Routing
  # ---------------------------------------------------------------------------

  # Build the command routing graph G_cmd and walk argv to find the deepest
  # matching command node.  Returns {command_path, remaining_argv, errors}.
  defp phase1_routing(spec, program_name, argv) do
    # Build a flat command map: "name_or_alias" -> canonical_name for quick lookup.
    # We use a recursive descent through the spec's "commands" tree, remembering
    # the parent context.

    command_path = [program_name]
    {path, remaining, errors} = route(argv, spec, command_path)
    {path, remaining, errors}
  end

  # Recursive router: at each step look at the current token.
  # If it matches a subcommand of the current node, follow it.
  # If it is a flag, pass it through to Phase 2 (do NOT drop it).
  # Otherwise stop.
  #
  # IMPORTANT: Flag tokens (and their values) must NOT be consumed here.
  # They are only passed through so that Phase 2 can parse them.  The old
  # "skip_flag_during_routing" approach silently dropped flag tokens,
  # which caused --version, -m "msg", etc. to disappear before Phase 2.
  #
  # The router collects "passed-through" flag tokens in `deferred` and
  # prepends them back onto `remaining` before returning.
  defp route([], _current_spec_node, command_path), do: {command_path, [], []}

  defp route(argv, current_spec_node, command_path) do
    route_with_deferred(argv, current_spec_node, command_path, [])
  end

  # Inner loop that tracks deferred (flag) tokens seen so far.
  defp route_with_deferred([], _current_spec_node, command_path, deferred) do
    {command_path, Enum.reverse(deferred), []}
  end

  defp route_with_deferred(argv, current_spec_node, command_path, deferred) do
    [token | rest] = argv

    cond do
      token == "--" ->
        # End-of-flags marker: stop routing; prepend all deferred flags then
        # the "--" and everything after it so Phase 2 sees the full stream.
        {command_path, Enum.reverse(deferred) ++ argv, []}

      String.starts_with?(token, "--") or
          (String.starts_with?(token, "-") and token != "-") ->
        # Flag token: defer it (and its inline value if "=" form) so Phase 2
        # can parse it.  We must peek at the NEXT token only to decide if it
        # is a value that should also be deferred, so we do not mistake a
        # flag value like "commit" for a subcommand name.
        {deferred_tokens, remaining_after_flag} =
          collect_flag_and_value(token, rest, current_spec_node)

        route_with_deferred(
          remaining_after_flag,
          current_spec_node,
          command_path,
          Enum.reverse(deferred_tokens) ++ deferred
        )

      true ->
        # Non-flag token: check if it is a subcommand name.
        commands = Map.get(current_spec_node, "commands", [])

        case find_command(token, commands) do
          nil ->
            # Not a subcommand — stop routing; flush deferred back first.
            {command_path, Enum.reverse(deferred) ++ argv, []}

          cmd ->
            canonical = cmd["name"]
            # Subcommand matched: do NOT defer this token (it is consumed by
            # routing). Any previously deferred flag tokens are kept in
            # `deferred` and will be flushed when routing ends.
            route_with_deferred(rest, cmd, command_path ++ [canonical], deferred)
        end
    end
  end

  # Collect a flag token and (if needed) its next-token value into a list.
  # Returns {[token, ...], remaining_after}.
  # For inline-value flags (--flag=val) no extra peeking is needed.
  # For space-separated values (-m "msg") we defer both the flag and the value.
  defp collect_flag_and_value(flag_token, rest, spec_node) do
    if String.contains?(flag_token, "=") do
      # Inline value: no extra token consumed.
      {[flag_token], rest}
    else
      active = build_active_flags_for_node(spec_node)
      flag_def = find_flag_def_for_token(flag_token, active)

      if flag_def != nil and flag_def["type"] not in ["boolean", "count"] do
        # Non-boolean flag: the next token is its value — defer both.
        case rest do
          [value | remainder] -> {[flag_token, value], remainder}
          [] -> {[flag_token], []}
        end
      else
        {[flag_token], rest}
      end
    end
  end

  # Find a command by name or alias in a commands list.
  defp find_command(token, commands) do
    Enum.find(commands, fn cmd ->
      cmd["name"] == token or token in Map.get(cmd, "aliases", [])
    end)
  end

  # Build a quick active flags list for a single spec node (for routing).
  defp build_active_flags_for_node(node) do
    Map.get(node, "flags", []) ++ Map.get(node, "global_flags", [])
  end

  # Find the flag definition that matches a raw token like "--verbose" or "-v".
  defp find_flag_def_for_token(token, active_flags) do
    cond do
      String.starts_with?(token, "--") ->
        name = token |> String.slice(2, String.length(token)) |> String.split("=") |> List.first()
        Enum.find(active_flags, fn f -> f["long"] == name end)

      String.starts_with?(token, "-") ->
        char = String.slice(token, 1, 1)
        Enum.find(active_flags, fn f -> f["short"] == char end)

      true ->
        nil
    end
  end

  # ---------------------------------------------------------------------------
  # Phase 2 — Scanning
  # ---------------------------------------------------------------------------

  # Walk every token in remaining_argv.  Return one of:
  # {:help, command_path}
  # {:version}
  # {:done, parsed_flags, positional_tokens, errors}
  defp phase2_scan(argv, active_flags, parsing_mode, command_path) do
    state = %{
      # Current parse mode: :scanning | :flag_value | :end_of_flags
      mode: :scanning,
      parsed_flags: %{},
      positional_tokens: [],
      errors: [],
      # The flag definition waiting for its value
      pending_flag: nil,
      # Whether to collect all subsequent tokens as positionals (posix after first positional)
      command_path: command_path,
      parsing_mode: parsing_mode,
      # -----------------------------------------------------------------------
      # explicit_flags (v1.1 — Feature 3: Flag Presence Detection)
      #
      # Every time a flag token is consumed from argv, its ID is appended here.
      # This lets callers distinguish "user typed --verbose" from "verbose
      # defaulted to false".  A flag may appear multiple times (e.g. count
      # flags), so the list can contain duplicates.
      # -----------------------------------------------------------------------
      explicit_flags: []
    }

    scan_tokens(argv, active_flags, state)
  end

  defp scan_tokens([], _active_flags, state) do
    # If we end in FLAG_VALUE mode, the last flag had no value.
    state =
      if state.mode == :flag_value and state.pending_flag != nil do
        flag = state.pending_flag

        err = %ParseError{
          error_type: "missing_required_argument",
          message: "Flag #{flag_label(flag)} requires a value but none was provided",
          suggestion: nil,
          context: state.command_path
        }

        %{state | errors: state.errors ++ [err], pending_flag: nil}
      else
        state
      end

    {:done, state.parsed_flags, Enum.reverse(state.positional_tokens), state.errors,
     Enum.reverse(state.explicit_flags)}
  end

  defp scan_tokens([token | rest], active_flags, state) do
    case state.mode do
      :flag_value ->
        # The current token is the value for the pending flag.
        flag = state.pending_flag

        case coerce_flag_value(token, flag) do
          {:ok, value} ->
            new_flags = put_flag_value(state.parsed_flags, flag, value)
            new_explicit = [flag["id"] | state.explicit_flags]
            new_state = %{state | mode: :scanning, parsed_flags: new_flags, pending_flag: nil, explicit_flags: new_explicit}
            scan_tokens(rest, active_flags, new_state)

          {:error, msg} ->
            err = %ParseError{
              error_type: "invalid_value",
              message: msg,
              suggestion: nil,
              context: state.command_path
            }

            new_state = %{
              state
              | mode: :scanning,
                errors: state.errors ++ [err],
                pending_flag: nil
            }

            scan_tokens(rest, active_flags, new_state)
        end

      :end_of_flags ->
        # All remaining tokens are positional, no classification needed.
        new_state = %{state | positional_tokens: [token | state.positional_tokens]}
        scan_tokens(rest, active_flags, new_state)

      :scanning ->
        classified = TokenClassifier.classify(token, active_flags)
        handle_classified(classified, token, rest, active_flags, state)
    end
  end

  # Process a classified token in SCANNING mode.
  defp handle_classified(classified, _raw_token, rest, active_flags, state) do
    case classified do
      :end_of_flags ->
        scan_tokens(rest, active_flags, %{state | mode: :end_of_flags})

      {:long_flag, name} ->
        handle_long_flag(name, rest, active_flags, state)

      {:long_flag_with_value, name, value} ->
        handle_long_flag_with_value(name, value, rest, active_flags, state)

      {:single_dash_long, name} ->
        handle_sdl_flag(name, rest, active_flags, state)

      {:short_flag, char} ->
        handle_short_flag(char, rest, active_flags, state)

      {:short_flag_with_value, char, value} ->
        handle_short_flag_with_value(char, value, rest, active_flags, state)

      {:stacked_flags, chars} ->
        handle_stacked_flags(chars, rest, active_flags, state)

      {:positional, value} ->
        handle_positional(value, rest, active_flags, state)

      {:unknown_flag, token} ->
        handle_unknown_flag(token, active_flags, rest, state)
    end
  end

  # --- Long flags ---

  defp handle_long_flag(name, rest, active_flags, state) do
    case lookup_flag_by_long(name, active_flags) do
      nil ->
        # Check for builtin help/version before reporting unknown.
        cond do
          name == "help" ->
            {:help, state.command_path}

          name == "version" ->
            {:version}

          true ->
            suggestion = fuzzy_match_flag(name, active_flags, :long)

            err = %ParseError{
              error_type: "unknown_flag",
              message: "Unknown flag '--#{name}'#{if suggestion, do: ". Did you mean '--#{suggestion}'?", else: ""}",
              suggestion: suggestion && "--#{suggestion}",
              context: state.command_path
            }

            scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
        end

      flag ->
        cond do
          flag["type"] == "boolean" ->
            new_flags = put_flag_value(state.parsed_flags, flag, true)
            new_explicit = [flag["id"] | state.explicit_flags]
            scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

          flag["type"] == "count" ->
            # ---------------------------------------------------------------
            # Count type (v1.1 — Feature 1)
            #
            # Each occurrence of a count flag increments a counter. Like a
            # boolean flag, it consumes no value token. The counter starts
            # at 0 (the default for absent count flags) and increments by 1
            # for each occurrence.
            # ---------------------------------------------------------------
            new_flags = increment_count_flag(state.parsed_flags, flag)
            new_explicit = [flag["id"] | state.explicit_flags]
            scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

          true ->
            # ---------------------------------------------------------------
            # default_when_present (v1.1 — Feature 2)
            #
            # For enum flags with default_when_present, when the flag appears
            # without a value (e.g. `--color` instead of `--color=always`),
            # we peek at the next token. If it is a valid enum value, consume
            # it; otherwise use default_when_present.
            # ---------------------------------------------------------------
            if flag["type"] == "enum" and flag["default_when_present"] != nil do
              handle_enum_default_when_present(flag, rest, active_flags, state)
            else
              scan_tokens(rest, active_flags, %{state | mode: :flag_value, pending_flag: flag})
            end
        end
    end
  end

  defp handle_long_flag_with_value(name, value, rest, active_flags, state) do
    case lookup_flag_by_long(name, active_flags) do
      nil ->
        suggestion = fuzzy_match_flag(name, active_flags, :long)

        err = %ParseError{
          error_type: "unknown_flag",
          message: "Unknown flag '--#{name}'#{if suggestion, do: ". Did you mean '--#{suggestion}'?", else: ""}",
          suggestion: suggestion && "--#{suggestion}",
          context: state.command_path
        }

        scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})

      flag ->
        case coerce_flag_value(value, flag) do
          {:ok, coerced} ->
            new_flags = put_flag_value(state.parsed_flags, flag, coerced)
            new_explicit = [flag["id"] | state.explicit_flags]
            scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

          {:error, msg} ->
            err = %ParseError{
              error_type: "invalid_value",
              message: msg,
              suggestion: nil,
              context: state.command_path
            }

            scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
        end
    end
  end

  # --- Single-dash-long flags ---

  defp handle_sdl_flag(name, rest, active_flags, state) do
    flag = Enum.find(active_flags, fn f -> f["single_dash_long"] == name end)

    if flag == nil do
      err = %ParseError{
        error_type: "unknown_flag",
        message: "Unknown flag '-#{name}'",
        suggestion: nil,
        context: state.command_path
      }

      scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
    else
      cond do
        flag["type"] == "boolean" ->
          new_flags = put_flag_value(state.parsed_flags, flag, true)
          new_explicit = [flag["id"] | state.explicit_flags]
          scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

        flag["type"] == "count" ->
          new_flags = increment_count_flag(state.parsed_flags, flag)
          new_explicit = [flag["id"] | state.explicit_flags]
          scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

        flag["type"] == "enum" and flag["default_when_present"] != nil ->
          handle_enum_default_when_present(flag, rest, active_flags, state)

        true ->
          scan_tokens(rest, active_flags, %{state | mode: :flag_value, pending_flag: flag})
      end
    end
  end

  # --- Short flags ---

  defp handle_short_flag(char, rest, active_flags, state) do
    case lookup_flag_by_short(char, active_flags) do
      nil ->
        cond do
          char == "h" ->
            {:help, state.command_path}

          true ->
            err = %ParseError{
              error_type: "unknown_flag",
              message: "Unknown flag '-#{char}'",
              suggestion: nil,
              context: state.command_path
            }

            scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
        end

      flag ->
        cond do
          flag["type"] == "boolean" ->
            new_flags = put_flag_value(state.parsed_flags, flag, true)
            new_explicit = [flag["id"] | state.explicit_flags]
            scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

          flag["type"] == "count" ->
            new_flags = increment_count_flag(state.parsed_flags, flag)
            new_explicit = [flag["id"] | state.explicit_flags]
            scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

          flag["type"] == "enum" and flag["default_when_present"] != nil ->
            handle_enum_default_when_present(flag, rest, active_flags, state)

          true ->
            scan_tokens(rest, active_flags, %{state | mode: :flag_value, pending_flag: flag})
        end
    end
  end

  defp handle_short_flag_with_value(char, value, rest, active_flags, state) do
    flag = lookup_flag_by_short(char, active_flags)

    if flag == nil do
      err = %ParseError{
        error_type: "unknown_flag",
        message: "Unknown flag '-#{char}'",
        suggestion: nil,
        context: state.command_path
      }

      scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
    else
      case coerce_flag_value(value, flag) do
        {:ok, coerced} ->
          new_flags = put_flag_value(state.parsed_flags, flag, coerced)
          new_explicit = [flag["id"] | state.explicit_flags]
          scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})

        {:error, msg} ->
          err = %ParseError{
            error_type: "invalid_value",
            message: msg,
            suggestion: nil,
            context: state.command_path
          }

          scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
      end
    end
  end

  # --- Stacked flags ---

  defp handle_stacked_flags(chars, rest, active_flags, state) do
    # Process each char in the stack.  All but possibly the last are boolean
    # or count flags. The last may be non-boolean (it will consume the next token).
    #
    # Count flags (v1.1 Feature 1): in a stack like `-vvv`, each `v` is a
    # separate occurrence that increments the counter.
    {new_state, _} =
      Enum.with_index(chars)
      |> Enum.reduce({state, rest}, fn {char, idx}, {st, remaining} ->
        is_last = idx == length(chars) - 1
        flag = lookup_flag_by_short(char, active_flags)

        if flag == nil do
          err = %ParseError{
            error_type: "unknown_flag",
            message: "Unknown flag '-#{char}' in stack '-#{Enum.join(chars)}'",
            suggestion: nil,
            context: st.command_path
          }

          {%{st | errors: st.errors ++ [err]}, remaining}
        else
          cond do
            flag["type"] == "count" ->
              # Count flags: increment, and record each occurrence
              new_flags = increment_count_flag(st.parsed_flags, flag)
              new_explicit = [flag["id"] | st.explicit_flags]
              {%{st | parsed_flags: new_flags, explicit_flags: new_explicit}, remaining}

            flag["type"] == "boolean" or is_last ->
              new_flags = put_flag_value(st.parsed_flags, flag, true)
              new_explicit = [flag["id"] | st.explicit_flags]
              {%{st | parsed_flags: new_flags, explicit_flags: new_explicit}, remaining}

            true ->
              # Non-boolean, non-count flag in middle of stack — should not
              # happen if TokenClassifier worked correctly, but handle gracefully.
              new_flags = put_flag_value(st.parsed_flags, flag, true)
              new_explicit = [flag["id"] | st.explicit_flags]
              {%{st | parsed_flags: new_flags, explicit_flags: new_explicit}, remaining}
          end
        end
      end)

    # Check if the last flag in the stack was non-boolean/non-count (needs next-token value).
    last_char = List.last(chars)
    last_flag = lookup_flag_by_short(last_char, active_flags)

    if last_flag != nil and last_flag["type"] not in ["boolean", "count"] do
      scan_tokens(rest, active_flags, %{new_state | mode: :flag_value, pending_flag: last_flag})
    else
      scan_tokens(rest, active_flags, new_state)
    end
  end

  # --- Positional tokens ---

  defp handle_positional(value, rest, active_flags, state) do
    new_state = %{state | positional_tokens: [value | state.positional_tokens]}

    # In POSIX mode, the first positional ends flag scanning.
    if state.parsing_mode == "posix" do
      scan_tokens(rest, active_flags, %{new_state | mode: :end_of_flags})
    else
      scan_tokens(rest, active_flags, new_state)
    end
  end

  # --- Unknown flags ---

  defp handle_unknown_flag(token, active_flags, rest, state) do
    # Check for -h shorthand regardless of spec.
    if token == "-h" do
      {:help, state.command_path}
    else
      # Try to produce a helpful suggestion.
      raw_name = String.trim_leading(token, "-")
      suggestion = fuzzy_match_flag(raw_name, active_flags, :any)

      err = %ParseError{
        error_type: "unknown_flag",
        message: "Unknown flag '#{token}'#{if suggestion, do: ". Did you mean '#{suggestion}'?", else: ""}",
        suggestion: suggestion,
        context: state.command_path
      }

      scan_tokens(rest, active_flags, %{state | errors: state.errors ++ [err]})
    end
  end

  # ---------------------------------------------------------------------------
  # Active flags construction
  # ---------------------------------------------------------------------------

  # Build the complete set of flags active after routing to command_path.
  # active_flags = global_flags (if inherited) + flags from every node in path.
  #
  # We walk the command tree level-by-level, tracking the current node so that
  # nested commands (e.g. "git remote add") are found correctly.
  defp build_active_flags(spec, command_path) do
    subcommand_names = Enum.drop(command_path, 1)
    global_flags = spec["global_flags"]

    # Start at the root node (the spec itself as a pseudo-node).
    root_node = %{"commands" => spec["commands"], "flags" => []}

    {node_flags, _final_node, inherit_global} =
      Enum.reduce(subcommand_names, {[], root_node, true}, fn name, {acc_flags, current_node, _inherit} ->
        cmds = Map.get(current_node, "commands", [])
        found = Enum.find(cmds, fn c -> c["name"] == name or name in Map.get(c, "aliases", []) end)

        if found == nil do
          {acc_flags, current_node, true}
        else
          inherit = Map.get(found, "inherit_global_flags", true)
          cmd_flags = Map.get(found, "flags", [])
          {acc_flags ++ cmd_flags, found, inherit}
        end
      end)

    # Root-level flags are included for root-level invocations.
    root_flags = if length(subcommand_names) == 0, do: spec["flags"], else: []

    effective_global = if inherit_global, do: global_flags, else: []
    effective_global ++ root_flags ++ node_flags
  end

  # Resolve the spec node for a given command_path.
  defp resolve_command_node(spec, command_path) do
    subcommand_names = Enum.drop(command_path, 1)

    root = %{
      "flags" => spec["flags"],
      "arguments" => spec["arguments"],
      "commands" => spec["commands"],
      "mutually_exclusive_groups" => spec["mutually_exclusive_groups"]
    }

    Enum.reduce(subcommand_names, root, fn name, current ->
      cmds = Map.get(current, "commands", [])

      case find_command(name, cmds) do
        nil -> current
        cmd -> cmd
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Flag value helpers
  # ---------------------------------------------------------------------------

  # Coerce a raw string value to the type required by a flag.
  defp coerce_flag_value(value, flag) do
    case flag["type"] do
      "boolean" ->
        case value do
          "true" -> {:ok, true}
          "false" -> {:ok, false}
          _ -> {:error, "Invalid boolean value for #{flag_label(flag)}: #{inspect(value)}"}
        end

      "integer" ->
        case Integer.parse(value) do
          {n, ""} ->
            # -----------------------------------------------------------------
            # int64 range validation (v1.1 — Feature 4)
            #
            # Elixir's Integer.parse handles arbitrary-precision integers, but
            # CLI Builder specifies int64 semantics. Values outside the signed
            # 64-bit range [-2^63, 2^63-1] are rejected as invalid.
            # -----------------------------------------------------------------
            int64_min = -9_223_372_036_854_775_808
            int64_max =  9_223_372_036_854_775_807

            if n < int64_min or n > int64_max do
              {:error,
               "Value #{inspect(value)} for #{flag_label(flag)} is outside int64 range " <>
                 "[#{int64_min}, #{int64_max}]"}
            else
              {:ok, n}
            end

          _ ->
            {:error, "Invalid integer for #{flag_label(flag)}: #{inspect(value)}"}
        end

      "float" ->
        case Float.parse(value) do
          {f, ""} ->
            {:ok, f}

          _ ->
            case Integer.parse(value) do
              {n, ""} -> {:ok, n * 1.0}
              _ -> {:error, "Invalid float for #{flag_label(flag)}: #{inspect(value)}"}
            end
        end

      "enum" ->
        valid = flag["enum_values"]

        if value in valid do
          {:ok, value}
        else
          {:error,
           "Invalid value #{inspect(value)} for #{flag_label(flag)}. Must be one of: #{Enum.join(valid, ", ")}"}
        end

      _ ->
        # string, path, file, directory — return as-is (existence not checked here).
        if value == "" do
          {:error, "Value for #{flag_label(flag)} must not be empty"}
        else
          {:ok, value}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # default_when_present handler (v1.1 — Feature 2)
  # ---------------------------------------------------------------------------

  # Handle an enum flag that has `default_when_present` set. When such a flag
  # appears without an inline `=value`, we peek at the next token:
  #
  # - If the next token is a valid enum value, consume it as the flag's value.
  # - Otherwise, use `default_when_present` and leave the token for Phase 2
  #   to process (it might be another flag or a positional argument).
  #
  # This mirrors the behaviour of `--color[=WHEN]` in GNU coreutils: you can
  # write `--color` (gets "always"), `--color=auto`, or `--color auto`.
  defp handle_enum_default_when_present(flag, rest, active_flags, state) do
    valid_values = flag["enum_values"]
    dwp = flag["default_when_present"]

    case rest do
      [next_token | remaining] ->
        if next_token in valid_values do
          # The next token is a valid enum value — consume it.
          new_flags = put_flag_value(state.parsed_flags, flag, next_token)
          new_explicit = [flag["id"] | state.explicit_flags]
          scan_tokens(remaining, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})
        else
          # Not a valid enum value — use default_when_present and don't
          # consume the token.
          new_flags = put_flag_value(state.parsed_flags, flag, dwp)
          new_explicit = [flag["id"] | state.explicit_flags]
          scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})
        end

      [] ->
        # No more tokens — use default_when_present.
        new_flags = put_flag_value(state.parsed_flags, flag, dwp)
        new_explicit = [flag["id"] | state.explicit_flags]
        scan_tokens(rest, active_flags, %{state | parsed_flags: new_flags, explicit_flags: new_explicit})
    end
  end

  # ---------------------------------------------------------------------------
  # Count flag helper (v1.1 — Feature 1)
  # ---------------------------------------------------------------------------

  # Increment the counter for a count-type flag. If the flag has not been seen
  # yet, it starts at 0 and becomes 1.  Each subsequent occurrence increments
  # by 1.  This is used for flags like `-v -v -v` → 3 or `-vvv` → 3.
  defp increment_count_flag(parsed_flags, flag) do
    id = flag["id"]
    current = Map.get(parsed_flags, id, 0)
    Map.put(parsed_flags, id, current + 1)
  end

  # Store a flag value, respecting the `repeatable` field.
  defp put_flag_value(parsed_flags, flag, value) do
    id = flag["id"]

    if flag["repeatable"] do
      existing = Map.get(parsed_flags, id, [])

      if is_list(existing) do
        Map.put(parsed_flags, id, existing ++ [value])
      else
        Map.put(parsed_flags, id, [existing, value])
      end
    else
      Map.put(parsed_flags, id, value)
    end
  end

  # Fill in defaults for all flags not present in parsed_flags.
  #
  # NOTE: normalise_flags always writes `"default" => nil` into the flag map
  # when no explicit default is given (rather than omitting the key entirely).
  # That means `Map.get(flag, "default", false)` always returns the stored
  # value — which is nil — and the fallback `false` is never triggered.
  # We therefore compute the effective default explicitly: a boolean flag with
  # no explicit default value should be false, not nil.
  defp apply_flag_defaults(parsed_flags, active_flags) do
    Enum.reduce(active_flags, parsed_flags, fn flag, acc ->
      id = flag["id"]

      if Map.has_key?(acc, id) do
        acc
      else
        explicit_default = Map.get(flag, "default")

        default =
          case flag["type"] do
            "boolean" ->
              if explicit_default == nil, do: false, else: explicit_default

            # Count flags (v1.1): default to 0 when absent, just as boolean
            # flags default to false.
            "count" ->
              if explicit_default == nil, do: 0, else: explicit_default

            _ ->
              explicit_default
          end

        Map.put(acc, id, default)
      end
    end)
  end

  # Validate enum values for all present flags.
  defp validate_enum_values(parsed_flags, active_flags, command_path) do
    Enum.flat_map(active_flags, fn flag ->
      if flag["type"] == "enum" do
        value = Map.get(parsed_flags, flag["id"])
        valid = flag["enum_values"]

        cond do
          value == nil ->
            []

          is_list(value) ->
            Enum.flat_map(value, fn v ->
              if v not in valid do
                [
                  %ParseError{
                    error_type: "invalid_enum_value",
                    message:
                      "Invalid value #{inspect(v)} for #{flag_label(flag)}. Must be one of: #{Enum.join(valid, ", ")}",
                    suggestion: nil,
                    context: command_path
                  }
                ]
              else
                []
              end
            end)

          value not in valid ->
            [
              %ParseError{
                error_type: "invalid_enum_value",
                message:
                  "Invalid value #{inspect(value)} for #{flag_label(flag)}. Must be one of: #{Enum.join(valid, ", ")}",
                suggestion: nil,
                context: command_path
              }
            ]

          true ->
            []
        end
      else
        []
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Lookup helpers
  # ---------------------------------------------------------------------------

  defp lookup_flag_by_long(name, flags) do
    Enum.find(flags, fn f -> f["long"] == name end)
  end

  defp lookup_flag_by_short(char, flags) do
    Enum.find(flags, fn f -> f["short"] == char end)
  end

  # ---------------------------------------------------------------------------
  # Traditional mode
  # ---------------------------------------------------------------------------

  # In traditional mode, if argv[0] does not start with "-" and is not a known
  # subcommand, treat it as a stack of short flag characters (tar-style).
  defp apply_traditional_mode([], _active_flags), do: []

  defp apply_traditional_mode([first | rest] = argv, active_flags) do
    if String.starts_with?(first, "-") do
      argv
    else
      # Check if it looks like stacked flags (all chars are known boolean shorts).
      chars = String.graphemes(first)
      short_map = Map.new(Enum.filter(active_flags, fn f -> f["short"] != nil end), fn f -> {f["short"], f} end)
      all_known = Enum.all?(chars, &Map.has_key?(short_map, &1))

      if all_known do
        ["-#{first}" | rest]
      else
        argv
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Fuzzy matching (Levenshtein, edit distance ≤ 2)
  # ---------------------------------------------------------------------------

  # Return the closest flag name (string) within edit distance 2, or nil.
  defp fuzzy_match_flag(unknown, active_flags, kind) do
    candidates =
      case kind do
        :long ->
          Enum.flat_map(active_flags, fn f ->
            [f["long"], f["single_dash_long"]] |> Enum.reject(&is_nil/1)
          end)

        :any ->
          Enum.flat_map(active_flags, fn f ->
            [f["long"], f["short"], f["single_dash_long"]] |> Enum.reject(&is_nil/1)
          end)

        _ ->
          []
      end

    case Enum.min_by(candidates, &levenshtein(unknown, &1), fn -> nil end) do
      nil ->
        nil

      best ->
        if levenshtein(unknown, best) <= 2, do: best, else: nil
    end
  end

  # Levenshtein edit distance between two strings.
  # Standard DP implementation, O(m*n).
  #
  # We keep the previous row as a plain list and compute each new row using
  # `Enum.at/2` for random access, which avoids off-by-one errors in the
  # diagonal/above lookups.
  #
  # Row layout:  prev_row[j]  = dp[i-1][j]
  #              new_row[j]   = dp[i][j]
  # where i indexes a_chars (1-based) and j indexes b_chars (1-based).
  defp levenshtein(a, b) do
    a_chars = String.graphemes(a)
    b_chars = String.graphemes(b)
    n = length(b_chars)

    # Base case: dp[0][j] = j (cost of inserting j chars of b)
    first_row = Enum.to_list(0..n)

    Enum.with_index(a_chars, 1)
    |> Enum.reduce(first_row, fn {a_char, i}, prev_row ->
      # For each column j in 0..n build the new row.
      # dp[i][0] = i (cost of deleting i chars of a)
      new_row =
        Enum.reduce(1..n, [i], fn j, acc ->
          b_char = Enum.at(b_chars, j - 1)
          above = Enum.at(prev_row, j)       # dp[i-1][j]   (delete from a)
          left  = hd(acc)                    # dp[i][j-1]   (insert into a)
          diag  = Enum.at(prev_row, j - 1)  # dp[i-1][j-1] (replace or match)
          cost  = if a_char == b_char, do: 0, else: 1
          val   = Enum.min([above + 1, left + 1, diag + cost])
          [val | acc]
        end)

      Enum.reverse(new_row)
    end)
    |> Enum.at(n)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp flag_label(flag) do
    parts =
      [
        if(flag["short"], do: "-#{flag["short"]}", else: nil),
        if(flag["long"], do: "--#{flag["long"]}", else: nil),
        if(flag["single_dash_long"], do: "-#{flag["single_dash_long"]}", else: nil)
      ]
      |> Enum.reject(&is_nil/1)

    Enum.join(parts, "/")
  end
end
