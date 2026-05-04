defmodule CodingAdventures.SqlExecutionEngine.Executor do
  @moduledoc """
  The main execution engine: walks the SELECT statement AST and produces a
  `QueryResult`.

  ## Pipeline overview

  A SELECT statement is executed in this order — the same logical order that
  all SQL databases follow (the "logical query processing order"), regardless
  of how physical optimisers may reorder operations internally:

  ```
  ┌──────────────────────────────────────────────────────────────────────┐
  │  SELECT pipeline — logical execution order                           │
  │                                                                      │
  │  1. FROM        — identify the base table, scan it, apply alias      │
  │  2. JOIN        — for each join_clause, extend rows left-to-right    │
  │  3. WHERE       — filter rows (no aggregates allowed here)           │
  │  4. GROUP BY    — group rows by key; compute aggregate values        │
  │  5. HAVING      — filter groups (aggregates allowed here)            │
  │  6. SELECT      — project columns, compute expressions, add aliases  │
  │  7. DISTINCT    — remove duplicate output rows                       │
  │  8. ORDER BY    — sort the output                                    │
  │  9. LIMIT/OFFSET— slice the output                                  │
  └──────────────────────────────────────────────────────────────────────┘
  ```

  ## Row representation

  Internally, rows are represented as flat maps with string keys:

      %{
        "employees.id"     => 1,
        "id"               => 1,
        "employees.name"   => "Alice",
        "name"             => "Alice",
        "employees.salary" => 90000,
        "salary"           => 90000,
        …
      }

  Both the qualified form (`table.col`) and bare form (`col`) are stored
  so that expressions can use either.

  ## Output representation

  After projection (step 6), rows become plain lists aligned with `columns`:

      %QueryResult{
        columns: ["name", "salary"],
        rows: [["Alice", 90000], ["Carol", 95000]]
      }
  """

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.SqlExecutionEngine.{Expression, Aggregate, Join, Result}
  alias CodingAdventures.SqlExecutionEngine.Errors.UnsupportedQueryError

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Execute a parsed SQL AST against `data_source`.

  `ast` is the root `%ASTNode{rule_name: "program"}` returned by
  `CodingAdventures.SqlParser.parse_sql/1`.

  Returns `{:ok, QueryResult.t()}` on success, or `{:error, message}` if the
  statement type is not supported.
  """
  @spec execute(%ASTNode{}, module()) :: {:ok, Result.t()} | {:error, String.t()}
  def execute(%ASTNode{rule_name: "program", children: children}, data_source) do
    # The program node contains: statement { ";" statement } [ ";" ]
    # Filter out SEMICOLON tokens, then execute the first statement.
    statements =
      children
      |> Enum.reject(fn
        %Token{type: "SEMICOLON"} -> true
        _ -> false
      end)

    case statements do
      [] ->
        {:ok, %Result{columns: [], rows: []}}

      [stmt_node | _] ->
        execute_statement(stmt_node, data_source)
    end
  end

  @doc """
  Execute multiple SELECT statements and return a list of results.
  """
  @spec execute_all(%ASTNode{}, module()) :: {:ok, [Result.t()]} | {:error, String.t()}
  def execute_all(%ASTNode{rule_name: "program", children: children}, data_source) do
    statements =
      children
      |> Enum.reject(fn
        %Token{type: "SEMICOLON"} -> true
        _ -> false
      end)

    results =
      Enum.map(statements, fn stmt_node ->
        case execute_statement(stmt_node, data_source) do
          {:ok, result} -> result
          {:error, msg} -> raise msg
        end
      end)

    {:ok, results}
  end

  # ---------------------------------------------------------------------------
  # Statement dispatch
  # ---------------------------------------------------------------------------

  defp execute_statement(%ASTNode{rule_name: "statement", children: [inner]}, data_source) do
    execute_statement(inner, data_source)
  end

  defp execute_statement(%ASTNode{rule_name: "query_stmt", children: [inner]}, data_source) do
    execute_statement(inner, data_source)
  end

  defp execute_statement(%ASTNode{rule_name: "select_stmt"} = node, data_source) do
    {:ok, execute_select(node, data_source)}
  end

  defp execute_statement(%ASTNode{rule_name: unsupported}, _data_source) do
    raise UnsupportedQueryError, unsupported
  end

  # ---------------------------------------------------------------------------
  # SELECT execution pipeline
  # ---------------------------------------------------------------------------

  defp execute_select(%ASTNode{rule_name: "select_stmt", children: children}, data_source) do
    # ── 0. Parse the select_stmt children ───────────────────────────────────
    #
    # The grammar rule is:
    #   select_stmt = "SELECT" [ "DISTINCT" | "ALL" ] select_list
    #                 "FROM" table_ref { join_clause }
    #                 [ where_clause ] [ group_clause ] [ having_clause ]
    #                 [ order_clause ] [ limit_clause ]

    {distinct, rest1} = extract_distinct(children)
    {select_list_node, rest2} = find_first_node("select_list", rest1)
    {_from_kw, rest3} = find_token_by_value("FROM", rest2)
    {table_ref_node, rest4} = find_first_node("table_ref", rest3)
    {join_nodes, rest5} = take_all_nodes("join_clause", rest4)
    {where_node, rest6} = find_optional_node("where_clause", rest5)
    {group_node, rest7} = find_optional_node("group_clause", rest6)
    {having_node, rest8} = find_optional_node("having_clause", rest7)
    {order_node, rest9} = find_optional_node("order_clause", rest8)
    {limit_node, _} = find_optional_node("limit_clause", rest9)

    # ── 1. FROM ──────────────────────────────────────────────────────────────

    {base_table_name, base_alias} = extract_table_ref(table_ref_node)
    base_schema = data_source.schema(base_table_name)
    base_rows = data_source.scan(base_table_name)
    rows = Enum.map(base_rows, &row_to_ctx(&1, base_alias, base_schema))

    # ── 2. JOINs ─────────────────────────────────────────────────────────────

    rows =
      Enum.reduce(join_nodes, rows, fn join_node, current_rows ->
        apply_join_clause(join_node, current_rows, data_source)
      end)

    # ── 3. WHERE ─────────────────────────────────────────────────────────────

    rows =
      case where_node do
        nil ->
          rows

        %ASTNode{rule_name: "where_clause", children: where_children} ->
          # Grammar: where_clause = "WHERE" expr
          # Children: [Token("WHERE"), expr_node]
          expr_node = Enum.find(where_children, &match?(%ASTNode{}, &1))

          Enum.filter(rows, fn row ->
            Expression.eval_expr(expr_node, row) == true
          end)
      end

    # ── 4. GROUP BY + aggregates ─────────────────────────────────────────────

    select_item_nodes = extract_select_items(select_list_node)
    having_expr = extract_having_expr(having_node)

    all_expr_nodes =
      Enum.map(select_item_nodes, fn {expr, _alias} -> expr end) ++
        List.wrap(having_expr)

    agg_specs = Aggregate.collect_agg_specs(all_expr_nodes)
    has_aggs = agg_specs != []

    rows =
      cond do
        group_node != nil ->
          group_by_exprs = extract_group_by_exprs(group_node)
          groups = Aggregate.group_rows(rows, group_by_exprs)

          Enum.map(groups, fn {_key, group_rows} ->
            base = hd(group_rows)
            agg_values = Aggregate.compute_group(group_rows, agg_specs)
            Map.merge(base, agg_values)
          end)

        has_aggs ->
          agg_values = Aggregate.compute_group(rows, agg_specs)
          rep = if rows == [], do: %{}, else: hd(rows)
          [Map.merge(rep, agg_values)]

        true ->
          rows
      end

    # ── 5. HAVING ────────────────────────────────────────────────────────────

    rows =
      case having_expr do
        nil -> rows
        expr -> Enum.filter(rows, fn row -> Expression.eval_expr(expr, row) == true end)
      end

    # ── 6. SELECT — project columns ──────────────────────────────────────────

    {columns, projected_rows} = project(select_item_nodes, rows)

    # ── 7. DISTINCT ───────────────────────────────────────────────────────────

    projected_rows =
      if distinct do
        Enum.uniq(projected_rows)
      else
        projected_rows
      end

    # ── 8. ORDER BY ──────────────────────────────────────────────────────────

    projected_rows =
      case order_node do
        nil ->
          projected_rows

        order ->
          order_items = extract_order_items_indexed(order, columns)
          sort_rows(projected_rows, order_items)
      end

    # ── 9. LIMIT / OFFSET ────────────────────────────────────────────────────

    projected_rows =
      case limit_node do
        nil ->
          projected_rows

        limit ->
          {limit_n, offset_n} = extract_limit(limit)

          projected_rows
          |> Enum.drop(offset_n)
          |> Enum.take(limit_n)
      end

    %Result{columns: columns, rows: projected_rows}
  end

  # ---------------------------------------------------------------------------
  # Step 1: FROM — build row context maps
  # ---------------------------------------------------------------------------
  #
  # Convert a raw data source row into a context map with both qualified and
  # bare keys.
  #
  # Example:
  #   raw: %{"id" => 1, "name" => "Alice"}
  #   alias: "e"
  #   schema: ["id", "name"]
  #   result: %{"e.id" => 1, "id" => 1, "e.name" => "Alice", "name" => "Alice"}

  defp row_to_ctx(raw_row, table_alias, schema) do
    Enum.reduce(schema, %{}, fn col, acc ->
      val = Map.get(raw_row, col)

      acc
      |> Map.put("#{table_alias}.#{col}", val)
      |> Map.put(col, val)
    end)
  end

  # ---------------------------------------------------------------------------
  # Step 2: JOINs
  # ---------------------------------------------------------------------------

  defp apply_join_clause(
         %ASTNode{rule_name: "join_clause", children: join_children},
         left_rows,
         data_source
       ) do
    # Grammar: join_clause = join_type "JOIN" table_ref "ON" expr
    # Children: [join_type_node, Token("JOIN"), table_ref_node, Token("ON"), expr_node]
    [join_type_node | rest] = join_children
    join_type = extract_join_type(join_type_node)

    # Skip the JOIN keyword token
    {_join_kw, rest2} = find_token_by_value("JOIN", rest)
    {table_ref_node, rest3} = find_first_node("table_ref", rest2)

    # Skip the ON keyword token; the remaining node is the expr
    on_expr = Enum.find(rest3, &match?(%ASTNode{}, &1))

    {right_table_name, right_alias} = extract_table_ref(table_ref_node)
    right_schema = data_source.schema(right_table_name)
    right_raw_rows = data_source.scan(right_table_name)
    right_rows = Enum.map(right_raw_rows, &row_to_ctx(&1, right_alias, right_schema))

    Join.apply_join(left_rows, right_rows, right_schema, right_alias, join_type, on_expr)
  end

  # Extract the join type keyword string from the join_type AST node.
  # Grammar: join_type = "CROSS" | "INNER" | ( "LEFT" [ "OUTER" ] )
  #                    | ( "RIGHT" [ "OUTER" ] ) | ( "FULL" [ "OUTER" ] )
  defp extract_join_type(%ASTNode{rule_name: "join_type", children: children}) do
    first_kw =
      Enum.find(children, fn
        %Token{type: "KEYWORD"} -> true
        _ -> false
      end)

    String.upcase(first_kw.value)
  end

  # ---------------------------------------------------------------------------
  # Step 6: Project columns
  # ---------------------------------------------------------------------------

  # SELECT * — expand all qualified columns from the first row.
  defp project([{:star, nil}], rows) do
    case rows do
      [] ->
        {[], []}

      [first_row | _] ->
        # Use qualified keys (containing ".") to determine column set and order.
        # Qualified keys avoid ambiguity across joined tables.
        all_qualified_keys =
          first_row
          |> Map.keys()
          |> Enum.filter(&String.contains?(&1, "."))
          |> Enum.sort()

        # Build display column names: use bare name if unique, else qualified.
        columns =
          Enum.map(all_qualified_keys, fn key ->
            [_table, col] = String.split(key, ".", parts: 2)

            # How many qualified keys end with ".col"? If more than one, use qualified.
            matches =
              Enum.count(all_qualified_keys, fn k ->
                String.ends_with?(k, ".#{col}")
              end)

            if matches > 1, do: key, else: col
          end)

        value_rows =
          Enum.map(rows, fn row ->
            Enum.map(all_qualified_keys, fn k -> Map.get(row, k) end)
          end)

        {columns, value_rows}
    end
  end

  # Standard projection — evaluate each select item expression.
  defp project(select_item_nodes, rows) do
    columns =
      Enum.map(select_item_nodes, fn {expr, alias_str} ->
        alias_str || default_column_name(expr)
      end)

    value_rows =
      Enum.map(rows, fn row ->
        Enum.map(select_item_nodes, fn {expr, _alias} ->
          Expression.eval_expr(expr, row)
        end)
      end)

    {columns, value_rows}
  end

  # Determine the default display name for an expression (used when no AS alias).
  #
  # The parser wraps column refs and function calls in many transparent layers:
  #   select_item > expr > or_expr > and_expr > not_expr > comparison >
  #   additive > multiplicative > unary > primary > column_ref / function_call
  #
  # We unwrap single-child transparent nodes until we reach something meaningful.
  defp default_column_name(%ASTNode{rule_name: "column_ref", children: children}) do
    case children do
      [%Token{value: t}, %Token{type: "DOT"}, %Token{value: c}] -> "#{t}.#{c}"
      [%Token{value: c}] -> c
    end
  end

  defp default_column_name(%ASTNode{rule_name: "function_call", children: children}) do
    # Reconstruct the text, e.g. "COUNT(*)" or "SUM(salary)"
    tokens = collect_tokens_flat(children)
    Enum.map_join(tokens, "", & &1.value)
  end

  defp default_column_name(%Token{value: v}), do: v

  # Transparent wrapper rules — drill down through single-child nodes.
  @transparent_rules ~w(expr or_expr and_expr not_expr comparison additive multiplicative unary primary select_item)

  defp default_column_name(%ASTNode{rule_name: rule, children: [single]})
       when rule in @transparent_rules do
    default_column_name(single)
  end

  defp default_column_name(_), do: "expr"

  # Flatten all tokens in an AST subtree for display purposes.
  defp collect_tokens_flat([]), do: []
  defp collect_tokens_flat([%Token{} = t | rest]), do: [t | collect_tokens_flat(rest)]

  defp collect_tokens_flat([%ASTNode{children: ch} | rest]) do
    collect_tokens_flat(ch) ++ collect_tokens_flat(rest)
  end

  # ---------------------------------------------------------------------------
  # Step 8: ORDER BY
  # ---------------------------------------------------------------------------

  # Resolve order expressions to column indices and sort.
  defp extract_order_items_indexed(order_node, columns) do
    # Grammar: order_clause = "ORDER" "BY" order_item { "," order_item }
    # Grammar: order_item   = expr [ "ASC" | "DESC" ]
    order_item_nodes =
      order_node.children
      |> Enum.reject(fn
        %Token{value: "ORDER"} -> true
        %Token{value: "BY"} -> true
        %Token{type: "COMMA"} -> true
        _ -> false
      end)

    Enum.map(order_item_nodes, fn item_node ->
      {expr, dir} = parse_order_item(item_node)
      name = default_column_name(expr)

      idx =
        Enum.find_index(columns, fn col ->
          col == name or
            String.ends_with?(col, ".#{name}") or
            String.ends_with?(name, ".#{col}")
        end) || 0

      {idx, dir}
    end)
  end

  # Grammar: order_item = expr [ "ASC" | "DESC" ]
  defp parse_order_item(%ASTNode{rule_name: "order_item", children: children}) do
    case children do
      [expr, %Token{value: dir}] when dir in ["ASC", "DESC"] ->
        direction = if dir == "DESC", do: :desc, else: :asc
        {expr, direction}

      [expr] ->
        {expr, :asc}
    end
  end

  defp sort_rows(rows, indexed_items) do
    Enum.sort(rows, fn row_a, row_b ->
      compare_by_index(row_a, row_b, indexed_items)
    end)
  end

  defp compare_by_index(row_a, row_b, indexed_items) do
    Enum.reduce_while(indexed_items, :eq, fn {idx, direction}, _acc ->
      val_a = Enum.at(row_a, idx)
      val_b = Enum.at(row_b, idx)

      cmp = compare_values(val_a, val_b)

      result =
        case direction do
          :asc -> cmp
          :desc -> negate_cmp(cmp)
        end

      case result do
        :lt -> {:halt, true}
        :gt -> {:halt, false}
        :eq -> {:cont, :eq}
      end
    end)
    |> case do
      :eq -> false
      result -> result
    end
  end

  defp compare_values(nil, nil), do: :eq
  # NULLs sort first (NULLS FIRST)
  defp compare_values(nil, _), do: :lt
  defp compare_values(_, nil), do: :gt
  defp compare_values(a, b) when a < b, do: :lt
  defp compare_values(a, b) when a > b, do: :gt
  defp compare_values(_, _), do: :eq

  defp negate_cmp(:lt), do: :gt
  defp negate_cmp(:gt), do: :lt
  defp negate_cmp(:eq), do: :eq

  # ---------------------------------------------------------------------------
  # AST extraction helpers
  # ---------------------------------------------------------------------------

  # Extract the table name string and alias string from a table_ref node.
  # Grammar: table_ref = table_name [ "AS" NAME ]
  # Grammar: table_name = NAME [ "." NAME ]
  defp extract_table_ref(%ASTNode{rule_name: "table_ref", children: children}) do
    {table_name_node, rest} = find_first_node("table_name", children)
    table_name = extract_table_name(table_name_node)

    alias_str =
      case rest do
        [%Token{value: "AS"}, %Token{value: a} | _] -> a
        _ -> table_name
      end

    {table_name, alias_str}
  end

  defp extract_table_name(%ASTNode{rule_name: "table_name", children: children}) do
    case children do
      [%Token{value: schema}, %Token{type: "DOT"}, %Token{value: name}] ->
        "#{schema}.#{name}"

      [%Token{value: name}] ->
        name
    end
  end

  # Determine if DISTINCT modifier is present; return rest of children.
  defp extract_distinct([%Token{value: "SELECT"} | rest]) do
    case rest do
      [%Token{value: "DISTINCT"} | rest2] -> {true, rest2}
      [%Token{value: "ALL"} | rest2] -> {false, rest2}
      _ -> {false, rest}
    end
  end

  # Find the first ASTNode with matching rule_name; return {node, remaining}.
  # Uses a linear scan — the children list is small so this is fine.
  defp find_first_node(target_rule, children) do
    idx =
      Enum.find_index(children, fn
        %ASTNode{rule_name: r} -> r == target_rule
        _ -> false
      end)

    node = Enum.at(children, idx)
    rest = Enum.drop(children, idx + 1)
    {node, rest}
  end

  # Find the first Token with matching value; return {token, remaining}.
  defp find_token_by_value(target_value, children) do
    idx =
      Enum.find_index(children, fn
        %Token{value: v} -> v == target_value
        _ -> false
      end)

    token = Enum.at(children, idx)
    rest = Enum.drop(children, idx + 1)
    {token, rest}
  end

  # Find an optional node; returns {nil, children} if absent.
  defp find_optional_node(target_rule, children) do
    idx =
      Enum.find_index(children, fn
        %ASTNode{rule_name: r} -> r == target_rule
        _ -> false
      end)

    if idx do
      node = Enum.at(children, idx)
      rest = List.delete_at(children, idx)
      {node, rest}
    else
      {nil, children}
    end
  end

  # Take ALL nodes matching target_rule; return {[nodes], remaining}.
  defp take_all_nodes(target_rule, children) do
    Enum.split_with(children, fn
      %ASTNode{rule_name: r} -> r == target_rule
      _ -> false
    end)
  end

  # Extract select items from the select_list node as {expr, alias | nil} tuples.
  # Grammar: select_list = STAR | select_item { "," select_item }
  defp extract_select_items(%ASTNode{rule_name: "select_list", children: children}) do
    case children do
      [%Token{type: "STAR"}] ->
        # SELECT * — sentinel
        [{:star, nil}]

      _ ->
        children
        |> Enum.reject(fn
          %Token{type: "COMMA"} -> true
          _ -> false
        end)
        |> Enum.map(&extract_select_item/1)
    end
  end

  # Grammar: select_item = expr [ "AS" NAME ]
  defp extract_select_item(%ASTNode{rule_name: "select_item", children: children}) do
    case children do
      [expr, %Token{value: "AS"}, %Token{value: alias_name}] ->
        {expr, alias_name}

      [expr] ->
        {expr, nil}
    end
  end

  # Extract the list of expression nodes from a group_clause.
  # Grammar: group_clause = "GROUP" "BY" column_ref { "," column_ref }
  defp extract_group_by_exprs(%ASTNode{rule_name: "group_clause", children: children}) do
    children
    |> Enum.reject(fn
      %Token{value: "GROUP"} -> true
      %Token{value: "BY"} -> true
      %Token{type: "COMMA"} -> true
      _ -> false
    end)
  end

  # Extract the HAVING expression from a having_clause node.
  defp extract_having_expr(nil), do: nil

  defp extract_having_expr(%ASTNode{rule_name: "having_clause", children: children}) do
    # Grammar: having_clause = "HAVING" expr
    Enum.find(children, &match?(%ASTNode{}, &1))
  end

  # Extract LIMIT and OFFSET values.
  # Grammar: limit_clause = "LIMIT" NUMBER [ "OFFSET" NUMBER ]
  defp extract_limit(%ASTNode{rule_name: "limit_clause", children: children}) do
    numbers =
      Enum.filter(children, fn
        %Token{type: "NUMBER"} -> true
        _ -> false
      end)

    case numbers do
      [%Token{value: n}] ->
        {String.to_integer(n), 0}

      [%Token{value: n}, %Token{value: offset}] ->
        {String.to_integer(n), String.to_integer(offset)}
    end
  end
end
