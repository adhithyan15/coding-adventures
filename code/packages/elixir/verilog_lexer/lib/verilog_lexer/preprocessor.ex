defmodule CodingAdventures.VerilogLexer.Preprocessor do
  @moduledoc """
  Verilog Preprocessor — Handles compiler directives before tokenization.

  Verilog's preprocessor is modeled after C's preprocessor. Source files
  can contain directives (lines starting with a backtick `` ` ``) that
  control macro expansion, conditional compilation, and file inclusion.
  These must be resolved before the lexer sees the source.

  ## Why Preprocess Before Lexing?

  Consider this Verilog source:

      `define WIDTH 8
      wire [`WIDTH-1:0] bus;

  Without preprocessing, the lexer would see `` `WIDTH `` as a DIRECTIVE
  token — it doesn't know that `WIDTH was defined as `8`. After
  preprocessing, the lexer sees:

      wire [8-1:0] bus;

  Now `8`, `-`, `1`, `:`, and `0` are all normal tokens.

  ## Supported Directives

  ### `define / `undef — Macro Definition

      `define WIDTH 8             — simple macro (text substitution)
      `define MAX(a, b) (a > b ? a : b)  — parameterized macro
      `undef WIDTH                — remove a macro definition

  Simple macros replace `` `NAME `` with the defined text wherever it
  appears. Parameterized macros accept arguments:

      `MAX(x, y)  →  (x > y ? x : y)

  ### `ifdef / `ifndef / `else / `endif — Conditional Compilation

      `ifdef DEBUG
        // This code is included only if DEBUG is defined
      `else
        // This code is included only if DEBUG is NOT defined
      `endif

  `ifndef is the inverse of `ifdef: code is included if the macro is
  NOT defined. Conditionals can be nested.

  ### `include — File Inclusion (Stubbed)

      `include "definitions.vh"

  In a real Verilog tool, this inserts the contents of the named file.
  Our implementation replaces it with a comment placeholder:

      // [include: definitions.vh]

  This is stubbed because file I/O depends on the project structure and
  search paths, which are beyond the scope of a standalone lexer.

  ### `timescale — Time Unit Specification

      `timescale 1ns/1ps

  This directive sets the simulation time unit and precision. It has no
  effect on the synthesizable design, so we strip it entirely (replace
  with an empty string).

  ## Implementation

  The preprocessor works line-by-line:

  1. Split the source into lines.
  2. For each line, check if it starts with a directive (after stripping
     leading whitespace).
  3. Process the directive (define, undef, ifdef, etc.) and update state.
  4. For non-directive lines, expand any macro references (`` `NAME ``).
  5. Conditionally include or exclude lines based on `ifdef/`ifndef state.
  6. Join the processed lines back together.

  State is carried in a map with three fields:
  - `:macros` — map of macro name to `{params, body}` tuples
  - `:condition_stack` — stack of booleans tracking nested ifdef/ifndef
  - `:active` — whether the current line should be included in output
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Process Verilog source code, resolving all preprocessor directives.

  Takes raw Verilog source with directives and returns clean source with:
  - All `define macros expanded
  - All `ifdef/`ifndef/`else/`endif blocks resolved
  - All `include directives replaced with comment placeholders
  - All `timescale directives stripped
  - All `undef directives processed (macros removed from table)

  ## Examples

      iex> CodingAdventures.VerilogLexer.Preprocessor.process(~s(`define WIDTH 8\\nwire [`WIDTH-1:0] bus;))
      "\\nwire [8-1:0] bus;"
  """
  @spec process(String.t()) :: String.t()
  def process(source) do
    # ---------------------------------------------------------------------------
    # Initial State
    # ---------------------------------------------------------------------------
    #
    # - macros: empty map — no macros defined yet
    # - condition_stack: empty list — not inside any ifdef/ifndef block
    # - active: true — lines are included by default
    #
    # The condition_stack is a list of booleans. Each `ifdef/`ifndef pushes
    # a boolean. `else flips the top. `endif pops the top. A line is active
    # only if ALL entries in the stack are true (nested conditionals must
    # all be satisfied).

    initial_state = %{
      macros: %{},
      condition_stack: [],
      active: true
    }

    source
    |> String.split("\n")
    |> Enum.reduce({[], initial_state}, fn line, {output_lines, state} ->
      process_line(String.trim_leading(line), line, output_lines, state)
    end)
    |> then(fn {output_lines, _state} ->
      output_lines
      |> Enum.reverse()
      |> Enum.join("\n")
    end)
  end

  # ---------------------------------------------------------------------------
  # Line Processing
  # ---------------------------------------------------------------------------
  #
  # Each line falls into one of these categories:
  #
  # 1. Conditional directive (`ifdef, `ifndef, `else, `endif)
  #    → Always processed, even inside inactive blocks. This is how we
  #      know when to re-activate after an `else or `endif.
  #
  # 2. Other directive (`define, `undef, `include, `timescale)
  #    → Only processed when active. If inside an inactive `ifdef block,
  #      the directive is skipped.
  #
  # 3. Regular code line
  #    → Only included in output when active. Macro references are expanded.

  # --- Conditional directives (always processed) ---

  defp process_line("`ifdef " <> macro_name, _original, output, state) do
    # `ifdef NAME — include the following lines if NAME is defined.
    #
    # We push the result onto the condition stack. The line itself is
    # never included in the output (it's a directive, not code).
    name = String.trim(macro_name)
    is_defined = Map.has_key?(state.macros, name)
    new_stack = [is_defined | state.condition_stack]
    new_state = %{state | condition_stack: new_stack, active: all_active?(new_stack)}
    {["" | output], new_state}
  end

  defp process_line("`ifndef " <> macro_name, _original, output, state) do
    # `ifndef NAME — include the following lines if NAME is NOT defined.
    # Opposite of `ifdef.
    name = String.trim(macro_name)
    is_not_defined = not Map.has_key?(state.macros, name)
    new_stack = [is_not_defined | state.condition_stack]
    new_state = %{state | condition_stack: new_stack, active: all_active?(new_stack)}
    {["" | output], new_state}
  end

  defp process_line("`else" <> _rest, _original, output, state) do
    # `else — flip the top of the condition stack.
    #
    # If we were inside an active `ifdef block, we become inactive.
    # If we were inside an inactive `ifdef block, we become active
    # (provided all parent conditions are still true).
    case state.condition_stack do
      [top | rest_stack] ->
        new_stack = [not top | rest_stack]
        new_state = %{state | condition_stack: new_stack, active: all_active?(new_stack)}
        {["" | output], new_state}

      [] ->
        # `else without a matching `ifdef — ignore gracefully
        {["" | output], state}
    end
  end

  defp process_line("`endif" <> _rest, _original, output, state) do
    # `endif — pop the top of the condition stack.
    #
    # This closes the most recent `ifdef/`ifndef block.
    case state.condition_stack do
      [_top | rest_stack] ->
        new_state = %{state | condition_stack: rest_stack, active: all_active?(rest_stack)}
        {["" | output], new_state}

      [] ->
        # `endif without a matching `ifdef — ignore gracefully
        {["" | output], state}
    end
  end

  # --- Directives that only apply when active ---

  defp process_line("`define " <> definition, _original, output, state) do
    if state.active do
      {name, params, body} = parse_define(definition)
      new_macros = Map.put(state.macros, name, {params, body})
      {["" | output], %{state | macros: new_macros}}
    else
      {["" | output], state}
    end
  end

  defp process_line("`undef " <> macro_name, _original, output, state) do
    if state.active do
      name = String.trim(macro_name)
      new_macros = Map.delete(state.macros, name)
      {["" | output], %{state | macros: new_macros}}
    else
      {["" | output], state}
    end
  end

  defp process_line("`include " <> path, _original, output, state) do
    # `include "file.vh" — stubbed implementation.
    #
    # In a real Verilog toolchain, this would read the file and recursively
    # preprocess it. We replace it with a comment placeholder so the intent
    # is visible in the output.
    if state.active do
      # Strip quotes from the path: "file.vh" -> file.vh
      clean_path = path |> String.trim() |> String.trim("\"")
      {["// [include: #{clean_path}]" | output], state}
    else
      {["" | output], state}
    end
  end

  defp process_line("`timescale" <> _rest, _original, output, state) do
    # `timescale 1ns/1ps — strip entirely.
    #
    # This directive only affects simulation timing and has no bearing
    # on the design's logical behavior or synthesis. We remove it so
    # the lexer doesn't see it.
    {["" | output], state}
  end

  # --- Regular code lines ---

  defp process_line(_trimmed, original, output, state) do
    if state.active do
      expanded = expand_macros(original, state.macros)
      {[expanded | output], state}
    else
      {["" | output], state}
    end
  end

  # ---------------------------------------------------------------------------
  # Macro Definition Parsing
  # ---------------------------------------------------------------------------
  #
  # A `define directive has three forms:
  #
  # 1. Simple macro:        `define WIDTH 8
  #    → name = "WIDTH", params = nil, body = "8"
  #
  # 2. Parameterized macro: `define ADD(a, b) (a + b)
  #    → name = "ADD", params = ["a", "b"], body = "(a + b)"
  #
  # 3. Empty macro:         `define DEBUG
  #    → name = "DEBUG", params = nil, body = ""
  #
  # The key parsing challenge is distinguishing case 1 from case 2.
  # If the character immediately after the name is `(`, it's parameterized.
  # If it's a space or end-of-string, it's simple.

  @doc false
  @spec parse_define(String.t()) :: {String.t(), list(String.t()) | nil, String.t()}
  def parse_define(definition) do
    trimmed = String.trim(definition)

    # Try to match a parameterized macro: NAME(param1, param2) body
    case Regex.run(~r/^([a-zA-Z_]\w*)\(([^)]*)\)\s*(.*)$/, trimmed) do
      [_, name, params_str, body] ->
        params =
          params_str
          |> String.split(",")
          |> Enum.map(&String.trim/1)
          |> Enum.reject(&(&1 == ""))

        {name, params, String.trim(body)}

      nil ->
        # Simple macro: NAME body  or  NAME (empty body means flag-style macro)
        case String.split(trimmed, ~r/\s+/, parts: 2) do
          [name, body] -> {name, nil, String.trim(body)}
          [name] -> {name, nil, ""}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Macro Expansion
  # ---------------------------------------------------------------------------
  #
  # After all directives are processed, we expand macro references in the
  # remaining code. A macro reference is a backtick followed by a name:
  #
  #   `WIDTH    → expands to the body of the WIDTH macro
  #   `ADD(x,y) → expands to the body of ADD with parameters substituted
  #
  # Expansion is done by scanning for backtick-name patterns and replacing
  # them with the macro body. For parameterized macros, we also parse the
  # argument list and substitute each parameter.

  @doc false
  @spec expand_macros(String.t(), map()) :: String.t()
  def expand_macros(line, macros) when map_size(macros) == 0, do: line

  def expand_macros(line, macros) do
    # Pattern: backtick followed by an identifier, optionally followed
    # by a parenthesized argument list.
    #
    # We use a regex to find all macro references and replace them.
    # The replacement is done iteratively until no more macros are found
    # (to handle macros that expand to other macro references). We cap
    # iterations to prevent infinite recursion from circular definitions.
    do_expand(line, macros, 0)
  end

  # Maximum expansion depth — prevents infinite loops from circular macros
  # like `define A `B and `define B `A.
  @max_expansion_depth 100

  defp do_expand(line, _macros, depth) when depth >= @max_expansion_depth, do: line

  defp do_expand(line, macros, depth) do
    # Try to find and replace one macro reference. If we find one, recurse
    # to handle any macros in the expanded text. If no macro reference is
    # found, we're done.
    case find_and_expand_one(line, macros) do
      {:expanded, new_line} -> do_expand(new_line, macros, depth + 1)
      :no_match -> line
    end
  end

  defp find_and_expand_one(line, macros) do
    # Find the first backtick-identifier in the line
    case Regex.run(~r/`([a-zA-Z_]\w*)/, line, return: :index) do
      [{match_start, match_len}, {_name_start, _name_len}] ->
        # Extract the macro name from the match
        match_text = String.slice(line, match_start, match_len)
        name = String.slice(match_text, 1..-1//1)

        case Map.get(macros, name) do
          nil ->
            # This backtick-identifier is not a defined macro. Skip it.
            # To avoid matching it again, we look for the next one.
            remaining = String.slice(line, (match_start + match_len)..-1//1)

            case find_and_expand_one(remaining, macros) do
              {:expanded, new_remaining} ->
                prefix = String.slice(line, 0, match_start + match_len)
                {:expanded, prefix <> new_remaining}

              :no_match ->
                :no_match
            end

          {nil, body} ->
            # Simple macro — replace `NAME with the body text.
            prefix = String.slice(line, 0, match_start)
            suffix = String.slice(line, (match_start + match_len)..-1//1)
            {:expanded, prefix <> body <> suffix}

          {params, body} ->
            # Parameterized macro — parse the argument list after `NAME(...)
            after_name = String.slice(line, (match_start + match_len)..-1//1)

            case parse_macro_args(after_name) do
              {:ok, args, args_consumed} ->
                # Substitute parameters in the body with the actual arguments
                expanded_body = substitute_params(body, params, args)
                prefix = String.slice(line, 0, match_start)
                suffix = String.slice(after_name, args_consumed..-1//1)
                {:expanded, prefix <> expanded_body <> suffix}

              :error ->
                # No valid argument list — treat as simple macro
                prefix = String.slice(line, 0, match_start)
                suffix = String.slice(line, (match_start + match_len)..-1//1)
                {:expanded, prefix <> body <> suffix}
            end
        end

      nil ->
        :no_match
    end
  end

  # ---------------------------------------------------------------------------
  # Argument Parsing for Parameterized Macros
  # ---------------------------------------------------------------------------
  #
  # When we encounter `MACRO(a, b, c), we need to parse the argument list.
  # Arguments are comma-separated and may contain nested parentheses:
  #
  #   `MAX(x+1, (y*2))  → args = ["x+1", "(y*2)"]
  #
  # We track parenthesis depth to handle nesting correctly.

  defp parse_macro_args(text) do
    if String.starts_with?(text, "(") do
      # Skip the opening paren
      inner = String.slice(text, 1..-1//1)
      parse_args_inner(inner, 0, "", [], 1)
    else
      :error
    end
  end

  # Parse the arguments inside the parentheses.
  # - depth: nesting depth of parentheses (0 = top level)
  # - current_arg: accumulator for the current argument text
  # - args: list of completed arguments
  # - consumed: total characters consumed from the original text (including opening paren)
  defp parse_args_inner("", _depth, _current, _args, _consumed), do: :error

  defp parse_args_inner(")" <> _rest_text, 0, current_arg, args, consumed) do
    # Closing paren at top level — we're done.
    # rest_text is ignored here — the consumed count tells the caller
    # where to resume in the original string.
    final_args = Enum.reverse([String.trim(current_arg) | args])
    # +1 for the closing paren
    {:ok, final_args, consumed + 1}
  end

  defp parse_args_inner(")" <> rest_text, depth, current_arg, args, consumed) do
    # Closing paren inside nested parens — add to current arg, decrease depth
    parse_args_inner(rest_text, depth - 1, current_arg <> ")", args, consumed + 1)
  end

  defp parse_args_inner("(" <> rest_text, depth, current_arg, args, consumed) do
    # Opening paren — increase depth
    parse_args_inner(rest_text, depth + 1, current_arg <> "(", args, consumed + 1)
  end

  defp parse_args_inner("," <> rest_text, 0, current_arg, args, consumed) do
    # Comma at top level — finish current arg, start next
    parse_args_inner(rest_text, 0, "", [String.trim(current_arg) | args], consumed + 1)
  end

  defp parse_args_inner(<<char::utf8, rest_text::binary>>, depth, current_arg, args, consumed) do
    # Any other character — add to current argument
    parse_args_inner(rest_text, depth, current_arg <> <<char::utf8>>, args, consumed + 1)
  end

  # ---------------------------------------------------------------------------
  # Parameter Substitution
  # ---------------------------------------------------------------------------
  #
  # Given a macro body like "(a + b)" and params ["a", "b"] with
  # args ["x", "y"], we replace each parameter name with the corresponding
  # argument: "(x + y)".
  #
  # We use word-boundary-aware replacement to avoid replacing "ab" when
  # we're looking for parameter "a".

  defp substitute_params(body, params, args) do
    # Zip params with args, then fold over them replacing each one
    Enum.zip(params, args)
    |> Enum.reduce(body, fn {param, arg}, acc ->
      # Use a word-boundary regex so "param_extra" doesn't get partially replaced
      # when we're substituting for "param".
      regex = Regex.compile!("\\b#{Regex.escape(param)}\\b")
      Regex.replace(regex, acc, arg)
    end)
  end

  # ---------------------------------------------------------------------------
  # Condition Stack Helper
  # ---------------------------------------------------------------------------
  #
  # A line is active only when ALL conditions in the stack are true.
  # An empty stack means we're at the top level — always active.
  #
  # Example:
  #   `ifdef A        → stack = [true]     → active = true
  #     `ifdef B      → stack = [false, true] → active = false
  #       code here   → not included (B is not defined)
  #     `endif        → stack = [true]     → active = true
  #   `endif          → stack = []         → active = true

  defp all_active?([]), do: true
  defp all_active?(stack), do: Enum.all?(stack)
end
