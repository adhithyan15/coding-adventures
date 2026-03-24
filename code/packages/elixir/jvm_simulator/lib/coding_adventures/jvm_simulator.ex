defmodule CodingAdventures.JvmTrace do
  @moduledoc """
  Trace of a single JVM instruction step.
  """

  @enforce_keys [:pc, :opcode, :stack_before, :stack_after, :locals_snapshot, :description]
  defstruct [:pc, :opcode, :stack_before, :stack_after, :locals_snapshot, :description]

  @type t :: %__MODULE__{
          pc: non_neg_integer(),
          opcode: String.t(),
          stack_before: [integer()],
          stack_after: [integer()],
          locals_snapshot: [integer() | nil],
          description: String.t()
        }
end

defmodule CodingAdventures.JvmSimulator do
  @moduledoc """
  Small JVM bytecode simulator for integer-oriented educational programs.
  """

  alias CodingAdventures.JvmTrace

  @iconst_0 0x03
  @iconst_5 0x08
  @bipush 0x10
  @ldc 0x12
  @iload 0x15
  @iload_0 0x1A
  @istore 0x36
  @istore_0 0x3B
  @iadd 0x60
  @isub 0x64
  @imul 0x68
  @idiv 0x6C
  @if_icmpeq 0x9F
  @if_icmpgt 0xA3
  @goto 0xA7
  @ireturn 0xAC
  @return 0xB1

  defstruct stack: [],
            locals: List.duplicate(nil, 16),
            constants: [],
            pc: 0,
            halted: false,
            return_value: nil,
            bytecode: <<>>

  @type t :: %__MODULE__{
          stack: [integer()],
          locals: [integer() | nil],
          constants: [term()],
          pc: non_neg_integer(),
          halted: boolean(),
          return_value: integer() | nil,
          bytecode: binary()
        }

  def iconst_0, do: @iconst_0
  def iconst_1, do: @iconst_0 + 1
  def iconst_2, do: @iconst_0 + 2
  def iconst_3, do: @iconst_0 + 3
  def iconst_4, do: @iconst_0 + 4
  def iconst_5, do: @iconst_5
  def bipush, do: @bipush
  def ldc, do: @ldc
  def iload, do: @iload
  def iload_0, do: @iload_0
  def istore, do: @istore
  def istore_0, do: @istore_0
  def iadd, do: @iadd
  def isub, do: @isub
  def imul, do: @imul
  def idiv, do: @idiv
  def if_icmpeq, do: @if_icmpeq
  def if_icmpgt, do: @if_icmpgt
  def goto_op, do: @goto
  def ireturn, do: @ireturn
  def return_op, do: @return

  @spec new() :: t()
  def new, do: %__MODULE__{}

  @spec load(t(), binary(), keyword()) :: t()
  def load(%__MODULE__{} = sim, bytecode, opts \\ []) when is_binary(bytecode) do
    constants = Keyword.get(opts, :constants, [])
    num_locals = Keyword.get(opts, :num_locals, 16)

    %{
      sim
      | bytecode: bytecode,
        constants: constants,
        stack: [],
        locals: List.duplicate(nil, num_locals),
        pc: 0,
        halted: false,
        return_value: nil
    }
  end

  @spec step(t()) :: {t(), JvmTrace.t()}
  def step(%__MODULE__{halted: true}), do: raise(RuntimeError, "JVM simulator has halted")

  def step(%__MODULE__{} = sim) do
    if sim.pc >= byte_size(sim.bytecode) do
      raise RuntimeError, "PC (#{sim.pc}) is past end of bytecode (#{byte_size(sim.bytecode)} bytes)"
    end

    stack_before = sim.stack
    opcode = byte_at(sim.bytecode, sim.pc)
    execute(sim, opcode, stack_before)
  end

  @spec run(t(), keyword()) :: {t(), [JvmTrace.t()]}
  def run(%__MODULE__{} = sim, opts \\ []) do
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

  @spec encode_iconst(integer()) :: binary()
  def encode_iconst(n) when n >= 0 and n <= 5, do: <<@iconst_0 + n>>
  def encode_iconst(n) when n >= -128 and n <= 127, do: <<@bipush, n::signed-8>>
  def encode_iconst(n), do: raise(ArgumentError, "encode_iconst: value #{n} outside signed byte range")

  @spec encode_istore(non_neg_integer()) :: binary()
  def encode_istore(slot) when slot >= 0 and slot <= 3, do: <<@istore_0 + slot>>
  def encode_istore(slot), do: <<@istore, slot>>

  @spec encode_iload(non_neg_integer()) :: binary()
  def encode_iload(slot) when slot >= 0 and slot <= 3, do: <<@iload_0 + slot>>
  def encode_iload(slot), do: <<@iload, slot>>

  @spec assemble_jvm([[integer()]]) :: binary()
  def assemble_jvm(instructions) when is_list(instructions) do
    Enum.map(instructions, fn
      [op] ->
        <<op>>

      [op, operand] when op in [@bipush, @ldc, @iload, @istore] ->
        if op == @bipush, do: <<op, operand::signed-8>>, else: <<op, operand>>

      [op, offset] when op in [@goto, @if_icmpeq, @if_icmpgt] ->
        <<op, offset::big-signed-16>>

      [op | _rest] ->
        raise ArgumentError, "Unknown opcode in assemble_jvm: 0x#{hex2(op)}"
    end)
    |> IO.iodata_to_binary()
  end

  defp execute(sim, opcode, stack_before) when opcode >= @iconst_0 and opcode <= @iconst_5 do
    value = opcode - @iconst_0
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "iconst_#{value}", stack_before, "push #{value}")}
  end

  defp execute(sim, @bipush, stack_before) do
    value = signed_byte_at(sim.bytecode, sim.pc + 1)
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 2}
    {sim, make_trace(sim, sim.pc - 2, "bipush", stack_before, "push #{value}")}
  end

  defp execute(sim, @ldc, stack_before) do
    index = byte_at(sim.bytecode, sim.pc + 1)

    if index >= length(sim.constants) do
      raise RuntimeError, "Constant pool index #{index} out of range"
    end

    value = Enum.at(sim.constants, index)

    unless is_integer(value) do
      raise RuntimeError, "ldc: constant pool entry #{index} is not an integer"
    end

    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 2}
    {sim, make_trace(sim, sim.pc - 2, "ldc", stack_before, "push constant[#{index}] = #{value}")}
  end

  defp execute(sim, opcode, stack_before) when opcode >= @iload_0 and opcode <= 0x1D do
    slot = opcode - @iload_0
    do_iload(sim, slot, "iload_#{slot}", stack_before, 1)
  end

  defp execute(sim, @iload, stack_before) do
    slot = byte_at(sim.bytecode, sim.pc + 1)
    do_iload(sim, slot, "iload", stack_before, 2)
  end

  defp execute(sim, opcode, stack_before) when opcode >= @istore_0 and opcode <= 0x3E do
    slot = opcode - @istore_0
    do_istore(sim, slot, "istore_#{slot}", stack_before, 1)
  end

  defp execute(sim, @istore, stack_before) do
    slot = byte_at(sim.bytecode, sim.pc + 1)
    do_istore(sim, slot, "istore", stack_before, 2)
  end

  defp execute(sim, @iadd, stack_before), do: do_binary_op(sim, "iadd", stack_before, &Kernel.+/2)
  defp execute(sim, @isub, stack_before), do: do_binary_op(sim, "isub", stack_before, &Kernel.-/2)
  defp execute(sim, @imul, stack_before), do: do_binary_op(sim, "imul", stack_before, &Kernel.*/2)

  defp execute(sim, @idiv, stack_before) do
    ensure_stack!(sim, 2, "idiv")
    if List.last(sim.stack) == 0, do: raise(RuntimeError, "ArithmeticException: division by zero")
    do_binary_op(sim, "idiv", stack_before, fn a, b -> trunc(a / b) end)
  end

  defp execute(sim, @goto, stack_before) do
    offset = signed_short_at(sim.bytecode, sim.pc + 1)
    target = sim.pc + offset
    sim = %{sim | pc: target}
    {sim, make_trace(sim, target - offset, "goto", stack_before, "jump to PC=#{target} (offset #{signed(offset)})")}
  end

  defp execute(sim, @if_icmpeq, stack_before), do: do_if_icmp(sim, "if_icmpeq", stack_before, &Kernel.==/2)
  defp execute(sim, @if_icmpgt, stack_before), do: do_if_icmp(sim, "if_icmpgt", stack_before, &Kernel.>/2)

  defp execute(sim, @ireturn, stack_before) do
    ensure_stack!(sim, 1, "ireturn")
    {sim, value} = pop(sim)
    sim = %{sim | halted: true, return_value: value, pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "ireturn", stack_before, "return #{value}")}
  end

  defp execute(sim, @return, stack_before) do
    sim = %{sim | halted: true, pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "return", stack_before, "return void")}
  end

  defp execute(sim, opcode, _stack_before) do
    raise RuntimeError, "Unknown JVM opcode: 0x#{hex2(opcode)} at PC=#{sim.pc}"
  end

  defp do_iload(sim, slot, mnemonic, stack_before, width) do
    value = Enum.at(sim.locals, slot)
    if is_nil(value), do: raise(RuntimeError, "Local variable #{slot} has not been initialized")
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + width}
    {sim, make_trace(sim, sim.pc - width, mnemonic, stack_before, "push locals[#{slot}] = #{value}")}
  end

  defp do_istore(sim, slot, mnemonic, stack_before, width) do
    ensure_stack!(sim, 1, mnemonic)
    {sim, value} = pop(sim)
    sim = %{sim | locals: List.replace_at(sim.locals, slot, value), pc: sim.pc + width}
    {sim, make_trace(sim, sim.pc - width, mnemonic, stack_before, "pop #{value}, store in locals[#{slot}]")}
  end

  defp do_binary_op(sim, mnemonic, stack_before, fun) do
    ensure_stack!(sim, 2, mnemonic)
    {sim, b} = pop(sim)
    {sim, a} = pop(sim)
    result = to_i32(fun.(a, b))
    sim = %{sim | stack: sim.stack ++ [result], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, mnemonic, stack_before, "pop #{b} and #{a}, push #{result}")}
  end

  defp do_if_icmp(sim, mnemonic, stack_before, pred) do
    ensure_stack!(sim, 2, mnemonic)
    offset = signed_short_at(sim.bytecode, sim.pc + 1)
    {sim, b} = pop(sim)
    {sim, a} = pop(sim)
    taken = pred.(a, b)

    {sim, description} =
      if taken do
        target = sim.pc + offset
        op = if String.contains?(mnemonic, "eq"), do: "==", else: ">"
        {%{sim | pc: target}, "pop #{b} and #{a}, #{a} #{op} #{b} is true, jump to PC=#{target}"}
      else
        op = if String.contains?(mnemonic, "eq"), do: "==", else: ">"
        {%{sim | pc: sim.pc + 3}, "pop #{b} and #{a}, #{a} #{op} #{b} is false, fall through"}
      end

    {sim, make_trace(sim, if(taken, do: sim.pc - offset, else: sim.pc - 3), mnemonic, stack_before, description)}
  end

  defp ensure_stack!(sim, count, mnemonic) do
    if length(sim.stack) < count do
      raise RuntimeError, "Stack underflow: #{mnemonic} requires #{count} operand#{if count == 1, do: "", else: "s"}"
    end
  end

  defp pop(%__MODULE__{stack: []}), do: raise(RuntimeError, "Stack underflow")
  defp pop(%__MODULE__{} = sim), do: {%{sim | stack: Enum.drop(sim.stack, -1)}, List.last(sim.stack)}

  defp make_trace(sim, pc, opcode, stack_before, description) do
    %JvmTrace{
      pc: pc,
      opcode: opcode,
      stack_before: stack_before,
      stack_after: sim.stack,
      locals_snapshot: sim.locals,
      description: description
    }
  end

  defp to_i32(value) do
    value = Bitwise.band(value, 0xFFFFFFFF)
    if value >= 0x80000000, do: value - 0x100000000, else: value
  end

  defp byte_at(binary, index), do: :binary.at(binary, index)
  defp signed_byte_at(binary, index) do
    value = :binary.at(binary, index)
    if value >= 128, do: value - 256, else: value
  end
  defp signed_short_at(binary, index) do
    <<value::big-signed-16>> = binary_part(binary, index, 2)
    value
  end

  defp signed(n) when n >= 0, do: "+#{n}"
  defp signed(n), do: Integer.to_string(n)
  defp hex2(n), do: n |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(2, "0")
end
