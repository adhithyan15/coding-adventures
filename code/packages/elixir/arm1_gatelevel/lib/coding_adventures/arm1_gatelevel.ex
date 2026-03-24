# ==========================================================================
# ARM1 Gate-Level Simulator — Elixir Port
# ==========================================================================
#
# Every arithmetic operation routes through actual logic gate functions —
# AND, OR, XOR, NOT — chained into adders, then into a 32-bit ALU. The
# barrel shifter is built from multiplexer trees. Registers are stored as
# bit arrays (lists of 0s and 1s).
#
# This is NOT the same as the behavioral simulator. Both produce identical
# results for any program. The difference is the execution path:
#
#   Behavioral:  opcode -> pattern match -> host arithmetic -> result
#   Gate-level:  opcode -> decoder -> barrel shifter muxes -> ALU gates
#                -> adder gates -> logic gates -> result
#
# The gate-level simulator composes packages from layers below:
#   - logic_gates: AND, OR, XOR, NOT, MUX
#   - arithmetic: half_adder, full_adder, ripple_carry_adder
#   - arm1_simulator: types, condition codes, instruction encoding helpers

defmodule CodingAdventures.Arm1Gatelevel do
  @moduledoc """
  Gate-level ARM1 processor simulator.

  Routes every data-path operation through gate-level primitives from the
  logic_gates and arithmetic packages. Memory is not gate-level (that would
  require millions of flip-flops).

  ## Usage

      cpu = CodingAdventures.Arm1Gatelevel.new(4096)
      cpu = CodingAdventures.Arm1Gatelevel.load_instructions(cpu, instructions)
      {cpu, traces} = CodingAdventures.Arm1Gatelevel.run(cpu, 100)
  """

  import Bitwise

  alias CodingAdventures.Arm1Simulator, as: Sim
  alias CodingAdventures.Arm1Simulator.{Flags, Trace, MemoryAccess}
  alias CodingAdventures.LogicGates.Gates
  alias CodingAdventures.LogicGates.Combinational
  alias CodingAdventures.Arithmetic

  @mask32 0xFFFFFFFF

  # =========================================================================
  # CPU State
  # =========================================================================

  defmodule CPU do
    @moduledoc """
    Gate-level ARM1 CPU state. Registers are stored as 27 lists of 32 bits
    (LSB-first), matching how real hardware flip-flops store state.
    """
    defstruct [
      regs: nil,     # tuple of 27 bit-lists (each 32 elements of 0/1)
      memory: nil,   # binary (not gate-level)
      halted: false,
      gate_ops: 0    # cumulative gate operation count
    ]
  end

  # =========================================================================
  # Bit Conversion Helpers
  # =========================================================================
  #
  # These bridge between the integer world (test programs, external API)
  # and the gate-level world (lists of 0s and 1s flowing through gates).
  #
  #   int_to_bits(5, 32) -> [1, 0, 1, 0, 0, 0, ..., 0]  (32 elements, LSB first)
  #   bits_to_int(...)   -> 5

  @doc "Converts a uint32 to a list of 32 bits (LSB first)."
  def int_to_bits(value, width \\ 32) do
    for i <- 0..(width - 1), do: (value >>> i) &&& 1
  end

  @doc "Converts a list of bits (LSB first) to a uint32."
  def bits_to_int(bits) do
    bits
    |> Enum.with_index()
    |> Enum.reduce(0, fn {bit, i}, acc ->
      if i < 32, do: acc ||| (bit <<< i), else: acc
    end)
  end

  # =========================================================================
  # Construction and Reset
  # =========================================================================

  @doc "Creates a new gate-level ARM1 simulator."
  def new(memory_size \\ 1024 * 1024) do
    memory_size = if memory_size <= 0, do: 1024 * 1024, else: memory_size
    empty_regs = List.to_tuple(for _ <- 0..26, do: List.duplicate(0, 32))
    cpu = %CPU{
      regs: empty_regs,
      memory: :binary.copy(<<0>>, memory_size),
      halted: false,
      gate_ops: 0
    }
    reset(cpu)
  end

  @doc "Resets the CPU to power-on state."
  def reset(%CPU{} = cpu) do
    empty_regs = List.to_tuple(for _ <- 0..26, do: List.duplicate(0, 32))
    r15_val = Sim.flag_i() ||| Sim.flag_f() ||| Sim.mode_svc()
    r15_bits = int_to_bits(r15_val, 32)
    regs = put_elem(empty_regs, 15, r15_bits)
    %{cpu | regs: regs, halted: false, gate_ops: 0}
  end

  # =========================================================================
  # Register Access (gate-level)
  # =========================================================================

  defp read_reg(%CPU{regs: regs} = cpu, index) do
    phys = physical_reg(cpu, index)
    bits_to_int(elem(regs, phys))
  end

  defp write_reg(%CPU{regs: regs} = cpu, index, value) do
    phys = physical_reg(cpu, index)
    bits = int_to_bits(value &&& @mask32, 32)
    %{cpu | regs: put_elem(regs, phys, bits)}
  end

  defp read_reg_bits(%CPU{regs: regs} = cpu, index) do
    phys = physical_reg(cpu, index)
    elem(regs, phys)
  end

  defp physical_reg(%CPU{regs: regs}, index) do
    mode_val = bits_to_int(elem(regs, 15)) &&& Sim.mode_mask()
    do_physical_reg(mode_val, index)
  end

  defp do_physical_reg(1, index) when index >= 8 and index <= 14, do: 16 + (index - 8)
  defp do_physical_reg(2, index) when index >= 13 and index <= 14, do: 23 + (index - 13)
  defp do_physical_reg(3, index) when index >= 13 and index <= 14, do: 25 + (index - 13)
  defp do_physical_reg(_, index), do: index

  @doc "Returns the current PC."
  def pc(%CPU{regs: regs}) do
    bits_to_int(elem(regs, 15)) &&& Sim.pc_mask()
  end

  @doc "Sets the PC portion of R15."
  def set_pc(%CPU{regs: regs} = cpu, addr) do
    r15 = bits_to_int(elem(regs, 15))
    r15 = (r15 &&& (bnot(Sim.pc_mask()) &&& @mask32)) ||| (addr &&& Sim.pc_mask())
    bits = int_to_bits(r15 &&& @mask32, 32)
    %{cpu | regs: put_elem(regs, 15, bits)}
  end

  @doc "Returns the current condition flags."
  def flags(%CPU{regs: regs}) do
    r15_bits = elem(regs, 15)
    %Flags{
      n: Enum.at(r15_bits, 31) == 1,
      z: Enum.at(r15_bits, 30) == 1,
      c: Enum.at(r15_bits, 29) == 1,
      v: Enum.at(r15_bits, 28) == 1
    }
  end

  defp set_flags_bits(%CPU{regs: regs} = cpu, n, z, c, v) do
    r15_bits = elem(regs, 15)
    r15_bits = List.replace_at(r15_bits, 31, n)
    r15_bits = List.replace_at(r15_bits, 30, z)
    r15_bits = List.replace_at(r15_bits, 29, c)
    r15_bits = List.replace_at(r15_bits, 28, v)
    %{cpu | regs: put_elem(regs, 15, r15_bits)}
  end

  @doc "Returns the current processor mode."
  def mode(%CPU{regs: regs}) do
    bits_to_int(elem(regs, 15)) &&& Sim.mode_mask()
  end

  @doc "Returns true if the CPU has been halted."
  def halted?(%CPU{halted: h}), do: h

  @doc "Returns the total number of gate operations performed."
  def gate_ops(%CPU{gate_ops: g}), do: g

  # =========================================================================
  # Memory (same as behavioral — not gate-level)
  # =========================================================================

  def read_word(%CPU{memory: mem}, addr) do
    addr = addr &&& Sim.pc_mask()
    a = addr &&& (bnot(3) &&& @mask32)
    if a + 3 >= byte_size(mem) do
      0
    else
      <<_::binary-size(a), b0, b1, b2, b3, _::binary>> = mem
      (b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)) &&& @mask32
    end
  end

  def write_word(%CPU{memory: mem} = cpu, addr, value) do
    addr = addr &&& Sim.pc_mask()
    a = addr &&& (bnot(3) &&& @mask32)
    if a + 3 >= byte_size(mem) do
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

  def read_byte(%CPU{memory: mem}, addr) do
    addr = addr &&& Sim.pc_mask()
    if addr >= byte_size(mem), do: 0, else: :binary.at(mem, addr)
  end

  def write_byte(%CPU{memory: mem} = cpu, addr, value) do
    addr = addr &&& Sim.pc_mask()
    if addr >= byte_size(mem) do
      cpu
    else
      <<before::binary-size(addr), _::8, rest::binary>> = mem
      %{cpu | memory: before <> <<value &&& 0xFF>> <> rest}
    end
  end

  def load_program(%CPU{memory: mem} = cpu, code, start_addr) when is_binary(code) do
    code_size = byte_size(code)
    mem_size = byte_size(mem)
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
  def load_instructions(%CPU{} = cpu, instructions, start_addr \\ 0) do
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
  # Gate-Level Condition Evaluation
  # =========================================================================

  defp evaluate_condition(cpu, condition, current_flags) do
    n = if current_flags.n, do: 1, else: 0
    z = if current_flags.z, do: 1, else: 0
    c = if current_flags.c, do: 1, else: 0
    v = if current_flags.v, do: 1, else: 0

    cpu = %{cpu | gate_ops: cpu.gate_ops + 4}

    result = case condition do
      0x0 -> z == 1                                              # EQ
      0x1 -> Gates.not_gate(z) == 1                              # NE
      0x2 -> c == 1                                              # CS
      0x3 -> Gates.not_gate(c) == 1                              # CC
      0x4 -> n == 1                                              # MI
      0x5 -> Gates.not_gate(n) == 1                              # PL
      0x6 -> v == 1                                              # VS
      0x7 -> Gates.not_gate(v) == 1                              # VC
      0x8 -> Gates.and_gate(c, Gates.not_gate(z)) == 1           # HI
      0x9 -> Gates.or_gate(Gates.not_gate(c), z) == 1            # LS
      0xA -> Gates.xnor_gate(n, v) == 1                          # GE
      0xB -> Gates.xor_gate(n, v) == 1                           # LT
      0xC -> Gates.and_gate(Gates.not_gate(z), Gates.xnor_gate(n, v)) == 1  # GT
      0xD -> Gates.or_gate(z, Gates.xor_gate(n, v)) == 1         # LE
      0xE -> true                                                 # AL
      0xF -> false                                                # NV
      _ -> false
    end

    {cpu, result}
  end

  # =========================================================================
  # Gate-Level ALU
  # =========================================================================
  #
  # Every operation routes through actual gate function calls:
  #   - Arithmetic: ripple_carry_adder (32 full adders -> 160+ gate calls)
  #   - Logical: AND/OR/XOR/NOT applied to each of 32 bits

  defmodule GateALUResult do
    @moduledoc "Output of a gate-level ALU operation."
    defstruct [:result_bits, :n, :z, :c, :v]
  end

  @doc "Performs one of the 16 ALU operations using gate-level logic."
  def gate_alu_execute(opcode, a_bits, b_bits, carry_in, shifter_carry, old_v) do
    {result_bits, carry, overflow} = case opcode do
      # Logical operations
      op when op in [0x0, 0x8] ->  # AND, TST
        {bitwise_gate(a_bits, b_bits, &Gates.and_gate/2), shifter_carry, old_v}

      op when op in [0x1, 0x9] ->  # EOR, TEQ
        {bitwise_gate(a_bits, b_bits, &Gates.xor_gate/2), shifter_carry, old_v}

      0xC ->  # ORR
        {bitwise_gate(a_bits, b_bits, &Gates.or_gate/2), shifter_carry, old_v}

      0xD ->  # MOV
        {Enum.map(b_bits, & &1), shifter_carry, old_v}

      0xE ->  # BIC = AND(a, NOT(b))
        not_b = bitwise_not(b_bits)
        {bitwise_gate(a_bits, not_b, &Gates.and_gate/2), shifter_carry, old_v}

      0xF ->  # MVN = NOT(b)
        {bitwise_not(b_bits), shifter_carry, old_v}

      # Arithmetic operations — all through ripple-carry adder
      op when op in [0x4, 0xB] ->  # ADD, CMN
        {sum, cout} = Arithmetic.ripple_carry_adder(a_bits, b_bits, 0)
        {sum, cout, compute_overflow(a_bits, b_bits, sum)}

      0x5 ->  # ADC
        {sum, cout} = Arithmetic.ripple_carry_adder(a_bits, b_bits, carry_in)
        {sum, cout, compute_overflow(a_bits, b_bits, sum)}

      op when op in [0x2, 0xA] ->  # SUB, CMP: A + NOT(B) + 1
        not_b = bitwise_not(b_bits)
        {sum, cout} = Arithmetic.ripple_carry_adder(a_bits, not_b, 1)
        {sum, cout, compute_overflow(a_bits, not_b, sum)}

      0x6 ->  # SBC: A + NOT(B) + C
        not_b = bitwise_not(b_bits)
        {sum, cout} = Arithmetic.ripple_carry_adder(a_bits, not_b, carry_in)
        {sum, cout, compute_overflow(a_bits, not_b, sum)}

      0x3 ->  # RSB: B + NOT(A) + 1
        not_a = bitwise_not(a_bits)
        {sum, cout} = Arithmetic.ripple_carry_adder(b_bits, not_a, 1)
        {sum, cout, compute_overflow(b_bits, not_a, sum)}

      0x7 ->  # RSC: B + NOT(A) + C
        not_a = bitwise_not(a_bits)
        {sum, cout} = Arithmetic.ripple_carry_adder(b_bits, not_a, carry_in)
        {sum, cout, compute_overflow(b_bits, not_a, sum)}

      _ ->
        {List.duplicate(0, 32), 0, 0}
    end

    # N flag = MSB
    n = Enum.at(result_bits, 31)

    # Z flag = NOR of all bits (1 when all bits are 0)
    z = compute_zero(result_bits)

    %GateALUResult{result_bits: result_bits, n: n, z: z, c: carry, v: overflow}
  end

  # Apply a 2-input gate function to each bit pair (32 parallel gates)
  defp bitwise_gate(a_bits, b_bits, gate_fn) do
    Enum.zip_with(a_bits, b_bits, gate_fn)
  end

  # Apply NOT to each bit
  defp bitwise_not(bits) do
    Enum.map(bits, &Gates.not_gate/1)
  end

  # Zero flag: OR all bits, then NOT (NOR tree)
  defp compute_zero(bits) do
    combined = Enum.reduce(bits, 0, &Gates.or_gate/2)
    Gates.not_gate(combined)
  end

  # Overflow: (a[31] XOR result[31]) AND (b[31] XOR result[31])
  defp compute_overflow(a_bits, b_bits, result_bits) do
    xor1 = Gates.xor_gate(Enum.at(a_bits, 31), Enum.at(result_bits, 31))
    xor2 = Gates.xor_gate(Enum.at(b_bits, 31), Enum.at(result_bits, 31))
    Gates.and_gate(xor1, xor2)
  end

  # =========================================================================
  # Gate-Level Barrel Shifter
  # =========================================================================
  #
  # On the real ARM1, the barrel shifter was a 32x32 crossbar network of
  # pass transistors. We model it with a 5-level tree of Mux2 gates.
  # Each level uses 32 Mux2 gates = 5 x 32 = 160 Mux2 gates total.

  @doc "Performs a shift using a tree of multiplexer gates. Returns {result_bits, carry_out}."
  def gate_barrel_shift(value_bits, _shift_type, 0, carry_in, true) do
    {Enum.map(value_bits, & &1), carry_in}
  end
  def gate_barrel_shift(value_bits, 0, amount, carry_in, by_reg) do
    gate_lsl(value_bits, amount, carry_in, by_reg)
  end
  def gate_barrel_shift(value_bits, 1, amount, carry_in, by_reg) do
    gate_lsr(value_bits, amount, carry_in, by_reg)
  end
  def gate_barrel_shift(value_bits, 2, amount, carry_in, by_reg) do
    gate_asr(value_bits, amount, carry_in, by_reg)
  end
  def gate_barrel_shift(value_bits, 3, amount, carry_in, by_reg) do
    gate_ror(value_bits, amount, carry_in, by_reg)
  end
  def gate_barrel_shift(value_bits, _, _, carry_in, _) do
    {Enum.map(value_bits, & &1), carry_in}
  end

  # LSL using 5-level multiplexer tree
  defp gate_lsl(value_bits, 0, carry_in, _), do: {value_bits, carry_in}
  defp gate_lsl(value_bits, amount, _carry_in, _) when amount >= 32 do
    if amount == 32 do
      {List.duplicate(0, 32), Enum.at(value_bits, 0)}
    else
      {List.duplicate(0, 32), 0}
    end
  end
  defp gate_lsl(value_bits, amount, _carry_in, _) do
    current = Enum.reduce(0..4, value_bits, fn level, curr ->
      shift = 1 <<< level
      sel = (amount >>> level) &&& 1
      for i <- 0..31 do
        shifted = if i >= shift, do: Enum.at(curr, i - shift), else: 0
        Combinational.mux2(Enum.at(curr, i), shifted, sel)
      end
    end)
    carry = Enum.at(value_bits, 32 - amount)
    {current, carry}
  end

  # LSR
  defp gate_lsr(value_bits, 0, _carry_in, false) do
    {List.duplicate(0, 32), Enum.at(value_bits, 31)}
  end
  defp gate_lsr(value_bits, 0, carry_in, true), do: {value_bits, carry_in}
  defp gate_lsr(value_bits, amount, _carry_in, _) when amount >= 32 do
    if amount == 32 do
      {List.duplicate(0, 32), Enum.at(value_bits, 31)}
    else
      {List.duplicate(0, 32), 0}
    end
  end
  defp gate_lsr(value_bits, amount, _carry_in, _) do
    current = Enum.reduce(0..4, value_bits, fn level, curr ->
      shift = 1 <<< level
      sel = (amount >>> level) &&& 1
      for i <- 0..31 do
        shifted = if i + shift < 32, do: Enum.at(curr, i + shift), else: 0
        Combinational.mux2(Enum.at(curr, i), shifted, sel)
      end
    end)
    carry = Enum.at(value_bits, amount - 1)
    {current, carry}
  end

  # ASR (sign-extending)
  defp gate_asr(value_bits, 0, _carry_in, false) do
    sign_bit = Enum.at(value_bits, 31)
    {List.duplicate(sign_bit, 32), sign_bit}
  end
  defp gate_asr(value_bits, 0, carry_in, true), do: {value_bits, carry_in}
  defp gate_asr(value_bits, amount, _carry_in, _) when amount >= 32 do
    sign_bit = Enum.at(value_bits, 31)
    {List.duplicate(sign_bit, 32), sign_bit}
  end
  defp gate_asr(value_bits, amount, _carry_in, _) do
    sign_bit = Enum.at(value_bits, 31)
    current = Enum.reduce(0..4, value_bits, fn level, curr ->
      shift = 1 <<< level
      sel = (amount >>> level) &&& 1
      for i <- 0..31 do
        shifted = if i + shift < 32, do: Enum.at(curr, i + shift), else: sign_bit
        Combinational.mux2(Enum.at(curr, i), shifted, sel)
      end
    end)
    carry = Enum.at(value_bits, amount - 1)
    {current, carry}
  end

  # ROR
  defp gate_ror(value_bits, 0, carry_in, false) do
    # RRX: 33-bit rotate through carry
    result = for i <- 0..31 do
      if i == 31, do: carry_in, else: Enum.at(value_bits, i + 1)
    end
    carry = Enum.at(value_bits, 0)
    {result, carry}
  end
  defp gate_ror(value_bits, 0, carry_in, true), do: {value_bits, carry_in}
  defp gate_ror(value_bits, amount, _carry_in, _) do
    amount = amount &&& 31
    if amount == 0 do
      {value_bits, Enum.at(value_bits, 31)}
    else
      current = Enum.reduce(0..4, value_bits, fn level, curr ->
        shift = 1 <<< level
        sel = (amount >>> level) &&& 1
        for i <- 0..31 do
          shifted = Enum.at(curr, rem(i + shift, 32))
          Combinational.mux2(Enum.at(curr, i), shifted, sel)
        end
      end)
      {current, Enum.at(current, 31)}
    end
  end

  @doc "Decodes a rotated immediate using gate-level rotation."
  def gate_decode_immediate(imm8, rotate) do
    bits = int_to_bits(imm8, 32)
    rotate_amount = rotate * 2
    if rotate_amount == 0 do
      {bits, 0}
    else
      gate_ror(bits, rotate_amount, 0, false)
    end
  end

  # =========================================================================
  # Execution
  # =========================================================================

  @doc "Executes one instruction and returns {new_cpu, trace}."
  def step(%CPU{} = cpu) do
    current_pc = pc(cpu)
    regs_before = capture_regs(cpu)
    flags_before = flags(cpu)

    instruction = read_word(cpu, current_pc)
    decoded = Sim.decode(instruction)

    {cpu, cond_met} = evaluate_condition(cpu, decoded.condition, flags_before)

    trace = %Trace{
      address: current_pc,
      raw: instruction,
      mnemonic: Sim.disassemble(decoded),
      condition: Sim.cond_string(decoded.condition),
      condition_met: cond_met,
      regs_before: regs_before,
      flags_before: flags_before
    }

    cpu = set_pc(cpu, (current_pc + 4) &&& Sim.pc_mask())

    {cpu, trace} = if cond_met do
      case decoded.type do
        0 -> execute_data_processing(cpu, decoded, trace)
        1 -> execute_load_store(cpu, decoded, trace)
        2 -> execute_block_transfer(cpu, decoded, trace)
        3 -> execute_branch(cpu, decoded, trace)
        4 -> execute_swi(cpu, decoded, trace)
        _ -> {trap_undefined(cpu, current_pc), trace}
      end
    else
      {cpu, trace}
    end

    trace = %{trace |
      regs_after: capture_regs(cpu),
      flags_after: flags(cpu)
    }

    {cpu, trace}
  end

  @doc "Executes instructions until halted or max_steps reached."
  def run(%CPU{} = cpu, max_steps) do
    do_run(cpu, max_steps, [])
  end

  defp do_run(%CPU{halted: true} = cpu, _, traces), do: {cpu, Enum.reverse(traces)}
  defp do_run(cpu, 0, traces), do: {cpu, Enum.reverse(traces)}
  defp do_run(cpu, remaining, traces) do
    {new_cpu, trace} = step(cpu)
    do_run(new_cpu, remaining - 1, [trace | traces])
  end

  defp capture_regs(cpu) do
    for i <- 0..15, into: %{}, do: {i, read_reg(cpu, i)}
  end

  # Read register for execution (R15 = PC + 8 from pipeline)
  defp read_reg_for_exec(cpu, 15) do
    (bits_to_int(elem(cpu.regs, 15)) + 4) &&& @mask32
  end
  defp read_reg_for_exec(cpu, index), do: read_reg(cpu, index)

  defp read_reg_bits_for_exec(cpu, 15) do
    val = (bits_to_int(elem(cpu.regs, 15)) + 4) &&& @mask32
    int_to_bits(val, 32)
  end
  defp read_reg_bits_for_exec(cpu, index), do: read_reg_bits(cpu, index)

  # =========================================================================
  # Data Processing (gate-level)
  # =========================================================================

  defp execute_data_processing(cpu, d, trace) do
    # Read Rn as bits
    a_bits = if d.opcode not in [Sim.op_mov(), Sim.op_mvn()] do
      read_reg_bits_for_exec(cpu, d.rn)
    else
      List.duplicate(0, 32)
    end

    # Get Operand2 through gate-level barrel shifter
    current_flags = flags(cpu)
    flag_c = if current_flags.c, do: 1, else: 0
    flag_v = if current_flags.v, do: 1, else: 0

    {b_bits, shifter_carry} = if d.immediate do
      {bits, sc} = gate_decode_immediate(d.imm8, d.rotate)
      sc = if d.rotate == 0, do: flag_c, else: sc
      {bits, sc}
    else
      rm_bits = read_reg_bits_for_exec(cpu, d.rm)
      shift_amount = if d.shift_by_reg do
        read_reg(cpu, d.rs) &&& 0xFF
      else
        d.shift_imm
      end
      gate_barrel_shift(rm_bits, d.shift_type, shift_amount, flag_c, d.shift_by_reg)
    end

    # Execute gate-level ALU
    alu_result = gate_alu_execute(d.opcode, a_bits, b_bits, flag_c, shifter_carry, flag_v)
    cpu = %{cpu | gate_ops: cpu.gate_ops + 200}

    result_val = bits_to_int(alu_result.result_bits)

    # Write result
    cpu = if not Sim.test_op?(d.opcode) do
      if d.rd == 15 do
        if d.s do
          %{cpu | regs: put_elem(cpu.regs, 15, int_to_bits(result_val, 32))}
        else
          set_pc(cpu, result_val &&& Sim.pc_mask())
        end
      else
        write_reg(cpu, d.rd, result_val)
      end
    else
      cpu
    end

    # Update flags
    cpu = if d.s and d.rd != 15 do
      set_flags_bits(cpu, alu_result.n, alu_result.z, alu_result.c, alu_result.v)
    else
      cpu
    end

    cpu = if Sim.test_op?(d.opcode) do
      set_flags_bits(cpu, alu_result.n, alu_result.z, alu_result.c, alu_result.v)
    else
      cpu
    end

    {cpu, trace}
  end

  # =========================================================================
  # Load/Store, Block Transfer, Branch, SWI
  # =========================================================================

  defp execute_load_store(cpu, d, trace) do
    offset = if d.immediate do
      rm_val = read_reg_for_exec(cpu, d.rm)
      if d.shift_imm != 0 do
        rm_bits = int_to_bits(rm_val, 32)
        flag_c = if flags(cpu).c, do: 1, else: 0
        {shifted, _} = gate_barrel_shift(rm_bits, d.shift_type, d.shift_imm, flag_c, false)
        bits_to_int(shifted)
      else
        rm_val
      end
    else
      d.offset12
    end

    base = read_reg_for_exec(cpu, d.rn)
    addr = if d.up, do: (base + offset) &&& @mask32, else: (base - offset) &&& @mask32
    transfer_addr = if d.pre_index, do: addr, else: base

    {cpu, trace} = if d.load do
      value = if d.byte do
        read_byte(cpu, transfer_addr)
      else
        word = read_word(cpu, transfer_addr)
        rotation = (transfer_addr &&& 3) * 8
        if rotation != 0 do
          ((word >>> rotation) ||| (word <<< (32 - rotation))) &&& @mask32
        else
          word
        end
      end
      trace = %{trace | memory_reads: trace.memory_reads ++ [%MemoryAccess{address: transfer_addr, value: value}]}
      cpu = if d.rd == 15 do
        %{cpu | regs: put_elem(cpu.regs, 15, int_to_bits(value &&& @mask32, 32))}
      else
        write_reg(cpu, d.rd, value)
      end
      {cpu, trace}
    else
      value = read_reg_for_exec(cpu, d.rd)
      cpu = if d.byte do
        write_byte(cpu, transfer_addr, value &&& 0xFF)
      else
        write_word(cpu, transfer_addr, value)
      end
      trace = %{trace | memory_writes: trace.memory_writes ++ [%MemoryAccess{address: transfer_addr, value: value}]}
      {cpu, trace}
    end

    cpu = if d.write_back or not d.pre_index do
      if d.rn != 15, do: write_reg(cpu, d.rn, addr), else: cpu
    else
      cpu
    end

    {cpu, trace}
  end

  defp execute_block_transfer(cpu, d, trace) do
    base = read_reg(cpu, d.rn)
    reg_list = d.register_list
    count = Enum.count(0..15, fn i -> ((reg_list >>> i) &&& 1) == 1 end)

    if count == 0 do
      {cpu, trace}
    else
      start_addr = case {d.pre_index, d.up} do
        {false, true} -> base
        {true, true} -> (base + 4) &&& @mask32
        {false, false} -> (base - count * 4 + 4) &&& @mask32
        {true, false} -> (base - count * 4) &&& @mask32
      end

      {cpu, trace, _} = Enum.reduce(0..15, {cpu, trace, start_addr}, fn i, {cpu_acc, trace_acc, addr_acc} ->
        if ((reg_list >>> i) &&& 1) == 0 do
          {cpu_acc, trace_acc, addr_acc}
        else
          if d.load do
            value = read_word(cpu_acc, addr_acc)
            trace_acc = %{trace_acc | memory_reads: trace_acc.memory_reads ++ [%MemoryAccess{address: addr_acc, value: value}]}
            cpu_acc = if i == 15 do
              %{cpu_acc | regs: put_elem(cpu_acc.regs, 15, int_to_bits(value &&& @mask32, 32))}
            else
              write_reg(cpu_acc, i, value)
            end
            {cpu_acc, trace_acc, (addr_acc + 4) &&& @mask32}
          else
            value = if i == 15 do
              (bits_to_int(elem(cpu_acc.regs, 15)) + 4) &&& @mask32
            else
              read_reg(cpu_acc, i)
            end
            cpu_acc = write_word(cpu_acc, addr_acc, value)
            trace_acc = %{trace_acc | memory_writes: trace_acc.memory_writes ++ [%MemoryAccess{address: addr_acc, value: value}]}
            {cpu_acc, trace_acc, (addr_acc + 4) &&& @mask32}
          end
        end
      end)

      cpu = if d.write_back do
        new_base = if d.up do
          (base + count * 4) &&& @mask32
        else
          (base - count * 4) &&& @mask32
        end
        write_reg(cpu, d.rn, new_base)
      else
        cpu
      end

      {cpu, trace}
    end
  end

  defp execute_branch(cpu, d, trace) do
    branch_base = (pc(cpu) + 4) &&& @mask32
    cpu = if d.link do
      return_addr = bits_to_int(elem(cpu.regs, 15))
      write_reg(cpu, 14, return_addr)
    else
      cpu
    end
    target = (branch_base + d.branch_offset) &&& @mask32
    cpu = set_pc(cpu, target &&& Sim.pc_mask())
    {cpu, trace}
  end

  defp execute_swi(cpu, d, trace) do
    if d.swi_comment == Sim.halt_swi() do
      {%{cpu | halted: true}, trace}
    else
      r15_bits = elem(cpu.regs, 15)
      regs = cpu.regs
      regs = put_elem(regs, 25, r15_bits)
      regs = put_elem(regs, 26, r15_bits)
      cpu = %{cpu | regs: regs}

      r15_val = bits_to_int(elem(cpu.regs, 15))
      r15_val = (r15_val &&& (bnot(Sim.mode_mask()) &&& @mask32)) ||| Sim.mode_svc()
      r15_val = r15_val ||| Sim.flag_i()
      cpu = %{cpu | regs: put_elem(cpu.regs, 15, int_to_bits(r15_val &&& @mask32, 32))}
      cpu = set_pc(cpu, 0x08)
      {cpu, trace}
    end
  end

  defp trap_undefined(cpu, _instr_addr) do
    r15_bits = elem(cpu.regs, 15)
    regs = put_elem(cpu.regs, 26, r15_bits)
    cpu = %{cpu | regs: regs}

    r15_val = bits_to_int(elem(cpu.regs, 15))
    r15_val = (r15_val &&& (bnot(Sim.mode_mask()) &&& @mask32)) ||| Sim.mode_svc()
    r15_val = r15_val ||| Sim.flag_i()
    cpu = %{cpu | regs: put_elem(cpu.regs, 15, int_to_bits(r15_val &&& @mask32, 32))}
    set_pc(cpu, 0x04)
  end
end
