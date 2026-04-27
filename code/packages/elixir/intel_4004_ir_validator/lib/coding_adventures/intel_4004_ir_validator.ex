defmodule CodingAdventures.Intel4004IrValidator.IrValidationError do
  @moduledoc """
  Rule-tagged validation error returned by the Intel 4004 IR validator.
  """

  defexception [:rule, :message]

  @type t :: %__MODULE__{rule: String.t(), message: String.t()}

  @impl true
  def exception(opts) do
    %__MODULE__{
      rule: Keyword.fetch!(opts, :rule),
      message: Keyword.fetch!(opts, :message)
    }
  end

  @impl true
  def message(%__MODULE__{message: message}), do: message
end

defimpl String.Chars, for: CodingAdventures.Intel4004IrValidator.IrValidationError do
  def to_string(%{rule: rule, message: message}), do: "[#{rule}] #{message}"
end

defmodule CodingAdventures.Intel4004IrValidator.IrValidator do
  @moduledoc """
  Validates compiler IR programs against Intel 4004 backend limits.
  """

  alias CodingAdventures.CompilerIr.{
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrProgram,
    IrRegister
  }

  alias CodingAdventures.Intel4004IrValidator.IrValidationError

  @max_ram_bytes 160
  @max_call_depth 2
  @max_virtual_registers 12
  @min_load_immediate 0
  @max_load_immediate 255

  @doc "Validate an IR program and return all detected errors."
  @spec validate(IrProgram.t()) :: [IrValidationError.t()]
  def validate(%IrProgram{} = program) do
    []
    |> Kernel.++(check_no_word_ops(program))
    |> Kernel.++(check_static_ram(program))
    |> Kernel.++(check_call_depth(program))
    |> Kernel.++(check_register_count(program))
    |> Kernel.++(check_operand_range(program))
  end

  defp check_no_word_ops(%IrProgram{} = program) do
    {errors, _saw_load, _saw_store} =
      Enum.reduce(program.instructions, {[], false, false}, fn instruction,
                                                               {errors, saw_load, saw_store} ->
        cond do
          instruction.opcode == :load_word and not saw_load ->
            {
              [
                error(
                  "no_word_ops",
                  "LOAD_WORD is not supported on Intel 4004. Replace it with byte-sized accesses."
                )
                | errors
              ],
              true,
              saw_store
            }

          instruction.opcode == :store_word and not saw_store ->
            {
              [
                error(
                  "no_word_ops",
                  "STORE_WORD is not supported on Intel 4004. Replace it with byte-sized accesses."
                )
                | errors
              ],
              saw_load,
              true
            }

          true ->
            {errors, saw_load, saw_store}
        end
      end)

    Enum.reverse(errors)
  end

  defp check_static_ram(%IrProgram{} = program) do
    total =
      Enum.reduce(program.data, 0, fn
        %IrDataDecl{size: size}, acc -> acc + size
        _decl, acc -> acc
      end)

    if total <= @max_ram_bytes do
      []
    else
      [
        error(
          "static_ram",
          "Static RAM usage #{total} bytes exceeds the Intel 4004 limit of #{@max_ram_bytes} bytes."
        )
      ]
    end
  end

  defp check_call_depth(%IrProgram{} = program) do
    graph = build_call_graph(program)

    cond do
      cycle = find_cycle(graph) ->
        [
          error(
            "call_depth",
            "Recursive call graphs are not supported on Intel 4004. Found cycle: #{Enum.join(cycle, " -> ")}."
          )
        ]

      (depth = max_call_depth(graph)) > @max_call_depth ->
        [
          error(
            "call_depth",
            "Call graph depth #{depth} exceeds the Intel 4004 hardware stack limit of #{@max_call_depth} nested calls."
          )
        ]

      true ->
        []
    end
  end

  defp build_call_graph(%IrProgram{} = program) do
    {graph, _current_label} =
      Enum.reduce(program.instructions, {%{}, nil}, fn
        %IrInstruction{opcode: :label, operands: [%IrLabel{name: name} | _]}, {graph, _current} ->
          {Map.put_new(graph, name, []), name}

        %IrInstruction{opcode: :call, operands: [%IrLabel{name: callee} | _]}, {graph, current}
        when is_binary(current) ->
          graph =
            graph
            |> Map.update(current, [callee], fn children -> children ++ [callee] end)
            |> Map.put_new(callee, [])

          {graph, current}

        _instruction, state ->
          state
      end)

    graph
  end

  defp find_cycle(graph) do
    Enum.reduce_while(Map.keys(graph), nil, fn node, _acc ->
      case dfs_cycle(node, graph, []) do
        nil -> {:cont, nil}
        cycle -> {:halt, cycle}
      end
    end)
  end

  defp dfs_cycle(node, graph, path) do
    if node in path do
      start = Enum.find_index(path, &(&1 == node)) || 0
      path |> Enum.slice(start..-1//1) |> Kernel.++([node])
    else
      Enum.reduce_while(Map.get(graph, node, []), nil, fn child, _acc ->
        case dfs_cycle(child, graph, path ++ [node]) do
          nil -> {:cont, nil}
          cycle -> {:halt, cycle}
        end
      end)
    end
  end

  defp max_call_depth(graph) when map_size(graph) == 0, do: 0

  defp max_call_depth(graph) do
    graph
    |> Map.keys()
    |> Enum.map(&walk_depth(graph, &1, 0, MapSet.new()))
    |> Enum.max()
  end

  defp walk_depth(graph, node, depth, visited) do
    if MapSet.member?(visited, node) do
      depth
    else
      children = Map.get(graph, node, [])

      if children == [] do
        depth
      else
        visited = MapSet.put(visited, node)

        children
        |> Enum.map(&walk_depth(graph, &1, depth + 1, visited))
        |> Enum.max()
      end
    end
  end

  defp check_register_count(%IrProgram{} = program) do
    register_count =
      program.instructions
      |> Enum.flat_map(fn instruction -> instruction.operands end)
      |> Enum.reduce(MapSet.new(), fn
        %IrRegister{index: index}, set -> MapSet.put(set, index)
        _operand, set -> set
      end)
      |> MapSet.size()

    if register_count <= @max_virtual_registers do
      []
    else
      [
        error(
          "register_count",
          "Program uses #{register_count} distinct virtual registers but Intel 4004 supports at most #{@max_virtual_registers}."
        )
      ]
    end
  end

  defp check_operand_range(%IrProgram{} = program) do
    program.instructions
    |> Enum.flat_map(fn
      %IrInstruction{opcode: :load_imm, operands: [_dest, %IrImmediate{value: value} | _]} ->
        if value < @min_load_immediate or value > @max_load_immediate do
          [
            error(
              "operand_range",
              "LOAD_IMM immediate #{value} is out of range for Intel 4004. Valid range is [#{@min_load_immediate}, #{@max_load_immediate}]."
            )
          ]
        else
          []
        end

      _instruction ->
        []
    end)
  end

  defp error(rule, message), do: %IrValidationError{rule: rule, message: message}
end

defmodule CodingAdventures.Intel4004IrValidator do
  @moduledoc """
  Convenience facade for Intel 4004 IR validation.
  """

  alias CodingAdventures.CompilerIr.IrProgram
  alias __MODULE__.{IrValidationError, IrValidator}

  @doc "Validate an IR program and return all detected errors."
  @spec validate(IrProgram.t()) :: [IrValidationError.t()]
  def validate(%IrProgram{} = program), do: IrValidator.validate(program)
end
