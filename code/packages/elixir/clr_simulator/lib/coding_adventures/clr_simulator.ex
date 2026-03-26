defmodule CodingAdventures.ClrTrace do
  @moduledoc """
  Trace of one CLR instruction execution.
  """

  @enforce_keys [:pc, :opcode, :stack_before, :stack_after, :locals_snapshot, :description]
  defstruct [:pc, :opcode, :stack_before, :stack_after, :locals_snapshot, :description]

  @type t :: %__MODULE__{
          pc: non_neg_integer(),
          opcode: String.t(),
          stack_before: [integer() | nil],
          stack_after: [integer() | nil],
          locals_snapshot: [integer() | nil],
          description: String.t()
        }
end

defmodule CodingAdventures.ClrSimulator do
  @moduledoc """
  CLR bytecode simulator for a small educational subset of IL.
  """

  alias CodingAdventures.ClrTrace

  @nop 0x00
  @ldnull 0x01
  @ldloc_0 0x06
  @stloc_0 0x0A
  @ldloc_s 0x11
  @stloc_s 0x13
  @ldc_i4_0 0x16
  @ldc_i4_8 0x1E
  @ldc_i4_s 0x1F
  @ldc_i4 0x20
  @ret 0x2A
  @br_s 0x2B
  @brfalse_s 0x2C
  @brtrue_s 0x2D
  @add 0x58
  @sub 0x59
  @mul 0x5A
  @div 0x5B
  @prefix_fe 0xFE

  @ceq_byte 0x01
  @cgt_byte 0x02
  @clt_byte 0x04

  defstruct stack: [],
            locals: List.duplicate(nil, 16),
            pc: 0,
            bytecode: <<>>,
            halted: false

  @type t :: %__MODULE__{
          stack: [integer() | nil],
          locals: [integer() | nil],
          pc: non_neg_integer(),
          bytecode: binary(),
          halted: boolean()
        }

  def nop, do: @nop
  def ldnull, do: @ldnull
  def ldloc_s, do: @ldloc_s
  def stloc_s, do: @stloc_s
  def ret, do: @ret
  def br_s, do: @br_s
  def brfalse_s, do: @brfalse_s
  def brtrue_s, do: @brtrue_s
  def add_op, do: @add
  def sub_op, do: @sub
  def mul_op, do: @mul
  def div_op, do: @div
  def prefix_fe, do: @prefix_fe
  def ceq_byte, do: @ceq_byte
  def cgt_byte, do: @cgt_byte
  def clt_byte, do: @clt_byte

  @spec new() :: t()
  def new, do: %__MODULE__{}

  @spec load(t(), binary(), keyword()) :: t()
  def load(%__MODULE__{} = sim, bytecode, opts \\ []) when is_binary(bytecode) do
    num_locals = Keyword.get(opts, :num_locals, 16)
    %{sim | bytecode: bytecode, stack: [], locals: List.duplicate(nil, num_locals), pc: 0, halted: false}
  end

  @spec step(t()) :: {t(), ClrTrace.t()}
  def step(%__MODULE__{halted: true}), do: raise(RuntimeError, "CLR simulator has halted")

  def step(%__MODULE__{} = sim) do
    if sim.pc >= byte_size(sim.bytecode) do
      raise RuntimeError, "PC (#{sim.pc}) is beyond end of bytecode"
    end

    stack_before = sim.stack
    opcode = byte_at(sim.bytecode, sim.pc)
    execute(sim, opcode, stack_before)
  end

  @spec run(t(), keyword()) :: {t(), [ClrTrace.t()]}
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

  @spec encode_ldc_i4(integer()) :: binary()
  def encode_ldc_i4(n) when n >= 0 and n <= 8, do: <<@ldc_i4_0 + n>>
  def encode_ldc_i4(n) when n >= -128 and n <= 127, do: <<@ldc_i4_s, n::signed-8>>
  def encode_ldc_i4(n), do: <<@ldc_i4, n::little-signed-32>>

  @spec encode_stloc(non_neg_integer()) :: binary()
  def encode_stloc(slot) when slot >= 0 and slot <= 3, do: <<@stloc_0 + slot>>
  def encode_stloc(slot), do: <<@stloc_s, slot>>

  @spec encode_ldloc(non_neg_integer()) :: binary()
  def encode_ldloc(slot) when slot >= 0 and slot <= 3, do: <<@ldloc_0 + slot>>
  def encode_ldloc(slot), do: <<@ldloc_s, slot>>

  @spec assemble_clr([binary() | [integer()]]) :: binary()
  def assemble_clr(parts) when is_list(parts) do
    parts
    |> Enum.map(fn
      part when is_binary(part) -> part
      part when is_list(part) -> :erlang.list_to_binary(part)
    end)
    |> IO.iodata_to_binary()
  end

  defp execute(sim, @prefix_fe, stack_before), do: execute_two_byte(sim, stack_before)

  defp execute(sim, @nop, stack_before) do
    sim = %{sim | pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "nop", stack_before, "no operation")}
  end

  defp execute(sim, @ldnull, stack_before) do
    sim = %{sim | stack: sim.stack ++ [nil], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "ldnull", stack_before, "push null")}
  end

  defp execute(sim, opcode, stack_before) when opcode >= @ldc_i4_0 and opcode <= @ldc_i4_8 do
    value = opcode - @ldc_i4_0
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "ldc.i4.#{value}", stack_before, "push #{value}")}
  end

  defp execute(sim, @ldc_i4_s, stack_before) do
    value = signed_byte_at(sim.bytecode, sim.pc + 1)
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 2}
    {sim, make_trace(sim, sim.pc - 2, "ldc.i4.s", stack_before, "push #{value}")}
  end

  defp execute(sim, @ldc_i4, stack_before) do
    value = little_signed_32_at(sim.bytecode, sim.pc + 1)
    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + 5}
    {sim, make_trace(sim, sim.pc - 5, "ldc.i4", stack_before, "push #{value}")}
  end

  defp execute(sim, opcode, stack_before) when opcode >= @ldloc_0 and opcode <= 0x09 do
    slot = opcode - @ldloc_0
    do_ldloc(sim, slot, stack_before, "ldloc.#{slot}", 1)
  end

  defp execute(sim, opcode, stack_before) when opcode >= @stloc_0 and opcode <= 0x0D do
    slot = opcode - @stloc_0
    do_stloc(sim, slot, stack_before, "stloc.#{slot}", 1)
  end

  defp execute(sim, @ldloc_s, stack_before) do
    slot = byte_at(sim.bytecode, sim.pc + 1)
    do_ldloc(sim, slot, stack_before, "ldloc.s", 2)
  end

  defp execute(sim, @stloc_s, stack_before) do
    slot = byte_at(sim.bytecode, sim.pc + 1)
    do_stloc(sim, slot, stack_before, "stloc.s", 2)
  end

  defp execute(sim, @add, stack_before), do: do_arithmetic(sim, stack_before, "add", &Kernel.+/2)
  defp execute(sim, @sub, stack_before), do: do_arithmetic(sim, stack_before, "sub", &Kernel.-/2)
  defp execute(sim, @mul, stack_before), do: do_arithmetic(sim, stack_before, "mul", &Kernel.*/2)

  defp execute(sim, @div, stack_before) do
    {sim, b} = pop(sim)
    {sim, a} = pop(sim)

    if b == 0 do
      raise ArithmeticError, "System.DivideByZeroException: division by zero"
    end

    result = trunc(a / b)
    sim = %{sim | stack: sim.stack ++ [result], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, "div", stack_before, "pop #{b} and #{a}, push #{result}")}
  end

  defp execute(sim, @ret, stack_before) do
    sim = %{sim | pc: sim.pc + 1, halted: true}
    {sim, make_trace(sim, sim.pc - 1, "ret", stack_before, "return")}
  end

  defp execute(sim, @br_s, stack_before) do
    offset = signed_byte_at(sim.bytecode, sim.pc + 1)
    next_pc = sim.pc + 2
    target = next_pc + offset
    sim = %{sim | pc: target}
    {sim, make_trace(sim, next_pc - 2, "br.s", stack_before, "branch to PC=#{target} (offset #{signed(offset)})")}
  end

  defp execute(sim, @brfalse_s, stack_before),
    do: do_conditional_branch(sim, stack_before, "brfalse.s", true)

  defp execute(sim, @brtrue_s, stack_before),
    do: do_conditional_branch(sim, stack_before, "brtrue.s", false)

  defp execute(sim, opcode, _stack_before) do
    raise ArgumentError, "Unknown CLR opcode: 0x#{hex2(opcode)} at PC=#{sim.pc}"
  end

  defp execute_two_byte(sim, stack_before) do
    if sim.pc + 1 >= byte_size(sim.bytecode) do
      raise ArgumentError, "Incomplete two-byte opcode at PC=#{sim.pc}"
    end

    second = byte_at(sim.bytecode, sim.pc + 1)
    {sim, b} = pop(sim)
    {sim, a} = pop(sim)

    {mnemonic, result, description} =
      case second do
        @ceq_byte -> {"ceq", if(a == b, do: 1, else: 0), "pop #{b} and #{a}, push #{if(a == b, do: 1, else: 0)} (#{a} == #{b})"}
        @cgt_byte -> {"cgt", if(a > b, do: 1, else: 0), "pop #{b} and #{a}, push #{if(a > b, do: 1, else: 0)} (#{a} > #{b})"}
        @clt_byte -> {"clt", if(a < b, do: 1, else: 0), "pop #{b} and #{a}, push #{if(a < b, do: 1, else: 0)} (#{a} < #{b})"}
        _ -> raise ArgumentError, "Unknown two-byte opcode: 0xFE 0x#{hex2(second)} at PC=#{sim.pc}"
      end

    sim = %{sim | stack: sim.stack ++ [result], pc: sim.pc + 2}
    {sim, make_trace(sim, sim.pc - 2, mnemonic, stack_before, description)}
  end

  defp do_arithmetic(sim, stack_before, mnemonic, fun) do
    {sim, b} = pop(sim)
    {sim, a} = pop(sim)
    result = fun.(a, b)
    sim = %{sim | stack: sim.stack ++ [result], pc: sim.pc + 1}
    {sim, make_trace(sim, sim.pc - 1, mnemonic, stack_before, "pop #{b} and #{a}, push #{result}")}
  end

  defp do_conditional_branch(sim, stack_before, mnemonic, take_if_zero) do
    offset = signed_byte_at(sim.bytecode, sim.pc + 1)
    next_pc = sim.pc + 2
    target = next_pc + offset
    {sim, value} = pop(sim)
    numeric = if is_nil(value), do: 0, else: value
    branch? = if take_if_zero, do: numeric == 0, else: numeric != 0

    if branch? do
      sim = %{sim | pc: target}
      {sim, make_trace(sim, next_pc - 2, mnemonic, stack_before, "pop #{inspect(value)}, branch taken to PC=#{target}")}
    else
      sim = %{sim | pc: next_pc}
      {sim, make_trace(sim, next_pc - 2, mnemonic, stack_before, "pop #{inspect(value)}, branch not taken")}
    end
  end

  defp do_ldloc(sim, slot, stack_before, mnemonic, width) do
    value = Enum.at(sim.locals, slot)

    if is_nil(value) do
      raise RuntimeError, "Local variable #{slot} is uninitialized"
    end

    sim = %{sim | stack: sim.stack ++ [value], pc: sim.pc + width}
    {sim, make_trace(sim, sim.pc - width, mnemonic, stack_before, "push locals[#{slot}] = #{value}")}
  end

  defp do_stloc(sim, slot, stack_before, mnemonic, width) do
    {sim, value} = pop(sim)
    locals = List.replace_at(sim.locals, slot, value)
    sim = %{sim | locals: locals, pc: sim.pc + width}
    {sim, make_trace(sim, sim.pc - width, mnemonic, stack_before, "pop #{inspect(value)}, store in locals[#{slot}]")}
  end

  defp pop(%__MODULE__{stack: []}), do: raise(RuntimeError, "Stack underflow")
  defp pop(%__MODULE__{} = sim), do: { %{sim | stack: Enum.drop(sim.stack, -1)}, List.last(sim.stack)}

  defp make_trace(sim, pc, opcode, stack_before, description) do
    %ClrTrace{
      pc: pc,
      opcode: opcode,
      stack_before: stack_before,
      stack_after: sim.stack,
      locals_snapshot: sim.locals,
      description: description
    }
  end

  defp byte_at(binary, index), do: :binary.at(binary, index)

  defp signed_byte_at(binary, index) do
    value = :binary.at(binary, index)
    if value >= 128, do: value - 256, else: value
  end

  defp little_signed_32_at(binary, index) do
    <<value::little-signed-32>> = binary_part(binary, index, 4)
    value
  end

  defp signed(n) when n >= 0, do: "+#{n}"
  defp signed(n), do: Integer.to_string(n)

  defp hex2(n), do: n |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(2, "0")
end
