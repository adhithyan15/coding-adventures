defmodule CodingAdventures.LatticeAstToCss.Evaluator do
  @moduledoc """
  Expression evaluator — compile-time evaluation of Lattice expressions.

  Lattice expressions appear in three contexts:

  1. `@if` conditions: `@if $theme == dark { ... }`
  2. `@for` bounds: `@for $i from 1 through $count { ... }`
  3. `@return` values: `@return $n * 8px;`

  The evaluator walks `lattice_expression` AST nodes and computes their
  values at compile time. All expressions are fully evaluated — there is no
  runtime in the Lattice compiler.

  ## Operator Precedence

  From tightest to loosest binding (matching the grammar):

  1. Unary minus: `-$x`
  2. Multiplication: `$a * $b`
  3. Addition/subtraction: `$a + $b`, `$a - $b`
  4. Comparison: `==`, `!=`, `>`, `>=`, `<=`
  5. Logical AND: `$a and $b`
  6. Logical OR: `$a or $b`

  The grammar already encodes this precedence via nested rules, so the evaluator
  just recursively evaluates the AST — no precedence climbing needed.

  ## Short-Circuit Evaluation

  - `A or B` — if A is truthy, B is not evaluated
  - `A and B` — if A is falsy, B is not evaluated

  ## Usage

      result = Evaluator.evaluate(expression_node, scope)
      # result is a lattice_value tagged tuple
  """

  alias CodingAdventures.LatticeAstToCss.{Scope, Values}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  @doc """
  Evaluate a `lattice_expression` AST node with the given scope.

  Dispatches on `rule_name` to the appropriate sub-evaluator. If the node
  is a token (leaf), converts it directly using `Values.token_to_value/1`.

  Returns a `lattice_value` tagged tuple (never errors — falls back to `:null`
  for unrecognized constructs).
  """
  @spec evaluate(ASTNode.t() | Token.t() | any(), Scope.t()) :: Values.lattice_value()
  def evaluate(%ASTNode{rule_name: rule_name, children: children} = node, scope) do
    case rule_name do
      "lattice_expression" -> eval_single_child(children, scope)
      "lattice_or_expr" -> eval_or(children, scope)
      "lattice_and_expr" -> eval_and(children, scope)
      "lattice_comparison" -> eval_comparison(children, scope)
      "comparison_op" -> eval_single_child(children, scope)
      "lattice_additive" -> eval_additive(children, scope)
      "lattice_multiplicative" -> eval_multiplicative(children, scope)
      "lattice_unary" -> eval_unary(children, scope)
      "lattice_primary" -> eval_primary(children, scope)
      "value_list" -> eval_value_list(children, scope)
      # For any other single-child wrapper rule, unwrap
      _ ->
        case children do
          [single] -> evaluate(single, scope)
          _ ->
            # Try the first meaningful child
            meaningful = Enum.find(children, fn
              %ASTNode{} -> true
              %Token{} -> true
              _ -> false
            end)
            if meaningful, do: evaluate(meaningful, scope), else: :null
        end
    end
    |> tap_debug(node)
  end

  # Raw token leaf — convert directly
  def evaluate(%Token{} = token, _scope) do
    Values.token_to_value(token)
  end

  # Fallback for anything else
  def evaluate(_, _scope), do: :null

  # ---------------------------------------------------------------------------
  # Rule evaluators (private)
  # ---------------------------------------------------------------------------

  # lattice_expression = lattice_or_expr ;
  # Just a single-child pass-through.
  defp eval_single_child([child | _], scope), do: evaluate(child, scope)
  defp eval_single_child([], _scope), do: :null

  # value_list — produced by variable substitution.
  # When expand_variable_declaration substitutes `$i + 1`, the evaluator
  # receives a value_list node whose children are [NUMBER(2), PLUS, NUMBER(1)].
  # If arithmetic operators are present, delegate to additive; otherwise
  # evaluate the first child.
  defp eval_value_list([], _scope), do: :null
  defp eval_value_list([single], scope), do: evaluate(single, scope)

  defp eval_value_list(children, scope) do
    has_ops = Enum.any?(children, fn
      %Token{value: v} when v in ["+", "-", "*", "/"] -> true
      _ -> false
    end)

    if has_ops do
      eval_additive(children, scope)
    else
      evaluate(hd(children), scope)
    end
  end

  # lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;
  #
  # Short-circuit: return first truthy operand, or the last value.
  # Children alternate: and_expr "or" and_expr "or" and_expr ...
  defp eval_or(children, scope) do
    # Filter out the "or" literal tokens; interleave with and_exprs
    and_exprs = Enum.reject(children, fn
      %Token{value: "or"} -> true
      _ -> false
    end)

    Enum.reduce_while(and_exprs, :null, fn child, _acc ->
      val = evaluate(child, scope)
      if Values.truthy?(val) do
        {:halt, val}
      else
        {:cont, val}
      end
    end)
  end

  # lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;
  #
  # Short-circuit: return first falsy operand, or the last value.
  defp eval_and(children, scope) do
    comparisons = Enum.reject(children, fn
      %Token{value: "and"} -> true
      _ -> false
    end)

    Enum.reduce_while(comparisons, :null, fn child, _acc ->
      val = evaluate(child, scope)
      if Values.truthy?(val) do
        {:cont, val}
      else
        {:halt, val}
      end
    end)
  end

  # lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;
  #
  # Three children: left, comparison_op node, right
  # or one child (just left, no comparison)
  defp eval_comparison([left], scope) do
    evaluate(left, scope)
  end

  defp eval_comparison(children, scope) do
    left = evaluate(hd(children), scope)

    # Find the comparison_op node and the right operand
    op_node = Enum.find(children, fn
      %ASTNode{rule_name: "comparison_op"} -> true
      _ -> false
    end)

    right_node =
      children
      |> Enum.drop_while(fn
        %ASTNode{rule_name: "comparison_op"} -> false
        _ -> true
      end)
      |> Enum.drop(1)  # skip the op_node itself
      |> List.first()

    case {op_node, right_node} do
      {nil, _} -> left
      {_, nil} -> left
      {%ASTNode{children: [op_token | _]}, right} ->
        right_val = evaluate(right, scope)
        op_name = op_token_type(op_token)
        Values.compare(left, right_val, op_name)
    end
  end

  # Extract the token type name from a comparison operator token
  defp op_token_type(%Token{type: type}), do: to_string(type)
  defp op_token_type(_), do: "EQUALS_EQUALS"

  # lattice_additive = lattice_multiplicative { ( PLUS | MINUS ) lattice_multiplicative } ;
  #
  # Collect operands and operators, apply left to right.
  defp eval_additive([first | rest], scope) do
    initial = evaluate(first, scope)

    # Process remaining children: op_token, operand, op_token, operand, ...
    eval_additive_rest(rest, initial, scope)
  end

  defp eval_additive([], _scope), do: :null

  defp eval_additive_rest([], acc, _scope), do: acc

  defp eval_additive_rest([%Token{value: op} | [right | rest]], acc, scope)
       when op in ["+", "-"] do
    right_val = evaluate(right, scope)

    result =
      case op do
        "+" ->
          case Values.add(acc, right_val) do
            {:ok, v} -> v
            {:error, _} -> acc  # fallback
          end

        "-" ->
          case Values.subtract(acc, right_val) do
            {:ok, v} -> v
            {:error, _} -> acc
          end
      end

    eval_additive_rest(rest, result, scope)
  end

  defp eval_additive_rest([_ | rest], acc, scope) do
    eval_additive_rest(rest, acc, scope)
  end

  # lattice_multiplicative = lattice_unary { ( STAR | SLASH ) lattice_unary } ;
  defp eval_multiplicative([first | rest], scope) do
    initial = evaluate(first, scope)
    eval_multiplicative_rest(rest, initial, scope)
  end

  defp eval_multiplicative([], _scope), do: :null

  defp eval_multiplicative_rest([], acc, _scope), do: acc

  defp eval_multiplicative_rest([%Token{value: "*"} | [right | rest]], acc, scope) do
    right_val = evaluate(right, scope)

    result =
      case Values.multiply(acc, right_val) do
        {:ok, v} -> v
        {:error, _} -> acc
      end

    eval_multiplicative_rest(rest, result, scope)
  end

  defp eval_multiplicative_rest([%Token{value: "/"} | [right | rest]], acc, scope) do
    right_val = evaluate(right, scope)

    result =
      case Values.divide(acc, right_val) do
        {:ok, v} -> v
        {:error, _} -> acc
      end

    eval_multiplicative_rest(rest, result, scope)
  end

  defp eval_multiplicative_rest([_ | rest], acc, scope) do
    eval_multiplicative_rest(rest, acc, scope)
  end

  # lattice_unary = MINUS lattice_unary | lattice_primary ;
  defp eval_unary([%Token{value: "-"}, operand | _], scope) do
    val = evaluate(operand, scope)
    case Values.negate(val) do
      {:ok, v} -> v
      {:error, _} -> val
    end
  end

  defp eval_unary([single | _], scope), do: evaluate(single, scope)
  defp eval_unary([], _scope), do: :null

  # lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
  #                 | STRING | IDENT | HASH
  #                 | "true" | "false" | "null"
  #                 | function_call
  #                 | LPAREN lattice_expression RPAREN ;
  defp eval_primary(children, scope) do
    Enum.reduce_while(children, :null, fn child, _acc ->
      case child do
        %Token{type: "VARIABLE", value: var_name} ->
          # Look up the variable in scope
          result = case Scope.get(scope, var_name) do
            {:ok, value} when is_tuple(value) or value == :null ->
              # Already a lattice_value — return it directly
              value
            {:ok, %ASTNode{} = ast_node} ->
              # It's an AST node (value_list) — extract the value
              extract_value_from_ast(ast_node, scope)
            {:ok, %Token{} = token} ->
              # It's a raw token — convert it
              Values.token_to_value(token)
            {:ok, other} ->
              # Some other value (e.g., from a previous evaluation)
              Values.token_to_value(other)
            :error ->
              # Variable not found — return as ident (transformer handles errors)
              {:ident, var_name}
          end
          {:halt, result}

        %Token{type: type} when type in ["LPAREN", "RPAREN"] ->
          # Skip parentheses — continue looking for the inner expression
          {:cont, :null}

        %Token{} = token ->
          {:halt, Values.token_to_value(token)}

        %ASTNode{rule_name: "lattice_expression"} = expr ->
          {:halt, evaluate(expr, scope)}

        %ASTNode{rule_name: "function_call"} ->
          # For now, function calls in expressions return :null (handled by transformer)
          {:halt, :null}

        %ASTNode{} = node ->
          {:halt, evaluate(node, scope)}

        _ ->
          {:cont, :null}
      end
    end)
  end

  # Extract a LatticeValue from an AST node (e.g., a value_list from a variable binding)
  defp extract_value_from_ast(%ASTNode{children: children}, scope) do
    # Walk children looking for the first meaningful token
    result = Enum.reduce_while(children, :null, fn
      %Token{} = token, _acc -> {:halt, Values.token_to_value(token)}
      %ASTNode{} = child, _acc ->
        val = extract_value_from_ast(child, scope)
        if val == :null, do: {:cont, :null}, else: {:halt, val}
      _, acc -> {:cont, acc}
    end)
    result
  end

  # Suppress debug output — this is a no-op tap used only for dev tracing
  defp tap_debug(value, _node), do: value
end
