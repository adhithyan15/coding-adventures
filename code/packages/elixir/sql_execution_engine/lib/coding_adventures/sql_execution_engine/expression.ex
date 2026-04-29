defmodule CodingAdventures.SqlExecutionEngine.Expression do
  @moduledoc """
  Recursive expression evaluator for SQL expressions.

  ## Overview

  The SQL grammar produces a deeply nested AST for expressions.  This module
  walks that tree and computes a scalar value — an integer, float, string,
  boolean, or nil — for a single row.

  The central function is:

      eval_expr(node, row_ctx) :: term()

  where `row_ctx` is a flat map of column values for the current row:

      %{
        "employees.id"     => 1,
        "id"               => 1,
        "employees.name"   => "Alice",
        "name"             => "Alice",
        …
      }

  Both the qualified form (`table.column`) and bare form (`column`) are
  stored so that expressions can use either.  If two tables have a column
  with the same bare name the bare form maps to whichever table was
  processed last (typically the left/first table), which matches how most
  SQL databases handle ambiguity in practice.

  ## Three-Valued Logic (NULL handling)

  SQL's NULL is not a value — it's the *absence* of a value.  Any comparison
  involving NULL yields NULL (unknown), not TRUE or FALSE.  Logical operators
  follow a three-valued truth table:

      AND truth table (T=true, F=false, N=nil):
         T AND T = T
         T AND F = F
         T AND N = N
         F AND T = F
         F AND F = F
         F AND N = F   ← F dominates
         N AND T = N
         N AND F = F   ← F dominates
         N AND N = N

      OR truth table:
         T OR T = T
         T OR F = T
         T OR N = T   ← T dominates
         F OR T = T
         F OR F = F
         F OR N = N
         N OR T = T   ← T dominates
         N OR F = N
         N OR N = N

      NOT truth table:
         NOT T = F
         NOT F = T
         NOT N = N   ← unknown stays unknown

  Rows are included in results only when the WHERE / HAVING predicate
  evaluates to exactly `true` (not nil, not false).

  ## LIKE pattern matching

  SQL LIKE uses `%` (any sequence) and `_` (any single character).  We
  convert the LIKE pattern to an Elixir regex:
    - `%`  → `.*`
    - `_`  → `.`
    - Everything else is regex-escaped.

  Matching is case-sensitive unless the DataSource normalises strings.

  ## Aggregate functions

  Aggregate functions (COUNT, SUM, AVG, MIN, MAX) are NOT evaluated here.
  They are evaluated by the Aggregate module after grouping.  During the
  expression evaluation phase, aggregate calls should only appear in the
  SELECT list or HAVING clause, and the Aggregate module substitutes their
  already-computed values into the row context before calling eval_expr.

  Non-aggregate scalar functions (UPPER, LOWER, LENGTH, etc.) are
  evaluated here if/when they are needed; only COUNT/SUM/AVG/MIN/MAX are
  treated as aggregates.
  """

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.SqlExecutionEngine.Errors.ColumnNotFoundError

  # Aggregate function names — these should already be resolved in row_ctx
  # by the time we evaluate them during projection.
  @aggregate_fns ~w(COUNT SUM AVG MIN MAX)

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Evaluate an expression AST node against a row context.

  `node` can be an `ASTNode` (from the grammar-driven parser) or a `Token`
  (for leaf grammar productions that reduce to a single token).

  Returns the scalar value of the expression, which may be:
  - `integer()` or `float()` for numeric expressions
  - `binary()` (String) for string expressions
  - `true` | `false` for boolean expressions
  - `nil` for SQL NULL

  ## Raises
  - `ColumnNotFoundError` if a referenced column is not in `row_ctx`.
  """
  @spec eval_expr(ASTNode.t() | Token.t(), map()) :: term()
  def eval_expr(node, row_ctx)

  # A Token is a leaf — dispatch directly on its type.
  def eval_expr(%Token{} = token, row_ctx) do
    eval_token(token, row_ctx)
  end

  # An ASTNode is dispatched by its rule_name.
  def eval_expr(%ASTNode{rule_name: rule} = node, row_ctx) do
    eval_rule(rule, node, row_ctx)
  end

  # ---------------------------------------------------------------------------
  # Three-valued logic helpers (exported for testing)
  # ---------------------------------------------------------------------------

  @doc """
  SQL three-valued AND.

  Returns `false` if either operand is `false` (false dominates).
  Returns `nil` if either operand is `nil` and the other is not `false`.
  Returns `true` only when both operands are `true`.
  """
  @spec sql_and(term(), term()) :: boolean() | nil
  def sql_and(false, _), do: false
  def sql_and(_, false), do: false
  def sql_and(nil, _), do: nil
  def sql_and(_, nil), do: nil
  def sql_and(true, true), do: true

  @doc """
  SQL three-valued OR.

  Returns `true` if either operand is `true` (true dominates).
  Returns `nil` if either operand is `nil` and the other is not `true`.
  Returns `false` only when both operands are `false`.
  """
  @spec sql_or(term(), term()) :: boolean() | nil
  def sql_or(true, _), do: true
  def sql_or(_, true), do: true
  def sql_or(nil, _), do: nil
  def sql_or(_, nil), do: nil
  def sql_or(false, false), do: false

  @doc """
  SQL three-valued NOT.

  Returns `nil` for `nil` (unknown stays unknown).
  """
  @spec sql_not(term()) :: boolean() | nil
  def sql_not(nil), do: nil
  def sql_not(true), do: false
  def sql_not(false), do: true

  # ---------------------------------------------------------------------------
  # Rule dispatch
  # ---------------------------------------------------------------------------
  #
  # Each grammar rule is handled by a clause of eval_rule/3.  The pattern is:
  #   eval_rule("rule_name", node, row_ctx) -> scalar_value
  #
  # Some rules are "transparent" — they just delegate to the single child.
  # Others perform computation (arithmetic, comparisons, etc.).

  # "expr" is transparent — it always wraps exactly one "or_expr" child.
  defp eval_rule("expr", %ASTNode{children: [child]}, row_ctx) do
    eval_expr(child, row_ctx)
  end

  # ---------------------------------------------------------------------------
  # Logical operators: or_expr / and_expr / not_expr
  # ---------------------------------------------------------------------------
  #
  # Grammar:
  #   or_expr  = and_expr { "OR" and_expr }
  #   and_expr = not_expr { "AND" not_expr }
  #   not_expr = "NOT" not_expr | comparison
  #
  # The parser produces all siblings as a flat list of children:
  #   or_expr children: [and_expr, Token("OR"), and_expr, Token("OR"), and_expr]
  #
  # We fold left through the list, accumulating the result.

  defp eval_rule("or_expr", %ASTNode{children: children}, row_ctx) do
    eval_binary_op_chain(children, "OR", row_ctx, &sql_or/2)
  end

  defp eval_rule("and_expr", %ASTNode{children: children}, row_ctx) do
    eval_binary_op_chain(children, "AND", row_ctx, &sql_and/2)
  end

  defp eval_rule("not_expr", %ASTNode{children: children}, row_ctx) do
    case children do
      # "NOT" not_expr — the first child is the NOT keyword token, second is the nested expr
      [%Token{type: "KEYWORD", value: "NOT"}, nested] ->
        sql_not(eval_expr(nested, row_ctx))

      # single child — just a comparison (no NOT)
      [comparison] ->
        eval_expr(comparison, row_ctx)
    end
  end

  # ---------------------------------------------------------------------------
  # Comparison expressions
  # ---------------------------------------------------------------------------
  #
  # Grammar:
  #   comparison = additive [ cmp_op additive
  #              | "BETWEEN" additive "AND" additive
  #              | "NOT" "BETWEEN" additive "AND" additive
  #              | "IN" "(" value_list ")"
  #              | "NOT" "IN" "(" value_list ")"
  #              | "LIKE" additive
  #              | "NOT" "LIKE" additive
  #              | "IS" "NULL"
  #              | "IS" "NOT" "NULL" ]
  #
  # The children list encodes what modifier is present.  We inspect the
  # token values at positions 1 and beyond to determine which form we have.

  defp eval_rule("comparison", %ASTNode{children: children}, row_ctx) do
    case children do
      # Simple additive — no comparison operator
      [single] ->
        eval_expr(single, row_ctx)

      # IS NULL — children: [additive, Token("IS"), Token("NULL")]
      [lhs_node, %Token{value: "IS"}, %Token{value: "NULL"}] ->
        lhs = eval_expr(lhs_node, row_ctx)
        lhs == nil

      # IS NOT NULL — children: [additive, Token("IS"), Token("NOT"), Token("NULL")]
      [lhs_node, %Token{value: "IS"}, %Token{value: "NOT"}, %Token{value: "NULL"}] ->
        lhs = eval_expr(lhs_node, row_ctx)
        lhs != nil

      # BETWEEN — children: [lhs, Token("BETWEEN"), low, Token("AND"), high]
      [lhs_node, %Token{value: "BETWEEN"}, low_node, %Token{value: "AND"}, high_node] ->
        lhs = eval_expr(lhs_node, row_ctx)
        low = eval_expr(low_node, row_ctx)
        high = eval_expr(high_node, row_ctx)
        eval_between(lhs, low, high)

      # NOT BETWEEN — children: [lhs, Token("NOT"), Token("BETWEEN"), low, Token("AND"), high]
      [
        lhs_node,
        %Token{value: "NOT"},
        %Token{value: "BETWEEN"},
        low_node,
        %Token{value: "AND"},
        high_node
      ] ->
        lhs = eval_expr(lhs_node, row_ctx)
        low = eval_expr(low_node, row_ctx)
        high = eval_expr(high_node, row_ctx)
        sql_not(eval_between(lhs, low, high))

      # IN (...) — children: [lhs, Token("IN"), Token("("), value_list_node, Token(")")]
      [
        lhs_node,
        %Token{value: "IN"},
        %Token{type: "LPAREN"},
        value_list_node,
        %Token{type: "RPAREN"}
      ] ->
        lhs = eval_expr(lhs_node, row_ctx)
        values = eval_value_list(value_list_node, row_ctx)
        eval_in(lhs, values)

      # NOT IN (...) — children: [lhs, Token("NOT"), Token("IN"), Token("("), value_list_node, Token(")")]
      [
        lhs_node,
        %Token{value: "NOT"},
        %Token{value: "IN"},
        %Token{type: "LPAREN"},
        value_list_node,
        %Token{type: "RPAREN"}
      ] ->
        lhs = eval_expr(lhs_node, row_ctx)
        values = eval_value_list(value_list_node, row_ctx)
        sql_not(eval_in(lhs, values))

      # LIKE pattern — children: [lhs, Token("LIKE"), rhs]
      [lhs_node, %Token{value: "LIKE"}, pattern_node] ->
        lhs = eval_expr(lhs_node, row_ctx)
        pattern = eval_expr(pattern_node, row_ctx)
        eval_like(lhs, pattern)

      # NOT LIKE — children: [lhs, Token("NOT"), Token("LIKE"), rhs]
      [lhs_node, %Token{value: "NOT"}, %Token{value: "LIKE"}, pattern_node] ->
        lhs = eval_expr(lhs_node, row_ctx)
        pattern = eval_expr(pattern_node, row_ctx)
        sql_not(eval_like(lhs, pattern))

      # Standard comparison: lhs cmp_op rhs
      # cmp_op is an ASTNode with rule_name "cmp_op"
      [lhs_node, %ASTNode{rule_name: "cmp_op", children: [op_token]}, rhs_node] ->
        lhs = eval_expr(lhs_node, row_ctx)
        rhs = eval_expr(rhs_node, row_ctx)
        eval_cmp(op_token.value, lhs, rhs)
    end
  end

  # ---------------------------------------------------------------------------
  # Arithmetic operators: additive / multiplicative / unary
  # ---------------------------------------------------------------------------
  #
  # Grammar:
  #   additive       = multiplicative { ( "+" | "-" ) multiplicative }
  #   multiplicative = unary { ( STAR | "/" | "%" ) unary }
  #   unary          = "-" unary | primary
  #
  # These follow the same flat-children pattern as the logical operators.
  # The token between operands is the operator.

  defp eval_rule("additive", %ASTNode{children: children}, row_ctx) do
    eval_arithmetic_chain(children, row_ctx)
  end

  defp eval_rule("multiplicative", %ASTNode{children: children}, row_ctx) do
    eval_arithmetic_chain(children, row_ctx)
  end

  defp eval_rule("unary", %ASTNode{children: children}, row_ctx) do
    case children do
      # Unary minus: "-" unary
      [%Token{value: "-"}, operand] ->
        val = eval_expr(operand, row_ctx)
        if val == nil, do: nil, else: -val

      # Pass-through to primary
      [single] ->
        eval_expr(single, row_ctx)
    end
  end

  # ---------------------------------------------------------------------------
  # Primary expressions
  # ---------------------------------------------------------------------------
  #
  # Grammar:
  #   primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
  #           | function_call | column_ref | "(" expr ")"

  defp eval_rule("primary", %ASTNode{children: children}, row_ctx) do
    case children do
      # Parenthesised expression: "(" expr ")"
      [%Token{type: "LPAREN"}, inner, %Token{type: "RPAREN"}] ->
        eval_expr(inner, row_ctx)

      # Single child — either a token literal, column_ref, or function_call
      [single] ->
        eval_expr(single, row_ctx)
    end
  end

  # ---------------------------------------------------------------------------
  # Column reference: column_ref = NAME [ "." NAME ]
  # ---------------------------------------------------------------------------
  #
  # Qualified:   employees.name → look up "employees.name"
  # Unqualified: name           → look up "name"
  #
  # We try the exact key first, then scan all keys that end with ".key" to
  # support unqualified references when only one table is in scope.

  defp eval_rule("column_ref", %ASTNode{children: children}, row_ctx) do
    case children do
      # Qualified: table.column
      [%Token{value: table}, %Token{type: "DOT"}, %Token{value: col}] ->
        key = "#{table}.#{col}"
        lookup_column(key, row_ctx)

      # Unqualified: column
      [%Token{value: col}] ->
        # Try bare name first (covers both bare and aliased single-table queries)
        if Map.has_key?(row_ctx, col) do
          Map.get(row_ctx, col)
        else
          # Try to find a qualified version like "sometable.col"
          qualified_key =
            Enum.find(Map.keys(row_ctx), fn k ->
              String.ends_with?(k, ".#{col}")
            end)

          if qualified_key do
            Map.get(row_ctx, qualified_key)
          else
            raise ColumnNotFoundError, col
          end
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Function calls: function_call = NAME "(" ( STAR | [ value_list ] ) ")"
  # ---------------------------------------------------------------------------
  #
  # Aggregate functions (COUNT, SUM, AVG, MIN, MAX) are pre-computed by the
  # Aggregate module and injected into the row_ctx under a special key like
  # "__agg:COUNT(*)" before this evaluator is called.
  #
  # If we encounter an aggregate in the row_ctx, we return its pre-computed
  # value.  Otherwise we fall through and raise an unsupported error.

  defp eval_rule("function_call", %ASTNode{children: children}, row_ctx) do
    # Children: [NAME_token, LPAREN_token, arg..., RPAREN_token]
    [%Token{value: fn_name} | rest] = children

    uname = String.upcase(fn_name)

    if uname in @aggregate_fns do
      # Build the canonical aggregate key and look it up in row_ctx.
      # The key format is "__agg:FNAME(arg)" — see Aggregate module.
      agg_key = build_agg_key(uname, rest)

      case Map.fetch(row_ctx, agg_key) do
        {:ok, value} ->
          value

        :error ->
          # Aggregate not yet computed (this eval is used for the raw row,
          # not the aggregate result). Return nil as a sentinel — the Aggregate
          # module handles this case during group processing.
          nil
      end
    else
      # Non-aggregate scalar functions — extend here as needed.
      eval_scalar_fn(uname, rest, row_ctx)
    end
  end

  # ---------------------------------------------------------------------------
  # Transparent rules
  # ---------------------------------------------------------------------------
  #
  # Some rules are just wrappers with a single child.  We evaluate the child.

  defp eval_rule(rule, %ASTNode{children: [single]}, row_ctx)
       when rule in ["select_item"] do
    eval_expr(single, row_ctx)
  end

  # ---------------------------------------------------------------------------
  # Token evaluation (leaf nodes)
  # ---------------------------------------------------------------------------

  defp eval_token(%Token{type: "NUMBER", value: v}, _row_ctx) do
    # Parse as integer first; if it contains a decimal point, parse as float.
    if String.contains?(v, ".") do
      String.to_float(v)
    else
      String.to_integer(v)
    end
  end

  defp eval_token(%Token{type: "STRING", value: v}, _row_ctx) do
    # The lexer already strips the surrounding quotes.
    v
  end

  defp eval_token(%Token{type: "KEYWORD", value: "NULL"}, _row_ctx), do: nil
  defp eval_token(%Token{type: "KEYWORD", value: "TRUE"}, _row_ctx), do: true
  defp eval_token(%Token{type: "KEYWORD", value: "FALSE"}, _row_ctx), do: false

  defp eval_token(%Token{type: "STAR"}, _row_ctx) do
    # STAR in a primary context means column "*" — unusual but parseable.
    # This case shouldn't normally occur in our evaluator; aggregates handle STAR.
    raise "STAR token in expression context — likely an aggregate not pre-resolved"
  end

  defp eval_token(%Token{type: type, value: v}, _row_ctx) do
    raise "Unhandled token in expression: type=#{type}, value=#{v}"
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Evaluate a chain of binary operations where operators and operands are
  # interleaved in the children list:
  #   [op1, Token(op), op2, Token(op), op3]
  # We fold left: ((op1 OP op2) OP op3)
  defp eval_binary_op_chain([first | rest], op_value, row_ctx, combiner) do
    first_val = eval_expr(first, row_ctx)

    Enum.chunk_every(rest, 2)
    |> Enum.reduce(first_val, fn [%Token{value: ^op_value}, rhs_node], acc ->
      combiner.(acc, eval_expr(rhs_node, row_ctx))
    end)
  end

  # Evaluate arithmetic chains: [operand, Token(op), operand, ...]
  defp eval_arithmetic_chain([first | rest], row_ctx) do
    first_val = eval_expr(first, row_ctx)

    Enum.chunk_every(rest, 2)
    |> Enum.reduce(first_val, fn [op_token, rhs_node], acc ->
      rhs = eval_expr(rhs_node, row_ctx)
      apply_arith(op_token.value, acc, rhs)
    end)
  end

  # Apply a standard comparison operator.
  # NULL-safe: any comparison involving NULL yields NULL.
  defp eval_cmp(_op, nil, _), do: nil
  defp eval_cmp(_op, _, nil), do: nil
  defp eval_cmp("=", a, b), do: a == b
  defp eval_cmp("!=", a, b), do: a != b
  defp eval_cmp("<>", a, b), do: a != b
  defp eval_cmp("<", a, b), do: a < b
  defp eval_cmp(">", a, b), do: a > b
  defp eval_cmp("<=", a, b), do: a <= b
  defp eval_cmp(">=", a, b), do: a >= b

  # BETWEEN low AND high — NULL-safe
  defp eval_between(nil, _, _), do: nil
  defp eval_between(_, nil, _), do: nil
  defp eval_between(_, _, nil), do: nil
  defp eval_between(val, low, high), do: val >= low && val <= high

  # IN (v1, v2, ...) — NULL-safe
  # SQL: NULL IN (1, 2) → NULL
  # SQL: 1 IN (1, NULL) → TRUE  (1 is found before NULL is reached)
  # SQL: 2 IN (1, NULL) → NULL  (not found, but NULL was in the list)
  defp eval_in(nil, _), do: nil

  defp eval_in(val, values) do
    cond do
      val in values -> true
      nil in values -> nil
      true -> false
    end
  end

  # LIKE pattern matching — convert SQL pattern to regex
  defp eval_like(nil, _), do: nil
  defp eval_like(_, nil), do: nil

  defp eval_like(val, pattern) when is_binary(val) and is_binary(pattern) do
    regex_str =
      pattern
      |> String.graphemes()
      |> Enum.map(fn
        "%" -> ".*"
        "_" -> "."
        c -> Regex.escape(c)
      end)
      |> Enum.join()

    Regex.match?(~r/^#{regex_str}$/, val)
  end

  defp eval_like(_, _), do: nil

  # Arithmetic operators
  defp apply_arith(_, nil, _), do: nil
  defp apply_arith(_, _, nil), do: nil
  defp apply_arith("+", a, b), do: a + b
  defp apply_arith("-", a, b), do: a - b
  defp apply_arith("*", a, b), do: a * b
  defp apply_arith("/", a, b) when b != 0, do: a / b
  defp apply_arith("/", _, 0), do: nil
  defp apply_arith("%", a, b) when b != 0, do: rem(a, b)
  defp apply_arith("%", _, 0), do: nil

  # Lookup a column key in the row context, raising if not found.
  defp lookup_column(key, row_ctx) do
    case Map.fetch(row_ctx, key) do
      {:ok, val} -> val
      :error -> raise ColumnNotFoundError, key
    end
  end

  # Evaluate a list of expressions from a value_list-like node.
  defp eval_value_list(%ASTNode{children: children}, row_ctx) do
    # children: [expr, Token(","), expr, Token(","), expr, ...]
    children
    |> Enum.reject(fn
      %Token{type: "COMMA"} -> true
      _ -> false
    end)
    |> Enum.flat_map(fn
      %ASTNode{rule_name: "value_list"} = node -> eval_value_list(node, row_ctx)
      node -> [eval_expr(node, row_ctx)]
    end)
  end

  defp eval_value_list(%Token{} = token, row_ctx), do: [eval_expr(token, row_ctx)]

  # Build the aggregate key string that the Aggregate module stores in row_ctx.
  #
  # This key MUST match the key produced by Aggregate.collect_agg_specs/1.
  # Both sides use the same token-collection logic: walk the AST of the
  # argument expression and concatenate all token values.
  #
  # Grammar: function_call = NAME "(" ( STAR | [ value_list ] ) ")"
  #          value_list    = expr { "," expr }
  #
  # The children list (passed as `rest`) after stripping NAME is:
  #   [LPAREN, STAR, RPAREN]           — for COUNT(*)
  #   [LPAREN, value_list_node, RPAREN] — for SUM(salary), AVG(col), etc.
  #   [LPAREN, RPAREN]                  — for COUNT() (unusual but possible)
  defp build_agg_key(fn_name, [%Token{type: "LPAREN"} | rest]) do
    rparen_idx =
      Enum.find_index(rest, fn
        %Token{type: "RPAREN"} -> true
        _ -> false
      end)

    args = Enum.take(rest, rparen_idx)

    arg_str =
      case args do
        [] ->
          ""

        [%Token{type: "STAR"}] ->
          "*"

        [%Token{value: v}] ->
          v

        [%ASTNode{} = node] ->
          # Single AST node argument (e.g., value_list wrapping an expression).
          # Collect all leaf tokens and join their values.
          collect_tokens_for_key(node)

        nodes ->
          # Multiple nodes — collect all leaf tokens.
          nodes
          |> Enum.reject(fn
            %Token{type: "COMMA"} -> true
            _ -> false
          end)
          |> Enum.map_join(", ", fn
            %Token{value: v} -> v
            %ASTNode{} = n -> collect_tokens_for_key(n)
          end)
      end

    "__agg:#{fn_name}(#{arg_str})"
  end

  # Collect all leaf token values from an AST node, concatenated.
  # This mirrors what Aggregate.node_to_key_string does.
  defp collect_tokens_for_key(%Token{value: v}), do: v

  defp collect_tokens_for_key(%ASTNode{children: children}) do
    children
    |> Enum.map_join("", &collect_tokens_for_key/1)
  end

  # Scalar (non-aggregate) function evaluation.
  # Only handles STAR/empty args case now; extend as needed.
  defp eval_scalar_fn(name, _args, _row_ctx) do
    raise "Unsupported scalar function: #{name}"
  end
end
