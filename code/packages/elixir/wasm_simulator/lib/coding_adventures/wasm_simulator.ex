defmodule CodingAdventures.WasmInstruction do
  @moduledoc """
  Decoded WASM instruction.
  """

  @enforce_keys [:opcode, :mnemonic, :operand, :size]
  defstruct [:opcode, :mnemonic, :operand, :size]

  @type t :: %__MODULE__{
          opcode: non_neg_integer(),
          mnemonic: String.t(),
          operand: integer() | nil,
          size: pos_integer()
        }
end

defmodule CodingAdventures.WasmStepTrace do
  @moduledoc """
  Trace of one WASM instruction execution.
  """

  @enforce_keys [:pc, :instruction, :stack_before, :stack_after, :locals_snapshot, :description]
  defstruct [:pc, :instruction, :stack_before, :stack_after, :locals_snapshot, :description, halted: false]

  @type t :: %__MODULE__{
          pc: non_neg_integer(),
          instruction: CodingAdventures.WasmInstruction.t(),
          stack_before: [integer()],
          stack_after: [integer()],
          locals_snapshot: [integer()],
          description: String.t(),
          halted: boolean()
        }
end

defmodule CodingAdventures.WasmDecoder do
  @moduledoc """
  Decoder for a small subset of WASM bytecode.
  """

  alias CodingAdventures.WasmInstruction

  @op_end 0x0B
  @op_local_get 0x20
  @op_local_set 0x21
  @op_i32_const 0x41
  @op_i32_add 0x6A
  @op_i32_sub 0x6B

  def decode(bytecode, pc) when is_binary(bytecode) and is_integer(pc) and pc >= 0 do
    opcode = :binary.at(bytecode, pc)

    case opcode do
      @op_i32_const ->
        <<value::little-signed-32>> = binary_part(bytecode, pc + 1, 4)
        %WasmInstruction{opcode: opcode, mnemonic: "i32.const", operand: value, size: 5}

      @op_i32_add ->
        %WasmInstruction{opcode: opcode, mnemonic: "i32.add", operand: nil, size: 1}

      @op_i32_sub ->
        %WasmInstruction{opcode: opcode, mnemonic: "i32.sub", operand: nil, size: 1}

      @op_local_get ->
        index = :binary.at(bytecode, pc + 1)
        %WasmInstruction{opcode: opcode, mnemonic: "local.get", operand: index, size: 2}

      @op_local_set ->
        index = :binary.at(bytecode, pc + 1)
        %WasmInstruction{opcode: opcode, mnemonic: "local.set", operand: index, size: 2}

      @op_end ->
        %WasmInstruction{opcode: opcode, mnemonic: "end", operand: nil, size: 1}

      _ ->
        raise ArgumentError, "Unknown WASM opcode: 0x#{hex2(opcode)} at PC=#{pc}"
    end
  end

  defp hex2(value), do: value |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(2, "0")
end

defmodule CodingAdventures.WasmExecutor do
  @moduledoc """
  Executes decoded WASM instructions.
  """

  alias CodingAdventures.WasmInstruction
  alias CodingAdventures.WasmStepTrace

  def execute(%WasmInstruction{} = instruction, stack, locals, pc)
      when is_list(stack) and is_list(locals) and is_integer(pc) and pc >= 0 do
    stack_before = Enum.to_list(stack)

    case instruction.mnemonic do
      "i32.const" ->
        value = instruction.operand
        stack = stack ++ [value]
        trace(pc, instruction, stack_before, stack, locals, "push #{value}")

      "i32.add" ->
        ensure_stack!(stack, 2, "i32.add")
        {stack, b} = pop(stack)
        {stack, a} = pop(stack)
        result = i32(a + b)
        stack = stack ++ [result]
        trace(pc, instruction, stack_before, stack, locals, "pop #{b} and #{a}, push #{result}")

      "i32.sub" ->
        ensure_stack!(stack, 2, "i32.sub")
        {stack, b} = pop(stack)
        {stack, a} = pop(stack)
        result = i32(a - b)
        stack = stack ++ [result]
        trace(pc, instruction, stack_before, stack, locals, "pop #{b} and #{a}, push #{result}")

      "local.get" ->
        index = instruction.operand
        value = Enum.at(locals, index)
        stack = stack ++ [value]
        trace(pc, instruction, stack_before, stack, locals, "push locals[#{index}] = #{value}")

      "local.set" ->
        ensure_stack!(stack, 1, "local.set")
        index = instruction.operand
        {stack, value} = pop(stack)
        locals = List.replace_at(locals, index, value)
        trace(pc, instruction, stack_before, stack, locals, "pop #{value}, store in locals[#{index}]")

      "end" ->
        %WasmStepTrace{
          pc: pc,
          instruction: instruction,
          stack_before: stack_before,
          stack_after: stack,
          locals_snapshot: locals,
          description: "halt",
          halted: true
        }
    end
  end

  defp trace(pc, instruction, stack_before, stack_after, locals, description) do
    %WasmStepTrace{
      pc: pc,
      instruction: instruction,
      stack_before: stack_before,
      stack_after: stack_after,
      locals_snapshot: locals,
      description: description,
      halted: false
    }
  end

  defp pop([]), do: raise(RuntimeError, "Stack underflow")
  defp pop(stack), do: {Enum.drop(stack, -1), List.last(stack)}

  defp ensure_stack!(stack, count, mnemonic) do
    if length(stack) < count do
      raise RuntimeError, "Stack underflow: #{mnemonic} requires #{count} operand#{if count == 1, do: "", else: "s"}"
    end
  end

  defp i32(value) do
    value = Bitwise.band(value, 0xFFFFFFFF)
    if value >= 0x80000000, do: value - 0x100000000, else: value
  end
end

defmodule CodingAdventures.WasmSimulator do
  @moduledoc """
  Standalone simulator for a tiny educational subset of WASM.
  """

  alias CodingAdventures.WasmDecoder
  alias CodingAdventures.WasmExecutor

  @op_end 0x0B
  @op_local_get 0x20
  @op_local_set 0x21
  @op_i32_const 0x41
  @op_i32_add 0x6A
  @op_i32_sub 0x6B

  defstruct stack: [], locals: [], pc: 0, bytecode: <<>>, halted: false, cycle: 0

  @type t :: %__MODULE__{
          stack: [integer()],
          locals: [integer()],
          pc: non_neg_integer(),
          bytecode: binary(),
          halted: boolean(),
          cycle: non_neg_integer()
        }

  def op_end, do: @op_end
  def op_local_get, do: @op_local_get
  def op_local_set, do: @op_local_set
  def op_i32_const, do: @op_i32_const
  def op_i32_add, do: @op_i32_add
  def op_i32_sub, do: @op_i32_sub

  def new(opts \\ []) do
    num_locals = Keyword.get(opts, :num_locals, 4)
    %__MODULE__{locals: List.duplicate(0, num_locals)}
  end

  def load(%__MODULE__{} = sim, bytecode) when is_binary(bytecode) do
    %{sim | bytecode: bytecode, pc: 0, halted: false, cycle: 0, stack: [], locals: List.duplicate(0, length(sim.locals))}
  end

  def step(%__MODULE__{halted: true}), do: raise(RuntimeError, "WASM simulator has halted")

  def step(%__MODULE__{} = sim) do
    instruction = WasmDecoder.decode(sim.bytecode, sim.pc)
    trace = WasmExecutor.execute(instruction, sim.stack, sim.locals, sim.pc)

    next =
      %{
        sim
        | stack: trace.stack_after,
          locals: trace.locals_snapshot,
          pc: sim.pc + instruction.size,
          halted: trace.halted,
          cycle: sim.cycle + 1
      }

    {next, trace}
  end

  def run(%__MODULE__{} = sim, program, opts \\ []) when is_binary(program) do
    sim = load(sim, program)
    max_steps = Keyword.get(opts, :max_steps, 10_000)

    Enum.reduce_while(1..max_steps, {sim, []}, fn _, {acc, traces} ->
      if acc.halted do
        {:halt, {acc, traces}}
      else
        {next, trace} = step(acc)
        {:cont, {next, traces ++ [trace]}}
      end
    end)
  end

  def encode_i32_const(value), do: <<@op_i32_const, value::little-signed-32>>
  def encode_i32_add, do: <<@op_i32_add>>
  def encode_i32_sub, do: <<@op_i32_sub>>
  def encode_local_get(index), do: <<@op_local_get, index>>
  def encode_local_set(index), do: <<@op_local_set, index>>
  def encode_end, do: <<@op_end>>
  def assemble_wasm(instructions) when is_list(instructions), do: IO.iodata_to_binary(instructions)
end
