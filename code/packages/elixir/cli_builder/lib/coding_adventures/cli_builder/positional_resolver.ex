defmodule CodingAdventures.CliBuilder.PositionalResolver do
  @moduledoc """
  Assigns positional tokens to argument definition slots.

  ## The Positional Resolution Algorithm (§6.4.1)

  After Phase 2 scanning produces a flat list of positional tokens, this module
  maps them onto the argument definitions for the resolved command node.

  Two cases exist depending on whether any argument is variadic:

  ### No variadic argument

  One-to-one assignment in order. If there are more tokens than definitions, that
  is a `too_many_arguments` error. If a required definition has no token, that is
  a `missing_required_argument` error.

  ### With a variadic argument

  The variadic argument acts as a "splat" absorber in the middle of the list.
  The algorithm is a *last-wins* partition:

  1. Assign the **leading** definitions (before the variadic) from the start.
  2. Assign the **trailing** definitions (after the variadic) from the end.
  3. Everything in between goes to the variadic definition.

  This naturally handles the classic `cp`/`mv` pattern:

  ```
  cp a.txt b.txt c.txt /dest/
    leading  = []          (variadic is first)
    variadic = ["a.txt", "b.txt", "c.txt"]
    trailing = ["/dest/"]
  ```

  ## Type Coercion

  Each token is coerced to the argument's declared type via
  `CodingAdventures.CliBuilder.PositionalResolver.coerce/2`. Type errors produce
  `invalid_value` parse errors; the algorithm continues to collect further errors
  rather than stopping at the first one.
  """

  alias CodingAdventures.CliBuilder.ParseError

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Resolve a list of positional tokens against a list of argument definitions.

  Returns `{:ok, map}` where `map` is from argument id to coerced value, or
  `{:error, errors}` where `errors` is a list of `ParseError` structs.

  `parsed_flags` is needed to evaluate `required_unless_flag` clauses.

  ## Parameters

  - `tokens` — list of raw positional strings (in order).
  - `arg_defs` — list of normalised argument definition maps from `SpecLoader`.
  - `parsed_flags` — map of flag id -> value for the current parse.
  - `command_path` — current command path for error context.

  ## Examples

      iex> alias CodingAdventures.CliBuilder.PositionalResolver
      iex> defs = [%{"id" => "file", "name" => "FILE", "type" => "string",
      ...>           "required" => true, "variadic" => false,
      ...>           "required_unless_flag" => []}]
      iex> PositionalResolver.resolve(["hello.txt"], defs, %{}, ["prog"])
      {:ok, %{"file" => "hello.txt"}}
  """
  @spec resolve([String.t()], [map()], map(), [String.t()]) ::
          {:ok, %{String.t() => term()}} | {:error, [ParseError.t()]}
  def resolve(tokens, arg_defs, parsed_flags, command_path) do
    variadic_idx = Enum.find_index(arg_defs, fn a -> a["variadic"] end)

    {assignments, errors} =
      if variadic_idx == nil do
        resolve_no_variadic(tokens, arg_defs, parsed_flags, command_path)
      else
        resolve_with_variadic(tokens, arg_defs, variadic_idx, parsed_flags, command_path)
      end

    if Enum.empty?(errors) do
      {:ok, assignments}
    else
      {:error, errors}
    end
  end

  # ---------------------------------------------------------------------------
  # No-variadic path
  # ---------------------------------------------------------------------------

  defp resolve_no_variadic(tokens, arg_defs, parsed_flags, command_path) do
    # Check for too many tokens first (before we start assigning)
    extra_errors =
      if length(tokens) > length(arg_defs) do
        [
          %ParseError{
            error_type: "too_many_arguments",
            message:
              "Too many arguments: expected at most #{length(arg_defs)}, got #{length(tokens)}",
            suggestion: nil,
            context: command_path
          }
        ]
      else
        []
      end

    # Assign tokens to defs in order.
    {assignments, assign_errors} =
      Enum.with_index(arg_defs)
      |> Enum.reduce({%{}, []}, fn {def, i}, {acc_map, acc_errors} ->
        if i < length(tokens) do
          token = Enum.at(tokens, i)

          case coerce(token, def["type"]) do
            {:ok, value} ->
              {Map.put(acc_map, def["id"], value), acc_errors}

            {:error, msg} ->
              err = %ParseError{
                error_type: "invalid_value",
                message: msg,
                suggestion: nil,
                context: command_path
              }

              {acc_map, [err | acc_errors]}
          end
        else
          # No token for this definition.
          required = def["required"]
          exempt = required_unless_flag_satisfied?(def, parsed_flags)

          if required and not exempt do
            err = %ParseError{
              error_type: "missing_required_argument",
              message: "Missing required argument: <#{def["display_name"] || def["name"]}>",
              suggestion: nil,
              context: command_path
            }

            {acc_map, [err | acc_errors]}
          else
            # Optional — apply default (may be nil).
            {Map.put(acc_map, def["id"], def["default"]), acc_errors}
          end
        end
      end)

    all_errors = Enum.reverse(assign_errors) ++ extra_errors
    {assignments, all_errors}
  end

  # ---------------------------------------------------------------------------
  # With-variadic path
  # ---------------------------------------------------------------------------

  defp resolve_with_variadic(tokens, arg_defs, variadic_idx, parsed_flags, command_path) do
    leading_defs = Enum.take(arg_defs, variadic_idx)
    variadic_def = Enum.at(arg_defs, variadic_idx)
    trailing_defs = Enum.drop(arg_defs, variadic_idx + 1)

    n_tokens = length(tokens)
    n_leading = length(leading_defs)
    n_trailing = length(trailing_defs)

    # Minimum tokens needed = leading required + variadic_min + trailing required
    # We assign leading from start, trailing from end, variadic gets the middle.
    trailing_start = n_tokens - n_trailing

    errors = []
    assignments = %{}

    # --- Leading ---
    {assignments, errors} =
      Enum.with_index(leading_defs)
      |> Enum.reduce({assignments, errors}, fn {def, i}, {acc_map, acc_errors} ->
        if i < n_tokens do
          token = Enum.at(tokens, i)
          assign_token(token, def, acc_map, acc_errors, command_path)
        else
          handle_missing_arg(def, parsed_flags, acc_map, acc_errors, command_path)
        end
      end)

    # --- Trailing ---
    # NOTE: trailing_start may be negative when n_tokens < n_trailing (i.e.
    # not enough tokens were provided to fill even the trailing required args).
    # In that case token_idx will be negative, which must be treated as
    # "missing" — do NOT attempt Enum.at(tokens, negative_idx) because
    # Elixir's Enum.at/2 with a negative index counts from the end, which
    # would silently reuse an earlier token and hide the missing-arg error.
    {assignments, errors} =
      Enum.with_index(trailing_defs)
      |> Enum.reduce({assignments, errors}, fn {def, i}, {acc_map, acc_errors} ->
        token_idx = trailing_start + i

        if token_idx < 0 or token_idx >= n_tokens do
          handle_missing_arg(def, parsed_flags, acc_map, acc_errors, command_path)
        else
          token = Enum.at(tokens, token_idx)
          assign_token(token, def, acc_map, acc_errors, command_path)
        end
      end)

    # --- Variadic (everything in between) ---
    variadic_end = max(n_leading, trailing_start)
    variadic_tokens = Enum.slice(tokens, n_leading, variadic_end - n_leading)
    count = length(variadic_tokens)
    variadic_min = variadic_def["variadic_min"]
    variadic_max = variadic_def["variadic_max"]

    errors =
      if count < variadic_min do
        [
          %ParseError{
            error_type: "too_few_arguments",
            message:
              "Expected at least #{variadic_min} <#{variadic_def["display_name"] || variadic_def["name"]}>, got #{count}",
            suggestion: nil,
            context: command_path
          }
          | errors
        ]
      else
        errors
      end

    errors =
      if variadic_max != nil and count > variadic_max do
        [
          %ParseError{
            error_type: "too_many_arguments",
            message:
              "Expected at most #{variadic_max} <#{variadic_def["display_name"] || variadic_def["name"]}>, got #{count}",
            suggestion: nil,
            context: command_path
          }
          | errors
        ]
      else
        errors
      end

    # Coerce each variadic token.
    {variadic_values, coerce_errors} =
      Enum.reduce(variadic_tokens, {[], []}, fn token, {vals, errs} ->
        case coerce(token, variadic_def["type"]) do
          {:ok, v} ->
            {[v | vals], errs}

          {:error, msg} ->
            err = %ParseError{
              error_type: "invalid_value",
              message: msg,
              suggestion: nil,
              context: command_path
            }

            {vals, [err | errs]}
        end
      end)

    assignments = Map.put(assignments, variadic_def["id"], Enum.reverse(variadic_values))
    errors = Enum.reverse(coerce_errors) ++ errors

    {assignments, Enum.reverse(errors)}
  end

  # ---------------------------------------------------------------------------
  # Type coercion
  # ---------------------------------------------------------------------------

  @doc """
  Coerce a raw string token to the target type.

  Returns `{:ok, value}` on success or `{:error, message}` on failure.

  ## Types

  | Type | Coercion |
  |---|---|
  | `"boolean"` | `"true"` → `true`, `"false"` → `false`, others → error |
  | `"string"` | Must be non-empty |
  | `"integer"` | `Integer.parse/1` |
  | `"float"` | `Float.parse/1` |
  | `"path"` | Any non-empty string (syntactic only; existence not checked) |
  | `"file"` | Must refer to an existing, readable file |
  | `"directory"` | Must refer to an existing directory |
  | `"enum"` | Validated separately; returns the string as-is |
  """
  @spec coerce(String.t(), String.t()) :: {:ok, term()} | {:error, String.t()}
  def coerce(token, type) do
    case type do
      "boolean" ->
        case token do
          "true" -> {:ok, true}
          "false" -> {:ok, false}
          _ -> {:error, "Invalid boolean value: #{inspect(token)}. Use 'true' or 'false'"}
        end

      "string" ->
        if token == "" do
          {:error, "String value must be non-empty"}
        else
          {:ok, token}
        end

      "integer" ->
        case Integer.parse(token) do
          {n, ""} -> {:ok, n}
          _ -> {:error, "Invalid integer: #{inspect(token)}"}
        end

      "float" ->
        case Float.parse(token) do
          {f, ""} -> {:ok, f}
          # Accept integers as floats too
          _ ->
            case Integer.parse(token) do
              {n, ""} -> {:ok, n * 1.0}
              _ -> {:error, "Invalid float: #{inspect(token)}"}
            end
        end

      "path" ->
        if token == "" do
          {:error, "Path value must be non-empty"}
        else
          {:ok, token}
        end

      "file" ->
        case File.stat(token) do
          {:ok, %{type: :regular}} -> {:ok, token}
          {:ok, _} -> {:error, "Not a regular file: #{inspect(token)}"}
          {:error, _} -> {:error, "File not found or not readable: #{inspect(token)}"}
        end

      "directory" ->
        case File.stat(token) do
          {:ok, %{type: :directory}} -> {:ok, token}
          {:ok, _} -> {:error, "Not a directory: #{inspect(token)}"}
          {:error, _} -> {:error, "Directory not found: #{inspect(token)}"}
        end

      "enum" ->
        # Enum values were validated at spec-load time; just return the string.
        {:ok, token}

      _ ->
        {:ok, token}
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Assign a single token to a single arg def, collecting errors.
  defp assign_token(token, def, acc_map, acc_errors, command_path) do
    case coerce(token, def["type"]) do
      {:ok, value} ->
        {Map.put(acc_map, def["id"], value), acc_errors}

      {:error, msg} ->
        err = %ParseError{
          error_type: "invalid_value",
          message: msg,
          suggestion: nil,
          context: command_path
        }

        {acc_map, [err | acc_errors]}
    end
  end

  # Handle the case where a definition has no token.
  defp handle_missing_arg(def, parsed_flags, acc_map, acc_errors, command_path) do
    required = def["required"]
    exempt = required_unless_flag_satisfied?(def, parsed_flags)

    if required and not exempt do
      err = %ParseError{
        error_type: "missing_required_argument",
        message: "Missing required argument: <#{def["display_name"] || def["name"]}>",
        suggestion: nil,
        context: command_path
      }

      {acc_map, [err | acc_errors]}
    else
      {Map.put(acc_map, def["id"], def["default"]), acc_errors}
    end
  end

  # Check whether a `required_unless_flag` clause exempts the argument from being
  # required.  The argument is exempt if ANY of the listed flag IDs is present
  # (i.e., has a truthy value) in parsed_flags.
  defp required_unless_flag_satisfied?(arg_def, parsed_flags) do
    unless_flags = Map.get(arg_def, "required_unless_flag", [])

    Enum.any?(unless_flags, fn flag_id ->
      val = Map.get(parsed_flags, flag_id)
      val != nil and val != false
    end)
  end
end
