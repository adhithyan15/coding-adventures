defmodule CodingAdventures.IrOptimizer.Pass do
  @moduledoc """
  Behaviour implemented by IR optimization passes.
  """

  alias CodingAdventures.CompilerIr.IrProgram

  @callback name() :: String.t()
  @callback run(IrProgram.t()) :: IrProgram.t()
end

defmodule CodingAdventures.IrOptimizer.OptimizationResult do
  @moduledoc """
  Result metadata returned by an optimizer run.
  """

  alias CodingAdventures.CompilerIr.IrProgram

  defstruct program: nil,
            passes_run: [],
            instructions_before: 0,
            instructions_after: 0,
            instructions_eliminated: 0

  @type t :: %__MODULE__{
          program: IrProgram.t(),
          passes_run: [String.t()],
          instructions_before: non_neg_integer(),
          instructions_after: non_neg_integer(),
          instructions_eliminated: integer()
        }
end

defmodule CodingAdventures.IrOptimizer do
  @moduledoc """
  Pipeline runner for compiler IR optimization passes.
  """

  alias CodingAdventures.CompilerIr.{IrInstruction, IrProgram}
  alias __MODULE__.{ConstantFolder, DeadCodeEliminator, OptimizationResult, PeepholeOptimizer}

  defstruct passes: []

  @type pass :: module()
  @type t :: %__MODULE__{passes: [pass()]}

  @doc "Create an optimizer with the given pass modules."
  @spec new([pass()]) :: t()
  def new(passes \\ []), do: %__MODULE__{passes: passes}

  @doc "Create the default optimization pipeline."
  @spec default_passes() :: t()
  def default_passes do
    new([DeadCodeEliminator, ConstantFolder, PeepholeOptimizer])
  end

  @doc "Create an optimizer that runs no passes."
  @spec no_op() :: t()
  def no_op, do: new([])

  @doc "Optimize a program with the default pipeline."
  @spec optimize(IrProgram.t()) :: OptimizationResult.t()
  def optimize(%IrProgram{} = program), do: optimize(default_passes(), program)

  @doc "Optimize a program with an explicit pass list or configured optimizer."
  @spec optimize(IrProgram.t(), [pass()]) :: OptimizationResult.t()
  @spec optimize(t(), IrProgram.t()) :: OptimizationResult.t()
  def optimize(%IrProgram{} = program, passes) when is_list(passes) do
    optimize(new(passes), program)
  end

  def optimize(%__MODULE__{passes: passes}, %IrProgram{} = program) do
    before_count = length(program.instructions)

    {optimized, passes_run} =
      Enum.reduce(passes, {clone_program(program), []}, fn pass, {current, names} ->
        {pass.run(current), names ++ [pass.name()]}
      end)

    after_count = length(optimized.instructions)

    %OptimizationResult{
      program: optimized,
      passes_run: passes_run,
      instructions_before: before_count,
      instructions_after: after_count,
      instructions_eliminated: before_count - after_count
    }
  end

  @doc false
  @spec clone_program(IrProgram.t(), [IrInstruction.t()] | nil) :: IrProgram.t()
  def clone_program(%IrProgram{} = program, instructions \\ nil) do
    %IrProgram{
      program
      | data: Enum.map(program.data, & &1),
        instructions: Enum.map(instructions || program.instructions, &clone_instruction/1)
    }
  end

  @doc false
  @spec clone_instruction(IrInstruction.t()) :: IrInstruction.t()
  def clone_instruction(%IrInstruction{} = instruction) do
    %{instruction | operands: Enum.map(instruction.operands, & &1)}
  end
end

defmodule CodingAdventures.IrOptimizer.DeadCodeEliminator do
  @moduledoc """
  Removes instructions after unconditional branches until the next label.
  """

  @behaviour CodingAdventures.IrOptimizer.Pass

  alias CodingAdventures.CompilerIr.IrProgram
  alias CodingAdventures.IrOptimizer

  @unconditional_branches MapSet.new([:jump, :ret, :halt])

  @impl true
  def name, do: "DeadCodeEliminator"

  @impl true
  def run(%IrProgram{} = program) do
    {live, _reachable} =
      Enum.reduce(program.instructions, {[], true}, fn instruction, {live, reachable} ->
        reachable = if instruction.opcode == :label, do: true, else: reachable
        live = if reachable, do: [IrOptimizer.clone_instruction(instruction) | live], else: live

        reachable =
          if MapSet.member?(@unconditional_branches, instruction.opcode),
            do: false,
            else: reachable

        {live, reachable}
      end)

    IrOptimizer.clone_program(program, Enum.reverse(live))
  end
end

defmodule CodingAdventures.IrOptimizer.ConstantFolder do
  @moduledoc """
  Folds simple constant load plus immediate arithmetic sequences.
  """

  @behaviour CodingAdventures.IrOptimizer.Pass

  alias CodingAdventures.CompilerIr.{IrImmediate, IrProgram, IrRegister}
  alias CodingAdventures.IrOptimizer

  @foldable_imm_ops MapSet.new([:add_imm, :and_imm])
  @writes_to_dest MapSet.new([
                    :load_imm,
                    :load_addr,
                    :load_byte,
                    :load_word,
                    :add,
                    :add_imm,
                    :sub,
                    :and,
                    :and_imm,
                    :cmp_eq,
                    :cmp_ne,
                    :cmp_lt,
                    :cmp_gt
                  ])

  @impl true
  def name, do: "ConstantFolder"

  @impl true
  def run(%IrProgram{} = program) do
    {instructions, _pending_loads} =
      Enum.reduce(program.instructions, {[], %{}}, fn instruction, {out, pending} ->
        cond do
          instruction.opcode == :load_imm ->
            handle_load_imm(instruction, out, pending)

          MapSet.member?(@foldable_imm_ops, instruction.opcode) ->
            handle_foldable_immediate(instruction, out, pending)

          true ->
            handle_passthrough(instruction, out, pending)
        end
      end)

    IrOptimizer.clone_program(program, instructions)
  end

  defp handle_load_imm(
         %{operands: [%IrRegister{index: index}, %IrImmediate{value: value}]} = instruction,
         out,
         pending
       ) do
    {out ++ [IrOptimizer.clone_instruction(instruction)], Map.put(pending, index, value)}
  end

  defp handle_load_imm(instruction, out, pending) do
    {out ++ [IrOptimizer.clone_instruction(instruction)], pending}
  end

  defp handle_foldable_immediate(
         %{
           opcode: opcode,
           operands: [
             %IrRegister{index: dest_index} = dest,
             %IrRegister{index: src_index},
             %IrImmediate{value: immediate}
           ]
         } = instruction,
         out,
         pending
       )
       when dest_index == src_index do
    case Map.fetch(pending, dest_index) do
      {:ok, base} ->
        new_value =
          if opcode == :add_imm, do: base + immediate, else: Bitwise.band(base, immediate)

        out = replace_last_load(out, dest, new_value)
        {out, Map.put(pending, dest_index, new_value)}

      :error ->
        handle_passthrough(instruction, out, pending)
    end
  end

  defp handle_foldable_immediate(instruction, out, pending) do
    handle_passthrough(instruction, out, pending)
  end

  defp handle_passthrough(instruction, out, pending) do
    pending =
      if MapSet.member?(@writes_to_dest, instruction.opcode) do
        case instruction.operands do
          [%IrRegister{index: index} | _] -> Map.delete(pending, index)
          _ -> pending
        end
      else
        pending
      end

    {out ++ [IrOptimizer.clone_instruction(instruction)], pending}
  end

  defp replace_last_load(out, %IrRegister{index: index} = dest, new_value) do
    out
    |> Enum.reverse()
    |> replace_first_load(index, dest, new_value)
    |> Enum.reverse()
  end

  defp replace_first_load(
         [%{opcode: :load_imm, operands: [%IrRegister{index: index} | _]} = instruction | rest],
         index,
         dest,
         value
       ) do
    [%{instruction | operands: [dest, %IrImmediate{value: value}]} | rest]
  end

  defp replace_first_load([instruction | rest], index, dest, value) do
    [instruction | replace_first_load(rest, index, dest, value)]
  end

  defp replace_first_load([], _index, _dest, _value), do: []
end

defmodule CodingAdventures.IrOptimizer.PeepholeOptimizer do
  @moduledoc """
  Applies local two-instruction rewrite patterns until a fixed point.
  """

  @behaviour CodingAdventures.IrOptimizer.Pass

  alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrProgram, IrRegister}
  alias CodingAdventures.IrOptimizer

  @max_iterations 10

  @impl true
  def name, do: "PeepholeOptimizer"

  @impl true
  def run(%IrProgram{} = program) do
    instructions =
      Enum.reduce_while(1..@max_iterations, program.instructions, fn _iteration, current ->
        next = apply_patterns(current)

        if length(next) == length(current) do
          {:halt, next}
        else
          {:cont, next}
        end
      end)

    IrOptimizer.clone_program(program, instructions)
  end

  defp apply_patterns([current, next | rest]) do
    case try_merge(current, next) do
      nil -> [IrOptimizer.clone_instruction(current) | apply_patterns([next | rest])]
      merged -> [merged | apply_patterns(rest)]
    end
  end

  defp apply_patterns([last]), do: [IrOptimizer.clone_instruction(last)]
  defp apply_patterns([]), do: []

  defp try_merge(
         %IrInstruction{
           opcode: :add_imm,
           operands: [
             %IrRegister{index: index} = dest,
             %IrRegister{index: index},
             %IrImmediate{value: left}
           ],
           id: id
         },
         %IrInstruction{
           opcode: :add_imm,
           operands: [
             %IrRegister{index: index},
             %IrRegister{index: index},
             %IrImmediate{value: right}
           ]
         }
       ) do
    %IrInstruction{
      opcode: :add_imm,
      operands: [dest, %IrRegister{index: index}, %IrImmediate{value: left + right}],
      id: id
    }
  end

  defp try_merge(
         %IrInstruction{opcode: opcode, operands: [%IrRegister{index: index} | operands]} =
           current,
         %IrInstruction{
           opcode: :and_imm,
           operands: [
             %IrRegister{index: index},
             %IrRegister{index: index},
             %IrImmediate{value: 255}
           ]
         }
       )
       when opcode in [:add_imm, :load_imm] do
    case List.last(operands) do
      %IrImmediate{value: value} when value >= 0 and value <= 255 ->
        IrOptimizer.clone_instruction(current)

      _other ->
        nil
    end
  end

  defp try_merge(
         %IrInstruction{
           opcode: :load_imm,
           operands: [%IrRegister{index: index} = dest, %IrImmediate{value: 0}],
           id: id
         },
         %IrInstruction{
           opcode: :add_imm,
           operands: [
             %IrRegister{index: index},
             %IrRegister{index: index},
             %IrImmediate{} = immediate
           ]
         }
       ) do
    %IrInstruction{opcode: :load_imm, operands: [dest, immediate], id: id}
  end

  defp try_merge(_current, _next), do: nil
end
