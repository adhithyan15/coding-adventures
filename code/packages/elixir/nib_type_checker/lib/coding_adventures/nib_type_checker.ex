defmodule CodingAdventures.NibTypeChecker.TypedAst do
  @enforce_keys [:root, :types]
  defstruct [:root, :types]

  @type t :: %__MODULE__{
          root: term(),
          types: %{optional(term()) => atom()}
        }

  def type_of(%__MODULE__{types: types}, node), do: Map.get(types, CodingAdventures.NibTypeChecker.node_key(node))
end

defmodule CodingAdventures.NibTypeChecker.ScopeChain do
  defstruct globals: %{}, locals: []

  def new, do: %__MODULE__{}

  def define_global(%__MODULE__{} = scope, name, symbol) do
    %{scope | globals: Map.put(scope.globals, name, symbol)}
  end

  def push(%__MODULE__{} = scope), do: %{scope | locals: [%{} | scope.locals]}

  def pop(%__MODULE__{locals: [_ | rest]} = scope), do: %{scope | locals: rest}
  def pop(%__MODULE__{} = scope), do: scope

  def define_local(%__MODULE__{locals: [frame | rest]} = scope, name, symbol) do
    %{scope | locals: [Map.put(frame, name, symbol) | rest]}
  end

  def define_local(%__MODULE__{} = scope, name, symbol), do: define_global(scope, name, symbol)

  def lookup(%__MODULE__{} = scope, name) do
    Enum.find_value(scope.locals, fn frame -> Map.get(frame, name) end) || Map.get(scope.globals, name)
  end
end

defmodule CodingAdventures.NibTypeChecker do
  alias CodingAdventures.Lexer.Token
  alias CodingAdventures.NibParser
  alias CodingAdventures.NibTypeChecker.{ScopeChain, TypedAst}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.TypeCheckerProtocol

  @u4 :u4
  @u8 :u8
  @bcd :bcd
  @bool :bool
  @void :void
  @literal :literal

  @expression_rules ~w(expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr primary call_expr)

  @spec check(ASTNode.t()) :: TypeCheckerProtocol.TypeCheckResult.t()
  def check(%ASTNode{} = ast) do
    scope =
      ast
      |> child_nodes()
      |> Enum.reduce(ScopeChain.new(), fn top_decl, acc ->
        case unwrap_top_decl(top_decl) do
          %ASTNode{rule_name: "const_decl"} = decl -> collect_const_or_static(decl, acc, true)
          %ASTNode{rule_name: "static_decl"} = decl -> collect_const_or_static(decl, acc, false)
          %ASTNode{rule_name: "fn_decl"} = decl -> collect_fn_signature(decl, acc)
          _ -> acc
        end
      end)

    state =
      ast
      |> child_nodes()
      |> Enum.reduce(%{errors: [], types: %{}}, fn top_decl, acc ->
        case unwrap_top_decl(top_decl) do
          %ASTNode{rule_name: "fn_decl"} = decl -> check_fn_body(decl, scope, acc)
          _ -> acc
        end
      end)

    TypeCheckerProtocol.new_result(%TypedAst{root: ast, types: state.types}, state.errors)
  end

  def check_source(source) when is_binary(source) do
    case NibParser.parse_nib(source) do
      {:ok, ast} -> check(ast)
      {:error, reason} -> TypeCheckerProtocol.new_result(%TypedAst{root: nil, types: %{}}, [TypeCheckerProtocol.new_diagnostic(reason, 1, 1)])
    end
  end

  def node_key(%ASTNode{} = node) do
    {node.rule_name, node.start_line, node.start_column, node.end_line, node.end_column, length(node.children)}
  end

  defp collect_const_or_static(node, scope, is_const) do
    with %Token{} = name <- first_name_token(node),
         %ASTNode{} = type_node <- type_node(node),
         nib_type when not is_nil(nib_type) <- resolve_type(type_node) do
      ScopeChain.define_global(scope, name.value, %{
        name: name.value,
        nib_type: nib_type,
        is_const: is_const,
        is_static: !is_const
      })
    else
      _ -> scope
    end
  end

  defp collect_fn_signature(node, scope) do
    case first_name_token(node) do
      %Token{} = name ->
        params = extract_params(node)
        return_type =
          node
          |> child_nodes()
          |> Enum.filter(&(&1.rule_name == "type"))
          |> List.last()
          |> resolve_type()

        ScopeChain.define_global(scope, name.value, %{
          name: name.value,
          is_fn: true,
          fn_params: params,
          fn_return_type: return_type || @void,
          nib_type: return_type || @void
        })

      _ ->
        scope
    end
  end

  defp check_fn_body(node, outer_scope, state) do
    with %Token{} = name <- first_name_token(node),
         symbol when not is_nil(symbol) <- ScopeChain.lookup(outer_scope, name.value),
         %ASTNode{} = block <- Enum.find(child_nodes(node), &(&1.rule_name == "block")) do
      scope =
        Enum.reduce(symbol.fn_params, ScopeChain.push(outer_scope), fn {param_name, param_type}, acc ->
          ScopeChain.define_local(acc, param_name, %{name: param_name, nib_type: param_type})
        end)

      {state, _scope} = check_block(block, scope, state, symbol.fn_return_type || @void)
      state
    else
      _ -> state
    end
  end

  defp check_block(block, scope, state, return_type) do
    Enum.reduce(child_nodes(block), {state, scope}, fn stmt, {inner_state, inner_scope} ->
      check_stmt(stmt, inner_scope, inner_state, return_type)
    end)
  end

  defp check_stmt(%ASTNode{rule_name: "stmt"} = stmt, scope, state, return_type) do
    case child_nodes(stmt) do
      [inner | _] -> check_stmt(inner, scope, state, return_type)
      _ -> {state, scope}
    end
  end

  defp check_stmt(%ASTNode{rule_name: "let_stmt"} = stmt, scope, state, _return_type) do
    with %Token{} = name <- first_name_token(stmt),
         %ASTNode{} = type_node <- type_node(stmt),
         nib_type when not is_nil(nib_type) <- resolve_type(type_node),
         %ASTNode{} = expr <- first_rule(stmt, "expr") do
      {actual, state} = check_expr(expr, scope, state)
      state = if compatible?(nib_type, actual), do: state, else: error(state, "let `#{name.value}` expects #{nib_type}, got #{inspect(actual)}", expr)
      {state, ScopeChain.define_local(scope, name.value, %{name: name.value, nib_type: nib_type})}
    else
      _ -> {state, scope}
    end
  end

  defp check_stmt(%ASTNode{rule_name: "assign_stmt"} = stmt, scope, state, _return_type) do
    with %Token{} = name <- first_name_token(stmt),
         symbol when not is_nil(symbol) <- ScopeChain.lookup(scope, name.value),
         %ASTNode{} = expr <- first_rule(stmt, "expr") do
      {actual, state} = check_expr(expr, scope, state)
      state = if compatible?(symbol.nib_type, actual), do: state, else: error(state, "assignment to `#{name.value}` expects #{symbol.nib_type}, got #{inspect(actual)}", expr)
      {state, scope}
    else
      nil ->
        {error(state, "unknown variable `#{first_name_token(stmt).value}`", stmt), scope}

      _ ->
        {state, scope}
    end
  end

  defp check_stmt(%ASTNode{rule_name: "return_stmt"} = stmt, scope, state, return_type) do
    case first_rule(stmt, "expr") do
      %ASTNode{} = expr ->
        {actual, state} = check_expr(expr, scope, state)
        state = if compatible?(return_type, actual), do: state, else: error(state, "return expects #{return_type}, got #{inspect(actual)}", expr)
        {state, scope}

      _ ->
        {state, scope}
    end
  end

  defp check_stmt(%ASTNode{rule_name: "for_stmt"} = stmt, scope, state, return_type) do
    exprs = Enum.filter(child_nodes(stmt), &(&1.rule_name == "expr"))
    block = Enum.find(child_nodes(stmt), &(&1.rule_name == "block"))

    with %Token{} = name <- first_name_token(stmt),
         %ASTNode{} = type_node <- type_node(stmt),
         nib_type when not is_nil(nib_type) <- resolve_type(type_node),
         [lower_expr, upper_expr | _] <- exprs,
         %ASTNode{} = loop_block <- block do
      {lower_type, state} = check_expr(lower_expr, scope, state)
      {upper_type, state} = check_expr(upper_expr, scope, state)
      state = if numericish?(lower_type) and numericish?(upper_type), do: state, else: error(state, "for loop bounds must be numeric", stmt)

      loop_scope =
        scope
        |> ScopeChain.push()
        |> ScopeChain.define_local(name.value, %{name: name.value, nib_type: nib_type})

      {state, _loop_scope} = check_block(loop_block, loop_scope, state, return_type)
      {state, scope}
    else
      _ -> {state, scope}
    end
  end

  defp check_stmt(%ASTNode{rule_name: "expr_stmt"} = stmt, scope, state, _return_type) do
    case first_rule(stmt, "expr") do
      %ASTNode{} = expr ->
        {_type, state} = check_expr(expr, scope, state)
        {state, scope}

      _ ->
        {state, scope}
    end
  end

  defp check_stmt(_stmt, scope, state, _return_type), do: {state, scope}

  defp check_expr(%ASTNode{rule_name: "add_expr"} = node, scope, state) do
    case expression_children(node) do
      [left_node, right_node | _] ->
        {left_type, state} = check_expr(left_node, scope, state)
        {right_type, state} = check_expr(right_node, scope, state)

        inferred =
          cond do
            left_type == @literal and numeric?(right_type) -> right_type
            right_type == @literal and numeric?(left_type) -> left_type
            left_type == @literal and right_type == @literal -> @literal
            left_type == right_type and numeric?(left_type) -> left_type
            true -> nil
          end

        state =
          if is_nil(inferred) do
            error(state, "binary expression type mismatch: #{inspect(left_type)} vs #{inspect(right_type)}", node)
          else
            annotate(state, node, inferred)
          end

        {inferred, state}

      [single | _] ->
        check_expr(single, scope, state)

      _ ->
        {nil, state}
    end
  end

  defp check_expr(%ASTNode{rule_name: "call_expr"} = node, scope, state) do
    case first_name_token(node) do
      %Token{} = name ->
        symbol = ScopeChain.lookup(scope, name.value)

        if is_nil(symbol) or !Map.get(symbol, :is_fn, false) do
          {nil, error(state, "unknown function `#{name.value}`", name)}
        else
          arg_nodes =
            node
            |> child_nodes()
            |> Enum.find(&(&1.rule_name == "arg_list"))
            |> then(fn
              nil -> []
              arg_list -> Enum.filter(child_nodes(arg_list), &(&1.rule_name == "expr"))
            end)

          state =
            if length(arg_nodes) == length(symbol.fn_params) do
              Enum.zip(symbol.fn_params, arg_nodes)
              |> Enum.reduce(state, fn {{param_name, param_type}, arg_node}, acc ->
                {actual, acc} = check_expr(arg_node, scope, acc)
                if compatible?(param_type, actual), do: acc, else: error(acc, "argument `#{param_name}` expects #{param_type}, got #{inspect(actual)}", arg_node)
              end)
            else
              error(state, "function `#{name.value}` expects #{length(symbol.fn_params)} args, got #{length(arg_nodes)}", node)
            end

          {symbol.fn_return_type, annotate(state, node, symbol.fn_return_type)}
        end

      _ ->
        {nil, state}
    end
  end

  defp check_expr(%ASTNode{} = node, scope, state) do
    expr_child = Enum.find(expression_children(node), fn child -> child != node end) || Enum.find(child_nodes(node), &(&1.rule_name in @expression_rules))

    cond do
      not is_nil(expr_child) and expr_child != node ->
        {inferred, state} = check_expr(expr_child, scope, state)
        {inferred, annotate(state, node, inferred)}

      true ->
        inferred = infer_primary(node, scope)
        {inferred, annotate(state, node, inferred)}
    end
  end

  defp infer_primary(node, scope) do
    case tokens_in(node) do
      [%Token{} = token | _] ->
        case token_type(token) do
          "INT_LIT" -> @literal
          "HEX_LIT" -> @literal
          "KEYWORD" when token.value in ["true", "false"] -> @bool
          "NAME" ->
            case ScopeChain.lookup(scope, token.value) do
              nil -> nil
              symbol -> symbol.nib_type
            end

          _ ->
            nil
        end

      _ ->
        nil
    end
  end

  defp extract_params(node) do
    node
    |> child_nodes()
    |> Enum.find(&(&1.rule_name == "param_list"))
    |> then(fn
      nil ->
        []

      param_list ->
        param_list
        |> child_nodes()
        |> Enum.filter(&(&1.rule_name == "param"))
        |> Enum.map(fn param ->
          {first_name_token(param).value, resolve_type(type_node(param))}
        end)
    end)
  end

  defp resolve_type(%ASTNode{} = node) do
    node
    |> tokens_in()
    |> List.first()
    |> then(fn
      %Token{value: value} ->
        case value do
          "u4" -> @u4
          "u8" -> @u8
          "bcd" -> @bcd
          "bool" -> @bool
          _ -> nil
        end

      _ ->
        nil
    end)
  end

  defp resolve_type(_), do: nil

  defp child_nodes(%ASTNode{} = node), do: Enum.filter(node.children, &match?(%ASTNode{}, &1))
  defp child_nodes(_), do: []

  defp expression_children(%ASTNode{} = node), do: Enum.filter(child_nodes(node), &(&1.rule_name in @expression_rules))

  defp unwrap_top_decl(%ASTNode{} = node), do: List.first(child_nodes(node))
  defp unwrap_top_decl(_), do: nil

  defp first_rule(%ASTNode{} = node, rule_name), do: Enum.find(child_nodes(node), &(&1.rule_name == rule_name))
  defp first_rule(_, _), do: nil

  defp type_node(%ASTNode{} = node), do: first_rule(node, "type")
  defp type_node(_), do: nil

  defp first_name_token(%ASTNode{} = node), do: Enum.find(tokens_in(node), &(token_type(&1) == "NAME"))
  defp first_name_token(_), do: nil

  defp tokens_in(%ASTNode{} = node) do
    Enum.flat_map(node.children, fn
      %ASTNode{} = child -> tokens_in(child)
      %Token{} = token -> [token]
    end)
  end

  defp token_type(%Token{type: type}) when is_binary(type), do: type
  defp token_type(%Token{type: type}), do: to_string(type)

  defp annotate(state, _node, nil), do: state
  defp annotate(state, %ASTNode{} = node, inferred), do: %{state | types: Map.put(state.types, node_key(node), inferred)}

  defp compatible?(expected, actual), do: expected == actual or (actual == @literal and numeric?(expected))

  defp numeric?(value), do: value in [@u4, @u8, @bcd]
  defp numericish?(value), do: numeric?(value) or value == @literal

  defp error(state, message, subject) do
    {line, column} = locate(subject)
    %{state | errors: state.errors ++ [TypeCheckerProtocol.new_diagnostic(message, line, column)]}
  end

  defp locate(%Token{line: line, column: column}), do: {line, column}
  defp locate(%ASTNode{} = node), do: {node.start_line || 1, node.start_column || 1}
  defp locate(_), do: {1, 1}
end
