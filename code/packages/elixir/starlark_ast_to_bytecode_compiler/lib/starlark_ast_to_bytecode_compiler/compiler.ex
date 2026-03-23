defmodule CodingAdventures.StarlarkAstToBytecodeCompiler.Compiler do
  @moduledoc """
  Starlark Compiler — Compiles Starlark ASTs to bytecode.

  ## Chapter 1: The Starlark Compilation Pipeline

  The full pipeline from source code to execution is:

      Starlark source code
          | (starlark_lexer)
      Token stream
          | (starlark_parser)
      AST (map tree with :rule_name and :children)
          | (THIS MODULE)
      CodeObject (bytecode)
          | (starlark_vm)
      Execution result

  This module handles the AST -> CodeObject step. It registers handlers for
  all Starlark grammar rules with the `GenericCompiler` framework, then
  provides a `compile_starlark/1` convenience function that does the
  full source -> bytecode path.

  ## Chapter 2: How Rule Handlers Work

  Each Starlark grammar rule (`file`, `assign_stmt`, `if_stmt`, etc.)
  gets a corresponding handler function. The handler receives the compiler
  and the AST node, then:

  1. Inspects the node's children to understand the source construct.
  2. Calls `GenericCompiler.compile_node(compiler, child)` to recursively
     compile sub-expressions.
  3. Calls `GenericCompiler.emit(compiler, opcode)` to emit bytecode
     instructions.

  For example, the `assign_stmt` handler for `x = 1 + 2`:

  1. Compiles the RHS expression (`1 + 2`) -> emits LOAD_CONST, LOAD_CONST, ADD
  2. Emits STORE_NAME for the LHS (`x`)

  ## Chapter 3: Grammar Rules Reference

  The Starlark grammar has approximately 55 rules. Here they are grouped
  by category:

  **Top-level:**
  file, suite, simple_stmt, small_stmt, compound_stmt

  **Statements:**
  assign_stmt, augmented_assign_stmt, return_stmt, pass_stmt, break_stmt,
  continue_stmt, if_stmt, elif_clause, else_clause, for_stmt, load_stmt,
  expression_stmt

  **Expressions:**
  expr, or_expr, and_expr, not_expr, comparison, star_expr, bitwise_or,
  bitwise_xor, bitwise_and, shift, arith, term, factor, unary, power_expr,
  primary, call, call_args, argument, dot_access, subscript, slice

  **Literals:**
  atom, number, string_node, list_expr, list_comp, dict_expr, dict_comp,
  dict_entry, tuple_expr, lambda_expr

  **Definitions:**
  def_stmt, param_list, param

  **Comprehension:**
  comp_clause, comp_if

  **Identifiers:**
  identifier
  """

  alias CodingAdventures.BytecodeCompiler.GenericCompiler
  alias CodingAdventures.StarlarkAstToBytecodeCompiler.Opcodes, as: Op

  # ===========================================================================
  # Public API
  # ===========================================================================

  @doc """
  Create a `GenericCompiler` fully configured for Starlark compilation.

  Returns a `GenericCompiler` struct with all ~55 Starlark grammar rule
  handlers registered. Pass AST nodes to `GenericCompiler.compile/2` to
  produce `CodeObject` structs.

  ## Example

      compiler = Compiler.create_compiler()
      {code_object, _compiler} = GenericCompiler.compile(compiler, ast)
  """
  def create_compiler do
    compiler = GenericCompiler.new()

    # Register all grammar rule handlers
    compiler
    |> register_top_level_rules()
    |> register_statement_rules()
    |> register_expression_rules()
    |> register_literal_rules()
    |> register_definition_rules()
    |> register_comprehension_rules()
    |> register_identifier_rules()
  end

  @doc """
  Compile Starlark source code to a CodeObject in one call.

  This is the highest-level API. It lexes, parses, and compiles the source
  into bytecode ready for the Starlark VM.

  Note: This function creates a mock AST for simple expressions. For full
  compilation from actual Starlark source, use the starlark_interpreter
  package which chains the lexer, parser, and compiler together.

  ## Parameters

  - `source` — Starlark source code string (should end with newline)

  ## Returns

  A `CodeObject` struct with instructions, constants, and names.

  ## Example

      code = Compiler.compile_starlark("x = 1 + 2\\n")
      # code.instructions contains [LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT]
  """
  def compile_starlark(source) when is_binary(source) do
    # Build the AST by parsing the source
    ast = parse_source(source)

    # Compile the AST to bytecode
    compiler = create_compiler()
    {code_object, _compiler} = GenericCompiler.compile(compiler, ast, Op.halt())
    code_object
  end

  @doc """
  Compile an AST node to a CodeObject.

  Takes a pre-parsed AST and produces bytecode. Use this when you already
  have an AST from the parser.

  ## Parameters

  - `ast` — an AST node (map with :rule_name and :children)

  ## Returns

  A `CodeObject` struct.
  """
  def compile_ast(ast) do
    compiler = create_compiler()
    {code_object, _compiler} = GenericCompiler.compile(compiler, ast, Op.halt())
    code_object
  end

  # ===========================================================================
  # Source Parsing
  # ===========================================================================
  #
  # This function tokenizes and parses the source code. It produces an AST
  # that the compiler can walk. The AST uses simple maps with :rule_name
  # and :children keys for interior nodes, and :type and :value keys for
  # leaf tokens.

  defp parse_source(source) do
    # Tokenize the source
    tokens = tokenize(source)

    # Parse tokens into AST
    parse_tokens(tokens)
  end

  # ===========================================================================
  # Tokenizer — Minimal Starlark Lexer
  # ===========================================================================
  #
  # This is a self-contained tokenizer for Starlark. It produces tokens
  # compatible with the AST format expected by the compiler. In a full
  # deployment, you would use the starlark_lexer package instead.

  defp tokenize(source) do
    raw_tokens = source
    |> String.graphemes()
    |> tokenize_chars([], "")
    |> Enum.reverse()
    |> Enum.reject(fn tok -> tok.type == "WHITESPACE" end)

    # Post-process: inject INDENT/DEDENT tokens based on source indentation.
    # We scan the original source to compute indentation levels per line,
    # then insert INDENT/DEDENT tokens at the corresponding NEWLINE positions.
    inject_indent_dedent(raw_tokens, source)
  end

  # ---------------------------------------------------------------------------
  # Indentation Tracking
  # ---------------------------------------------------------------------------
  #
  # Python-family languages use indentation to delimit blocks. The tokenizer
  # must emit INDENT and DEDENT tokens so the parser knows when a block starts
  # and ends. We do this as a post-processing step:
  #
  # 1. Split source into lines and compute each line's indentation level.
  # 2. Walk the raw token stream. At each NEWLINE, look at the next line's
  #    indentation. If it increased, emit INDENT. If it decreased, emit
  #    one or more DEDENT tokens.
  #
  # This is exactly how Python's tokenizer works (PEP 3120, tokenize module).

  defp inject_indent_dedent(tokens, source) do
    # Compute indentation for each non-empty, non-comment line
    lines = String.split(source, "\n")
    indents = lines |> Enum.map(fn line ->
      stripped = String.trim_leading(line)
      if stripped == "" or String.starts_with?(stripped, "#") do
        nil  # blank/comment line — no indent change
      else
        String.length(line) - String.length(stripped)
      end
    end)

    # Walk tokens and inject INDENT/DEDENT
    inject_tokens(tokens, indents, _line_idx = 0, _indent_stack = [0], [])
    |> Enum.reverse()
  end

  defp inject_tokens([], _indents, _line_idx, indent_stack, acc) do
    # At end of file, emit DEDENT for any remaining indent levels
    Enum.reduce(tl(indent_stack), acc, fn _level, inner_acc ->
      [%{type: "DEDENT", value: ""} | inner_acc]
    end)
  end

  defp inject_tokens([%{type: "NEWLINE"} = tok | rest], indents, line_idx, indent_stack, acc) do
    # After a newline, find the next non-blank line's indentation
    next_line_idx = line_idx + 1
    {next_indent, effective_idx} = find_next_indent(indents, next_line_idx)
    current_indent = hd(indent_stack)

    cond do
      next_indent > current_indent ->
        # Indent — push new level
        inject_tokens(rest, indents, effective_idx,
          [next_indent | indent_stack],
          [%{type: "INDENT", value: ""}, tok | acc])

      next_indent < current_indent ->
        # Dedent — pop levels until we match
        {new_stack, dedent_tokens} = pop_indent_stack(indent_stack, next_indent)
        inject_tokens(rest, indents, effective_idx,
          new_stack,
          dedent_tokens ++ [tok | acc])

      true ->
        # Same level
        inject_tokens(rest, indents, effective_idx, indent_stack, [tok | acc])
    end
  end

  defp inject_tokens([tok | rest], indents, line_idx, indent_stack, acc) do
    inject_tokens(rest, indents, line_idx, indent_stack, [tok | acc])
  end

  defp find_next_indent(indents, from_idx) do
    # Skip blank/comment lines (nil indent)
    case Enum.at(indents, from_idx) do
      nil ->
        if from_idx >= length(indents) do
          {0, from_idx}  # End of file — back to zero indent
        else
          find_next_indent(indents, from_idx + 1)
        end
      indent_val ->
        {indent_val, from_idx}
    end
  end

  defp pop_indent_stack([top | rest] = stack, target) do
    if top <= target do
      {stack, []}
    else
      {new_stack, more} = pop_indent_stack(rest, target)
      {new_stack, [%{type: "DEDENT", value: ""} | more]}
    end
  end

  defp tokenize_chars([], tokens, "") do
    [%{type: "NEWLINE", value: "\n"} | tokens]
  end

  defp tokenize_chars([], tokens, current) do
    tok = classify_token(current)
    [%{type: "NEWLINE", value: "\n"}, tok | tokens]
  end

  defp tokenize_chars(["\n" | rest], tokens, "") do
    tokenize_chars(rest, [%{type: "NEWLINE", value: "\n"} | tokens], "")
  end

  defp tokenize_chars(["\n" | rest], tokens, current) do
    tok = classify_token(current)
    tokenize_chars(rest, [%{type: "NEWLINE", value: "\n"}, tok | tokens], "")
  end

  defp tokenize_chars(["\"" | rest], tokens, "") do
    {str_val, remaining} = consume_string(rest, "")
    tok = %{type: "STRING", value: str_val}
    tokenize_chars(remaining, [tok | tokens], "")
  end

  defp tokenize_chars(["\"" | rest], tokens, current) do
    tok = classify_token(current)
    {str_val, remaining} = consume_string(rest, "")
    str_tok = %{type: "STRING", value: str_val}
    tokenize_chars(remaining, [str_tok, tok | tokens], "")
  end

  defp tokenize_chars(["'" | rest], tokens, "") do
    {str_val, remaining} = consume_single_string(rest, "")
    tok = %{type: "STRING", value: str_val}
    tokenize_chars(remaining, [tok | tokens], "")
  end

  defp tokenize_chars(["'" | rest], tokens, current) do
    tok = classify_token(current)
    {str_val, remaining} = consume_single_string(rest, "")
    str_tok = %{type: "STRING", value: str_val}
    tokenize_chars(remaining, [str_tok, tok | tokens], "")
  end

  defp tokenize_chars(["#" | rest], tokens, current) do
    # Skip comment to end of line
    remaining = Enum.drop_while(rest, fn c -> c != "\n" end)
    if current == "" do
      tokenize_chars(remaining, tokens, "")
    else
      tok = classify_token(current)
      tokenize_chars(remaining, [tok | tokens], "")
    end
  end

  defp tokenize_chars([" " | rest], tokens, "") do
    tokenize_chars(rest, tokens, "")
  end

  defp tokenize_chars([" " | rest], tokens, current) do
    tok = classify_token(current)
    tokenize_chars(rest, [tok | tokens], "")
  end

  defp tokenize_chars(["\t" | rest], tokens, current) do
    if current == "" do
      tokenize_chars(rest, tokens, "")
    else
      tok = classify_token(current)
      tokenize_chars(rest, [tok | tokens], "")
    end
  end

  # Two-character operators
  defp tokenize_chars(["=" , "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "EQEQ", value: "=="} | tokens], "")
  end

  defp tokenize_chars(["!", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "NEQ", value: "!="} | tokens], "")
  end

  defp tokenize_chars(["<", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "LEQ", value: "<="} | tokens], "")
  end

  defp tokenize_chars([">", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "GEQ", value: ">="} | tokens], "")
  end

  defp tokenize_chars(["<", "<" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "LSHIFT", value: "<<"} | tokens], "")
  end

  defp tokenize_chars([">", ">" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "RSHIFT", value: ">>"} | tokens], "")
  end

  defp tokenize_chars(["/", "/" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "FLOORDIV", value: "//"} | tokens], "")
  end

  defp tokenize_chars(["*", "*" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "POWER", value: "**"} | tokens], "")
  end

  defp tokenize_chars(["+", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "PLUSEQ", value: "+="} | tokens], "")
  end

  defp tokenize_chars(["-", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "MINUSEQ", value: "-="} | tokens], "")
  end

  defp tokenize_chars(["*", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "STAREQ", value: "*="} | tokens], "")
  end

  defp tokenize_chars(["/", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "SLASHEQ", value: "/="} | tokens], "")
  end

  defp tokenize_chars(["%", "=" | rest], tokens, current) do
    tokens = maybe_push_token(tokens, current)
    tokenize_chars(rest, [%{type: "PERCENTEQ", value: "%="} | tokens], "")
  end

  # Dot: check if it's part of a float (e.g., 3.14)
  defp tokenize_chars(["." | rest], tokens, current) do
    if current != "" and String.match?(current, ~r/^[0-9]+$/) do
      # This dot is part of a float literal — keep building the number
      tokenize_chars(rest, tokens, current <> ".")
    else
      tokens = maybe_push_token(tokens, current)
      tokenize_chars(rest, [%{type: "DOT", value: "."} | tokens], "")
    end
  end

  # Single-character operators
  defp tokenize_chars([ch | rest], tokens, current)
       when ch in ["(", ")", "[", "]", "{", "}", ",", ":", ";", "+", "-",
                    "*", "/", "%", "=", "<", ">", "&", "|", "^", "~"] do
    tokens = maybe_push_token(tokens, current)

    type = case ch do
      "(" -> "LPAREN"
      ")" -> "RPAREN"
      "[" -> "LBRACKET"
      "]" -> "RBRACKET"
      "{" -> "LBRACE"
      "}" -> "RBRACE"
      "," -> "COMMA"
      ":" -> "COLON"
      ";" -> "SEMICOLON"
      "+" -> "PLUS"
      "-" -> "MINUS"
      "*" -> "STAR"
      "/" -> "SLASH"
      "%" -> "PERCENT"
      "=" -> "EQUALS"
      "<" -> "LT"
      ">" -> "GT"
      "&" -> "AMP"
      "|" -> "PIPE"
      "^" -> "CARET"
      "~" -> "TILDE"
    end

    tokenize_chars(rest, [%{type: type, value: ch} | tokens], "")
  end

  defp tokenize_chars([ch | rest], tokens, current) do
    tokenize_chars(rest, tokens, current <> ch)
  end

  defp consume_string([], acc), do: {acc, []}
  defp consume_string(["\\", ch | rest], acc), do: consume_string(rest, acc <> unescape(ch))
  defp consume_string(["\"" | rest], acc), do: {acc, rest}
  defp consume_string([ch | rest], acc), do: consume_string(rest, acc <> ch)

  defp consume_single_string([], acc), do: {acc, []}
  defp consume_single_string(["\\", ch | rest], acc), do: consume_single_string(rest, acc <> unescape(ch))
  defp consume_single_string(["'" | rest], acc), do: {acc, rest}
  defp consume_single_string([ch | rest], acc), do: consume_single_string(rest, acc <> ch)

  defp unescape("n"), do: "\n"
  defp unescape("t"), do: "\t"
  defp unescape("\\"), do: "\\"
  defp unescape("\""), do: "\""
  defp unescape("'"), do: "'"
  defp unescape(ch), do: ch

  defp maybe_push_token(tokens, ""), do: tokens
  defp maybe_push_token(tokens, current), do: [classify_token(current) | tokens]

  defp classify_token(text) do
    cond do
      text in ~w(and or not if else elif for in def return pass break continue
                 load lambda True False None) ->
        %{type: "KEYWORD", value: text}

      String.match?(text, ~r/^[0-9]+\.[0-9]*$/) or String.match?(text, ~r/^\.[0-9]+$/) ->
        %{type: "FLOAT", value: text}

      String.match?(text, ~r/^[0-9]+$/) ->
        %{type: "INT", value: text}

      String.match?(text, ~r/^[a-zA-Z_][a-zA-Z0-9_]*$/) ->
        %{type: "NAME", value: text}

      true ->
        %{type: "UNKNOWN", value: text}
    end
  end

  # ===========================================================================
  # Parser — Minimal Recursive Descent Parser for Starlark
  # ===========================================================================
  #
  # This parser transforms a token list into an AST. It handles the core
  # Starlark grammar rules needed for compilation. In a full deployment,
  # you would use the starlark_parser package instead.

  defp parse_tokens(tokens) do
    {stmts, _rest} = parse_file(tokens)
    %{rule_name: "file", children: stmts}
  end

  defp parse_file(tokens) do
    parse_statements(tokens, [])
  end

  defp parse_statements([], acc), do: {Enum.reverse(acc), []}
  defp parse_statements([%{type: "NEWLINE"} | rest], acc), do: parse_statements(rest, acc)
  defp parse_statements([%{type: "INDENT"} | rest], acc), do: parse_statements(rest, acc)
  defp parse_statements([%{type: "DEDENT"} | rest], acc), do: parse_statements(rest, acc)

  defp parse_statements(tokens, acc) do
    case parse_statement(tokens) do
      {nil, rest} -> parse_statements(rest, acc)
      {stmt, rest} -> parse_statements(rest, [stmt | acc])
    end
  end

  defp parse_statement([%{type: "KEYWORD", value: "def"} | rest]) do
    parse_def_stmt(rest)
  end

  defp parse_statement([%{type: "KEYWORD", value: "if"} | rest]) do
    parse_if_stmt(rest)
  end

  defp parse_statement([%{type: "KEYWORD", value: "for"} | rest]) do
    parse_for_stmt(rest)
  end

  defp parse_statement([%{type: "KEYWORD", value: "return"} | rest]) do
    parse_return_stmt(rest)
  end

  defp parse_statement([%{type: "KEYWORD", value: "pass"} | rest]) do
    {%{rule_name: "pass_stmt", children: []}, skip_newlines(rest)}
  end

  defp parse_statement([%{type: "KEYWORD", value: "break"} | rest]) do
    {%{rule_name: "break_stmt", children: []}, skip_newlines(rest)}
  end

  defp parse_statement([%{type: "KEYWORD", value: "continue"} | rest]) do
    {%{rule_name: "continue_stmt", children: []}, skip_newlines(rest)}
  end

  defp parse_statement([%{type: "KEYWORD", value: "load"} | rest]) do
    parse_load_stmt(rest)
  end

  defp parse_statement(tokens) do
    parse_assign_or_expr_stmt(tokens)
  end

  # Parse assignment: name = expr, name += expr, or expression statement
  defp parse_assign_or_expr_stmt(tokens) do
    {lhs, rest} = parse_expression(tokens)

    case rest do
      [%{type: "EQUALS"} | rest2] ->
        {rhs, rest3} = parse_expression(rest2)
        stmt = %{rule_name: "assign_stmt", children: [lhs, %{type: "EQUALS", value: "="}, rhs]}
        {stmt, skip_newlines(rest3)}

      [%{type: type, value: op_val} | rest2]
      when type in ["PLUSEQ", "MINUSEQ", "STAREQ", "SLASHEQ", "PERCENTEQ"] ->
        {rhs, rest3} = parse_expression(rest2)
        stmt = %{rule_name: "augmented_assign_stmt", children: [lhs, %{type: type, value: op_val}, rhs]}
        {stmt, skip_newlines(rest3)}

      _ ->
        stmt = %{rule_name: "expression_stmt", children: [lhs]}
        {stmt, skip_newlines(rest)}
    end
  end

  # ===========================================================================
  # Expression Parsing — Operator Precedence
  # ===========================================================================
  #
  # Expressions are parsed using recursive descent with explicit precedence
  # levels. From lowest to highest:
  #
  # 1. or_expr     (or)
  # 2. and_expr    (and)
  # 3. not_expr    (not)
  # 4. comparison  (==, !=, <, >, <=, >=, in, not in)
  # 5. bitwise_or  (|)
  # 6. bitwise_xor (^)
  # 7. bitwise_and (&)
  # 8. shift       (<<, >>)
  # 9. arith       (+, -)
  # 10. term       (*, /, //, %)
  # 11. factor     (unary -, +, ~)
  # 12. power      (**)
  # 13. primary    (calls, subscripts, attributes)
  # 14. atom       (literals, identifiers, parenthesized exprs)

  defp parse_expression(tokens) do
    parse_ternary(tokens)
  end

  # Ternary: expr if condition else expr
  defp parse_ternary(tokens) do
    {left, rest} = parse_or_expr(tokens)

    case rest do
      [%{type: "KEYWORD", value: "if"} | rest2] ->
        {condition, rest3} = parse_or_expr(rest2)
        case rest3 do
          [%{type: "KEYWORD", value: "else"} | rest4] ->
            {else_val, rest5} = parse_expression(rest4)
            node = %{rule_name: "ternary_expr", children: [left, condition, else_val]}
            {node, rest5}
          _ ->
            # Not a ternary, backtrack — this is tricky; for simplicity, just return
            {left, rest}
        end
      _ ->
        {left, rest}
    end
  end

  defp parse_or_expr(tokens) do
    {left, rest} = parse_and_expr(tokens)
    parse_or_expr_rest(left, rest)
  end

  defp parse_or_expr_rest(left, [%{type: "KEYWORD", value: "or"} | rest]) do
    {right, rest2} = parse_and_expr(rest)
    node = %{rule_name: "or_expr", children: [left, right]}
    parse_or_expr_rest(node, rest2)
  end

  defp parse_or_expr_rest(left, rest), do: {left, rest}

  defp parse_and_expr(tokens) do
    {left, rest} = parse_not_expr(tokens)
    parse_and_expr_rest(left, rest)
  end

  defp parse_and_expr_rest(left, [%{type: "KEYWORD", value: "and"} | rest]) do
    {right, rest2} = parse_not_expr(rest)
    node = %{rule_name: "and_expr", children: [left, right]}
    parse_and_expr_rest(node, rest2)
  end

  defp parse_and_expr_rest(left, rest), do: {left, rest}

  defp parse_not_expr([%{type: "KEYWORD", value: "not"} | rest]) do
    {operand_node, rest2} = parse_not_expr(rest)
    {%{rule_name: "not_expr", children: [operand_node]}, rest2}
  end

  defp parse_not_expr(tokens), do: parse_comparison(tokens)

  defp parse_comparison(tokens) do
    {left, rest} = parse_bitwise_or(tokens)
    parse_comparison_rest(left, rest)
  end

  defp parse_comparison_rest(left, [%{type: type, value: op_val} | rest])
       when type in ["EQEQ", "NEQ", "LT", "GT", "LEQ", "GEQ"] do
    {right, rest2} = parse_bitwise_or(rest)
    node = %{rule_name: "comparison", children: [left, %{type: type, value: op_val}, right]}
    parse_comparison_rest(node, rest2)
  end

  defp parse_comparison_rest(left, [%{type: "KEYWORD", value: "in"} | rest]) do
    {right, rest2} = parse_bitwise_or(rest)
    node = %{rule_name: "comparison", children: [left, %{type: "KEYWORD", value: "in"}, right]}
    parse_comparison_rest(node, rest2)
  end

  defp parse_comparison_rest(left, [%{type: "KEYWORD", value: "not"}, %{type: "KEYWORD", value: "in"} | rest]) do
    {right, rest2} = parse_bitwise_or(rest)
    node = %{rule_name: "comparison", children: [left, %{type: "KEYWORD", value: "not in"}, right]}
    parse_comparison_rest(node, rest2)
  end

  defp parse_comparison_rest(left, rest), do: {left, rest}

  defp parse_bitwise_or(tokens) do
    {left, rest} = parse_bitwise_xor(tokens)
    parse_bitwise_or_rest(left, rest)
  end

  defp parse_bitwise_or_rest(left, [%{type: "PIPE"} | rest]) do
    {right, rest2} = parse_bitwise_xor(rest)
    node = %{rule_name: "bitwise_or", children: [left, %{type: "PIPE", value: "|"}, right]}
    parse_bitwise_or_rest(node, rest2)
  end

  defp parse_bitwise_or_rest(left, rest), do: {left, rest}

  defp parse_bitwise_xor(tokens) do
    {left, rest} = parse_bitwise_and(tokens)
    parse_bitwise_xor_rest(left, rest)
  end

  defp parse_bitwise_xor_rest(left, [%{type: "CARET"} | rest]) do
    {right, rest2} = parse_bitwise_and(rest)
    node = %{rule_name: "bitwise_xor", children: [left, %{type: "CARET", value: "^"}, right]}
    parse_bitwise_xor_rest(node, rest2)
  end

  defp parse_bitwise_xor_rest(left, rest), do: {left, rest}

  defp parse_bitwise_and(tokens) do
    {left, rest} = parse_shift(tokens)
    parse_bitwise_and_rest(left, rest)
  end

  defp parse_bitwise_and_rest(left, [%{type: "AMP"} | rest]) do
    {right, rest2} = parse_shift(rest)
    node = %{rule_name: "bitwise_and", children: [left, %{type: "AMP", value: "&"}, right]}
    parse_bitwise_and_rest(node, rest2)
  end

  defp parse_bitwise_and_rest(left, rest), do: {left, rest}

  defp parse_shift(tokens) do
    {left, rest} = parse_arith(tokens)
    parse_shift_rest(left, rest)
  end

  defp parse_shift_rest(left, [%{type: type, value: op_val} | rest])
       when type in ["LSHIFT", "RSHIFT"] do
    {right, rest2} = parse_arith(rest)
    node = %{rule_name: "shift", children: [left, %{type: type, value: op_val}, right]}
    parse_shift_rest(node, rest2)
  end

  defp parse_shift_rest(left, rest), do: {left, rest}

  defp parse_arith(tokens) do
    {left, rest} = parse_term(tokens)
    parse_arith_rest(left, rest)
  end

  defp parse_arith_rest(left, [%{type: type, value: op_val} | rest])
       when type in ["PLUS", "MINUS"] do
    {right, rest2} = parse_term(rest)
    node = %{rule_name: "arith", children: [left, %{type: type, value: op_val}, right]}
    parse_arith_rest(node, rest2)
  end

  defp parse_arith_rest(left, rest), do: {left, rest}

  defp parse_term(tokens) do
    {left, rest} = parse_factor(tokens)
    parse_term_rest(left, rest)
  end

  defp parse_term_rest(left, [%{type: type, value: _op_val} | rest])
       when type in ["STAR", "SLASH", "FLOORDIV", "PERCENT"] do
    op_str = case type do
      "STAR" -> "*"
      "SLASH" -> "/"
      "FLOORDIV" -> "//"
      "PERCENT" -> "%"
    end
    {right, rest2} = parse_factor(rest)
    node = %{rule_name: "term", children: [left, %{type: type, value: op_str}, right]}
    parse_term_rest(node, rest2)
  end

  defp parse_term_rest(left, rest), do: {left, rest}

  defp parse_factor([%{type: "MINUS"} | rest]) do
    {operand_node, rest2} = parse_factor(rest)
    {%{rule_name: "factor", children: [%{type: "MINUS", value: "-"}, operand_node]}, rest2}
  end

  defp parse_factor([%{type: "PLUS"} | rest]) do
    {operand_node, rest2} = parse_factor(rest)
    {%{rule_name: "factor", children: [%{type: "PLUS", value: "+"}, operand_node]}, rest2}
  end

  defp parse_factor([%{type: "TILDE"} | rest]) do
    {operand_node, rest2} = parse_factor(rest)
    {%{rule_name: "factor", children: [%{type: "TILDE", value: "~"}, operand_node]}, rest2}
  end

  defp parse_factor(tokens), do: parse_power(tokens)

  defp parse_power(tokens) do
    {base_node, rest} = parse_primary(tokens)

    case rest do
      [%{type: "POWER"} | rest2] ->
        {exp_node, rest3} = parse_factor(rest2)
        {%{rule_name: "power_expr", children: [base_node, exp_node]}, rest3}

      _ ->
        {base_node, rest}
    end
  end

  defp parse_primary(tokens) do
    {atom_node, rest} = parse_atom(tokens)
    parse_postfix(atom_node, rest)
  end

  defp parse_postfix(node, [%{type: "LPAREN"} | rest]) do
    {args, rest2} = parse_call_args(rest)
    call = %{rule_name: "call", children: [node | args]}
    parse_postfix(call, rest2)
  end

  defp parse_postfix(node, [%{type: "LBRACKET"} | rest]) do
    # Check for slice
    {idx_or_slice, rest2} = parse_subscript_or_slice(rest)
    sub = %{rule_name: "subscript", children: [node, idx_or_slice]}

    case rest2 do
      [%{type: "RBRACKET"} | rest3] -> parse_postfix(sub, rest3)
      _ -> parse_postfix(sub, rest2)
    end
  end

  defp parse_postfix(node, [%{type: "DOT"}, %{type: "NAME", value: attr} | rest]) do
    dot = %{rule_name: "dot_access", children: [node, %{type: "NAME", value: attr}]}
    parse_postfix(dot, rest)
  end

  defp parse_postfix(node, rest), do: {node, rest}

  defp parse_subscript_or_slice(tokens) do
    # Check if this is a slice (contains colon at top level)
    case tokens do
      [%{type: "COLON"} | _] ->
        parse_slice(nil, tokens)

      _ ->
        {idx_node, rest} = parse_expression(tokens)
        case rest do
          [%{type: "COLON"} | _] ->
            parse_slice(idx_node, rest)
          [%{type: "RBRACKET"} | _] ->
            {idx_node, rest}
          _ ->
            {idx_node, rest}
        end
    end
  end

  defp parse_slice(start_node, [%{type: "COLON"} | rest]) do
    {stop_node, rest2} = case rest do
      [%{type: "RBRACKET"} | _] -> {nil, rest}
      [%{type: "COLON"} | _] -> {nil, rest}
      _ ->
        {node, r} = parse_expression(rest)
        {node, r}
    end

    {step_node, rest3} = case rest2 do
      [%{type: "COLON"} | rest4] ->
        case rest4 do
          [%{type: "RBRACKET"} | _] -> {nil, rest4}
          _ ->
            {node, r} = parse_expression(rest4)
            {node, r}
        end
      _ -> {nil, rest2}
    end

    slice = %{rule_name: "slice", children:
      [start_node, stop_node, step_node] |> Enum.reject(&is_nil/1)}
    {slice, rest3}
  end

  defp parse_call_args(tokens) do
    parse_call_args(tokens, [])
  end

  defp parse_call_args([%{type: "RPAREN"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  # Skip newlines, INDENT, and DEDENT inside call arguments (multiline calls)
  defp parse_call_args([%{type: "NEWLINE"} | rest], acc) do
    parse_call_args(rest, acc)
  end

  defp parse_call_args([%{type: "INDENT"} | rest], acc) do
    parse_call_args(rest, acc)
  end

  defp parse_call_args([%{type: "DEDENT"} | rest], acc) do
    parse_call_args(rest, acc)
  end

  defp parse_call_args(tokens, acc) do
    # Check for keyword argument: name = expr
    {arg, rest} = case tokens do
      [%{type: "NAME", value: name}, %{type: "EQUALS"} | rest2] ->
        {val_node, rest3} = parse_expression(rest2)
        kw = %{rule_name: "keyword_arg", children: [%{type: "NAME", value: name}, val_node]}
        {kw, rest3}

      _ ->
        parse_expression(tokens)
    end

    case skip_whitespace(rest) do
      [%{type: "COMMA"} | rest2] -> parse_call_args(skip_whitespace(rest2), [arg | acc])
      [%{type: "RPAREN"} | rest2] -> {Enum.reverse([arg | acc]), rest2}
      _ -> {Enum.reverse([arg | acc]), rest}
    end
  end

  defp parse_atom([%{type: "INT", value: val} | rest]) do
    {%{rule_name: "number", children: [%{type: "INT", value: val}]}, rest}
  end

  defp parse_atom([%{type: "FLOAT", value: val} | rest]) do
    {%{rule_name: "number", children: [%{type: "FLOAT", value: val}]}, rest}
  end

  defp parse_atom([%{type: "STRING", value: val} | rest]) do
    {%{rule_name: "string_node", children: [%{type: "STRING", value: val}]}, rest}
  end

  defp parse_atom([%{type: "KEYWORD", value: "True"} | rest]) do
    {%{rule_name: "atom", children: [%{type: "KEYWORD", value: "True"}]}, rest}
  end

  defp parse_atom([%{type: "KEYWORD", value: "False"} | rest]) do
    {%{rule_name: "atom", children: [%{type: "KEYWORD", value: "False"}]}, rest}
  end

  defp parse_atom([%{type: "KEYWORD", value: "None"} | rest]) do
    {%{rule_name: "atom", children: [%{type: "KEYWORD", value: "None"}]}, rest}
  end

  defp parse_atom([%{type: "KEYWORD", value: "lambda"} | rest]) do
    parse_lambda(rest)
  end

  defp parse_atom([%{type: "NAME", value: name} | rest]) do
    {%{rule_name: "identifier", children: [%{type: "NAME", value: name}]}, rest}
  end

  defp parse_atom([%{type: "LPAREN"} | rest]) do
    # Parenthesized expression or tuple
    case rest do
      [%{type: "RPAREN"} | rest2] ->
        {%{rule_name: "tuple_expr", children: []}, rest2}

      _ ->
        {first_expr, rest2} = parse_expression(rest)
        case rest2 do
          [%{type: "COMMA"} | rest3] ->
            # This is a tuple
            {more_items, rest4} = parse_comma_separated(rest3, [first_expr])
            {%{rule_name: "tuple_expr", children: more_items}, rest4}

          [%{type: "RPAREN"} | rest3] ->
            # Just a parenthesized expression
            {first_expr, rest3}

          _ ->
            {first_expr, rest2}
        end
    end
  end

  defp parse_atom([%{type: "LBRACKET"} | rest]) do
    parse_list_expr(rest)
  end

  defp parse_atom([%{type: "LBRACE"} | rest]) do
    parse_dict_expr(rest)
  end

  defp parse_atom(tokens) do
    # Fallback — return nil node
    {%{rule_name: "atom", children: []}, tokens}
  end

  defp parse_comma_separated([%{type: "RPAREN"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_comma_separated(tokens, acc) do
    {item, rest} = parse_expression(tokens)
    case rest do
      [%{type: "COMMA"} | rest2] -> parse_comma_separated(rest2, [item | acc])
      [%{type: "RPAREN"} | rest2] -> {Enum.reverse([item | acc]), rest2}
      _ -> {Enum.reverse([item | acc]), rest}
    end
  end

  defp parse_list_expr([%{type: "RBRACKET"} | rest]) do
    {%{rule_name: "list_expr", children: []}, rest}
  end

  defp parse_list_expr(tokens) do
    {first, rest} = parse_expression(tokens)

    # Check for list comprehension: [expr for x in iterable]
    case rest do
      [%{type: "KEYWORD", value: "for"} | rest2] ->
        {comp, rest3} = parse_list_comp_tail(first, rest2)
        {comp, rest3}

      _ ->
        {items, rest2} = parse_list_items(rest, [first])
        {%{rule_name: "list_expr", children: items}, rest2}
    end
  end

  defp parse_list_items([%{type: "RBRACKET"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_list_items([%{type: "COMMA"}, %{type: "RBRACKET"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_list_items([%{type: "COMMA"} | rest], acc) do
    {item, rest2} = parse_expression(rest)
    parse_list_items(rest2, [item | acc])
  end

  defp parse_list_items(rest, acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_list_comp_tail(expr_node, tokens) do
    # Parse: var in iterable (optional if clause) ]
    {var_node, rest} = parse_for_target(tokens)

    rest = case rest do
      [%{type: "KEYWORD", value: "in"} | r] -> r
      r -> r
    end

    {iter_node, rest2} = parse_expression(rest)

    {filter_node, rest3} = case rest2 do
      [%{type: "KEYWORD", value: "if"} | rest4] ->
        {cond_node, rest5} = parse_expression(rest4)
        {cond_node, rest5}
      _ -> {nil, rest2}
    end

    rest3 = case rest3 do
      [%{type: "RBRACKET"} | r] -> r
      r -> r
    end

    children = [expr_node, var_node, iter_node]
    children = if filter_node, do: children ++ [filter_node], else: children

    {%{rule_name: "list_comp", children: children}, rest3}
  end

  defp parse_dict_expr([%{type: "RBRACE"} | rest]) do
    {%{rule_name: "dict_expr", children: []}, rest}
  end

  defp parse_dict_expr(tokens) do
    {key_node, rest} = parse_expression(tokens)

    case rest do
      [%{type: "COLON"} | rest2] ->
        {val_node, rest3} = parse_expression(rest2)
        entry = %{rule_name: "dict_entry", children: [key_node, val_node]}

        # Check for dict comprehension
        case rest3 do
          [%{type: "KEYWORD", value: "for"} | rest4] ->
            parse_dict_comp_tail(key_node, val_node, rest4)

          _ ->
            {entries, rest4} = parse_dict_entries(rest3, [entry])
            {%{rule_name: "dict_expr", children: entries}, rest4}
        end

      _ ->
        {%{rule_name: "dict_expr", children: []}, rest}
    end
  end

  defp parse_dict_entries([%{type: "RBRACE"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_dict_entries([%{type: "COMMA"}, %{type: "RBRACE"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_dict_entries([%{type: "COMMA"} | rest], acc) do
    {key_node, rest2} = parse_expression(rest)
    case rest2 do
      [%{type: "COLON"} | rest3] ->
        {val_node, rest4} = parse_expression(rest3)
        entry = %{rule_name: "dict_entry", children: [key_node, val_node]}
        parse_dict_entries(rest4, [entry | acc])
      _ ->
        {Enum.reverse(acc), rest2}
    end
  end

  defp parse_dict_entries(rest, acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_dict_comp_tail(key_node, val_node, tokens) do
    {var_node, rest} = parse_for_target(tokens)

    rest = case rest do
      [%{type: "KEYWORD", value: "in"} | r] -> r
      r -> r
    end

    {iter_node, rest2} = parse_expression(rest)

    {filter_node, rest3} = case rest2 do
      [%{type: "KEYWORD", value: "if"} | rest4] ->
        {cond_node, rest5} = parse_expression(rest4)
        {cond_node, rest5}
      _ -> {nil, rest2}
    end

    rest3 = case rest3 do
      [%{type: "RBRACE"} | r] -> r
      r -> r
    end

    children = [key_node, val_node, var_node, iter_node]
    children = if filter_node, do: children ++ [filter_node], else: children

    {%{rule_name: "dict_comp", children: children}, rest3}
  end

  defp parse_lambda(tokens) do
    # lambda params: expr
    {params, rest} = parse_lambda_params(tokens, [])
    rest = case rest do
      [%{type: "COLON"} | r] -> r
      r -> r
    end
    {body_node, rest2} = parse_expression(rest)
    {%{rule_name: "lambda_expr", children: params ++ [body_node]}, rest2}
  end

  defp parse_lambda_params([%{type: "COLON"} | _] = tokens, acc) do
    {Enum.reverse(acc), tokens}
  end

  defp parse_lambda_params([%{type: "NAME", value: name} | rest], acc) do
    param = %{rule_name: "param", children: [%{type: "NAME", value: name}]}
    case rest do
      [%{type: "COMMA"} | rest2] -> parse_lambda_params(rest2, [param | acc])
      _ -> {Enum.reverse([param | acc]), rest}
    end
  end

  defp parse_lambda_params(tokens, acc), do: {Enum.reverse(acc), tokens}

  # ===========================================================================
  # Compound Statement Parsing
  # ===========================================================================

  defp parse_if_stmt(tokens) do
    {condition, rest} = parse_expression(tokens)
    rest = skip_colon_newline(rest)
    {body, rest2} = parse_block(rest)

    {elif_clauses, rest3} = parse_elif_clauses(rest2, [])

    {else_clause, rest4} = case skip_newlines(rest3) do
      [%{type: "KEYWORD", value: "else"} | rest5] ->
        rest5 = skip_colon_newline(rest5)
        {else_body, rest6} = parse_block(rest5)
        {[%{rule_name: "else_clause", children: else_body}], rest6}
      other ->
        {[], other}
    end

    children = [condition | body] ++ elif_clauses ++ else_clause
    {%{rule_name: "if_stmt", children: children}, rest4}
  end

  defp parse_elif_clauses(tokens, acc) do
    case skip_newlines(tokens) do
      [%{type: "KEYWORD", value: "elif"} | rest] ->
        {condition, rest2} = parse_expression(rest)
        rest2 = skip_colon_newline(rest2)
        {body, rest3} = parse_block(rest2)
        clause = %{rule_name: "elif_clause", children: [condition | body]}
        parse_elif_clauses(rest3, [clause | acc])

      _ ->
        {Enum.reverse(acc), tokens}
    end
  end

  defp parse_for_stmt(tokens) do
    # Parse only a simple name (or tuple of names) for the loop variable,
    # NOT a full expression — otherwise "x in [1,2,3]" gets parsed as
    # a comparison expression.
    {var_node, rest} = parse_for_target(tokens)

    rest = case rest do
      [%{type: "KEYWORD", value: "in"} | r] -> r
      r -> r
    end

    {iter_node, rest2} = parse_expression(rest)
    rest2 = skip_colon_newline(rest2)
    {body, rest3} = parse_block(rest2)

    {%{rule_name: "for_stmt", children: [var_node, iter_node | body]}, rest3}
  end

  # Parse a for-loop target: just a name or comma-separated names
  defp parse_for_target([%{type: "NAME", value: name}, %{type: "COMMA"} | rest]) do
    # Tuple unpacking: for x, y in ...
    {more, rest2} = parse_for_target(rest)
    names = case more do
      %{rule_name: "tuple_expr", children: items} ->
        [%{rule_name: "identifier", children: [%{type: "NAME", value: name}]} | items]
      single ->
        [%{rule_name: "identifier", children: [%{type: "NAME", value: name}]}, single]
    end
    {%{rule_name: "tuple_expr", children: names}, rest2}
  end

  defp parse_for_target([%{type: "NAME", value: name} | rest]) do
    {%{rule_name: "identifier", children: [%{type: "NAME", value: name}]}, rest}
  end

  defp parse_for_target(tokens) do
    # Fallback to atom parsing for complex targets
    parse_atom(tokens)
  end

  defp parse_def_stmt(tokens) do
    case tokens do
      [%{type: "NAME", value: func_name} | rest] ->
        rest = case rest do
          [%{type: "LPAREN"} | r] -> r
          r -> r
        end

        {params, rest2} = parse_param_list(rest, [])
        rest2 = skip_colon_newline(rest2)
        {body, rest3} = parse_block(rest2)

        param_list = %{rule_name: "param_list", children: params}
        children = [%{type: "NAME", value: func_name}, param_list | body]
        {%{rule_name: "def_stmt", children: children}, rest3}

      _ ->
        {nil, tokens}
    end
  end

  defp parse_param_list([%{type: "RPAREN"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  defp parse_param_list([%{type: "NAME", value: name} | rest], acc) do
    # Check for default value
    {param, rest2} = case rest do
      [%{type: "EQUALS"} | rest2] ->
        {default_val, rest3} = parse_expression(rest2)
        {%{rule_name: "param", children: [%{type: "NAME", value: name}, default_val]}, rest3}
      _ ->
        {%{rule_name: "param", children: [%{type: "NAME", value: name}]}, rest}
    end

    case rest2 do
      [%{type: "COMMA"} | rest3] -> parse_param_list(rest3, [param | acc])
      [%{type: "RPAREN"} | rest3] -> {Enum.reverse([param | acc]), rest3}
      _ -> {Enum.reverse([param | acc]), rest2}
    end
  end

  defp parse_param_list(tokens, acc), do: {Enum.reverse(acc), tokens}

  defp parse_return_stmt(tokens) do
    case tokens do
      [%{type: "NEWLINE"} | _] ->
        {%{rule_name: "return_stmt", children: []}, skip_newlines(tokens)}
      [] ->
        {%{rule_name: "return_stmt", children: []}, []}
      _ ->
        {expr_node, rest} = parse_expression(tokens)
        {%{rule_name: "return_stmt", children: [expr_node]}, skip_newlines(rest)}
    end
  end

  defp parse_load_stmt(tokens) do
    # load("module.star", "symbol1", "symbol2", ...)
    tokens = case tokens do
      [%{type: "LPAREN"} | r] -> r
      r -> r
    end

    {module_node, rest} = parse_expression(tokens)

    {symbols, rest2} = parse_load_symbols(rest, [])

    rest2 = case rest2 do
      [%{type: "RPAREN"} | r] -> r
      r -> r
    end

    {%{rule_name: "load_stmt", children: [module_node | symbols]}, skip_newlines(rest2)}
  end

  defp parse_load_symbols([%{type: "COMMA"} | rest], acc) do
    {sym_node, rest2} = parse_expression(rest)
    case rest2 do
      [%{type: "RPAREN"} | _] -> {Enum.reverse([sym_node | acc]), rest2}
      _ -> parse_load_symbols(rest2, [sym_node | acc])
    end
  end

  defp parse_load_symbols(tokens, acc), do: {Enum.reverse(acc), tokens}

  # ===========================================================================
  # Block Parsing
  # ===========================================================================
  #
  # A block is a sequence of indented statements. Since our tokenizer does
  # not track indentation explicitly, we use a heuristic: keep parsing
  # statements until we hit an unindented keyword (else, elif, def at top
  # level, etc.) or run out of tokens.

  defp parse_block(tokens) do
    # Skip optional INDENT token (should be present for properly indented blocks)
    tokens = case tokens do
      [%{type: "INDENT"} | rest] -> rest
      other -> other
    end
    parse_block(tokens, [])
  end

  defp parse_block([], acc), do: {Enum.reverse(acc), []}

  defp parse_block([%{type: "NEWLINE"} | rest], acc) do
    parse_block(rest, acc)
  end

  # DEDENT marks the end of a block
  defp parse_block([%{type: "DEDENT"} | rest], acc) do
    {Enum.reverse(acc), rest}
  end

  # Stop at dedent markers (keywords that start a new clause at same level)
  defp parse_block([%{type: "KEYWORD", value: kw} | _] = tokens, acc)
       when kw in ["else", "elif"] do
    {Enum.reverse(acc), tokens}
  end

  defp parse_block(tokens, acc) do
    case parse_statement(tokens) do
      {nil, rest} -> {Enum.reverse(acc), rest}
      {stmt, rest} -> parse_block(rest, [stmt | acc])
    end
  end

  defp skip_newlines([%{type: "NEWLINE"} | rest]), do: skip_newlines(rest)
  defp skip_newlines(tokens), do: tokens

  # Like skip_newlines but also skips INDENT/DEDENT tokens.
  # Used in contexts where indentation tokens are noise (e.g., between
  # elif/else clauses, inside multiline function calls).
  defp skip_whitespace([%{type: "NEWLINE"} | rest]), do: skip_whitespace(rest)
  defp skip_whitespace([%{type: "INDENT"} | rest]), do: skip_whitespace(rest)
  defp skip_whitespace([%{type: "DEDENT"} | rest]), do: skip_whitespace(rest)
  defp skip_whitespace(tokens), do: tokens

  defp skip_colon_newline([%{type: "COLON"} | rest]), do: skip_newlines(rest)
  defp skip_colon_newline([%{type: "NEWLINE"} | rest]), do: skip_newlines(rest)
  defp skip_colon_newline(tokens), do: tokens

  # ===========================================================================
  # Rule Handler Registration
  # ===========================================================================
  #
  # Each function below registers a group of related rule handlers with
  # the GenericCompiler. The handlers are closures that capture the Op
  # module for opcode constants.

  defp register_top_level_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("file", &handle_file/2)
    |> GenericCompiler.register_rule("suite", &handle_suite/2)
    |> GenericCompiler.register_rule("simple_stmt", &handle_simple_stmt/2)
    |> GenericCompiler.register_rule("small_stmt", &handle_small_stmt/2)
    |> GenericCompiler.register_rule("compound_stmt", &handle_compound_stmt/2)
  end

  defp register_statement_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("assign_stmt", &handle_assign_stmt/2)
    |> GenericCompiler.register_rule("augmented_assign_stmt", &handle_augmented_assign_stmt/2)
    |> GenericCompiler.register_rule("return_stmt", &handle_return_stmt/2)
    |> GenericCompiler.register_rule("pass_stmt", &handle_pass_stmt/2)
    |> GenericCompiler.register_rule("break_stmt", &handle_break_stmt/2)
    |> GenericCompiler.register_rule("continue_stmt", &handle_continue_stmt/2)
    |> GenericCompiler.register_rule("if_stmt", &handle_if_stmt/2)
    |> GenericCompiler.register_rule("elif_clause", &handle_elif_clause/2)
    |> GenericCompiler.register_rule("else_clause", &handle_else_clause/2)
    |> GenericCompiler.register_rule("for_stmt", &handle_for_stmt/2)
    |> GenericCompiler.register_rule("load_stmt", &handle_load_stmt/2)
    |> GenericCompiler.register_rule("expression_stmt", &handle_expression_stmt/2)
  end

  defp register_expression_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("expr", &handle_expr/2)
    |> GenericCompiler.register_rule("or_expr", &handle_or_expr/2)
    |> GenericCompiler.register_rule("and_expr", &handle_and_expr/2)
    |> GenericCompiler.register_rule("not_expr", &handle_not_expr/2)
    |> GenericCompiler.register_rule("comparison", &handle_comparison/2)
    |> GenericCompiler.register_rule("ternary_expr", &handle_ternary_expr/2)
    |> GenericCompiler.register_rule("bitwise_or", &handle_binary_op/2)
    |> GenericCompiler.register_rule("bitwise_xor", &handle_binary_op/2)
    |> GenericCompiler.register_rule("bitwise_and", &handle_binary_op/2)
    |> GenericCompiler.register_rule("shift", &handle_binary_op/2)
    |> GenericCompiler.register_rule("arith", &handle_binary_op/2)
    |> GenericCompiler.register_rule("term", &handle_binary_op/2)
    |> GenericCompiler.register_rule("factor", &handle_factor/2)
    |> GenericCompiler.register_rule("power_expr", &handle_power_expr/2)
    |> GenericCompiler.register_rule("primary", &handle_primary/2)
    |> GenericCompiler.register_rule("call", &handle_call/2)
    |> GenericCompiler.register_rule("call_args", &handle_call_args/2)
    |> GenericCompiler.register_rule("keyword_arg", &handle_keyword_arg/2)
    |> GenericCompiler.register_rule("dot_access", &handle_dot_access/2)
    |> GenericCompiler.register_rule("subscript", &handle_subscript/2)
    |> GenericCompiler.register_rule("slice", &handle_slice/2)
  end

  defp register_literal_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("atom", &handle_atom/2)
    |> GenericCompiler.register_rule("number", &handle_number/2)
    |> GenericCompiler.register_rule("string_node", &handle_string_node/2)
    |> GenericCompiler.register_rule("list_expr", &handle_list_expr/2)
    |> GenericCompiler.register_rule("list_comp", &handle_list_comp/2)
    |> GenericCompiler.register_rule("dict_expr", &handle_dict_expr/2)
    |> GenericCompiler.register_rule("dict_comp", &handle_dict_comp/2)
    |> GenericCompiler.register_rule("dict_entry", &handle_dict_entry/2)
    |> GenericCompiler.register_rule("tuple_expr", &handle_tuple_expr/2)
    |> GenericCompiler.register_rule("lambda_expr", &handle_lambda_expr/2)
  end

  defp register_definition_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("def_stmt", &handle_def_stmt/2)
    |> GenericCompiler.register_rule("param_list", &handle_param_list/2)
    |> GenericCompiler.register_rule("param", &handle_param/2)
  end

  defp register_comprehension_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("comp_clause", &handle_comp_clause/2)
    |> GenericCompiler.register_rule("comp_if", &handle_comp_if/2)
  end

  defp register_identifier_rules(compiler) do
    compiler
    |> GenericCompiler.register_rule("identifier", &handle_identifier/2)
  end

  # ===========================================================================
  # Top-Level Rule Handlers
  # ===========================================================================

  defp handle_file(compiler, node) do
    # A file is a sequence of statements. Compile each one.
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_suite(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_simple_stmt(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_small_stmt(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_compound_stmt(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  # ===========================================================================
  # Statement Rule Handlers
  # ===========================================================================

  defp handle_assign_stmt(compiler, node) do
    # Children: [lhs, "=", rhs]
    # Compile RHS first (push value onto stack), then store to LHS
    children = node.children
    lhs = Enum.at(children, 0)
    rhs = Enum.at(children, 2) || Enum.at(children, 1)

    # Compile the value expression
    compiler = GenericCompiler.compile_node(compiler, rhs)

    # Store to the target
    emit_store(compiler, lhs)
  end

  defp handle_augmented_assign_stmt(compiler, node) do
    # Children: [lhs, op_token, rhs]
    # Equivalent to: lhs = lhs op rhs
    children = node.children
    lhs = Enum.at(children, 0)
    op_token = Enum.at(children, 1)
    rhs = Enum.at(children, 2)

    # Load current value
    compiler = emit_load(compiler, lhs)

    # Compile the RHS
    compiler = GenericCompiler.compile_node(compiler, rhs)

    # Emit the arithmetic operation
    opcode = Map.get(Op.augmented_assign_map(), op_token.value, Op.add())
    {_idx, compiler} = GenericCompiler.emit(compiler, opcode)

    # Store back
    emit_store(compiler, lhs)
  end

  defp handle_return_stmt(compiler, node) do
    if node.children == [] do
      # return with no value -> return None
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_none())
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.return_op())
      compiler
    else
      # return expr
      compiler = GenericCompiler.compile_node(compiler, hd(node.children))
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.return_op())
      compiler
    end
  end

  defp handle_pass_stmt(compiler, _node) do
    # pass is a no-op — emit nothing
    compiler
  end

  defp handle_break_stmt(compiler, _node) do
    # break — emit a jump placeholder. The for loop handler will patch it.
    # For simplicity, we emit a JUMP with operand 0 (will be patched)
    {_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump())
    compiler
  end

  defp handle_continue_stmt(compiler, _node) do
    # continue — emit a jump back to loop start (placeholder)
    {_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump())
    compiler
  end

  defp handle_if_stmt(compiler, node) do
    # Children: [condition, body_stmts..., elif_clauses..., else_clause?]
    children = node.children

    # First child is the condition
    condition = hd(children)
    rest_children = tl(children)

    # Separate body statements from elif/else clauses
    {body_stmts, clause_children} = Enum.split_while(rest_children, fn child ->
      case child do
        %{rule_name: name} when name in ["elif_clause", "else_clause"] -> false
        _ -> true
      end
    end)

    # Compile condition
    compiler = GenericCompiler.compile_node(compiler, condition)

    # Jump over body if false
    {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_false())

    # Compile body
    compiler = Enum.reduce(body_stmts, compiler, fn stmt, comp ->
      GenericCompiler.compile_node(comp, stmt)
    end)

    # Jump to end (skip elif/else)
    {end_jump_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump())

    # Patch the false jump to here
    compiler = GenericCompiler.patch_jump(compiler, jump_idx)

    # Compile elif and else clauses
    {compiler, end_jumps} = Enum.reduce(clause_children, {compiler, [end_jump_idx]}, fn clause, {comp, jumps} ->
      case clause.rule_name do
        "elif_clause" ->
          [elif_cond | elif_body] = clause.children
          comp = GenericCompiler.compile_node(comp, elif_cond)
          {elif_jump_idx, comp} = GenericCompiler.emit_jump(comp, Op.jump_if_false())
          comp = Enum.reduce(elif_body, comp, fn stmt, c ->
            GenericCompiler.compile_node(c, stmt)
          end)
          {elif_end_jump, comp} = GenericCompiler.emit_jump(comp, Op.jump())
          comp = GenericCompiler.patch_jump(comp, elif_jump_idx)
          {comp, [elif_end_jump | jumps]}

        "else_clause" ->
          comp = Enum.reduce(clause.children, comp, fn stmt, c ->
            GenericCompiler.compile_node(c, stmt)
          end)
          {comp, jumps}
      end
    end)

    # Patch all end jumps to here
    Enum.reduce(end_jumps, compiler, fn jump_idx_val, comp ->
      GenericCompiler.patch_jump(comp, jump_idx_val)
    end)
  end

  defp handle_elif_clause(compiler, node) do
    # Handled inline by if_stmt
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_else_clause(compiler, node) do
    # Handled inline by if_stmt
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_for_stmt(compiler, node) do
    # Children: [var, iterable, body_stmts...]
    [var_node, iter_node | body] = node.children

    # Compile the iterable expression
    compiler = GenericCompiler.compile_node(compiler, iter_node)

    # Get an iterator
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.get_iter())

    # Loop start: FOR_ITER (jumps to end when exhausted)
    loop_start = GenericCompiler.current_offset(compiler)
    {for_iter_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.for_iter())

    # Store the loop variable
    compiler = emit_store(compiler, var_node)

    # Compile body
    compiler = Enum.reduce(body, compiler, fn stmt, comp ->
      GenericCompiler.compile_node(comp, stmt)
    end)

    # Jump back to loop start
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.jump(), loop_start)

    # Patch FOR_ITER to jump here when exhausted
    compiler = GenericCompiler.patch_jump(compiler, for_iter_idx)

    compiler
  end

  defp handle_load_stmt(compiler, node) do
    # Children: [module_string, symbol1, symbol2, ...]
    [module_node | symbols] = node.children

    # Get the module path string
    module_path = get_string_value(module_node)

    # Emit LOAD_MODULE
    {module_name_idx, compiler} = GenericCompiler.add_name(compiler, module_path)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_module(), module_name_idx)

    # For each symbol, emit DUP + IMPORT_FROM + STORE_NAME
    compiler = Enum.reduce(symbols, compiler, fn sym, comp ->
      sym_name = get_string_value(sym)
      {sym_name_idx, comp} = GenericCompiler.add_name(comp, sym_name)
      {_idx, comp} = GenericCompiler.emit(comp, Op.dup())
      {_idx, comp} = GenericCompiler.emit(comp, Op.import_from(), sym_name_idx)
      {_idx, comp} = GenericCompiler.emit(comp, Op.store_name(), sym_name_idx)
      comp
    end)

    # Pop the module dict
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.pop())
    compiler
  end

  defp handle_expression_stmt(compiler, node) do
    # Compile the expression, then pop the result (expression statements
    # are evaluated for side effects only)
    compiler = GenericCompiler.compile_node(compiler, hd(node.children))
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.pop())
    compiler
  end

  # ===========================================================================
  # Expression Rule Handlers
  # ===========================================================================

  defp handle_expr(compiler, node) do
    # An expr node just wraps another expression — pass through
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_or_expr(compiler, node) do
    # Short-circuit or: if left is truthy, skip right
    [left, right] = node.children
    compiler = GenericCompiler.compile_node(compiler, left)
    {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_true_or_pop())
    compiler = GenericCompiler.compile_node(compiler, right)
    GenericCompiler.patch_jump(compiler, jump_idx)
  end

  defp handle_and_expr(compiler, node) do
    # Short-circuit and: if left is falsy, skip right
    [left, right] = node.children
    compiler = GenericCompiler.compile_node(compiler, left)
    {jump_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_false_or_pop())
    compiler = GenericCompiler.compile_node(compiler, right)
    GenericCompiler.patch_jump(compiler, jump_idx)
  end

  defp handle_not_expr(compiler, node) do
    [operand_node] = node.children
    compiler = GenericCompiler.compile_node(compiler, operand_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.logical_not())
    compiler
  end

  defp handle_comparison(compiler, node) do
    # Children: [left, op_token, right]
    [left, op_token, right] = node.children
    compiler = GenericCompiler.compile_node(compiler, left)
    compiler = GenericCompiler.compile_node(compiler, right)

    opcode = Map.get(Op.compare_op_map(), op_token.value, Op.cmp_eq())
    {_idx, compiler} = GenericCompiler.emit(compiler, opcode)
    compiler
  end

  defp handle_ternary_expr(compiler, node) do
    # Children: [value_if_true, condition, value_if_false]
    [val_true, condition, val_false] = node.children

    # Compile condition
    compiler = GenericCompiler.compile_node(compiler, condition)
    {false_jump, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_false())

    # Compile true branch
    compiler = GenericCompiler.compile_node(compiler, val_true)
    {end_jump, compiler} = GenericCompiler.emit_jump(compiler, Op.jump())

    # Patch false jump
    compiler = GenericCompiler.patch_jump(compiler, false_jump)

    # Compile false branch
    compiler = GenericCompiler.compile_node(compiler, val_false)

    # Patch end jump
    GenericCompiler.patch_jump(compiler, end_jump)
  end

  defp handle_binary_op(compiler, node) do
    # Children: [left, op_token, right]
    [left, op_token, right] = node.children

    compiler = GenericCompiler.compile_node(compiler, left)
    compiler = GenericCompiler.compile_node(compiler, right)

    opcode = Map.get(Op.binary_op_map(), op_token.value, Op.add())
    {_idx, compiler} = GenericCompiler.emit(compiler, opcode)
    compiler
  end

  defp handle_factor(compiler, node) do
    # Children: [op_token, operand] for unary ops
    case node.children do
      [%{type: _, value: op_val}, operand_node] ->
        compiler = GenericCompiler.compile_node(compiler, operand_node)

        case op_val do
          "-" ->
            {_idx, compiler} = GenericCompiler.emit(compiler, Op.negate())
            compiler
          "~" ->
            {_idx, compiler} = GenericCompiler.emit(compiler, Op.bit_not())
            compiler
          "+" ->
            # Unary + is a no-op
            compiler
        end

      [single] ->
        GenericCompiler.compile_node(compiler, single)
    end
  end

  defp handle_power_expr(compiler, node) do
    [base_node, exp_node] = node.children
    compiler = GenericCompiler.compile_node(compiler, base_node)
    compiler = GenericCompiler.compile_node(compiler, exp_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.power())
    compiler
  end

  defp handle_primary(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_call(compiler, node) do
    # Children: [callee, arg1, arg2, ...]
    [callee | args] = node.children

    # Check for keyword args
    has_kwargs = Enum.any?(args, fn
      %{rule_name: "keyword_arg"} -> true
      _ -> false
    end)

    # Compile the callee
    compiler = GenericCompiler.compile_node(compiler, callee)

    # Compile each argument
    {compiler, kw_names} = Enum.reduce(args, {compiler, []}, fn arg, {comp, kws} ->
      case arg do
        %{rule_name: "keyword_arg", children: [%{type: "NAME", value: name}, val]} ->
          comp = GenericCompiler.compile_node(comp, val)
          {comp, [name | kws]}

        _ ->
          comp = GenericCompiler.compile_node(comp, arg)
          {comp, kws}
      end
    end)

    if has_kwargs do
      # Push keyword names as a tuple constant
      kw_names = Enum.reverse(kw_names)
      kw_tuple = List.to_tuple(kw_names)
      {kw_idx, compiler} = GenericCompiler.add_constant(compiler, kw_tuple)
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), kw_idx)
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.call_function_kw(), length(args))
      compiler
    else
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.call_function(), length(args))
      compiler
    end
  end

  defp handle_call_args(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_keyword_arg(compiler, node) do
    [_name, val_node] = node.children
    GenericCompiler.compile_node(compiler, val_node)
  end

  defp handle_dot_access(compiler, node) do
    [obj_node, %{type: "NAME", value: attr_name}] = node.children
    compiler = GenericCompiler.compile_node(compiler, obj_node)
    {attr_idx, compiler} = GenericCompiler.add_name(compiler, attr_name)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_attr(), attr_idx)
    compiler
  end

  defp handle_subscript(compiler, node) do
    [obj_node, idx_node] = node.children
    compiler = GenericCompiler.compile_node(compiler, obj_node)

    case idx_node do
      %{rule_name: "slice"} ->
        handle_slice_emit(compiler, idx_node)

      _ ->
        compiler = GenericCompiler.compile_node(compiler, idx_node)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_subscript())
        compiler
    end
  end

  defp handle_slice(compiler, node) do
    handle_slice_emit(compiler, node)
  end

  defp handle_slice_emit(compiler, node) do
    # Slice children are the present components (start, stop, step)
    # We need to figure out which are present
    children = node.children
    count = length(children)

    # Emit each component
    compiler = Enum.reduce(children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)

    # Flags encode which components are present
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_slice(), count)
    compiler
  end

  # ===========================================================================
  # Literal Rule Handlers
  # ===========================================================================

  defp handle_atom(compiler, node) do
    case node.children do
      [%{type: "KEYWORD", value: "True"}] ->
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_true())
        compiler

      [%{type: "KEYWORD", value: "False"}] ->
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_false())
        compiler

      [%{type: "KEYWORD", value: "None"}] ->
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_none())
        compiler

      [] ->
        # Empty atom — push None
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_none())
        compiler

      [child] ->
        GenericCompiler.compile_node(compiler, child)
    end
  end

  defp handle_number(compiler, node) do
    token = hd(node.children)
    value = case token.type do
      "INT" -> String.to_integer(token.value)
      "FLOAT" -> String.to_float(normalize_float(token.value))
      _ -> String.to_integer(token.value)
    end

    {idx, compiler} = GenericCompiler.add_constant(compiler, value)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), idx)
    compiler
  end

  defp normalize_float(str) do
    cond do
      String.starts_with?(str, ".") -> "0" <> str
      String.ends_with?(str, ".") -> str <> "0"
      true -> str
    end
  end

  defp handle_string_node(compiler, node) do
    token = hd(node.children)
    {idx, compiler} = GenericCompiler.add_constant(compiler, token.value)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), idx)
    compiler
  end

  defp handle_list_expr(compiler, node) do
    # Compile each element
    compiler = Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)

    {_idx, compiler} = GenericCompiler.emit(compiler, Op.build_list(), length(node.children))
    compiler
  end

  defp handle_list_comp(compiler, node) do
    # Children: [expr, var, iterable, optional_filter]
    [expr_node, var_node, iter_node | filter] = node.children

    # Build empty list
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.build_list(), 0)

    # Compile iterable
    compiler = GenericCompiler.compile_node(compiler, iter_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.get_iter())

    # Loop start
    loop_start = GenericCompiler.current_offset(compiler)
    {for_iter_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.for_iter())

    # Store loop variable
    compiler = emit_store(compiler, var_node)

    # Optional filter
    {compiler, filter_jump} = case filter do
      [filter_node] ->
        compiler = GenericCompiler.compile_node(compiler, filter_node)
        {idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_false())
        {compiler, idx}
      [] ->
        {compiler, nil}
    end

    # Compile expression and append to list
    compiler = GenericCompiler.compile_node(compiler, expr_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.list_append())

    # Patch filter jump if present
    compiler = if filter_jump, do: GenericCompiler.patch_jump(compiler, filter_jump), else: compiler

    # Jump back to loop start
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.jump(), loop_start)

    # Patch FOR_ITER
    GenericCompiler.patch_jump(compiler, for_iter_idx)
  end

  defp handle_dict_expr(compiler, node) do
    # Children are dict_entry nodes
    compiler = Enum.reduce(node.children, compiler, fn entry, comp ->
      GenericCompiler.compile_node(comp, entry)
    end)

    {_idx, compiler} = GenericCompiler.emit(compiler, Op.build_dict(), length(node.children))
    compiler
  end

  defp handle_dict_comp(compiler, node) do
    # Children: [key, value, var, iterable, optional_filter]
    [key_node, val_node, var_node, iter_node | filter] = node.children

    # Build empty dict
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.build_dict(), 0)

    # Compile iterable
    compiler = GenericCompiler.compile_node(compiler, iter_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.get_iter())

    loop_start = GenericCompiler.current_offset(compiler)
    {for_iter_idx, compiler} = GenericCompiler.emit_jump(compiler, Op.for_iter())

    compiler = emit_store(compiler, var_node)

    # Optional filter
    {compiler, filter_jump} = case filter do
      [filter_node] ->
        compiler = GenericCompiler.compile_node(compiler, filter_node)
        {idx, compiler} = GenericCompiler.emit_jump(compiler, Op.jump_if_false())
        {compiler, idx}
      [] ->
        {compiler, nil}
    end

    # Compile key and value, then set in dict
    compiler = GenericCompiler.compile_node(compiler, key_node)
    compiler = GenericCompiler.compile_node(compiler, val_node)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.dict_set())

    compiler = if filter_jump, do: GenericCompiler.patch_jump(compiler, filter_jump), else: compiler

    {_idx, compiler} = GenericCompiler.emit(compiler, Op.jump(), loop_start)
    GenericCompiler.patch_jump(compiler, for_iter_idx)
  end

  defp handle_dict_entry(compiler, node) do
    # Children: [key, value]
    [key_node, val_node] = node.children
    compiler = GenericCompiler.compile_node(compiler, key_node)
    GenericCompiler.compile_node(compiler, val_node)
  end

  defp handle_tuple_expr(compiler, node) do
    compiler = Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)

    {_idx, compiler} = GenericCompiler.emit(compiler, Op.build_tuple(), length(node.children))
    compiler
  end

  defp handle_lambda_expr(compiler, node) do
    # Children: [param1, param2, ..., body_expr]
    params = Enum.filter(node.children, fn
      %{rule_name: "param"} -> true
      _ -> false
    end)

    body_node = List.last(node.children)
    param_names = Enum.map(params, fn p ->
      case p.children do
        [%{type: "NAME", value: name} | _] -> name
        _ -> "_"
      end
    end)

    # Compile body as nested code object
    body_with_return = %{rule_name: "return_stmt", children: [body_node]}
    {nested_code, compiler} = GenericCompiler.compile_nested(compiler, body_with_return)

    # Add the code object as a constant
    {code_idx, compiler} = GenericCompiler.add_constant(compiler, nested_code)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), code_idx)

    # Add parameter names tuple as a constant
    {names_idx, compiler} = GenericCompiler.add_constant(compiler, List.to_tuple(param_names))
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), names_idx)

    # Emit MAKE_FUNCTION
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.make_function(), length(params))
    compiler
  end

  # ===========================================================================
  # Definition Rule Handlers
  # ===========================================================================

  defp handle_def_stmt(compiler, node) do
    # Children: [name_token, param_list, body_stmts...]
    [%{type: "NAME", value: func_name}, param_list_node | body] = node.children

    # Extract parameter names
    param_names = Enum.map(param_list_node.children, fn p ->
      case p.children do
        [%{type: "NAME", value: name} | _] -> name
        _ -> "_"
      end
    end)

    # Check for default values
    defaults = param_list_node.children
    |> Enum.filter(fn p -> length(p.children) > 1 end)
    |> Enum.map(fn p -> Enum.at(p.children, 1) end)

    # Compile defaults onto the stack
    compiler = Enum.reduce(defaults, compiler, fn default_node, comp ->
      GenericCompiler.compile_node(comp, default_node)
    end)

    compiler = if length(defaults) > 0 do
      {_idx, comp} = GenericCompiler.emit(compiler, Op.build_tuple(), length(defaults))
      comp
    else
      compiler
    end

    # Compile body as nested code object
    body_node = %{rule_name: "suite", children: body}
    {nested_code, compiler} = GenericCompiler.compile_nested(compiler, body_node)

    # Add the code object as a constant
    {code_idx, compiler} = GenericCompiler.add_constant(compiler, nested_code)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), code_idx)

    # Add parameter names tuple as a constant
    {names_idx, compiler} = GenericCompiler.add_constant(compiler, List.to_tuple(param_names))
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_const(), names_idx)

    # Flags: bit 0 = has defaults
    flags = if length(defaults) > 0, do: 1, else: 0
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.make_function(), flags)

    # Store the function
    {name_idx, compiler} = GenericCompiler.add_name(compiler, func_name)
    {_idx, compiler} = GenericCompiler.emit(compiler, Op.store_name(), name_idx)

    compiler
  end

  defp handle_param_list(compiler, node) do
    # Parameters are handled by def_stmt, no direct bytecode needed
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_param(compiler, _node) do
    # Parameters are handled by def_stmt
    compiler
  end

  # ===========================================================================
  # Comprehension Rule Handlers
  # ===========================================================================

  defp handle_comp_clause(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  defp handle_comp_if(compiler, node) do
    Enum.reduce(node.children, compiler, fn child, comp ->
      GenericCompiler.compile_node(comp, child)
    end)
  end

  # ===========================================================================
  # Identifier Rule Handler
  # ===========================================================================

  defp handle_identifier(compiler, node) do
    [%{type: "NAME", value: name}] = node.children

    # Check if we're in a scope with locals
    if compiler.scope != nil do
      case GenericCompiler.CompilerScope.get_local(compiler.scope, name) do
        nil ->
          # Not a local — use LOAD_NAME
          {idx, compiler} = GenericCompiler.add_name(compiler, name)
          {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_name(), idx)
          compiler

        slot ->
          {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_local(), slot)
          compiler
      end
    else
      {idx, compiler} = GenericCompiler.add_name(compiler, name)
      {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_name(), idx)
      compiler
    end
  end

  # ===========================================================================
  # Helper Functions
  # ===========================================================================

  defp emit_store(compiler, node) do
    case node do
      %{rule_name: "identifier", children: [%{type: "NAME", value: name}]} ->
        {idx, compiler} = GenericCompiler.add_name(compiler, name)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.store_name(), idx)
        compiler

      %{rule_name: "subscript", children: [obj, idx_node]} ->
        compiler = GenericCompiler.compile_node(compiler, obj)
        compiler = GenericCompiler.compile_node(compiler, idx_node)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.store_subscript())
        compiler

      %{rule_name: "dot_access", children: [obj, %{type: "NAME", value: attr}]} ->
        compiler = GenericCompiler.compile_node(compiler, obj)
        {attr_idx, compiler} = GenericCompiler.add_name(compiler, attr)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.store_attr(), attr_idx)
        compiler

      %{type: "NAME", value: name} ->
        {idx, compiler} = GenericCompiler.add_name(compiler, name)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.store_name(), idx)
        compiler

      _ ->
        # Fallback: try to get a name from the node
        compiler
    end
  end

  defp emit_load(compiler, node) do
    case node do
      %{rule_name: "identifier", children: [%{type: "NAME", value: name}]} ->
        {idx, compiler} = GenericCompiler.add_name(compiler, name)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_name(), idx)
        compiler

      %{rule_name: "subscript", children: [obj, idx_node]} ->
        compiler = GenericCompiler.compile_node(compiler, obj)
        compiler = GenericCompiler.compile_node(compiler, idx_node)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_subscript())
        compiler

      %{rule_name: "dot_access", children: [obj, %{type: "NAME", value: attr}]} ->
        compiler = GenericCompiler.compile_node(compiler, obj)
        {attr_idx, compiler} = GenericCompiler.add_name(compiler, attr)
        {_idx, compiler} = GenericCompiler.emit(compiler, Op.load_attr(), attr_idx)
        compiler

      _ ->
        GenericCompiler.compile_node(compiler, node)
    end
  end

  defp get_string_value(node) do
    case node do
      %{rule_name: "string_node", children: [%{type: "STRING", value: val}]} -> val
      %{type: "STRING", value: val} -> val
      %{rule_name: "identifier", children: [%{type: "NAME", value: val}]} -> val
      %{type: "NAME", value: val} -> val
      _ -> inspect(node)
    end
  end
end
