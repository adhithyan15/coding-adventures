defmodule CodingAdventures.ArmDecodeResult do
  @moduledoc """
  Decoded ARM instruction fields.
  """

  @enforce_keys [:mnemonic, :fields, :raw_instruction]
  defstruct [:mnemonic, :fields, :raw_instruction]
end

defmodule CodingAdventures.ArmExecuteResult do
  @moduledoc """
  Result of executing one ARM instruction.
  """

  @enforce_keys [:description, :registers_changed, :memory_changed, :next_pc]
  defstruct [:description, :registers_changed, :memory_changed, :next_pc, halted: false]
end

defmodule CodingAdventures.ArmTrace do
  @moduledoc """
  Trace of one ARM simulation step.
  """

  @enforce_keys [:pc, :decoded, :execution]
  defstruct [:pc, :decoded, :execution]
end

defmodule CodingAdventures.ArmDecoder do
  @moduledoc """
  Decoder for a tiny subset of ARM data-processing instructions.
  """

  import Bitwise

  alias CodingAdventures.ArmDecodeResult

  @opcode_mov 0b1101
  @opcode_add 0b0100
  @opcode_sub 0b0010
  @hlt_instruction 0xFFFFFFFF

  def hlt_instruction, do: @hlt_instruction

  def decode(raw, _pc \\ 0) when is_integer(raw) do
    if raw == @hlt_instruction do
      %ArmDecodeResult{mnemonic: "hlt", fields: %{}, raw_instruction: raw}
    else
      cond = raw >>> 28 &&& 0xF
      i_bit = raw >>> 25 &&& 0x1
      opcode = raw >>> 21 &&& 0xF
      s_bit = raw >>> 20 &&& 0x1
      rn = raw >>> 16 &&& 0xF
      rd = raw >>> 12 &&& 0xF
      operand2 = raw &&& 0xFFF

      mnemonic =
        case opcode do
          @opcode_mov -> "mov"
          @opcode_add -> "add"
          @opcode_sub -> "sub"
          _ -> "dp_op(#{Integer.to_string(opcode, 2) |> String.pad_leading(4, "0")})"
        end

      fields =
        if i_bit == 1 do
          rotate = operand2 >>> 8 &&& 0xF
          imm8 = operand2 &&& 0xFF
          shift = rotate * 2

          imm_value =
            if shift > 0 do
              ((imm8 >>> shift) ||| ((imm8 <<< (32 - shift)) &&& 0xFFFFFFFF)) &&& 0xFFFFFFFF
            else
              imm8
            end

          %{
            cond: cond,
            i_bit: i_bit,
            opcode: opcode,
            s_bit: s_bit,
            rn: rn,
            rd: rd,
            imm: imm_value
          }
        else
          rm = operand2 &&& 0xF

          %{
            cond: cond,
            i_bit: i_bit,
            opcode: opcode,
            s_bit: s_bit,
            rn: rn,
            rd: rd,
            rm: rm
          }
        end

      %ArmDecodeResult{mnemonic: mnemonic, fields: fields, raw_instruction: raw}
    end
  end
end

defmodule CodingAdventures.ArmExecutor do
  @moduledoc """
  Executor for a small ARM instruction subset.
  """

  import Bitwise

  alias CodingAdventures.ArmDecodeResult
  alias CodingAdventures.ArmExecuteResult

  def execute(%ArmDecodeResult{mnemonic: "mov", fields: fields}, registers, _memory, pc) do
    rd = fields.rd
    imm = fields.imm &&& 0xFFFFFFFF
    registers = Map.put(registers, rd, imm)

    {registers,
     %ArmExecuteResult{
       description: "R#{rd} = #{imm}",
       registers_changed: %{"R#{rd}" => imm},
       memory_changed: %{},
       next_pc: pc + 4
     }}
  end

  def execute(%ArmDecodeResult{mnemonic: "add", fields: fields}, registers, _memory, pc) do
    rd = fields.rd
    rn = fields.rn
    rm = fields.rm
    rn_val = Map.get(registers, rn, 0)
    rm_val = Map.get(registers, rm, 0)
    result = (rn_val + rm_val) &&& 0xFFFFFFFF
    registers = Map.put(registers, rd, result)

    {registers,
     %ArmExecuteResult{
       description: "R#{rd} = R#{rn}(#{rn_val}) + R#{rm}(#{rm_val}) = #{result}",
       registers_changed: %{"R#{rd}" => result},
       memory_changed: %{},
       next_pc: pc + 4
     }}
  end

  def execute(%ArmDecodeResult{mnemonic: "sub", fields: fields}, registers, _memory, pc) do
    rd = fields.rd
    rn = fields.rn
    rm = fields.rm
    rn_val = Map.get(registers, rn, 0)
    rm_val = Map.get(registers, rm, 0)
    result = (rn_val - rm_val) &&& 0xFFFFFFFF
    registers = Map.put(registers, rd, result)

    {registers,
     %ArmExecuteResult{
       description: "R#{rd} = R#{rn}(#{rn_val}) - R#{rm}(#{rm_val}) = #{result}",
       registers_changed: %{"R#{rd}" => result},
       memory_changed: %{},
       next_pc: pc + 4
     }}
  end

  def execute(%ArmDecodeResult{mnemonic: "hlt"}, registers, _memory, pc) do
    {registers,
     %ArmExecuteResult{
       description: "Halt",
       registers_changed: %{},
       memory_changed: %{},
       next_pc: pc,
       halted: true
     }}
  end

  def execute(%ArmDecodeResult{mnemonic: mnemonic}, registers, _memory, pc) do
    {registers,
     %ArmExecuteResult{
       description: "Unknown instruction: #{mnemonic}",
       registers_changed: %{},
       memory_changed: %{},
       next_pc: pc + 4
     }}
  end
end

defmodule CodingAdventures.ArmSimulator do
  @moduledoc """
  ARMv7 subset simulator with MOV/ADD/SUB and a custom HLT sentinel.
  """

  import Bitwise

  alias CodingAdventures.ArmDecodeResult
  alias CodingAdventures.ArmDecoder
  alias CodingAdventures.ArmExecutor
  alias CodingAdventures.ArmTrace

  @cond_al 0b1110
  @opcode_mov 0b1101
  @opcode_add 0b0100
  @opcode_sub 0b0010
  @hlt_instruction 0xFFFFFFFF

  defstruct registers: %{}, memory: %{}, pc: 0, program: [], halted: false, cycle: 0

  @type t :: %__MODULE__{
          registers: %{optional(non_neg_integer()) => non_neg_integer()},
          memory: map(),
          pc: non_neg_integer(),
          program: [non_neg_integer()],
          halted: boolean(),
          cycle: non_neg_integer()
        }

  def new do
    registers = Enum.into(0..15, %{}, fn reg -> {reg, 0} end)
    %__MODULE__{registers: registers}
  end

  def load_program(%__MODULE__{} = sim, program) when is_list(program) do
    %{sim | program: program, pc: 0, halted: false, cycle: 0, registers: Enum.into(0..15, %{}, fn reg -> {reg, 0} end)}
  end

  def step(%__MODULE__{halted: true}), do: raise(RuntimeError, "ARM simulator has halted")

  def step(%__MODULE__{} = sim) do
    raw = Enum.at(sim.program, div(sim.pc, 4))

    if is_nil(raw) do
      raise RuntimeError, "PC (#{sim.pc}) is past end of program"
    end

    decoded = ArmDecoder.decode(raw, sim.pc)
    {registers, execution} = ArmExecutor.execute(decoded, sim.registers, sim.memory, sim.pc)

    next =
      %{
        sim
        | registers: registers,
          pc: execution.next_pc,
          halted: execution.halted,
          cycle: sim.cycle + 1
      }

    {next, %ArmTrace{pc: sim.pc, decoded: decoded, execution: execution}}
  end

  def run(%__MODULE__{} = sim, program) when is_list(program) do
    sim = load_program(sim, program)

    Enum.reduce_while(1..10_000, {sim, []}, fn _, {acc, traces} ->
      if acc.halted do
        {:halt, {acc, traces}}
      else
        {next, trace} = step(acc)
        {:cont, {next, traces ++ [trace]}}
      end
    end)
  end

  def encode_mov_imm(rd, imm) do
    (@cond_al <<< 28) ||| (0b00 <<< 26) ||| (1 <<< 25) ||| (@opcode_mov <<< 21) |||
      (0 <<< 20) ||| (0 <<< 16) ||| (rd <<< 12) ||| (0 <<< 8) ||| (imm &&& 0xFF)
  end

  def encode_add(rd, rn, rm) do
    (@cond_al <<< 28) ||| (0b00 <<< 26) ||| (0 <<< 25) ||| (@opcode_add <<< 21) |||
      (0 <<< 20) ||| (rn <<< 16) ||| (rd <<< 12) ||| rm
  end

  def encode_sub(rd, rn, rm) do
    (@cond_al <<< 28) ||| (0b00 <<< 26) ||| (0 <<< 25) ||| (@opcode_sub <<< 21) |||
      (0 <<< 20) ||| (rn <<< 16) ||| (rd <<< 12) ||| rm
  end

  def encode_hlt, do: @hlt_instruction

  def assemble(instructions) when is_list(instructions), do: instructions

  import Bitwise
end
