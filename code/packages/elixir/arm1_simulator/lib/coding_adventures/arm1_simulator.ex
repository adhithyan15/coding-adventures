# ==========================================================================
# ARM1 Behavioral Simulator — Elixir Port
# ==========================================================================
#
# The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
# in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
# correctly on the very first attempt. This module is a cycle-accurate
# behavioral simulator of the complete ARMv1 instruction set.
#
# The ARM1 had just 25,000 transistors and a 26-bit address space (64 MiB).
# Its accidental low power consumption (~0.1W) later made the ARM architecture
# dominant in mobile computing, with over 250 billion chips shipped.
#
# # Functional Design
#
# This Elixir port uses immutable data structures throughout:
#   - CPU state is a %CPU{} struct
#   - step(cpu) returns {new_cpu, trace}
#   - run(cpu, max_steps) returns {final_cpu, [traces]}
#   - Memory is an Elixir binary (immutable byte sequence)
#   - Registers are stored as a tuple of 27 unsigned 32-bit integers
#
# All values are plain integers. We use `Bitwise` for bit manipulation
# and mask with 0xFFFFFFFF to simulate 32-bit arithmetic.

defmodule CodingAdventures.Arm1Simulator do
  @moduledoc """
  ARM1 (ARMv1) behavioral instruction set simulator.

  Implements the complete ARMv1 ISA: 16 data processing operations, load/store,
  block transfer, branch, SWI, conditional execution, barrel shifter, and 4
  processor modes with banked registers.

  ## Usage

      cpu = CodingAdventures.Arm1Simulator.new(1024)
      cpu = CodingAdventures.Arm1Simulator.load_program(cpu, machine_code, 0)
      {cpu, traces} = CodingAdventures.Arm1Simulator.run(cpu, 10000)
  """

  import Bitwise

  # =========================================================================
  # Constants — Processor Modes
  # =========================================================================
  #
  # The ARM1 supports 4 processor modes. Each mode has its own banked copies
  # of certain registers, allowing fast context switching.
  #
  #   Mode  M1:M0  Banked Registers
  #   ----  -----  ----------------
  #   USR   0b00   (none - base set)
  #   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
  #   IRQ   0b10   R13_irq, R14_irq
  #   SVC   0b11   R13_svc, R14_svc

  @mode_usr 0
  @mode_fiq 1
  @mode_irq 2
  @mode_svc 3

  def mode_usr, do: @mode_usr
  def mode_fiq, do: @mode_fiq
  def mode_irq, do: @mode_irq
  def mode_svc, do: @mode_svc

  @doc "Returns human-readable name for a processor mode."
  def mode_string(@mode_usr), do: "USR"
  def mode_string(@mode_fiq), do: "FIQ"
  def mode_string(@mode_irq), do: "IRQ"
  def mode_string(@mode_svc), do: "SVC"
  def mode_string(_), do: "???"

  # =========================================================================
  # Constants — Condition Codes
  # =========================================================================
  #
  # Every ARM instruction has a 4-bit condition code in bits 31:28.
  # The instruction only executes if the condition is met.

  @cond_eq 0x0
  @cond_ne 0x1
  @cond_cs 0x2
  @cond_cc 0x3
  @cond_mi 0x4
  @cond_pl 0x5
  @cond_vs 0x6
  @cond_vc 0x7
  @cond_hi 0x8
  @cond_ls 0x9
  @cond_ge 0xA
  @cond_lt 0xB
  @cond_gt 0xC
  @cond_le 0xD
  @cond_al 0xE
  @cond_nv 0xF

  def cond_eq, do: @cond_eq
  def cond_ne, do: @cond_ne
  def cond_cs, do: @cond_cs
  def cond_cc, do: @cond_cc
  def cond_mi, do: @cond_mi
  def cond_pl, do: @cond_pl
  def cond_vs, do: @cond_vs
  def cond_vc, do: @cond_vc
  def cond_hi, do: @cond_hi
  def cond_ls, do: @cond_ls
  def cond_ge, do: @cond_ge
  def cond_lt, do: @cond_lt
  def cond_gt, do: @cond_gt
  def cond_le, do: @cond_le
  def cond_al, do: @cond_al
  def cond_nv, do: @cond_nv

  @doc "Returns the assembly-language suffix for a condition code."
  def cond_string(@cond_eq), do: "EQ"
  def cond_string(@cond_ne), do: "NE"
  def cond_string(@cond_cs), do: "CS"
  def cond_string(@cond_cc), do: "CC"
  def cond_string(@cond_mi), do: "MI"
  def cond_string(@cond_pl), do: "PL"
  def cond_string(@cond_vs), do: "VS"
  def cond_string(@cond_vc), do: "VC"
  def cond_string(@cond_hi), do: "HI"
  def cond_string(@cond_ls), do: "LS"
  def cond_string(@cond_ge), do: "GE"
  def cond_string(@cond_lt), do: "LT"
  def cond_string(@cond_gt), do: "GT"
  def cond_string(@cond_le), do: "LE"
  def cond_string(@cond_al), do: ""
  def cond_string(@cond_nv), do: "NV"
  def cond_string(_), do: "??"

  # =========================================================================
  # Constants — ALU Opcodes
  # =========================================================================
  #
  # The ARM1's ALU supports 16 operations, selected by bits 24:21 of a data
  # processing instruction.

  @op_and 0x0
  @op_eor 0x1
  @op_sub 0x2
  @op_rsb 0x3
  @op_add 0x4
  @op_adc 0x5
  @op_sbc 0x6
  @op_rsc 0x7
  @op_tst 0x8
  @op_teq 0x9
  @op_cmp 0xA
  @op_cmn 0xB
  @op_orr 0xC
  @op_mov 0xD
  @op_bic 0xE
  @op_mvn 0xF

  def op_and, do: @op_and
  def op_eor, do: @op_eor
  def op_sub, do: @op_sub
  def op_rsb, do: @op_rsb
  def op_add, do: @op_add
  def op_adc, do: @op_adc
  def op_sbc, do: @op_sbc
  def op_rsc, do: @op_rsc
  def op_tst, do: @op_tst
  def op_teq, do: @op_teq
  def op_cmp, do: @op_cmp
  def op_cmn, do: @op_cmn
  def op_orr, do: @op_orr
  def op_mov, do: @op_mov
  def op_bic, do: @op_bic
  def op_mvn, do: @op_mvn

  @op_names {
    "AND", "EOR", "SUB", "RSB", "ADD", "ADC", "SBC", "RSC",
    "TST", "TEQ", "CMP", "CMN", "ORR", "MOV", "BIC", "MVN"
  }

  @doc "Returns the mnemonic for an ALU opcode."
  def op_string(opcode) when opcode >= 0 and opcode < 16, do: elem(@op_names, opcode)
  def op_string(_), do: "???"

  @doc "Returns true if the ALU opcode is test-only (TST, TEQ, CMP, CMN)."
  def test_op?(opcode), do: opcode >= @op_tst and opcode <= @op_cmn

  @doc "Returns true if the ALU opcode is a logical operation."
  def logical_op?(opcode) do
    opcode in [@op_and, @op_eor, @op_tst, @op_teq, @op_orr, @op_mov, @op_bic, @op_mvn]
  end

  # =========================================================================
  # Constants — Shift Types
  # =========================================================================

  @shift_lsl 0
  @shift_lsr 1
  @shift_asr 2
  @shift_ror 3

  def shift_lsl, do: @shift_lsl
  def shift_lsr, do: @shift_lsr
  def shift_asr, do: @shift_asr
  def shift_ror, do: @shift_ror

  @shift_names {"LSL", "LSR", "ASR", "ROR"}

  def shift_string(shift_type) when shift_type >= 0 and shift_type < 4 do
    elem(@shift_names, shift_type)
  end
  def shift_string(_), do: "???"

  # =========================================================================
  # Constants — R15 bit positions
  # =========================================================================
  #
  # R15 is the combined PC + status register:
  #   Bit 31: N (Negative)     Bit 27: I (IRQ disable)
  #   Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
  #   Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
  #   Bit 28: V (Overflow)     Bits 1:0: Processor Mode

  @flag_n 1 <<< 31
  @flag_z 1 <<< 30
  @flag_c 1 <<< 29
  @flag_v 1 <<< 28
  @flag_i 1 <<< 27
  @flag_f 1 <<< 26
  @pc_mask 0x03FFFFFC
  @mode_mask 0x3
  @halt_swi 0x123456
  @mask32 0xFFFFFFFF

  def flag_n, do: @flag_n
  def flag_z, do: @flag_z
  def flag_c, do: @flag_c
  def flag_v, do: @flag_v
  def flag_i, do: @flag_i
  def flag_f, do: @flag_f
  def pc_mask, do: @pc_mask
  def mode_mask, do: @mode_mask
  def halt_swi, do: @halt_swi

  # =========================================================================
  # Constants — Instruction types
  # =========================================================================

  @inst_data_processing 0
  @inst_load_store 1
  @inst_block_transfer 2
  @inst_branch 3
  @inst_swi 4
  @inst_coprocessor 5
  @inst_undefined 6

  def inst_data_processing, do: @inst_data_processing
  def inst_load_store, do: @inst_load_store
  def inst_block_transfer, do: @inst_block_transfer
  def inst_branch, do: @inst_branch
  def inst_swi, do: @inst_swi
  def inst_coprocessor, do: @inst_coprocessor
  def inst_undefined, do: @inst_undefined

  # =========================================================================
  # Structs
  # =========================================================================

  defmodule Flags do
    @moduledoc "ARM1 condition flags: N (Negative), Z (Zero), C (Carry), V (Overflow)."
    defstruct n: false, z: false, c: false, v: false
  end

  defmodule MemoryAccess do
    @moduledoc "Records a single memory read or write."
    defstruct [:address, :value]
  end

  defmodule Trace do
    @moduledoc """
    Records the state change caused by executing one instruction.
    Captures the complete before/after snapshot for debugging and
    cross-language validation.
    """
    defstruct [
      :address,
      :raw,
      :mnemonic,
      :condition,
      :condition_met,
      :regs_before,
      :regs_after,
      :flags_before,
      :flags_after,
      memory_reads: [],
      memory_writes: []
    ]
  end

  defmodule DecodedInstruction do
    @moduledoc "All fields extracted from a 32-bit ARM instruction."
    defstruct [
      raw: 0,
      type: 0,
      condition: 0,
      # Data processing
      opcode: 0,
      s: false,
      rn: 0,
      rd: 0,
      immediate: false,
      # Immediate form
      imm8: 0,
      rotate: 0,
      # Register form
      rm: 0,
      shift_type: 0,
      shift_by_reg: false,
      shift_imm: 0,
      rs: 0,
      # Load/Store
      load: false,
      byte: false,
      pre_index: false,
      up: false,
      write_back: false,
      offset12: 0,
      # Block Transfer
      register_list: 0,
      force_user: false,
      # Branch
      link: false,
      branch_offset: 0,
      # SWI
      swi_comment: 0
    ]
  end

  defmodule ALUResult do
    @moduledoc "Output of an ALU operation."
    defstruct [:result, :n, :z, :c, :v, :write_result]
  end

  defmodule CPU do
    @moduledoc """
    ARM1 CPU state. Immutable struct — each step returns a new CPU.

    Fields:
      - regs: tuple of 27 uint32 values (R0-R15 + banked registers)
      - memory: binary (byte-addressable, little-endian)
      - halted: boolean
    """
    defstruct [
      regs: nil,
      memory: nil,
      halted: false
    ]
  end

  # =========================================================================
  # CPU — Construction and Reset
  # =========================================================================

  @doc """
  Creates a new ARM1 simulator with the given memory size in bytes.

  The ARM1 has a 26-bit address space (64 MiB). The default memory size
  is 1 MiB, which is more than enough for most test programs.

  On power-on, the ARM1 enters Supervisor mode with IRQs and FIQs disabled,
  and begins executing from address 0x00000000 (the Reset vector).
  """
  def new(memory_size \\ 1024 * 1024) do
    memory_size = if memory_size <= 0, do: 1024 * 1024, else: memory_size
    reset(%CPU{
      regs: List.to_tuple(List.duplicate(0, 27)),
      memory: :binary.copy(<<0>>, memory_size),
      halted: false
    })
  end

  @doc """
  Resets the CPU to power-on state: SVC mode, IRQ/FIQ disabled, PC = 0.
  """
  def reset(%CPU{} = cpu) do
    regs = List.to_tuple(List.duplicate(0, 27))
    # Set R15: SVC mode (bits 1:0 = 11), IRQ/FIQ disabled (bits 27,26 = 11)
    regs = put_elem(regs, 15, @flag_i ||| @flag_f ||| @mode_svc)
    %{cpu | regs: regs, halted: false}
  end

  # =========================================================================
  # Register Access
  # =========================================================================

  @doc "Reads a register (R0-R15), respecting mode banking."
  def read_register(%CPU{regs: regs} = cpu, index) do
    elem(regs, physical_reg(cpu, index))
  end

  @doc "Writes a register (R0-R15), respecting mode banking."
  def write_register(%CPU{regs: regs} = cpu, index, value) do
    %{cpu | regs: put_elem(regs, physical_reg(cpu, index), value &&& @mask32)}
  end

  # Maps a logical register index (0-15) to a physical register index (0-26)
  # based on the current processor mode.
  defp physical_reg(%CPU{} = cpu, index) do
    mode_val = mode(cpu)
    do_physical_reg(mode_val, index)
  end

  defp do_physical_reg(@mode_fiq, index) when index >= 8 and index <= 14, do: 16 + (index - 8)
  defp do_physical_reg(@mode_irq, index) when index >= 13 and index <= 14, do: 23 + (index - 13)
  defp do_physical_reg(@mode_svc, index) when index >= 13 and index <= 14, do: 25 + (index - 13)
  defp do_physical_reg(_, index), do: index

  @doc "Returns the current program counter (26-bit address)."
  def pc(%CPU{regs: regs}), do: elem(regs, 15) &&& @pc_mask

  @doc "Sets the program counter portion of R15 without changing flags/mode."
  def set_pc(%CPU{regs: regs} = cpu, addr) do
    r15 = elem(regs, 15)
    new_r15 = (r15 &&& bnot(@pc_mask) &&& @mask32) ||| (addr &&& @pc_mask)
    %{cpu | regs: put_elem(regs, 15, new_r15 &&& @mask32)}
  end

  @doc "Returns the current condition flags as a Flags struct."
  def flags(%CPU{regs: regs}) do
    r15 = elem(regs, 15)
    %Flags{
      n: (r15 &&& @flag_n) != 0,
      z: (r15 &&& @flag_z) != 0,
      c: (r15 &&& @flag_c) != 0,
      v: (r15 &&& @flag_v) != 0
    }
  end

  @doc "Updates the condition flags in R15."
  def set_flags(%CPU{regs: regs} = cpu, %Flags{} = f) do
    r15 = elem(regs, 15) &&& (bnot(@flag_n ||| @flag_z ||| @flag_c ||| @flag_v) &&& @mask32)
    r15 = if f.n, do: r15 ||| @flag_n, else: r15
    r15 = if f.z, do: r15 ||| @flag_z, else: r15
    r15 = if f.c, do: r15 ||| @flag_c, else: r15
    r15 = if f.v, do: r15 ||| @flag_v, else: r15
    %{cpu | regs: put_elem(regs, 15, r15 &&& @mask32)}
  end

  @doc "Returns the current processor mode."
  def mode(%CPU{regs: regs}), do: elem(regs, 15) &&& @mode_mask

  @doc "Returns true if the CPU has been halted."
  def halted?(%CPU{halted: halted}), do: halted

  # =========================================================================
  # Memory Access
  # =========================================================================

  @doc "Reads a 32-bit word from memory (little-endian, word-aligned)."
  def read_word(%CPU{memory: mem}, addr) do
    addr = addr &&& @pc_mask
    a = addr &&& bnot(3) &&& @mask32
    mem_size = byte_size(mem)
    if a + 3 >= mem_size do
      0
    else
      <<_::binary-size(a), b0, b1, b2, b3, _::binary>> = mem
      (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& @mask32
    end
  end

  @doc "Writes a 32-bit word to memory (little-endian, word-aligned)."
  def write_word(%CPU{memory: mem} = cpu, addr, value) do
    addr = addr &&& @pc_mask
    a = addr &&& bnot(3) &&& @mask32
    mem_size = byte_size(mem)
    if a + 3 >= mem_size do
      cpu
    else
      value = value &&& @mask32
      <<before::binary-size(a), _::binary-size(4), rest::binary>> = mem
      new_mem = before <>
        <<value &&& 0xFF, (value >>> 8) &&& 0xFF,
          (value >>> 16) &&& 0xFF, (value >>> 24) &&& 0xFF>> <>
        rest
      %{cpu | memory: new_mem}
    end
  end

  @doc "Reads a single byte from memory."
  def read_byte(%CPU{memory: mem}, addr) do
    addr = addr &&& @pc_mask
    if addr >= byte_size(mem), do: 0, else: :binary.at(mem, addr)
  end

  @doc "Writes a single byte to memory."
  def write_byte(%CPU{memory: mem} = cpu, addr, value) do
    addr = addr &&& @pc_mask
    if addr >= byte_size(mem) do
      cpu
    else
      <<before::binary-size(addr), _::8, rest::binary>> = mem
      %{cpu | memory: before <> <<value &&& 0xFF>> <> rest}
    end
  end

  @doc "Loads machine code (binary) into memory at the given start address."
  def load_program(%CPU{memory: mem} = cpu, code, start_addr) when is_binary(code) do
    code_size = byte_size(code)
    mem_size = byte_size(mem)
    # Truncate if code exceeds memory
    actual_size = min(code_size, mem_size - start_addr)
    if actual_size <= 0 do
      cpu
    else
      code_to_copy = binary_part(code, 0, actual_size)
      <<before::binary-size(start_addr), _::binary-size(actual_size), rest::binary>> = mem
      %{cpu | memory: before <> code_to_copy <> rest}
    end
  end

  @doc "Loads a list of 32-bit instruction words into memory."
  def load_instructions(%CPU{} = cpu, instructions, start_addr \\ 0) when is_list(instructions) do
    code =
      instructions
      |> Enum.map(fn inst ->
        <<(inst &&& 0xFF), ((inst >>> 8) &&& 0xFF),
          ((inst >>> 16) &&& 0xFF), ((inst >>> 24) &&& 0xFF)>>
      end)
      |> IO.iodata_to_binary()
    load_program(cpu, code, start_addr)
  end

  # =========================================================================
  # Condition Evaluation
  # =========================================================================
  #
  # Tests whether the given condition code is satisfied by the current flags.
  # This is the behavioral equivalent of the ARM1's condition evaluation
  # hardware.
  #
  #   Code  Suffix  Test
  #   ----  ------  ----
  #   0000  EQ      Z == 1
  #   0001  NE      Z == 0
  #   0010  CS      C == 1
  #   0011  CC      C == 0
  #   0100  MI      N == 1
  #   0101  PL      N == 0
  #   0110  VS      V == 1
  #   0111  VC      V == 0
  #   1000  HI      C == 1 AND Z == 0
  #   1001  LS      C == 0 OR  Z == 1
  #   1010  GE      N == V
  #   1011  LT      N != V
  #   1100  GT      Z == 0 AND N == V
  #   1101  LE      Z == 1 OR  N != V
  #   1110  AL      true
  #   1111  NV      false

  @doc "Evaluates whether the given condition code is satisfied by the flags."
  def evaluate_condition(@cond_eq, %Flags{z: z}), do: z
  def evaluate_condition(@cond_ne, %Flags{z: z}), do: not z
  def evaluate_condition(@cond_cs, %Flags{c: c}), do: c
  def evaluate_condition(@cond_cc, %Flags{c: c}), do: not c
  def evaluate_condition(@cond_mi, %Flags{n: n}), do: n
  def evaluate_condition(@cond_pl, %Flags{n: n}), do: not n
  def evaluate_condition(@cond_vs, %Flags{v: v}), do: v
  def evaluate_condition(@cond_vc, %Flags{v: v}), do: not v
  def evaluate_condition(@cond_hi, %Flags{c: c, z: z}), do: c and not z
  def evaluate_condition(@cond_ls, %Flags{c: c, z: z}), do: not c or z
  def evaluate_condition(@cond_ge, %Flags{n: n, v: v}), do: n == v
  def evaluate_condition(@cond_lt, %Flags{n: n, v: v}), do: n != v
  def evaluate_condition(@cond_gt, %Flags{n: n, z: z, v: v}), do: not z and n == v
  def evaluate_condition(@cond_le, %Flags{n: n, z: z, v: v}), do: z or n != v
  def evaluate_condition(@cond_al, _flags), do: true
  def evaluate_condition(@cond_nv, _flags), do: false
  def evaluate_condition(_, _flags), do: false

  # =========================================================================
  # Barrel Shifter
  # =========================================================================
  #
  # The barrel shifter is the ARM1's most distinctive hardware feature. On
  # the real chip, it was a 32x32 crossbar network of pass transistors.
  # Every data processing instruction has a "second operand" that passes
  # through the barrel shifter before reaching the ALU — shifts are free.
  #
  # Returns {result, carry_out}.

  @doc """
  Applies a shift operation to a 32-bit value.

  Parameters:
    - value: the 32-bit input
    - shift_type: 0=LSL, 1=LSR, 2=ASR, 3=ROR
    - amount: number of positions to shift
    - carry_in: current carry flag (boolean)
    - by_register: true if shift amount comes from a register

  Returns {shifted_value, carry_out}.
  """
  def barrel_shift(value, _shift_type, 0, carry_in, true), do: {value, carry_in}

  def barrel_shift(value, @shift_lsl, amount, carry_in, _by_register) do
    shift_lsl(value, amount, carry_in)
  end
  def barrel_shift(value, @shift_lsr, amount, carry_in, by_register) do
    shift_lsr(value, amount, carry_in, by_register)
  end
  def barrel_shift(value, @shift_asr, amount, carry_in, by_register) do
    shift_asr(value, amount, carry_in, by_register)
  end
  def barrel_shift(value, @shift_ror, amount, carry_in, by_register) do
    shift_ror(value, amount, carry_in, by_register)
  end
  def barrel_shift(value, _, _, carry_in, _), do: {value, carry_in}

  # LSL: Logical Shift Left
  defp shift_lsl(value, 0, carry_in), do: {value, carry_in}
  defp shift_lsl(value, amount, _carry_in) when amount >= 32 do
    if amount == 32 do
      {0, (value &&& 1) != 0}
    else
      {0, false}
    end
  end
  defp shift_lsl(value, amount, _carry_in) do
    carry = ((value >>> (32 - amount)) &&& 1) != 0
    result = (value <<< amount) &&& @mask32
    {result, carry}
  end

  # LSR: Logical Shift Right
  # Special case: immediate LSR #0 encodes LSR #32
  defp shift_lsr(value, 0, _carry_in, false), do: {0, (value >>> 31) != 0}
  defp shift_lsr(value, 0, carry_in, true), do: {value, carry_in}
  defp shift_lsr(value, amount, _carry_in, _by_register) when amount >= 32 do
    if amount == 32, do: {0, (value >>> 31) != 0}, else: {0, false}
  end
  defp shift_lsr(value, amount, _carry_in, _by_register) do
    carry = ((value >>> (amount - 1)) &&& 1) != 0
    {value >>> amount, carry}
  end

  # ASR: Arithmetic Shift Right (sign-extending)
  # Special case: immediate ASR #0 encodes ASR #32
  defp shift_asr(value, 0, _carry_in, false) do
    if (value >>> 31) != 0, do: {@mask32, true}, else: {0, false}
  end
  defp shift_asr(value, 0, carry_in, true), do: {value, carry_in}
  defp shift_asr(value, amount, _carry_in, _by_register) when amount >= 32 do
    if (value >>> 31) != 0, do: {@mask32, true}, else: {0, false}
  end
  defp shift_asr(value, amount, _carry_in, _by_register) do
    # Sign-extend: if bit 31 is set, fill upper bits with 1s
    carry = ((value >>> (amount - 1)) &&& 1) != 0
    if (value >>> 31) != 0 do
      # Arithmetic right shift: fill upper bits with 1s
      result = (value >>> amount) ||| ((@mask32 <<< (32 - amount)) &&& @mask32)
      {result &&& @mask32, carry}
    else
      {value >>> amount, carry}
    end
  end

  # ROR: Rotate Right
  # Special case: immediate ROR #0 encodes RRX (33-bit rotate through carry)
  defp shift_ror(value, 0, carry_in, false) do
    carry = (value &&& 1) != 0
    result = value >>> 1
    result = if carry_in, do: result ||| 0x80000000, else: result
    {result &&& @mask32, carry}
  end
  defp shift_ror(value, 0, carry_in, true), do: {value, carry_in}
  defp shift_ror(value, amount, _carry_in, _by_register) do
    amount = amount &&& 31
    if amount == 0 do
      {value, (value >>> 31) != 0}
    else
      result = ((value >>> amount) ||| (value <<< (32 - amount))) &&& @mask32
      carry = ((result >>> 31) &&& 1) != 0
      {result, carry}
    end
  end

  @doc """
  Decodes a rotated immediate value from the Operand2 field (I bit = 1).
  Returns {value, carry_out}.
  """
  def decode_immediate(imm8, 0), do: {imm8, false}
  def decode_immediate(imm8, rotate_field) do
    rotate_amount = rotate_field * 2
    value = ((imm8 >>> rotate_amount) ||| (imm8 <<< (32 - rotate_amount))) &&& @mask32
    carry = (value >>> 31) != 0
    {value, carry}
  end

  # =========================================================================
  # ALU
  # =========================================================================
  #
  # The ARM1's ALU performs 16 operations. Flag computation differs for
  # logical vs arithmetic ops:
  #
  # Arithmetic: C = adder carry out, V = signed overflow
  # Logical:    C = barrel shifter carry out, V = unchanged

  @doc """
  Performs one of the 16 ALU operations.

  Returns an %ALUResult{} with the computed result and flags.
  """
  def alu_execute(opcode, a, b, carry_in, shifter_carry, old_v) do
    write_result = not test_op?(opcode)
    {result, carry, overflow} = do_alu(opcode, a, b, carry_in, shifter_carry, old_v)
    result = result &&& @mask32

    %ALUResult{
      result: result,
      n: (result >>> 31) != 0,
      z: result == 0,
      c: carry,
      v: overflow,
      write_result: write_result
    }
  end

  # Logical operations: C from barrel shifter, V preserved
  defp do_alu(op, a, b, _carry_in, shifter_carry, old_v)
       when op in [@op_and, @op_tst] do
    {(a &&& b) &&& @mask32, shifter_carry, old_v}
  end

  defp do_alu(op, a, b, _carry_in, shifter_carry, old_v)
       when op in [@op_eor, @op_teq] do
    {bxor(a, b) &&& @mask32, shifter_carry, old_v}
  end

  defp do_alu(@op_orr, a, b, _carry_in, shifter_carry, old_v) do
    {(a ||| b) &&& @mask32, shifter_carry, old_v}
  end

  defp do_alu(@op_mov, _a, b, _carry_in, shifter_carry, old_v) do
    {b &&& @mask32, shifter_carry, old_v}
  end

  defp do_alu(@op_bic, a, b, _carry_in, shifter_carry, old_v) do
    {(a &&& (bnot(b) &&& @mask32)) &&& @mask32, shifter_carry, old_v}
  end

  defp do_alu(@op_mvn, _a, b, _carry_in, shifter_carry, old_v) do
    {bnot(b) &&& @mask32, shifter_carry, old_v}
  end

  # Arithmetic operations: C from adder, V from overflow detection
  defp do_alu(op, a, b, _carry_in, _shifter_carry, _old_v)
       when op in [@op_add, @op_cmn] do
    add32(a, b, false)
  end

  defp do_alu(@op_adc, a, b, carry_in, _shifter_carry, _old_v) do
    add32(a, b, carry_in)
  end

  defp do_alu(op, a, b, _carry_in, _shifter_carry, _old_v)
       when op in [@op_sub, @op_cmp] do
    add32(a, bnot(b) &&& @mask32, true)
  end

  defp do_alu(@op_sbc, a, b, carry_in, _shifter_carry, _old_v) do
    add32(a, bnot(b) &&& @mask32, carry_in)
  end

  defp do_alu(@op_rsb, a, b, _carry_in, _shifter_carry, _old_v) do
    add32(b, bnot(a) &&& @mask32, true)
  end

  defp do_alu(@op_rsc, a, b, carry_in, _shifter_carry, _old_v) do
    add32(b, bnot(a) &&& @mask32, carry_in)
  end

  # 32-bit addition with carry, computing carry-out and overflow.
  # Uses 64-bit arithmetic for clarity.
  defp add32(a, b, carry_in) do
    cin = if carry_in, do: 1, else: 0
    sum = a + b + cin
    result = sum &&& @mask32
    carry = (sum >>> 32) != 0
    # Overflow: both operands have same sign but result differs
    overflow = ((bxor(a, result) &&& bxor(b, result)) >>> 31) != 0
    {result, carry, overflow}
  end

  # =========================================================================
  # Decoder
  # =========================================================================
  #
  # Extracts all fields from a 32-bit ARM instruction. The real ARM1
  # decoder was a PLA with just 42 rows of 36-bit microinstructions.

  @doc "Decodes a 32-bit ARM instruction into a DecodedInstruction."
  def decode(instruction) do
    d = %DecodedInstruction{
      raw: instruction,
      condition: (instruction >>> 28) &&& 0xF
    }

    bits2726 = (instruction >>> 26) &&& 0x3
    bit25 = (instruction >>> 25) &&& 0x1

    case {bits2726, bit25} do
      {0, _} ->
        %{d | type: @inst_data_processing}
        |> decode_data_processing(instruction)

      {1, _} ->
        %{d | type: @inst_load_store}
        |> decode_load_store(instruction)

      {2, 0} ->
        %{d | type: @inst_block_transfer}
        |> decode_block_transfer(instruction)

      {2, 1} ->
        %{d | type: @inst_branch}
        |> decode_branch(instruction)

      {3, _} ->
        if ((instruction >>> 24) &&& 0xF) == 0xF do
          %{d | type: @inst_swi, swi_comment: instruction &&& 0x00FFFFFF}
        else
          %{d | type: @inst_coprocessor}
        end

      _ ->
        %{d | type: @inst_undefined}
    end
  end

  defp decode_data_processing(d, inst) do
    is_immediate = ((inst >>> 25) &&& 1) == 1
    d = %{d |
      immediate: is_immediate,
      opcode: (inst >>> 21) &&& 0xF,
      s: ((inst >>> 20) &&& 1) == 1,
      rn: (inst >>> 16) &&& 0xF,
      rd: (inst >>> 12) &&& 0xF
    }

    if is_immediate do
      %{d | imm8: inst &&& 0xFF, rotate: (inst >>> 8) &&& 0xF}
    else
      shift_by_reg = ((inst >>> 4) &&& 1) == 1
      d = %{d |
        rm: inst &&& 0xF,
        shift_type: (inst >>> 5) &&& 0x3,
        shift_by_reg: shift_by_reg
      }
      if shift_by_reg do
        %{d | rs: (inst >>> 8) &&& 0xF}
      else
        %{d | shift_imm: (inst >>> 7) &&& 0x1F}
      end
    end
  end

  defp decode_load_store(d, inst) do
    %{d |
      immediate: ((inst >>> 25) &&& 1) == 1,
      pre_index: ((inst >>> 24) &&& 1) == 1,
      up: ((inst >>> 23) &&& 1) == 1,
      byte: ((inst >>> 22) &&& 1) == 1,
      write_back: ((inst >>> 21) &&& 1) == 1,
      load: ((inst >>> 20) &&& 1) == 1,
      rn: (inst >>> 16) &&& 0xF,
      rd: (inst >>> 12) &&& 0xF,
      rm: inst &&& 0xF,
      shift_type: (inst >>> 5) &&& 0x3,
      shift_imm: (inst >>> 7) &&& 0x1F,
      offset12: inst &&& 0xFFF
    }
  end

  defp decode_block_transfer(d, inst) do
    %{d |
      pre_index: ((inst >>> 24) &&& 1) == 1,
      up: ((inst >>> 23) &&& 1) == 1,
      force_user: ((inst >>> 22) &&& 1) == 1,
      write_back: ((inst >>> 21) &&& 1) == 1,
      load: ((inst >>> 20) &&& 1) == 1,
      rn: (inst >>> 16) &&& 0xF,
      register_list: inst &&& 0xFFFF
    }
  end

  defp decode_branch(d, inst) do
    is_link = ((inst >>> 24) &&& 1) == 1
    offset = inst &&& 0x00FFFFFF
    # Sign-extend from 24 bits to 32 bits
    offset = if (offset >>> 23) != 0, do: offset ||| 0xFF000000, else: offset
    # Convert to signed and shift left by 2
    # Elixir integers are arbitrary precision, so we need to handle sign manually
    signed_offset = if offset >= 0x80000000, do: offset - 0x100000000, else: offset
    branch_offset = signed_offset * 4

    %{d | link: is_link, branch_offset: branch_offset}
  end

  # =========================================================================
  # Disassembly
  # =========================================================================

  @doc "Returns a human-readable assembly string for a decoded instruction."
  def disassemble(%DecodedInstruction{} = d) do
    condition = cond_string(d.condition)

    case d.type do
      @inst_data_processing -> disasm_data_processing(d, condition)
      @inst_load_store -> disasm_load_store(d, condition)
      @inst_block_transfer -> disasm_block_transfer(d, condition)
      @inst_branch -> disasm_branch(d, condition)
      @inst_swi ->
        if d.swi_comment == @halt_swi do
          "HLT#{condition}"
        else
          "SWI#{condition} #0x#{Integer.to_string(d.swi_comment, 16)}"
        end
      @inst_coprocessor -> "CDP#{condition} (undefined)"
      _ -> "UND#{condition} #0x#{String.pad_leading(Integer.to_string(d.raw, 16), 8, "0")}"
    end
  end

  defp disasm_data_processing(d, condition) do
    op = op_string(d.opcode)
    suf = if d.s and not test_op?(d.opcode), do: "S", else: ""
    op2 = disasm_operand2(d)

    cond do
      d.opcode in [@op_mov, @op_mvn] ->
        "#{op}#{condition}#{suf} R#{d.rd}, #{op2}"
      test_op?(d.opcode) ->
        "#{op}#{condition} R#{d.rn}, #{op2}"
      true ->
        "#{op}#{condition}#{suf} R#{d.rd}, R#{d.rn}, #{op2}"
    end
  end

  defp disasm_operand2(d) do
    if d.immediate do
      {val, _} = decode_immediate(d.imm8, d.rotate)
      "##{val}"
    else
      if not d.shift_by_reg and d.shift_imm == 0 and d.shift_type == @shift_lsl do
        "R#{d.rm}"
      else
        if d.shift_by_reg do
          "R#{d.rm}, #{shift_string(d.shift_type)} R#{d.rs}"
        else
          amount = d.shift_imm
          amount = case d.shift_type do
            t when t in [@shift_lsr, @shift_asr] and amount == 0 -> 32
            @shift_ror when amount == 0 -> :rrx
            _ -> amount
          end
          if amount == :rrx do
            "R#{d.rm}, RRX"
          else
            "R#{d.rm}, #{shift_string(d.shift_type)} ##{amount}"
          end
        end
      end
    end
  end

  defp disasm_load_store(d, condition) do
    op = if d.load, do: "LDR", else: "STR"
    b_suf = if d.byte, do: "B", else: ""

    offset = if d.immediate do
      rm_str = "R#{d.rm}"
      if d.shift_imm != 0 do
        "#{rm_str}, #{shift_string(d.shift_type)} ##{d.shift_imm}"
      else
        rm_str
      end
    else
      "##{d.offset12}"
    end

    sign = if d.up, do: "", else: "-"

    if d.pre_index do
      wb = if d.write_back, do: "!", else: ""
      "#{op}#{condition}#{b_suf} R#{d.rd}, [R#{d.rn}, #{sign}#{offset}]#{wb}"
    else
      "#{op}#{condition}#{b_suf} R#{d.rd}, [R#{d.rn}], #{sign}#{offset}"
    end
  end

  defp disasm_block_transfer(d, condition) do
    op = if d.load, do: "LDM", else: "STM"
    bt_mode = case {d.pre_index, d.up} do
      {false, true} -> "IA"
      {true, true} -> "IB"
      {false, false} -> "DA"
      {true, false} -> "DB"
    end
    wb = if d.write_back, do: "!", else: ""
    regs = disasm_reg_list(d.register_list)
    "#{op}#{condition}#{bt_mode} R#{d.rn}#{wb}, {#{regs}}"
  end

  defp disasm_branch(d, condition) do
    op = if d.link, do: "BL", else: "B"
    "#{op}#{condition} ##{d.branch_offset}"
  end

  defp disasm_reg_list(list) do
    0..15
    |> Enum.filter(fn i -> ((list >>> i) &&& 1) == 1 end)
    |> Enum.map(fn
      15 -> "PC"
      14 -> "LR"
      13 -> "SP"
      i -> "R#{i}"
    end)
    |> Enum.join(", ")
  end

  # =========================================================================
  # Execution — Step
  # =========================================================================

  @doc """
  Executes one instruction and returns {new_cpu, trace}.

  This is the core fetch-decode-execute cycle:
    1. FETCH: read 32-bit instruction at PC
    2. DECODE: extract fields
    3. CHECK: evaluate condition code
    4. EXECUTE: if condition met, perform operation
    5. ADVANCE: PC += 4 (unless branch or PC write)
  """
  def step(%CPU{} = cpu) do
    current_pc = pc(cpu)
    regs_before = capture_regs(cpu)
    flags_before = flags(cpu)

    # Fetch
    instruction = read_word(cpu, current_pc)

    # Decode
    decoded = decode(instruction)

    # Evaluate condition
    cond_met = evaluate_condition(decoded.condition, flags_before)

    trace = %Trace{
      address: current_pc,
      raw: instruction,
      mnemonic: disassemble(decoded),
      condition: cond_string(decoded.condition),
      condition_met: cond_met,
      regs_before: regs_before,
      flags_before: flags_before
    }

    # Advance PC
    cpu = set_pc(cpu, (current_pc + 4) &&& @pc_mask)

    # Execute if condition met
    {cpu, trace} = if cond_met do
      case decoded.type do
        @inst_data_processing -> execute_data_processing(cpu, decoded, trace)
        @inst_load_store -> execute_load_store(cpu, decoded, trace)
        @inst_block_transfer -> execute_block_transfer(cpu, decoded, trace)
        @inst_branch -> execute_branch(cpu, decoded, trace)
        @inst_swi -> execute_swi(cpu, decoded, trace)
        @inst_coprocessor -> {trap_undefined(cpu, current_pc), trace}
        @inst_undefined -> {trap_undefined(cpu, current_pc), trace}
        _ -> {cpu, trace}
      end
    else
      {cpu, trace}
    end

    # Capture state after
    trace = %{trace |
      regs_after: capture_regs(cpu),
      flags_after: flags(cpu)
    }

    {cpu, trace}
  end

  defp capture_regs(cpu) do
    for i <- 0..15, into: %{}, do: {i, read_register(cpu, i)}
  end

  # Reads register value as seen during execution (R15 = PC + 8 due to pipeline)
  defp read_reg_for_exec(cpu, 15) do
    # We already advanced PC by 4 in step(), so add 4 more for pipeline effect
    (elem(cpu.regs, 15) + 4) &&& @mask32
  end
  defp read_reg_for_exec(cpu, index), do: read_register(cpu, index)

  # =========================================================================
  # Data Processing Execution
  # =========================================================================

  defp execute_data_processing(cpu, d, trace) do
    # Get first operand (Rn)
    a = if d.opcode not in [@op_mov, @op_mvn] do
      read_reg_for_exec(cpu, d.rn)
    else
      0
    end

    # Get second operand through barrel shifter
    current_flags = flags(cpu)

    {b, shifter_carry} = if d.immediate do
      {val, sc} = decode_immediate(d.imm8, d.rotate)
      # Carry unchanged when no rotation
      sc = if d.rotate == 0, do: current_flags.c, else: sc
      {val, sc}
    else
      rm_val = read_reg_for_exec(cpu, d.rm)
      shift_amount = if d.shift_by_reg do
        read_reg_for_exec(cpu, d.rs) &&& 0xFF
      else
        d.shift_imm
      end
      barrel_shift(rm_val, d.shift_type, shift_amount, current_flags.c, d.shift_by_reg)
    end

    # Execute ALU
    alu_result = alu_execute(d.opcode, a, b, current_flags.c, shifter_carry, current_flags.v)

    # Write result to Rd (unless test-only)
    cpu = if alu_result.write_result do
      if d.rd == 15 do
        if d.s do
          # MOVS PC, LR: restore entire R15
          %{cpu | regs: put_elem(cpu.regs, 15, alu_result.result &&& @mask32)}
        else
          set_pc(cpu, alu_result.result &&& @pc_mask)
        end
      else
        write_register(cpu, d.rd, alu_result.result)
      end
    else
      cpu
    end

    # Update flags if S bit set (and Rd is not R15)
    cpu = if d.s and d.rd != 15 do
      set_flags(cpu, %Flags{n: alu_result.n, z: alu_result.z, c: alu_result.c, v: alu_result.v})
    else
      cpu
    end

    # Test ops always update flags
    cpu = if test_op?(d.opcode) do
      set_flags(cpu, %Flags{n: alu_result.n, z: alu_result.z, c: alu_result.c, v: alu_result.v})
    else
      cpu
    end

    {cpu, trace}
  end

  # =========================================================================
  # Load/Store Execution
  # =========================================================================

  defp execute_load_store(cpu, d, trace) do
    # Compute offset
    offset = if d.immediate do
      rm_val = read_reg_for_exec(cpu, d.rm)
      if d.shift_imm != 0 do
        {shifted, _} = barrel_shift(rm_val, d.shift_type, d.shift_imm, flags(cpu).c, false)
        shifted
      else
        rm_val
      end
    else
      d.offset12
    end

    # Base address
    base = read_reg_for_exec(cpu, d.rn)

    # Compute effective address
    addr = if d.up, do: (base + offset) &&& @mask32, else: (base - offset) &&& @mask32

    # Pre/post-indexed
    transfer_addr = if d.pre_index, do: addr, else: base

    {cpu, trace} = if d.load do
      # LDR / LDRB
      {value, cpu_read} = if d.byte do
        {read_byte(cpu, transfer_addr), cpu}
      else
        word = read_word(cpu, transfer_addr)
        # ARM1 quirk: unaligned word loads rotate the data
        rotation = (transfer_addr &&& 3) * 8
        word = if rotation != 0 do
          ((word >>> rotation) ||| (word <<< (32 - rotation))) &&& @mask32
        else
          word
        end
        {word, cpu}
      end

      trace = %{trace | memory_reads: trace.memory_reads ++ [%MemoryAccess{address: transfer_addr, value: value}]}

      cpu = if d.rd == 15 do
        %{cpu_read | regs: put_elem(cpu_read.regs, 15, value &&& @mask32)}
      else
        write_register(cpu_read, d.rd, value)
      end

      {cpu, trace}
    else
      # STR / STRB
      value = read_reg_for_exec(cpu, d.rd)
      cpu = if d.byte do
        write_byte(cpu, transfer_addr, value &&& 0xFF)
      else
        write_word(cpu, transfer_addr, value)
      end
      trace = %{trace | memory_writes: trace.memory_writes ++ [%MemoryAccess{address: transfer_addr, value: value}]}
      {cpu, trace}
    end

    # Write-back
    cpu = if d.write_back or not d.pre_index do
      if d.rn != 15, do: write_register(cpu, d.rn, addr), else: cpu
    else
      cpu
    end

    {cpu, trace}
  end

  # =========================================================================
  # Block Transfer Execution (LDM/STM)
  # =========================================================================

  defp execute_block_transfer(cpu, d, trace) do
    base = read_register(cpu, d.rn)
    reg_list = d.register_list

    # Count registers
    count = Enum.count(0..15, fn i -> ((reg_list >>> i) &&& 1) == 1 end)

    if count == 0 do
      {cpu, trace}
    else
      # Calculate start address
      start_addr = case {d.pre_index, d.up} do
        {false, true} -> base                                    # IA
        {true, true} -> (base + 4) &&& @mask32                  # IB
        {false, false} -> (base - count * 4 + 4) &&& @mask32    # DA
        {true, false} -> (base - count * 4) &&& @mask32         # DB
      end

      # Process each register in the list
      {cpu, trace, _addr} =
        Enum.reduce(0..15, {cpu, trace, start_addr}, fn i, {cpu_acc, trace_acc, addr_acc} ->
          if ((reg_list >>> i) &&& 1) == 0 do
            {cpu_acc, trace_acc, addr_acc}
          else
            if d.load do
              value = read_word(cpu_acc, addr_acc)
              trace_acc = %{trace_acc | memory_reads: trace_acc.memory_reads ++ [%MemoryAccess{address: addr_acc, value: value}]}
              cpu_acc = if i == 15 do
                %{cpu_acc | regs: put_elem(cpu_acc.regs, 15, value &&& @mask32)}
              else
                write_register(cpu_acc, i, value)
              end
              {cpu_acc, trace_acc, (addr_acc + 4) &&& @mask32}
            else
              value = if i == 15 do
                (elem(cpu_acc.regs, 15) + 4) &&& @mask32
              else
                read_register(cpu_acc, i)
              end
              cpu_acc = write_word(cpu_acc, addr_acc, value)
              trace_acc = %{trace_acc | memory_writes: trace_acc.memory_writes ++ [%MemoryAccess{address: addr_acc, value: value}]}
              {cpu_acc, trace_acc, (addr_acc + 4) &&& @mask32}
            end
          end
        end)

      # Write-back
      cpu = if d.write_back do
        new_base = if d.up do
          (base + count * 4) &&& @mask32
        else
          (base - count * 4) &&& @mask32
        end
        write_register(cpu, d.rn, new_base)
      else
        cpu
      end

      {cpu, trace}
    end
  end

  # =========================================================================
  # Branch Execution
  # =========================================================================

  defp execute_branch(cpu, d, trace) do
    # PC already advanced by 4 in step(). Branch is relative to PC+8.
    branch_base = (pc(cpu) + 4) &&& @mask32

    cpu = if d.link do
      # BL: save return address (full R15 with flags/mode) in R14
      return_addr = elem(cpu.regs, 15)
      write_register(cpu, 14, return_addr)
    else
      cpu
    end

    # Compute target
    # branch_offset is already a signed integer (can be negative)
    target = (branch_base + d.branch_offset) &&& @mask32
    cpu = set_pc(cpu, target &&& @pc_mask)

    {cpu, trace}
  end

  # =========================================================================
  # SWI Execution
  # =========================================================================

  defp execute_swi(cpu, d, trace) do
    if d.swi_comment == @halt_swi do
      {%{cpu | halted: true}, trace}
    else
      # Real SWI: enter Supervisor mode
      r15_val = elem(cpu.regs, 15)
      regs = cpu.regs
      regs = put_elem(regs, 25, r15_val)  # R13_svc
      regs = put_elem(regs, 26, r15_val)  # R14_svc
      cpu = %{cpu | regs: regs}

      # Set SVC mode, disable IRQs
      r15 = elem(cpu.regs, 15)
      r15 = (r15 &&& (bnot(@mode_mask) &&& @mask32)) ||| @mode_svc
      r15 = r15 ||| @flag_i
      cpu = %{cpu | regs: put_elem(cpu.regs, 15, r15 &&& @mask32)}
      cpu = set_pc(cpu, 0x08)

      {cpu, trace}
    end
  end

  # =========================================================================
  # Exception Handling
  # =========================================================================

  defp trap_undefined(cpu, _instr_addr) do
    r15_val = elem(cpu.regs, 15)
    regs = put_elem(cpu.regs, 26, r15_val)
    cpu = %{cpu | regs: regs}

    r15 = elem(cpu.regs, 15)
    r15 = (r15 &&& (bnot(@mode_mask) &&& @mask32)) ||| @mode_svc
    r15 = r15 ||| @flag_i
    cpu = %{cpu | regs: put_elem(cpu.regs, 15, r15 &&& @mask32)}
    set_pc(cpu, 0x04)
  end

  # =========================================================================
  # Run
  # =========================================================================

  @doc """
  Executes instructions until halted or max_steps reached.
  Returns {final_cpu, [traces]}.
  """
  def run(%CPU{} = cpu, max_steps) do
    do_run(cpu, max_steps, [])
  end

  defp do_run(%CPU{halted: true} = cpu, _remaining, traces) do
    {cpu, Enum.reverse(traces)}
  end
  defp do_run(cpu, 0, traces) do
    {cpu, Enum.reverse(traces)}
  end
  defp do_run(cpu, remaining, traces) do
    {new_cpu, trace} = step(cpu)
    do_run(new_cpu, remaining - 1, [trace | traces])
  end

  # =========================================================================
  # Encoding Helpers
  # =========================================================================
  #
  # These functions create instruction words for test programs, eliminating
  # the need for a full assembler.

  @doc "Creates a data processing instruction word."
  def encode_data_processing(condition, opcode, s, rn, rd, operand2) do
    ((condition <<< 28) ||| operand2 |||
     (opcode <<< 21) ||| (s <<< 20) |||
     (rn <<< 16) ||| (rd <<< 12)) &&& @mask32
  end

  @doc "Creates a MOV immediate instruction. Example: encode_mov_imm(cond_al, 0, 42) -> MOV R0, #42"
  def encode_mov_imm(condition, rd, imm8) do
    encode_data_processing(condition, @op_mov, 0, 0, rd, (1 <<< 25) ||| imm8)
  end

  @doc "Creates a data processing instruction with a register operand."
  def encode_alu_reg(condition, opcode, s, rd, rn, rm) do
    encode_data_processing(condition, opcode, s, rn, rd, rm)
  end

  @doc "Creates a Branch or Branch-with-Link instruction."
  def encode_branch(condition, link, offset) do
    inst = (condition <<< 28) ||| 0x0A000000
    inst = if link, do: inst ||| 0x01000000, else: inst
    # offset is in bytes. We encode (offset / 4) in 24 bits.
    encoded = div(offset, 4) &&& 0x00FFFFFF
    (inst ||| encoded) &&& @mask32
  end

  @doc "Creates our pseudo-halt instruction (SWI 0x123456)."
  def encode_halt do
    ((@cond_al <<< 28) ||| 0x0F000000 ||| @halt_swi) &&& @mask32
  end

  @doc "Creates a Load Register instruction with immediate offset."
  def encode_ldr(condition, rd, rn, offset, pre_index) do
    inst = (condition <<< 28) ||| 0x04100000  # bits 27:26=01, L=1, I=0
    inst = inst ||| (rd <<< 12) ||| (rn <<< 16)
    inst = if pre_index, do: inst ||| (1 <<< 24), else: inst
    inst = if offset >= 0 do
      inst ||| (1 <<< 23) ||| (offset &&& 0xFFF)
    else
      inst ||| ((-offset) &&& 0xFFF)
    end
    inst &&& @mask32
  end

  @doc "Creates a Store Register instruction with immediate offset."
  def encode_str(condition, rd, rn, offset, pre_index) do
    inst = (condition <<< 28) ||| 0x04000000  # bits 27:26=01, L=0, I=0
    inst = inst ||| (rd <<< 12) ||| (rn <<< 16)
    inst = if pre_index, do: inst ||| (1 <<< 24), else: inst
    inst = if offset >= 0 do
      inst ||| (1 <<< 23) ||| (offset &&& 0xFFF)
    else
      inst ||| ((-offset) &&& 0xFFF)
    end
    inst &&& @mask32
  end

  @doc "Creates a Load Multiple instruction."
  def encode_ldm(condition, rn, reg_list, write_back, bt_mode) do
    inst = (condition <<< 28) ||| 0x08100000  # bits 27:25=100, L=1
    inst = inst ||| (rn <<< 16) ||| reg_list
    inst = if write_back, do: inst ||| (1 <<< 21), else: inst
    inst = case bt_mode do
      "IA" -> inst ||| (1 <<< 23)
      "IB" -> inst ||| (1 <<< 24) ||| (1 <<< 23)
      "DA" -> inst
      "DB" -> inst ||| (1 <<< 24)
      _ -> inst
    end
    inst &&& @mask32
  end

  @doc "Creates a Store Multiple instruction."
  def encode_stm(condition, rn, reg_list, write_back, bt_mode) do
    inst = encode_ldm(condition, rn, reg_list, write_back, bt_mode)
    (inst &&& bnot(1 <<< 20)) &&& @mask32  # Clear L bit
  end
end
