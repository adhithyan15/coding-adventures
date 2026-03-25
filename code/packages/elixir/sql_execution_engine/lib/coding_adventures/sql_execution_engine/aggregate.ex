defmodule CodingAdventures.SqlExecutionEngine.Aggregate do
  @moduledoc """
  Aggregate function computation for GROUP BY queries.

  ## What aggregates do

  An aggregate function collapses a *group* of rows into a single scalar
  value.  For example:

      SELECT dept_id, COUNT(*), AVG(salary)
      FROM employees
      GROUP BY dept_id

  After grouping, each group is a list of rows sharing the same `dept_id`.
  The aggregate module computes COUNT(*) and AVG(salary) over that group
  and produces one output row per group.

  ## Supported aggregate functions

  | Function | Behaviour |
  |----------|-----------|
  | COUNT(*) | Count of all rows (NULLs included) |
  | COUNT(col) | Count of non-NULL values in col |
  | SUM(col)   | Sum of non-NULL values (nil if all NULL) |
  | AVG(col)   | Arithmetic mean of non-NULL values (nil if all NULL) |
  | MIN(col)   | Minimum non-NULL value (nil if all NULL) |
  | MAX(col)   | Maximum non-NULL value (nil if all NULL) |

  ## How aggregate keys work

  To bridge between the expression evaluator and the aggregate results we
  store computed aggregates in the row context under special keys:

      "__agg:COUNT(*)"    => 4
      "__agg:SUM(salary)" => 320000
      "__agg:AVG(salary)" => 80000.0

  The key is built by `build_agg_key/2` which matches the format used in
  `Expression.build_agg_key/2`.  The expression evaluator then looks up
  these keys when it encounters aggregate function calls.

  ## GROUP BY mechanics

      rows
        ↓ group_rows/2
      %{group_key => [row, row, ...]}
        ↓ compute_aggregates/3
      [row_with_agg_values, ...]

  When there is no GROUP BY clause but the SELECT list contains aggregates,
  we treat the entire result set as a single group.

  ## NULL handling in aggregates

  Following SQL standard behaviour:
  - COUNT(*) counts all rows including those with NULL values.
  - COUNT(col) counts non-NULL values only.
  - SUM, AVG, MIN, MAX ignore NULL values.
  - If all values are NULL, SUM/AVG/MIN/MAX return NULL (nil in Elixir).
  """

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.SqlExecutionEngine.Expression

  # ---------------------------------------------------------------------------
  # Detecting aggregates in an expression tree
  # ---------------------------------------------------------------------------

  @doc """
  Return true if the expression tree contains any aggregate function call.

  We walk the AST and check if any `function_call` node uses an aggregate
  function name (COUNT, SUM, AVG, MIN, MAX).
  """
  @spec has_aggregate?(ASTNode.t() | Token.t()) :: boolean()
  def has_aggregate?(%Token{}), do: false
  def has_aggregate?({:star, nil}), do: false

  def has_aggregate?(%ASTNode{rule_name: "function_call", children: children}) do
    case children do
      [%Token{value: name} | _] -> String.upcase(name) in ~w(COUNT SUM AVG MIN MAX)
      _ -> false
    end
  end

  def has_aggregate?(%ASTNode{children: children}) do
    Enum.any?(children, &has_aggregate?/1)
  end

  # ---------------------------------------------------------------------------
  # Group rows by GROUP BY key
  # ---------------------------------------------------------------------------

  @doc """
  Group a list of rows by the values of the group_by_exprs.

  Returns a list of `{group_key_list, [row]}` pairs where `group_key_list`
  is the evaluated list of GROUP BY expressions for that group.

  We preserve insertion order of groups (using a list of `{key, rows}` pairs
  rather than a plain map) to match typical SQL behaviour where the order of
  groups follows the order of first occurrence in the input.

  ## Parameters
  - `rows` — list of row context maps
  - `group_by_exprs` — list of ASTNode/Token from the group_clause
  """
  @spec group_rows([map()], [ASTNode.t() | Token.t()]) :: [{[term()], [map()]}]
  def group_rows(rows, group_by_exprs) do
    # Accumulate into a keyword-style list to preserve insertion order.
    # We use a list of {key_list, rows} rather than a map because group keys
    # may contain nil, which is a valid Elixir map key but semantically tricky.
    Enum.reduce(rows, [], fn row, acc ->
      key = Enum.map(group_by_exprs, &Expression.eval_expr(&1, row))
      update_group(acc, key, row)
    end)
    |> Enum.map(fn {k, rows_rev} -> {k, Enum.reverse(rows_rev)} end)
  end

  # Insert row into the group list, creating a new group if the key is new.
  defp update_group([], key, row), do: [{key, [row]}]

  defp update_group([{key, rows} | rest], key, row) do
    [{key, [row | rows]} | rest]
  end

  defp update_group([head | rest], key, row) do
    [head | update_group(rest, key, row)]
  end

  # ---------------------------------------------------------------------------
  # Compute aggregate values for a group of rows
  # ---------------------------------------------------------------------------

  @doc """
  Compute aggregate values for all aggregates found in `select_items` and
  `having_expr` (if any), given a group of rows.

  Returns a context map with "__agg:FNAME(arg)" keys suitable for injection
  into the expression evaluator.

  ## Parameters
  - `group_rows` — list of row context maps for this group
  - `agg_specs`  — list of `{fn_name, arg_node_or_star}` tuples describing
    which aggregates to compute; produced by `collect_agg_specs/1`
  """
  @spec compute_group(
          group_rows :: [map()],
          agg_specs :: [{String.t(), :star | ASTNode.t() | Token.t()}]
        ) :: map()
  def compute_group(group_rows, agg_specs) do
    Enum.reduce(agg_specs, %{}, fn {fn_name, arg, key}, acc ->
      value = compute_one(fn_name, arg, group_rows)
      Map.put(acc, key, value)
    end)
  end

  # ---------------------------------------------------------------------------
  # Collect aggregate specs from a list of expression trees
  # ---------------------------------------------------------------------------

  @doc """
  Walk a list of expression nodes and collect all aggregate function calls.

  Returns a list of `{fn_name, arg, key}` tuples:
  - `fn_name` — uppercase function name ("COUNT", "SUM", etc.)
  - `arg`     — `:star` or an expression node representing the argument
  - `key`     — the `"__agg:..."` string key for the row context map

  Duplicates (same key) are deduplicated.
  """
  @spec collect_agg_specs([ASTNode.t() | Token.t() | nil]) ::
          [{String.t(), :star | ASTNode.t(), String.t()}]
  def collect_agg_specs(nodes) do
    nodes
    |> Enum.flat_map(&walk_for_aggs/1)
    |> Enum.uniq_by(fn {_fn, _arg, key} -> key end)
  end

  # Recursively walk an AST node and collect aggregate specs.
  defp walk_for_aggs(nil), do: []
  defp walk_for_aggs(%Token{}), do: []
  # Handle {:star, nil} sentinel from SELECT * — not an aggregate.
  defp walk_for_aggs({:star, nil}), do: []
  # Handle :star atom (when SELECT * item is destructured: {expr, alias} = {:star, nil} → expr = :star).
  defp walk_for_aggs(:star), do: []
  # Handle any other atoms or non-AST values gracefully.
  defp walk_for_aggs(other) when not is_map(other), do: []

  defp walk_for_aggs(%ASTNode{rule_name: "function_call", children: children} = _node) do
    case children do
      [%Token{value: name}, %Token{type: "LPAREN"} | rest] ->
        uname = String.upcase(name)

        if uname in ~w(COUNT SUM AVG MIN MAX) do
          {arg, key} = extract_fn_arg(uname, rest)
          [{uname, arg, key}]
        else
          []
        end

      _ ->
        []
    end
  end

  defp walk_for_aggs(%ASTNode{children: children}) do
    Enum.flat_map(children, &walk_for_aggs/1)
  end

  # Extract the aggregate function argument from the tokens after "(".
  # Returns {arg, key} where arg is :star or an expression node.
  #
  # The grammar for function_call is:
  #   function_call = NAME "(" ( STAR | [ value_list ] ) ")"
  #   value_list    = expr { "," expr }
  #
  # For single-argument aggregates like SUM(salary), the parser produces:
  #   [NAME_token, LPAREN_token, value_list_node, RPAREN_token]
  # where value_list_node wraps the expression in expr → or_expr → … → column_ref.
  #
  # We unwrap the value_list to expose the actual expression, since compute_one
  # calls eval_expr on the arg directly.
  defp extract_fn_arg(fn_name, rest) do
    # rest = [arg_or_star..., RPAREN]
    # Drop the trailing RPAREN
    rparen_idx =
      Enum.find_index(rest, fn
        %Token{type: "RPAREN"} -> true
        _ -> false
      end) || length(rest)

    args = Enum.take(rest, rparen_idx)

    case args do
      [] ->
        {:star, "__agg:#{fn_name}()"}

      [%Token{type: "STAR"}] ->
        {:star, "__agg:#{fn_name}(*)"}

      [%ASTNode{rule_name: "value_list", children: vl_children}] ->
        # Unwrap value_list — for single-arg aggregates, take the first expr.
        # Filter out commas, take first expression.
        first_expr =
          vl_children
          |> Enum.reject(&match?(%Token{type: "COMMA"}, &1))
          |> hd()

        key_str = node_to_key_string(first_expr)
        {first_expr, "__agg:#{fn_name}(#{key_str})"}

      [single] ->
        # Single token (shouldn't usually happen for column args, but be safe).
        key_str = node_to_key_string(single)
        {single, "__agg:#{fn_name}(#{key_str})"}

      multiple ->
        # Multiple args — very unusual for standard aggregates, but handle gracefully.
        key_str =
          multiple
          |> Enum.reject(fn
            %Token{type: "COMMA"} -> true
            _ -> false
          end)
          |> Enum.map(&node_to_key_string/1)
          |> Enum.join(", ")

        first = hd(multiple)
        {first, "__agg:#{fn_name}(#{key_str})"}
    end
  end

  # Convert a node to a string representation for use in the aggregate key.
  defp node_to_key_string(%Token{value: v}), do: v

  defp node_to_key_string(%ASTNode{rule_name: "column_ref", children: children}) do
    case children do
      [%Token{value: t}, %Token{type: "DOT"}, %Token{value: c}] -> "#{t}.#{c}"
      [%Token{value: c}] -> c
    end
  end

  defp node_to_key_string(%ASTNode{} = node) do
    # Fallback: collect all token values
    collect_tokens(node) |> Enum.map(& &1.value) |> Enum.join("")
  end

  defp collect_tokens(%Token{} = t), do: [t]
  defp collect_tokens(%ASTNode{children: ch}), do: Enum.flat_map(ch, &collect_tokens/1)

  # ---------------------------------------------------------------------------
  # Individual aggregate computation
  # ---------------------------------------------------------------------------

  # Compute a single aggregate over the group.
  defp compute_one("COUNT", :star, group_rows) do
    length(group_rows)
  end

  defp compute_one("COUNT", arg, group_rows) do
    # COUNT(col) — count non-NULL values
    group_rows
    |> Enum.map(&Expression.eval_expr(arg, &1))
    |> Enum.reject(&is_nil/1)
    |> length()
  end

  defp compute_one("SUM", arg, group_rows) do
    values =
      group_rows
      |> Enum.map(&Expression.eval_expr(arg, &1))
      |> Enum.reject(&is_nil/1)

    case values do
      [] -> nil
      vs -> Enum.sum(vs)
    end
  end

  defp compute_one("AVG", arg, group_rows) do
    values =
      group_rows
      |> Enum.map(&Expression.eval_expr(arg, &1))
      |> Enum.reject(&is_nil/1)

    case values do
      [] -> nil
      vs -> Enum.sum(vs) / length(vs)
    end
  end

  defp compute_one("MIN", arg, group_rows) do
    values =
      group_rows
      |> Enum.map(&Expression.eval_expr(arg, &1))
      |> Enum.reject(&is_nil/1)

    case values do
      [] -> nil
      vs -> Enum.min(vs)
    end
  end

  defp compute_one("MAX", arg, group_rows) do
    values =
      group_rows
      |> Enum.map(&Expression.eval_expr(arg, &1))
      |> Enum.reject(&is_nil/1)

    case values do
      [] -> nil
      vs -> Enum.max(vs)
    end
  end
end
