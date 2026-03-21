defmodule CodingAdventures.Core.Decoder do
  @moduledoc """
  ISADecoder -- the behaviour (interface) between the Core and any
  instruction set architecture.

  ## Why a Behaviour?

  The Core knows how to move instructions through a pipeline, but it does
  NOT know what any instruction means. That is the ISA decoder's job.

  This separation mirrors real CPU design:
    - ARM defines the decoder semantics (what ADD, LDR, BEQ mean)
    - Apple/Qualcomm build the pipeline and caches
    - The decoder plugs into the pipeline via a well-defined interface

  Any ISA (ARM, RISC-V, x86, or a custom teaching ISA) can implement this
  behaviour and immediately run on any Core configuration.

  ## The Three Callbacks

    1. `decode/2` -- turn raw instruction bits into a structured PipelineToken
    2. `execute/2` -- perform the actual computation (ALU operation, branch)
    3. `instruction_size/0` -- how many bytes per instruction (4 for ARM/RISC-V)
  """

  alias CodingAdventures.CpuPipeline.Token
  alias CodingAdventures.Core.RegisterFile

  @doc "Decode raw instruction bits into a structured PipelineToken."
  @callback decode(raw_instruction :: integer(), token :: Token.t()) :: Token.t()

  @doc "Execute the ALU operation for a decoded instruction."
  @callback execute(token :: Token.t(), reg_file :: RegisterFile.t()) :: Token.t()

  @doc "Return the size of one instruction in bytes."
  @callback instruction_size() :: pos_integer()
end

# =========================================================================
# MockDecoder -- a simple decoder for testing the Core
# =========================================================================

defmodule CodingAdventures.Core.MockDecoder do
  @moduledoc """
  A minimal ISA decoder for testing purposes.

  Supports a handful of instructions encoded in a simple format:

      Bits 31-24: opcode (0=NOP, 1=ADD, 2=LOAD, 3=STORE, 4=BRANCH, 5=HALT,
                           6=ADDI, 7=SUB)
      Bits 23-20: Rd  (destination register)
      Bits 19-16: Rs1 (first source register)
      Bits 15-12: Rs2 (second source register)
      Bits 11-0:  immediate (12-bit, sign-extended)

  This encoding does not match any real ISA. It exists solely to exercise
  the Core's pipeline.

  ## Instruction Reference

      NOP    (0x00): Do nothing.
      ADD    (0x01): Rd = Rs1 + Rs2
      LOAD   (0x02): Rd = Memory[Rs1 + imm]
      STORE  (0x03): Memory[Rs1 + imm] = Rs2
      BRANCH (0x04): If Rs1 == Rs2, PC = PC + imm*4
      HALT   (0x05): Stop execution.
      ADDI   (0x06): Rd = Rs1 + imm
      SUB    (0x07): Rd = Rs1 - Rs2
  """

  @behaviour CodingAdventures.Core.Decoder

  import Bitwise

  alias CodingAdventures.CpuPipeline.Token
  alias CodingAdventures.Core.RegisterFile

  @impl true
  def instruction_size, do: 4

  @impl true
  def decode(raw, %Token{} = token) do
    opcode = (raw >>> 24) &&& 0xFF
    rd = (raw >>> 20) &&& 0x0F
    rs1 = (raw >>> 16) &&& 0x0F
    rs2 = (raw >>> 12) &&& 0x0F
    imm = raw &&& 0xFFF

    # Sign-extend the 12-bit immediate.
    imm = if (imm &&& 0x800) != 0, do: imm ||| ~~~0xFFF, else: imm

    case opcode do
      0x00 -> # NOP
        %{token | opcode: "NOP", rd: -1, rs1: -1, rs2: -1}

      0x01 -> # ADD Rd, Rs1, Rs2
        %{token | opcode: "ADD", rd: rd, rs1: rs1, rs2: rs2, reg_write: true}

      0x02 -> # LOAD Rd, [Rs1 + imm]
        %{token | opcode: "LOAD", rd: rd, rs1: rs1, rs2: -1, immediate: imm,
          reg_write: true, mem_read: true}

      0x03 -> # STORE [Rs1 + imm], Rs2
        %{token | opcode: "STORE", rd: -1, rs1: rs1, rs2: rs2, immediate: imm,
          mem_write: true}

      0x04 -> # BRANCH Rs1, Rs2, imm
        %{token | opcode: "BRANCH", rd: -1, rs1: rs1, rs2: rs2, immediate: imm,
          is_branch: true}

      0x05 -> # HALT
        %{token | opcode: "HALT", rd: -1, rs1: -1, rs2: -1, is_halt: true}

      0x06 -> # ADDI Rd, Rs1, imm
        %{token | opcode: "ADDI", rd: rd, rs1: rs1, rs2: -1, immediate: imm,
          reg_write: true}

      0x07 -> # SUB Rd, Rs1, Rs2
        %{token | opcode: "SUB", rd: rd, rs1: rs1, rs2: rs2, reg_write: true}

      _ -> # Unknown -> NOP
        %{token | opcode: "NOP", rd: -1, rs1: -1, rs2: -1}
    end
  end

  @impl true
  def execute(%Token{} = token, %RegisterFile{} = reg_file) do
    rs1_val = if token.rs1 >= 0, do: RegisterFile.read(reg_file, token.rs1), else: 0
    rs2_val = if token.rs2 >= 0, do: RegisterFile.read(reg_file, token.rs2), else: 0

    case token.opcode do
      "ADD" ->
        result = rs1_val + rs2_val
        %{token | alu_result: result, write_data: result}

      "SUB" ->
        result = rs1_val - rs2_val
        %{token | alu_result: result, write_data: result}

      "ADDI" ->
        result = rs1_val + token.immediate
        %{token | alu_result: result, write_data: result}

      "LOAD" ->
        # Effective address = Rs1 + immediate.
        # Actual memory read happens in the MEM stage (handled by Core).
        %{token | alu_result: rs1_val + token.immediate}

      "STORE" ->
        # Effective address = Rs1 + immediate; data from Rs2.
        %{token | alu_result: rs1_val + token.immediate, write_data: rs2_val}

      "BRANCH" ->
        taken = rs1_val == rs2_val
        target = token.pc + token.immediate * 4

        if taken do
          %{token | branch_taken: true, branch_target: target, alu_result: target}
        else
          %{token | branch_taken: false, branch_target: target, alu_result: token.pc + 4}
        end

      _ ->
        token
    end
  end

  # =========================================================================
  # Instruction Encoding Helpers
  # =========================================================================

  @doc "Encode a NOP instruction."
  def encode_nop, do: 0x00 <<< 24

  @doc "Encode ADD Rd, Rs1, Rs2."
  def encode_add(rd, rs1, rs2) do
    (0x01 <<< 24) ||| (rd <<< 20) ||| (rs1 <<< 16) ||| (rs2 <<< 12)
  end

  @doc "Encode SUB Rd, Rs1, Rs2."
  def encode_sub(rd, rs1, rs2) do
    (0x07 <<< 24) ||| (rd <<< 20) ||| (rs1 <<< 16) ||| (rs2 <<< 12)
  end

  @doc "Encode ADDI Rd, Rs1, imm."
  def encode_addi(rd, rs1, imm) do
    (0x06 <<< 24) ||| (rd <<< 20) ||| (rs1 <<< 16) ||| (imm &&& 0xFFF)
  end

  @doc "Encode LOAD Rd, [Rs1 + imm]."
  def encode_load(rd, rs1, imm) do
    (0x02 <<< 24) ||| (rd <<< 20) ||| (rs1 <<< 16) ||| (imm &&& 0xFFF)
  end

  @doc "Encode STORE [Rs1 + imm], Rs2."
  def encode_store(rs1, rs2, imm) do
    (0x03 <<< 24) ||| (rs1 <<< 16) ||| (rs2 <<< 12) ||| (imm &&& 0xFFF)
  end

  @doc "Encode BRANCH Rs1, Rs2, imm."
  def encode_branch(rs1, rs2, imm) do
    (0x04 <<< 24) ||| (rs1 <<< 16) ||| (rs2 <<< 12) ||| (imm &&& 0xFFF)
  end

  @doc "Encode a HALT instruction."
  def encode_halt, do: 0x05 <<< 24

  @doc """
  Converts a sequence of raw instruction ints into a byte list
  suitable for `MemoryController.load_program/3`.

  Each instruction is encoded as 4 bytes in little-endian order.
  """
  def encode_program(instructions) do
    Enum.flat_map(instructions, fn instr ->
      [
        Bitwise.band(instr, 0xFF),
        Bitwise.band(Bitwise.bsr(instr, 8), 0xFF),
        Bitwise.band(Bitwise.bsr(instr, 16), 0xFF),
        Bitwise.band(Bitwise.bsr(instr, 24), 0xFF)
      ]
    end)
  end
end
