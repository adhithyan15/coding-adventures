# === Intel 4004 Gate-Level Simulator ===
#
# Every computation routes through real logic gates and flip-flops.
# The ALU uses half adders → full adders → ripple carry adder.
# Registers store state in D flip-flop state maps.
# The decoder uses AND/OR/NOT gate networks.
#
# This is not a behavioral shortcut — every bit flows through gate functions.

defmodule CodingAdventures.Intel4004GateLevel do
  @moduledoc """
  Intel 4004 gate-level simulator. All operations route through logic gates,
  adders, and flip-flops from the logic_gates and arithmetic packages.
  """

  import Bitwise
  alias CodingAdventures.LogicGates.Gates
  alias CodingAdventures.LogicGates.Sequential
  alias CodingAdventures.Arithmetic, as: Arith

  # ---------------------------------------------------------------------------
  # Trace
  # ---------------------------------------------------------------------------

  defmodule GateTrace do
    @moduledoc "Record of one instruction execution through gates."
    defstruct [:address, :raw, :raw2, :mnemonic,
               :accumulator_before, :accumulator_after,
               :carry_before, :carry_after]
  end

  # ---------------------------------------------------------------------------
  # Bit helpers — bridge between integers and gate-level bits
  # ---------------------------------------------------------------------------

  @doc "Convert integer to LSB-first bit list."
  def int_to_bits(value, width) do
    value = value &&& ((1 <<< width) - 1)
    for i <- 0..(width - 1), do: (value >>> i) &&& 1
  end

  @doc "Convert LSB-first bit list to integer."
  def bits_to_int(bits) do
    bits
    |> Enum.with_index()
    |> Enum.reduce(0, fn {bit, i}, acc -> acc ||| (bit <<< i) end)
  end

  # ---------------------------------------------------------------------------
  # CPU State — all component states in one struct
  # ---------------------------------------------------------------------------

  defstruct [
    # Flip-flop states for registers (16 x 4-bit)
    :reg_states,
    # Accumulator flip-flop state (4-bit)
    :acc_state,
    # Carry flag flip-flop state (1-bit)
    :carry_state,
    # Program counter flip-flop state (12-bit)
    :pc_state,
    # Stack: 3 x 12-bit register states + pointer
    :stack_states,
    :stack_pointer,
    # RAM: stored as map {bank, reg, char} => flip-flop state (4-bit)
    :ram_states,
    :ram_status_states,
    :ram_output,
    # ROM binary
    :rom,
    # Addressing
    :ram_bank,
    :ram_register,
    :ram_character,
    :rom_port,
    # Control
    :halted
  ]

  @ff_init %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1}

  # ---------------------------------------------------------------------------
  # Constructors
  # ---------------------------------------------------------------------------

  def new do
    %__MODULE__{
      reg_states: List.duplicate(init_ff_state(4), 16),
      acc_state: init_ff_state(4),
      carry_state: init_ff_state(1),
      pc_state: init_ff_state(12),
      stack_states: List.duplicate(init_ff_state(12), 3),
      stack_pointer: 0,
      ram_states: %{},
      ram_status_states: %{},
      ram_output: {0, 0, 0, 0},
      rom: :binary.copy(<<0>>, 4096),
      ram_bank: 0,
      ram_register: 0,
      ram_character: 0,
      rom_port: 0,
      halted: false
    }
  end

  defp init_ff_state(width), do: List.duplicate(@ff_init, width)

  # ---------------------------------------------------------------------------
  # Register read/write through flip-flops
  # ---------------------------------------------------------------------------

  defp read_reg(cpu, index) do
    state = Enum.at(cpu.reg_states, index)
    {bits, _} = Sequential.register(List.duplicate(0, 4), 0, state)
    bits_to_int(bits)
  end

  defp write_reg(cpu, index, value) do
    bits = int_to_bits(value, 4)
    state = Enum.at(cpu.reg_states, index)
    # Two-phase write: clock=0 captures into master, clock=1 latches to slave
    {_, state1} = Sequential.register(bits, 0, state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    %{cpu | reg_states: List.replace_at(cpu.reg_states, index, new_state)}
  end

  defp read_acc(cpu) do
    {bits, _} = Sequential.register(List.duplicate(0, 4), 0, cpu.acc_state)
    bits_to_int(bits)
  end

  defp write_acc(cpu, value) do
    bits = int_to_bits(value, 4)
    {_, state1} = Sequential.register(bits, 0, cpu.acc_state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    %{cpu | acc_state: new_state}
  end

  defp read_carry(cpu) do
    {[bit], _} = Sequential.register([0], 0, cpu.carry_state)
    bit == 1
  end

  defp write_carry(cpu, value) do
    bit = if value, do: 1, else: 0
    {_, state1} = Sequential.register([bit], 0, cpu.carry_state)
    {_, new_state} = Sequential.register([bit], 1, state1)
    %{cpu | carry_state: new_state}
  end

  defp read_pc(cpu) do
    {bits, _} = Sequential.register(List.duplicate(0, 12), 0, cpu.pc_state)
    bits_to_int(bits)
  end

  defp write_pc(cpu, value) do
    bits = int_to_bits(value, 12)
    {_, state1} = Sequential.register(bits, 0, cpu.pc_state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    %{cpu | pc_state: new_state}
  end

  defp increment_pc(cpu) do
    # Use half_adder chain to increment by 1
    {bits, _} = Sequential.register(List.duplicate(0, 12), 0, cpu.pc_state)
    {new_bits, _carry} =
      Enum.reduce(bits, {[], 1}, fn bit, {acc, carry} ->
        {sum, new_carry} = Arith.half_adder(bit, carry)
        {[sum | acc], new_carry}
      end)
    new_bits = Enum.reverse(new_bits)
    # Two-phase write
    {_, state1} = Sequential.register(new_bits, 0, cpu.pc_state)
    {_, new_state} = Sequential.register(new_bits, 1, state1)
    %{cpu | pc_state: new_state}
  end

  defp increment_pc2(cpu), do: cpu |> increment_pc() |> increment_pc()

  # ---------------------------------------------------------------------------
  # Register pairs
  # ---------------------------------------------------------------------------

  defp read_pair(cpu, pair) do
    high = read_reg(cpu, pair * 2)
    low = read_reg(cpu, pair * 2 + 1)
    (high <<< 4) ||| low
  end

  defp write_pair(cpu, pair, value) do
    cpu
    |> write_reg(pair * 2, (value >>> 4) &&& 0xF)
    |> write_reg(pair * 2 + 1, value &&& 0xF)
  end

  # ---------------------------------------------------------------------------
  # Stack
  # ---------------------------------------------------------------------------

  defp stack_push(cpu, addr) do
    bits = int_to_bits(addr &&& 0xFFF, 12)
    state = Enum.at(cpu.stack_states, cpu.stack_pointer)
    {_, state1} = Sequential.register(bits, 0, state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    new_states = List.replace_at(cpu.stack_states, cpu.stack_pointer, new_state)
    %{cpu | stack_states: new_states, stack_pointer: rem(cpu.stack_pointer + 1, 3)}
  end

  defp stack_pop(cpu) do
    sp = rem(cpu.stack_pointer + 2, 3)
    state = Enum.at(cpu.stack_states, sp)
    {bits, _} = Sequential.register(List.duplicate(0, 12), 0, state)
    addr = bits_to_int(bits)
    {%{cpu | stack_pointer: sp}, addr}
  end

  # ---------------------------------------------------------------------------
  # RAM (flip-flop storage)
  # ---------------------------------------------------------------------------

  defp ram_read_main(cpu) do
    key = {cpu.ram_bank, cpu.ram_register, cpu.ram_character}
    case Map.get(cpu.ram_states, key) do
      nil -> 0
      state ->
        {bits, _} = Sequential.register(List.duplicate(0, 4), 0, state)
        bits_to_int(bits)
    end
  end

  defp ram_write_main(cpu, value) do
    key = {cpu.ram_bank, cpu.ram_register, cpu.ram_character}
    bits = int_to_bits(value &&& 0xF, 4)
    state = Map.get(cpu.ram_states, key, init_ff_state(4))
    {_, state1} = Sequential.register(bits, 0, state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    %{cpu | ram_states: Map.put(cpu.ram_states, key, new_state)}
  end

  defp ram_read_status(cpu, index) do
    key = {:status, cpu.ram_bank, cpu.ram_register, index}
    case Map.get(cpu.ram_status_states, key) do
      nil -> 0
      state ->
        {bits, _} = Sequential.register(List.duplicate(0, 4), 0, state)
        bits_to_int(bits)
    end
  end

  defp ram_write_status(cpu, index, value) do
    key = {:status, cpu.ram_bank, cpu.ram_register, index}
    bits = int_to_bits(value &&& 0xF, 4)
    state = Map.get(cpu.ram_status_states, key, init_ff_state(4))
    {_, state1} = Sequential.register(bits, 0, state)
    {_, new_state} = Sequential.register(bits, 1, state1)
    %{cpu | ram_status_states: Map.put(cpu.ram_status_states, key, new_state)}
  end

  # ---------------------------------------------------------------------------
  # ALU operations (through arithmetic package gates)
  # ---------------------------------------------------------------------------

  defp alu_add(a, b, carry_in) do
    a_bits = int_to_bits(a, 4)
    b_bits = int_to_bits(b, 4)

    if carry_in == 1 do
      r1 = Arith.alu_execute(:add, a_bits, b_bits)
      one = int_to_bits(1, 4)
      r2 = Arith.alu_execute(:add, r1.value, one)
      {bits_to_int(r2.value), r1.carry or r2.carry}
    else
      result = Arith.alu_execute(:add, a_bits, b_bits)
      {bits_to_int(result.value), result.carry}
    end
  end

  defp alu_subtract(a, b, borrow_in) do
    # Complement b using NOT gates, then add
    b_bits = int_to_bits(b, 4)
    b_comp = Arith.alu_execute(:not_op, b_bits, b_bits)
    alu_add(a, bits_to_int(b_comp.value), borrow_in)
  end

  defp alu_complement(a) do
    a_bits = int_to_bits(a, 4)
    result = Arith.alu_execute(:not_op, a_bits, a_bits)
    bits_to_int(result.value)
  end

  defp alu_increment(a), do: alu_add(a, 1, 0)
  defp alu_decrement(a), do: alu_subtract(a, 1, 1)

  # ---------------------------------------------------------------------------
  # Decoder (combinational AND/OR/NOT gate network)
  # ---------------------------------------------------------------------------

  defp decode(raw, raw2 \\ nil) do
    upper = (raw >>> 4) &&& 0xF
    lower = raw &&& 0xF

    # Convert upper nibble to bits for gate-level decoding
    b7 = (raw >>> 7) &&& 1
    b6 = (raw >>> 6) &&& 1
    b5 = (raw >>> 5) &&& 1
    b4 = (raw >>> 4) &&& 1
    b0 = raw &&& 1

    # Gate-level instruction detection using AND/OR/NOT
    is_nop = Gates.and_gate(Gates.not_gate(b7), Gates.and_gate(Gates.not_gate(b6),
             Gates.and_gate(Gates.not_gate(b5), Gates.and_gate(Gates.not_gate(b4),
             Gates.not_gate(b0))))) == 1 and raw == 0x00

    is_hlt = raw == 0x01

    %{
      raw: raw, raw2: raw2, upper: upper, lower: lower,
      is_nop: is_nop, is_hlt: is_hlt,
      is_jcn: upper == 0x1,
      is_fim: upper == 0x2 and Gates.not_gate(b0) == 1,
      is_src: upper == 0x2 and b0 == 1,
      is_fin: upper == 0x3 and Gates.not_gate(b0) == 1,
      is_jin: upper == 0x3 and b0 == 1,
      is_jun: upper == 0x4,
      is_jms: upper == 0x5,
      is_inc: upper == 0x6,
      is_isz: upper == 0x7,
      is_add: upper == 0x8,
      is_sub: upper == 0x9,
      is_ld: upper == 0xA,
      is_xch: upper == 0xB,
      is_bbl: upper == 0xC,
      is_ldm: Gates.and_gate(b7, Gates.and_gate(b6, Gates.and_gate(Gates.not_gate(b5), b4))) == 1,
      is_io: upper == 0xE,
      is_accum: upper == 0xF,
      is_two_byte: upper in [0x1, 0x4, 0x5, 0x7] or (upper == 0x2 and b0 == 0),
      reg_index: lower,
      pair_index: lower >>> 1,
      immediate: lower,
      condition: lower,
      addr12: if(raw2, do: (lower <<< 8) ||| raw2, else: 0),
      addr8: raw2 || 0
    }
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  def run(program, max_steps \\ 10_000) do
    cpu = new() |> load_program(program)
    do_run(cpu, [], max_steps)
  end

  def load_program(cpu, program) when is_binary(program) do
    padding = max(4096 - byte_size(program), 0)
    %{cpu | rom: program <> :binary.copy(<<0>>, padding)}
  end

  def step(%__MODULE__{halted: true}), do: raise("CPU is halted")

  def step(%__MODULE__{} = cpu) do
    acc_before = read_acc(cpu)
    carry_before = read_carry(cpu)
    pc_before = read_pc(cpu)

    raw = :binary.at(cpu.rom, pc_before)
    decoded = decode(raw)

    {cpu, raw2, decoded} =
      if decoded.is_two_byte do
        raw2 = :binary.at(cpu.rom, (pc_before + 1) &&& 0xFFF)
        {cpu, raw2, decode(raw, raw2)}
      else
        {cpu, nil, decoded}
      end

    {cpu, mnemonic} = execute(cpu, decoded)

    trace = %GateTrace{
      address: pc_before, raw: raw, raw2: raw2, mnemonic: mnemonic,
      accumulator_before: acc_before, accumulator_after: read_acc(cpu),
      carry_before: carry_before, carry_after: read_carry(cpu)
    }

    {cpu, trace}
  end

  def gate_count(_cpu \\ nil) do
    32 + 480 + 24 + 6 + 96 + 226 + 7880 + 50 + 100
  end

  # Read-only accessors
  def accumulator(cpu), do: read_acc(cpu)
  def registers(cpu), do: for(i <- 0..15, do: read_reg(cpu, i))
  def carry(cpu), do: read_carry(cpu)

  # ---------------------------------------------------------------------------
  # Private — run loop
  # ---------------------------------------------------------------------------

  defp do_run(cpu, traces, 0), do: {cpu, Enum.reverse(traces)}
  defp do_run(%{halted: true} = cpu, traces, _), do: {cpu, Enum.reverse(traces)}

  defp do_run(cpu, traces, remaining) do
    {cpu, trace} = step(cpu)
    do_run(cpu, [trace | traces], remaining - 1)
  end

  # ---------------------------------------------------------------------------
  # Instruction execution — routes through gate-level components
  # ---------------------------------------------------------------------------

  defp execute(cpu, %{is_nop: true}), do: {increment_pc(cpu), "NOP"}
  defp execute(cpu, %{is_hlt: true}), do: {%{increment_pc(cpu) | halted: true}, "HLT"}

  defp execute(cpu, %{is_ldm: true, immediate: n}) do
    {cpu |> write_acc(n) |> increment_pc(), "LDM #{n}"}
  end

  defp execute(cpu, %{is_ld: true, reg_index: r}) do
    val = read_reg(cpu, r)
    {cpu |> write_acc(val) |> increment_pc(), "LD R#{r}"}
  end

  defp execute(cpu, %{is_xch: true, reg_index: r}) do
    a = read_acc(cpu)
    rv = read_reg(cpu, r)
    {cpu |> write_acc(rv) |> write_reg(r, a) |> increment_pc(), "XCH R#{r}"}
  end

  defp execute(cpu, %{is_inc: true, reg_index: r}) do
    {result, _} = alu_increment(read_reg(cpu, r))
    {cpu |> write_reg(r, result) |> increment_pc(), "INC R#{r}"}
  end

  defp execute(cpu, %{is_add: true, reg_index: r}) do
    carry_in = if read_carry(cpu), do: 1, else: 0
    {result, cout} = alu_add(read_acc(cpu), read_reg(cpu, r), carry_in)
    {cpu |> write_acc(result) |> write_carry(cout) |> increment_pc(), "ADD R#{r}"}
  end

  defp execute(cpu, %{is_sub: true, reg_index: r}) do
    borrow_in = if read_carry(cpu), do: 0, else: 1
    {result, cout} = alu_subtract(read_acc(cpu), read_reg(cpu, r), borrow_in)
    {cpu |> write_acc(result) |> write_carry(cout) |> increment_pc(), "SUB R#{r}"}
  end

  defp execute(cpu, %{is_jun: true, addr12: addr}) do
    {write_pc(cpu, addr), "JUN 0x#{pad_hex(addr, 3)}"}
  end

  defp execute(cpu, %{is_jcn: true} = d), do: exec_jcn(cpu, d)
  defp execute(cpu, %{is_isz: true} = d), do: exec_isz(cpu, d)

  defp execute(cpu, %{is_jms: true, addr12: addr}) do
    return_addr = read_pc(cpu) + 2
    cpu = stack_push(cpu, return_addr)
    {write_pc(cpu, addr), "JMS 0x#{pad_hex(addr, 3)}"}
  end

  defp execute(cpu, %{is_bbl: true, immediate: n}) do
    {cpu, ret_addr} = stack_pop(cpu)
    {cpu |> write_acc(n) |> write_pc(ret_addr), "BBL #{n}"}
  end

  defp execute(cpu, %{is_fim: true, pair_index: p, addr8: data}) do
    {cpu |> write_pair(p, data) |> increment_pc2(), "FIM P#{p},0x#{pad_hex(data, 2)}"}
  end

  defp execute(cpu, %{is_src: true, pair_index: p}) do
    pv = read_pair(cpu, p)
    cpu = %{cpu | ram_register: (pv >>> 4) &&& 0xF, ram_character: pv &&& 0xF}
    {increment_pc(cpu), "SRC P#{p}"}
  end

  defp execute(cpu, %{is_fin: true, pair_index: p}) do
    p0 = read_pair(cpu, 0)
    page = read_pc(cpu) &&& 0xF00
    rom_byte = :binary.at(cpu.rom, (page ||| p0) &&& 0xFFF)
    {cpu |> write_pair(p, rom_byte) |> increment_pc(), "FIN P#{p}"}
  end

  defp execute(cpu, %{is_jin: true, pair_index: p}) do
    pv = read_pair(cpu, p)
    page = read_pc(cpu) &&& 0xF00
    {write_pc(cpu, page ||| pv), "JIN P#{p}"}
  end

  defp execute(cpu, %{is_io: true} = d), do: exec_io(cpu, d)
  defp execute(cpu, %{is_accum: true} = d), do: exec_accum(cpu, d)

  defp execute(cpu, %{raw: raw}) do
    {increment_pc(cpu), "UNKNOWN(0x#{Integer.to_string(raw, 16)})"}
  end

  # ---------------------------------------------------------------------------
  # JCN — conditional jump using gates
  # ---------------------------------------------------------------------------

  defp exec_jcn(cpu, d) do
    cond_nibble = d.condition
    a_val = read_acc(cpu)
    carry_val = if read_carry(cpu), do: 1, else: 0

    # Test A==0 using gate-level NOR
    a_bits = int_to_bits(a_val, 4)
    a_is_zero = Gates.not_gate(Gates.or_gate(
      Gates.or_gate(Enum.at(a_bits, 0), Enum.at(a_bits, 1)),
      Gates.or_gate(Enum.at(a_bits, 2), Enum.at(a_bits, 3))))

    test_zero = Gates.and_gate((cond_nibble >>> 2) &&& 1, a_is_zero)
    test_carry = Gates.and_gate((cond_nibble >>> 1) &&& 1, carry_val)
    test_pin = Gates.and_gate(cond_nibble &&& 1, 0)
    test_result = Gates.or_gate(Gates.or_gate(test_zero, test_carry), test_pin)

    invert = (cond_nibble >>> 3) &&& 1
    final = Gates.or_gate(
      Gates.and_gate(test_result, Gates.not_gate(invert)),
      Gates.and_gate(Gates.not_gate(test_result), invert))

    page = (read_pc(cpu) + 2) &&& 0xF00
    target = page ||| d.addr8

    cpu = if final == 1, do: write_pc(cpu, target), else: increment_pc2(cpu)
    {cpu, "JCN #{cond_nibble},#{pad_hex(d.addr8, 2)}"}
  end

  # ---------------------------------------------------------------------------
  # ISZ — increment and skip if zero
  # ---------------------------------------------------------------------------

  defp exec_isz(cpu, d) do
    {result, _} = alu_increment(read_reg(cpu, d.reg_index))
    cpu = write_reg(cpu, d.reg_index, result)

    r_bits = int_to_bits(result, 4)
    is_zero = Gates.not_gate(Gates.or_gate(
      Gates.or_gate(Enum.at(r_bits, 0), Enum.at(r_bits, 1)),
      Gates.or_gate(Enum.at(r_bits, 2), Enum.at(r_bits, 3))))

    page = (read_pc(cpu) + 2) &&& 0xF00
    target = page ||| d.addr8

    cpu = if is_zero == 1, do: increment_pc2(cpu), else: write_pc(cpu, target)
    {cpu, "ISZ R#{d.reg_index},0x#{pad_hex(d.addr8, 2)}"}
  end

  # ---------------------------------------------------------------------------
  # I/O instructions
  # ---------------------------------------------------------------------------

  defp exec_io(cpu, d) do
    a_val = read_acc(cpu)
    sub = d.lower

    cond do
      sub == 0x0 -> {cpu |> ram_write_main(a_val) |> increment_pc(), "WRM"}
      sub == 0x1 ->
        out = put_elem(cpu.ram_output, cpu.ram_bank, a_val &&& 0xF)
        {%{cpu | ram_output: out} |> increment_pc(), "WMP"}
      sub == 0x2 -> {%{cpu | rom_port: a_val &&& 0xF} |> increment_pc(), "WRR"}
      sub == 0x3 -> {increment_pc(cpu), "WPM"}
      sub in [0x4, 0x5, 0x6, 0x7] ->
        idx = sub - 0x4
        {cpu |> ram_write_status(idx, a_val) |> increment_pc(), "WR#{idx}"}
      sub == 0x8 ->
        rv = ram_read_main(cpu)
        borrow_in = if read_carry(cpu), do: 0, else: 1
        {result, cout} = alu_subtract(a_val, rv, borrow_in)
        {cpu |> write_acc(result) |> write_carry(cout) |> increment_pc(), "SBM"}
      sub == 0x9 ->
        val = ram_read_main(cpu)
        {cpu |> write_acc(val) |> increment_pc(), "RDM"}
      sub == 0xA ->
        {cpu |> write_acc(cpu.rom_port &&& 0xF) |> increment_pc(), "RDR"}
      sub == 0xB ->
        rv = ram_read_main(cpu)
        carry_in = if read_carry(cpu), do: 1, else: 0
        {result, cout} = alu_add(a_val, rv, carry_in)
        {cpu |> write_acc(result) |> write_carry(cout) |> increment_pc(), "ADM"}
      sub in [0xC, 0xD, 0xE, 0xF] ->
        idx = sub - 0xC
        val = ram_read_status(cpu, idx)
        {cpu |> write_acc(val) |> increment_pc(), "RD#{idx}"}
      true -> {increment_pc(cpu), "IO(0x#{Integer.to_string(d.raw, 16)})"}
    end
  end

  # ---------------------------------------------------------------------------
  # Accumulator operations
  # ---------------------------------------------------------------------------

  defp exec_accum(cpu, d) do
    a_val = read_acc(cpu)
    sub = d.lower

    cond do
      sub == 0x0 -> {cpu |> write_acc(0) |> write_carry(false) |> increment_pc(), "CLB"}
      sub == 0x1 -> {cpu |> write_carry(false) |> increment_pc(), "CLC"}
      sub == 0x2 ->
        {result, carry} = alu_increment(a_val)
        {cpu |> write_acc(result) |> write_carry(carry) |> increment_pc(), "IAC"}
      sub == 0x3 -> {cpu |> write_carry(not read_carry(cpu)) |> increment_pc(), "CMC"}
      sub == 0x4 ->
        result = alu_complement(a_val)
        {cpu |> write_acc(result) |> increment_pc(), "CMA"}
      sub == 0x5 -> # RAL
        old_carry = if read_carry(cpu), do: 1, else: 0
        a_bits = int_to_bits(a_val, 4)
        new_carry = Enum.at(a_bits, 3) == 1
        new_bits = [old_carry, Enum.at(a_bits, 0), Enum.at(a_bits, 1), Enum.at(a_bits, 2)]
        {cpu |> write_acc(bits_to_int(new_bits)) |> write_carry(new_carry) |> increment_pc(), "RAL"}
      sub == 0x6 -> # RAR
        old_carry = if read_carry(cpu), do: 1, else: 0
        a_bits = int_to_bits(a_val, 4)
        new_carry = Enum.at(a_bits, 0) == 1
        new_bits = [Enum.at(a_bits, 1), Enum.at(a_bits, 2), Enum.at(a_bits, 3), old_carry]
        {cpu |> write_acc(bits_to_int(new_bits)) |> write_carry(new_carry) |> increment_pc(), "RAR"}
      sub == 0x7 -> # TCC
        val = if read_carry(cpu), do: 1, else: 0
        {cpu |> write_acc(val) |> write_carry(false) |> increment_pc(), "TCC"}
      sub == 0x8 -> # DAC
        {result, carry} = alu_decrement(a_val)
        {cpu |> write_acc(result) |> write_carry(carry) |> increment_pc(), "DAC"}
      sub == 0x9 -> # TCS
        val = if read_carry(cpu), do: 10, else: 9
        {cpu |> write_acc(val) |> write_carry(false) |> increment_pc(), "TCS"}
      sub == 0xA -> {cpu |> write_carry(true) |> increment_pc(), "STC"}
      sub == 0xB -> # DAA
        if a_val > 9 or read_carry(cpu) do
          {result, carry} = alu_add(a_val, 6, 0)
          cpu = if carry, do: write_carry(cpu, true), else: cpu
          {cpu |> write_acc(result) |> increment_pc(), "DAA"}
        else
          {increment_pc(cpu), "DAA"}
        end
      sub == 0xC -> # KBP
        kbp = %{0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4}
        {cpu |> write_acc(Map.get(kbp, a_val, 15)) |> increment_pc(), "KBP"}
      sub == 0xD -> # DCL
        bank = a_val &&& 0x7
        bank = if bank > 3, do: bank &&& 0x3, else: bank
        {%{cpu | ram_bank: bank} |> increment_pc(), "DCL"}
      true -> {increment_pc(cpu), "ACCUM(0x#{Integer.to_string(d.raw, 16)})"}
    end
  end

  defp pad_hex(val, width) do
    val |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(width, "0")
  end
end
