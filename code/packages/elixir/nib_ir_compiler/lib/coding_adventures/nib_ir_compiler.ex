defmodule CodingAdventures.NibIrCompiler.BuildConfig do
  defstruct optimize: true

  @type t :: %__MODULE__{
          optimize: boolean()
        }
end

defmodule CodingAdventures.NibIrCompiler.CompileResult do
  alias CodingAdventures.CompilerIr.IrProgram

  defstruct [:program]

  @type t :: %__MODULE__{
          program: IrProgram.t()
        }
end

defmodule CodingAdventures.NibIrCompiler do
  alias CodingAdventures.CompilerIr.{
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrProgram,
    IrRegister
  }

  alias CodingAdventures.NibIrCompiler.{BuildConfig, CompileResult}
  alias CodingAdventures.NibTypeChecker.TypedAst
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  defmodule State do
    defstruct [:program, :id_gen, :registers, :next_register, :loop_index]
  end

  @spec release_config() :: BuildConfig.t()
  def release_config, do: %BuildConfig{}

  @spec compile_nib(TypedAst.t(), BuildConfig.t()) :: CompileResult.t()
  def compile_nib(%TypedAst{root: root}, _config \\ release_config()) do
    state = %State{
      program: IrProgram.new("_start"),
      id_gen: IDGenerator.new(),
      registers: %{},
      next_register: 2,
      loop_index: 0
    }

    state =
      state
      |> emit_label("_start")
      |> maybe_call_main(root)
      |> emit(:halt, [])

    state =
      root
      |> function_nodes()
      |> Enum.reduce(state, &compile_function/2)

    %CompileResult{program: state.program}
  end

  defp maybe_call_main(state, root) do
    if Enum.any?(function_nodes(root), &(function_name(&1) == "main")) do
      emit(state, :call, [%IrLabel{name: "_fn_main"}])
    else
      state
    end
  end

  defp compile_function(node, state) do
    params = params(node)

    state =
      %{state | registers: %{}, next_register: 2}
      |> emit_label("_fn_#{function_name(node)}")

    {registers, next_register} =
      Enum.with_index(params, 2)
      |> Enum.reduce({%{}, 2}, fn {{name, _type}, register}, {acc, _next} ->
        {Map.put(acc, name, register), register + 1}
      end)

    state = %{state | registers: registers, next_register: next_register}

    state =
      case Enum.find(child_nodes(node), &(&1.rule_name == "block")) do
        nil -> state
        block -> compile_block(block, state)
      end

    emit(state, :ret, [])
  end

  defp compile_block(block, state) do
    Enum.reduce(child_nodes(block), state, &compile_stmt/2)
  end

  defp compile_stmt(%ASTNode{rule_name: "stmt"} = stmt, state) do
    case child_nodes(stmt) do
      [inner | _] -> compile_stmt(inner, state)
      _ -> state
    end
  end

  defp compile_stmt(%ASTNode{rule_name: "let_stmt"} = stmt, state) do
    with name when is_binary(name) <- first_name(stmt),
         %ASTNode{} = expr <- first_rule(stmt, "expr") do
      {register, state} = ensure_register(state, name)
      emit_expr_into(expr, register, state)
    else
      _ -> state
    end
  end

  defp compile_stmt(%ASTNode{rule_name: "assign_stmt"} = stmt, state) do
    with name when is_binary(name) <- first_name(stmt),
         register when not is_nil(register) <- Map.get(state.registers, name),
         %ASTNode{} = expr <- first_rule(stmt, "expr") do
      emit_expr_into(expr, register, state)
    else
      _ -> state
    end
  end

  defp compile_stmt(%ASTNode{rule_name: "return_stmt"} = stmt, state) do
    case first_rule(stmt, "expr") do
      %ASTNode{} = expr ->
        state = emit_expr_into(expr, 1, state)
        emit(state, :ret, [])

      _ ->
        state
    end
  end

  defp compile_stmt(%ASTNode{rule_name: "expr_stmt"} = stmt, state) do
    case first_rule(stmt, "expr") do
      %ASTNode{} = expr -> emit_expr_into(expr, 1, state)
      _ -> state
    end
  end

  defp compile_stmt(%ASTNode{rule_name: "for_stmt"} = stmt, state) do
    exprs = Enum.filter(child_nodes(stmt), &(&1.rule_name == "expr"))
    block = Enum.find(child_nodes(stmt), &(&1.rule_name == "block"))

    with name when is_binary(name) <- first_name(stmt),
      [lower_expr, upper_expr | _] <- exprs,
      %ASTNode{} = loop_block <- block do
      {loop_register, state} = ensure_register(state, name)
      state = emit_expr_into(lower_expr, loop_register, state)

      {end_register, state} = reserve_register(state)
      state = emit_expr_into(upper_expr, end_register, state)

      {cond_register, state} = reserve_register(state)

      start_label = "loop_#{state.loop_index}_start"
      end_label = "loop_#{state.loop_index}_end"
      state = %{state | loop_index: state.loop_index + 1}

      state
      |> emit_label(start_label)
      |> emit(:cmp_lt, [%IrRegister{index: cond_register}, %IrRegister{index: loop_register}, %IrRegister{index: end_register}])
      |> emit(:branch_z, [%IrRegister{index: cond_register}, %IrLabel{name: end_label}])
      |> then(&compile_block(loop_block, &1))
      |> emit(:add_imm, [%IrRegister{index: loop_register}, %IrRegister{index: loop_register}, %IrImmediate{value: 1}])
      |> emit(:jump, [%IrLabel{name: start_label}])
      |> emit_label(end_label)
    else
      _ -> state
    end
  end

  defp compile_stmt(_stmt, state), do: state

  defp emit_expr_into(node, register_index, state) do
    cond do
      node.rule_name == "call_expr" ->
        compile_call(node, register_index, state)

      node.rule_name == "add_expr" ->
        compile_add(node, register_index, state)

      expression_rule?(node.rule_name) and length(child_nodes(node)) == 1 ->
        [inner | _] = child_nodes(node)
        emit_expr_into(inner, register_index, state)

      token = direct_token(node) ->
        case token_type(token) do
          "INT_LIT" ->
            emit(state, :load_imm, [%IrRegister{index: register_index}, %IrImmediate{value: String.to_integer(token.value)}])

          "HEX_LIT" ->
            emit(state, :load_imm, [%IrRegister{index: register_index}, %IrImmediate{value: String.to_integer(String.replace_prefix(token.value, "0x", ""), 16)}])

          "KEYWORD" ->
            case token.value do
              "true" -> emit(state, :load_imm, [%IrRegister{index: register_index}, %IrImmediate{value: 1}])
              "false" -> emit(state, :load_imm, [%IrRegister{index: register_index}, %IrImmediate{value: 0}])
              _ -> state
            end

          "NAME" ->
            case Map.get(state.registers, token.value) do
              nil -> state
              source -> emit(state, :add_imm, [%IrRegister{index: register_index}, %IrRegister{index: source}, %IrImmediate{value: 0}])
            end

          _ ->
            state
        end

      true ->
        case child_nodes(node) do
          [inner | _] -> emit_expr_into(inner, register_index, state)
          _ -> state
        end
    end
  end

  defp compile_call(node, register_index, state) do
    arg_nodes =
      node
      |> child_nodes()
      |> Enum.find(&(&1.rule_name == "arg_list"))
      |> then(fn
        nil -> []
        arg_list -> Enum.filter(child_nodes(arg_list), &(&1.rule_name == "expr"))
      end)

    state =
      Enum.with_index(arg_nodes, 2)
      |> Enum.reduce(state, fn {arg, arg_register}, acc ->
        emit_expr_into(arg, arg_register, acc)
      end)

    state = emit(state, :call, [%IrLabel{name: "_fn_#{first_name(node)}"}])

    if register_index == 1 do
      state
    else
      emit(state, :add_imm, [%IrRegister{index: register_index}, %IrRegister{index: 1}, %IrImmediate{value: 0}])
    end
  end

  defp compile_add(node, register_index, state) do
    operands = expression_children(node)

    case operands do
      [left, right | _] ->
        state = emit_expr_into(left, register_index, state)

        case literal_value(right) do
          nil ->
            scratch = state.next_register
            state = %{state | next_register: scratch + 1}
            state = emit_expr_into(right, scratch, state)

            if Enum.member?(operator_tokens(node), "MINUS") do
              emit(state, :sub, [%IrRegister{index: register_index}, %IrRegister{index: register_index}, %IrRegister{index: scratch}])
            else
              emit(state, :add, [%IrRegister{index: register_index}, %IrRegister{index: register_index}, %IrRegister{index: scratch}])
            end

          value ->
            immediate = if Enum.member?(operator_tokens(node), "MINUS"), do: -value, else: value
            emit(state, :add_imm, [%IrRegister{index: register_index}, %IrRegister{index: register_index}, %IrImmediate{value: immediate}])
        end

      [single | _] ->
        emit_expr_into(single, register_index, state)

      _ ->
        state
    end
  end

  defp emit(%State{} = state, opcode, operands) do
    {id, id_gen} = IDGenerator.next(state.id_gen)
    instruction = %IrInstruction{opcode: opcode, operands: operands, id: id}
    %{state | id_gen: id_gen, program: IrProgram.add_instruction(state.program, instruction)}
  end

  defp emit_label(%State{} = state, name) do
    instruction = %IrInstruction{opcode: :label, operands: [%IrLabel{name: name}], id: -1}
    %{state | program: IrProgram.add_instruction(state.program, instruction)}
  end

  defp ensure_register(state, name) do
    case Map.fetch(state.registers, name) do
      {:ok, register} ->
        {register, state}

      :error ->
        register = state.next_register

        {register,
         %{
           state
           | registers: Map.put(state.registers, name, register),
             next_register: register + 1
         }}
    end
  end

  defp reserve_register(state) do
    register = state.next_register
    {register, %{state | next_register: register + 1}}
  end

  defp function_nodes(%ASTNode{} = root) do
    root
    |> child_nodes()
    |> Enum.filter(fn
      %ASTNode{rule_name: "top_decl"} -> true
      %ASTNode{rule_name: "fn_decl"} -> true
      _ -> false
    end)
    |> Enum.map(fn
      %ASTNode{rule_name: "top_decl"} = node -> List.first(child_nodes(node))
      node -> node
    end)
    |> Enum.filter(&(&1.rule_name == "fn_decl"))
  end

  defp function_name(node), do: first_name(node)

  defp params(node) do
    case Enum.find(child_nodes(node), &(&1.rule_name == "param_list")) do
      nil ->
        []

      param_list ->
        param_list
        |> child_nodes()
        |> Enum.filter(&(&1.rule_name == "param"))
        |> Enum.map(fn param -> {first_name(param), first_type_name(param)} end)
    end
  end

  defp child_nodes(%ASTNode{} = node), do: Enum.filter(node.children, &match?(%ASTNode{}, &1))
  defp child_nodes(_), do: []

  defp expression_children(%ASTNode{} = node),
    do: Enum.filter(child_nodes(node), &(&1.rule_name in ~w(expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr primary call_expr)))

  defp expression_rule?(name), do: name in ~w(expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr primary call_expr)

  defp first_rule(%ASTNode{} = node, rule_name), do: Enum.find(child_nodes(node), &(&1.rule_name == rule_name))
  defp first_rule(_, _), do: nil

  defp first_name(%ASTNode{} = node) do
    node
    |> tokens_in()
    |> Enum.find_value(fn
      %Token{type: "NAME", value: value} -> value
      %Token{type: type, value: value} when is_atom(type) and type == :NAME -> value
      _ -> nil
    end)
  end

  defp first_type_name(%ASTNode{} = node) do
    node
    |> child_nodes()
    |> Enum.find(&(&1.rule_name == "type"))
    |> tokens_in()
    |> List.first()
    |> then(fn
      %Token{value: value} -> value
      _ -> nil
    end)
  end

  defp literal_value(%ASTNode{} = node) do
    Enum.find_value(tokens_in(node), fn
      %Token{value: "true"} -> 1
      %Token{value: "false"} -> 0
      %Token{type: "INT_LIT", value: value} -> String.to_integer(value)
      %Token{type: "HEX_LIT", value: value} -> String.to_integer(String.replace_prefix(value, "0x", ""), 16)
      _ -> nil
    end)
  end

  defp operator_tokens(%ASTNode{} = node) do
    Enum.map(tokens_in(node), fn %Token{type: type} when is_binary(type) -> type end)
  end

  defp direct_token(%ASTNode{} = node) do
    tokens = tokens_in(node)

    if child_nodes(node) == [] and length(tokens) == 1 do
      List.first(tokens)
    else
      nil
    end
  end

  defp token_type(%Token{type: type}) when is_binary(type), do: type
  defp token_type(%Token{type: type}), do: to_string(type)

  defp tokens_in(%ASTNode{} = node) do
    Enum.flat_map(node.children, fn
      %ASTNode{} = child -> tokens_in(child)
      %Token{} = token -> [token]
    end)
  end
end
