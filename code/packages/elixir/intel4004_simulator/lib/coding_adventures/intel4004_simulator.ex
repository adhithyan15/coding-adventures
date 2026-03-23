# === Intel 4004 Simulator — the world's first commercial microprocessor ===
#
# The Intel 4004 was released in 1971, designed by Federico Faggin, Ted Hoff,
# and Stanley Mazor for the Busicom 141-PF calculator. It contained just 2,300
# transistors and ran at 740 kHz — about a million times slower than modern CPUs.
# Yet it proved a general-purpose processor could be built on a single chip,
# launching the microprocessor revolution.
#
# === Architecture ===
#
# The 4004 is a 4-bit processor with an accumulator architecture:
#
#     Data width:      4 bits (values 0–15)
#     Instruction:     8 bits (some are 2 bytes)
#     Registers:       16 × 4-bit (R0–R15), organized as 8 pairs
#     Accumulator:     4-bit (A) — most arithmetic goes through here
#     Carry flag:      1 bit — set on overflow/borrow
#     Program counter: 12 bits (addresses 4096 bytes of ROM)
#     Stack:           3-level hardware stack (12-bit return addresses)
#     RAM:             4 banks × 4 registers × (16 main + 4 status) nibbles
#
# === Why Elixir for a CPU simulator? ===
#
# Elixir is a functional language, so we model the CPU as an immutable struct.
# Each instruction transforms the CPU state into a new state — no mutation.
# This is actually a natural fit: hardware is also a pure function from
# current state + input → next state.
#
# === Complete Instruction Set (46 instructions + HLT) ===
#
#     0x00       NOP          No operation
#     0x01       HLT          Halt (simulator-only)
#     0x1_       JCN c,a  *   Conditional jump
#     0x2_ even  FIM Pp,d *   Fetch immediate to register pair
#     0x2_ odd   SRC Pp       Send register control
#     0x3_ even  FIN Pp       Fetch indirect from ROM via P0
#     0x3_ odd   JIN Pp       Jump indirect via register pair
#     0x4_       JUN a    *   Unconditional jump
#     0x5_       JMS a    *   Jump to subroutine
#     0x6_       INC Rn       Increment register
#     0x7_       ISZ Rn,a *   Increment and skip if zero
#     0x8_       ADD Rn       Add register to accumulator
#     0x9_       SUB Rn       Subtract register from accumulator
#     0xA_       LD Rn        Load register into accumulator
#     0xB_       XCH Rn       Exchange accumulator and register
#     0xC_       BBL n        Branch back and load
#     0xD_       LDM n        Load immediate
#     0xE0–0xEF  I/O ops      RAM/ROM read/write
#     0xF0–0xFD  Accum ops    Accumulator manipulation
#
#     * = 2-byte instruction

defmodule CodingAdventures.Intel4004Simulator do
  @moduledoc """
  A complete Intel 4004 microprocessor simulator implementing all 46 real
  instructions plus HLT. Uses an immutable struct to model CPU state.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Trace — what happened during one instruction
  # ---------------------------------------------------------------------------

  defmodule Trace do
    @moduledoc "Record of a single instruction execution."
    defstruct [
      :address,
      :raw,
      :raw2,
      :mnemonic,
      :accumulator_before,
      :accumulator_after,
      :carry_before,
      :carry_after
    ]
  end

  # ---------------------------------------------------------------------------
  # CPU State
  # ---------------------------------------------------------------------------

  defstruct accumulator: 0,
            registers: List.duplicate(0, 16),
            carry: false,
            rom: <<>>,
            pc: 0,
            halted: false,
            hw_stack: {0, 0, 0},
            stack_pointer: 0,
            ram: nil,
            ram_status: nil,
            ram_output: {0, 0, 0, 0},
            ram_bank: 0,
            ram_register: 0,
            ram_character: 0,
            rom_port: 0

  @type t :: %__MODULE__{}

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc "Create a new Intel 4004 simulator with zeroed state."
  def new do
    %__MODULE__{
      ram: init_ram(),
      ram_status: init_ram_status()
    }
  end

  @doc """
  Load and run a program, returning {final_cpu_state, traces}.

  ## Example

      iex> {cpu, traces} = CodingAdventures.Intel4004Simulator.run(<<0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01>>)
      iex> Enum.at(cpu.registers, 1)
      3
  """
  def run(program, max_steps \\ 10_000) do
    cpu = new() |> load_program(program)
    do_run(cpu, [], max_steps)
  end

  @doc "Load a program (binary) into ROM."
  def load_program(%__MODULE__{} = cpu, program) when is_binary(program) do
    # Pad to 4096 bytes
    padding_size = max(4096 - byte_size(program), 0)
    padded = program <> :binary.copy(<<0>>, padding_size)
    %{cpu | rom: padded, pc: 0}
  end

  @doc "Execute one instruction, returning {new_cpu, trace}."
  def step(%__MODULE__{halted: true}), do: raise("CPU is halted — cannot step further")

  def step(%__MODULE__{} = cpu) do
    address = cpu.pc
    raw = :binary.at(cpu.rom, cpu.pc)
    cpu = %{cpu | pc: cpu.pc + 1}

    {cpu, raw2} =
      if two_byte?(raw) do
        byte2 = :binary.at(cpu.rom, cpu.pc)
        {%{cpu | pc: cpu.pc + 1}, byte2}
      else
        {cpu, nil}
      end

    acc_before = cpu.accumulator
    carry_before = cpu.carry

    {cpu, mnemonic} = execute(cpu, raw, raw2, address)

    trace = %Trace{
      address: address,
      raw: raw,
      raw2: raw2,
      mnemonic: mnemonic,
      accumulator_before: acc_before,
      accumulator_after: cpu.accumulator,
      carry_before: carry_before,
      carry_after: cpu.carry
    }

    {cpu, trace}
  end

  # ---------------------------------------------------------------------------
  # Private — run loop
  # ---------------------------------------------------------------------------

  defp do_run(cpu, traces, 0), do: {cpu, Enum.reverse(traces)}

  defp do_run(%__MODULE__{halted: true} = cpu, traces, _),
    do: {cpu, Enum.reverse(traces)}

  defp do_run(%__MODULE__{pc: pc} = cpu, traces, _remaining) when pc >= 4096,
    do: {cpu, Enum.reverse(traces)}

  defp do_run(cpu, traces, remaining) do
    {cpu, trace} = step(cpu)
    do_run(cpu, [trace | traces], remaining - 1)
  end

  # ---------------------------------------------------------------------------
  # Private — 2-byte detection
  # ---------------------------------------------------------------------------

  defp two_byte?(raw) do
    upper = (raw >>> 4) &&& 0xF

    cond do
      upper in [0x1, 0x4, 0x5, 0x7] -> true
      upper == 0x2 and (raw &&& 0x1) == 0 -> true
      true -> false
    end
  end

  # ---------------------------------------------------------------------------
  # Private — instruction execution
  # ---------------------------------------------------------------------------

  defp execute(cpu, 0x00, _raw2, _addr), do: {cpu, "NOP"}

  defp execute(cpu, 0x01, _raw2, _addr),
    do: {%{cpu | halted: true}, "HLT"}

  defp execute(cpu, raw, raw2, addr) do
    upper = (raw >>> 4) &&& 0xF
    lower = raw &&& 0xF

    case upper do
      0x1 -> exec_jcn(cpu, lower, raw2, addr)
      0x2 when (raw &&& 1) == 0 -> exec_fim(cpu, lower >>> 1, raw2)
      0x2 -> exec_src(cpu, lower >>> 1)
      0x3 when (raw &&& 1) == 0 -> exec_fin(cpu, lower >>> 1, addr)
      0x3 -> exec_jin(cpu, lower >>> 1, addr)
      0x4 -> exec_jun(cpu, lower, raw2)
      0x5 -> exec_jms(cpu, lower, raw2, addr)
      0x6 -> exec_inc(cpu, lower)
      0x7 -> exec_isz(cpu, lower, raw2, addr)
      0x8 -> exec_add(cpu, lower)
      0x9 -> exec_sub(cpu, lower)
      0xA -> exec_ld(cpu, lower)
      0xB -> exec_xch(cpu, lower)
      0xC -> exec_bbl(cpu, lower)
      0xD -> exec_ldm(cpu, lower)
      0xE -> exec_io(cpu, raw)
      0xF -> exec_accum(cpu, raw)
      _ -> {cpu, "UNKNOWN(0x#{Integer.to_string(raw, 16)})"}
    end
  end

  # ---------------------------------------------------------------------------
  # Instruction implementations
  # ---------------------------------------------------------------------------

  # --- LDM: Load immediate into accumulator ---
  defp exec_ldm(cpu, n), do: {%{cpu | accumulator: n &&& 0xF}, "LDM #{n}"}

  # --- LD: Load register into accumulator ---
  defp exec_ld(cpu, reg) do
    val = Enum.at(cpu.registers, reg) &&& 0xF
    {%{cpu | accumulator: val}, "LD R#{reg}"}
  end

  # --- XCH: Exchange accumulator with register ---
  defp exec_xch(cpu, reg) do
    old_a = cpu.accumulator
    reg_val = Enum.at(cpu.registers, reg) &&& 0xF
    regs = List.replace_at(cpu.registers, reg, old_a &&& 0xF)
    {%{cpu | accumulator: reg_val, registers: regs}, "XCH R#{reg}"}
  end

  # --- INC: Increment register (no carry effect) ---
  defp exec_inc(cpu, reg) do
    val = (Enum.at(cpu.registers, reg) + 1) &&& 0xF
    regs = List.replace_at(cpu.registers, reg, val)
    {%{cpu | registers: regs}, "INC R#{reg}"}
  end

  # --- ADD: Add register to accumulator with carry ---
  # A = A + Rn + carry_in. The carry flag participates in the addition —
  # this is how multi-digit BCD arithmetic chains carry between digits.
  defp exec_add(cpu, reg) do
    reg_val = Enum.at(cpu.registers, reg)
    carry_in = if cpu.carry, do: 1, else: 0
    result = cpu.accumulator + reg_val + carry_in
    {%{cpu | accumulator: result &&& 0xF, carry: result > 0xF}, "ADD R#{reg}"}
  end

  # --- SUB: Subtract register from accumulator (complement-add) ---
  # A = A + ~Rn + borrow_in where borrow_in = carry ? 0 : 1
  # carry=true means NO borrow (MCS-4 semantics)
  defp exec_sub(cpu, reg) do
    reg_val = Enum.at(cpu.registers, reg)
    complement = Bitwise.bnot(reg_val) &&& 0xF
    borrow_in = if cpu.carry, do: 0, else: 1
    result = cpu.accumulator + complement + borrow_in
    {%{cpu | accumulator: result &&& 0xF, carry: result > 0xF}, "SUB R#{reg}"}
  end

  # --- JUN: Unconditional jump ---
  defp exec_jun(cpu, lower, raw2) do
    addr = (lower <<< 8) ||| raw2
    {%{cpu | pc: addr}, "JUN 0x#{pad_hex(addr, 3)}"}
  end

  # --- JCN: Conditional jump ---
  # Condition nibble bits: 8=invert, 4=test_zero, 2=test_carry, 1=test_pin
  # Multiple test bits are OR'd. If the (possibly inverted) result is true, jump.
  defp exec_jcn(cpu, cond_nibble, raw2, addr) do
    test_zero = (cond_nibble &&& 0x4) != 0 and cpu.accumulator == 0
    test_carry = (cond_nibble &&& 0x2) != 0 and cpu.carry
    test_pin = (cond_nibble &&& 0x1) != 0 and false
    test_result = test_zero or test_carry or test_pin
    test_result = if (cond_nibble &&& 0x8) != 0, do: not test_result, else: test_result

    # Target is page-relative: (addr_of_instruction + 2) & 0xF00 | raw2
    page = (addr + 2) &&& 0xF00
    target = page ||| raw2

    cpu = if test_result, do: %{cpu | pc: target}, else: cpu
    {cpu, "JCN #{cond_nibble},#{pad_hex(raw2, 2)}"}
  end

  # --- JMS: Jump to subroutine ---
  defp exec_jms(cpu, lower, raw2, addr) do
    target = (lower <<< 8) ||| raw2
    # Push return address (address after this 2-byte instruction)
    return_addr = addr + 2
    cpu = stack_push(cpu, return_addr)
    {%{cpu | pc: target}, "JMS 0x#{pad_hex(target, 3)}"}
  end

  # --- BBL: Branch back and load ---
  defp exec_bbl(cpu, n) do
    {cpu, return_addr} = stack_pop(cpu)
    {%{cpu | accumulator: n &&& 0xF, pc: return_addr}, "BBL #{n}"}
  end

  # --- ISZ: Increment and skip if zero ---
  defp exec_isz(cpu, reg, raw2, addr) do
    val = (Enum.at(cpu.registers, reg) + 1) &&& 0xF
    regs = List.replace_at(cpu.registers, reg, val)
    cpu = %{cpu | registers: regs}

    page = (addr + 2) &&& 0xF00
    target = page ||| raw2

    cpu = if val != 0, do: %{cpu | pc: target}, else: cpu
    {cpu, "ISZ R#{reg},0x#{pad_hex(raw2, 2)}"}
  end

  # --- FIM: Fetch immediate to register pair ---
  defp exec_fim(cpu, pair, data) do
    high_reg = pair * 2
    low_reg = high_reg + 1
    regs =
      cpu.registers
      |> List.replace_at(high_reg, (data >>> 4) &&& 0xF)
      |> List.replace_at(low_reg, data &&& 0xF)

    {%{cpu | registers: regs}, "FIM P#{pair},0x#{pad_hex(data, 2)}"}
  end

  # --- SRC: Send register control ---
  defp exec_src(cpu, pair) do
    pair_val = read_pair(cpu, pair)
    ram_reg = (pair_val >>> 4) &&& 0xF
    ram_char = pair_val &&& 0xF
    {%{cpu | ram_register: ram_reg, ram_character: ram_char}, "SRC P#{pair}"}
  end

  # --- FIN: Fetch indirect from ROM ---
  defp exec_fin(cpu, pair, addr) do
    p0_val = read_pair(cpu, 0)
    current_page = addr &&& 0xF00
    rom_addr = current_page ||| p0_val
    rom_byte = :binary.at(cpu.rom, rom_addr)
    cpu = write_pair(cpu, pair, rom_byte)
    {cpu, "FIN P#{pair}"}
  end

  # --- JIN: Jump indirect ---
  defp exec_jin(cpu, pair, addr) do
    pair_val = read_pair(cpu, pair)
    current_page = addr &&& 0xF00
    target = current_page ||| pair_val
    {%{cpu | pc: target}, "JIN P#{pair}"}
  end

  # --- I/O instructions (0xE0–0xEF) ---
  defp exec_io(cpu, raw) do
    case raw do
      0xE0 -> # WRM: Write accumulator to RAM main character
        cpu = ram_write_main(cpu, cpu.accumulator)
        {cpu, "WRM"}

      0xE1 -> # WMP: Write accumulator to RAM output port
        output = put_elem(cpu.ram_output, cpu.ram_bank, cpu.accumulator &&& 0xF)
        {%{cpu | ram_output: output}, "WMP"}

      0xE2 -> # WRR: Write accumulator to ROM I/O port
        {%{cpu | rom_port: cpu.accumulator &&& 0xF}, "WRR"}

      0xE3 -> # WPM: Write program RAM (not simulated)
        {cpu, "WPM"}

      0xE4 -> {ram_write_status(cpu, 0, cpu.accumulator), "WR0"}
      0xE5 -> {ram_write_status(cpu, 1, cpu.accumulator), "WR1"}
      0xE6 -> {ram_write_status(cpu, 2, cpu.accumulator), "WR2"}
      0xE7 -> {ram_write_status(cpu, 3, cpu.accumulator), "WR3"}

      0xE8 -> # SBM: Subtract RAM from accumulator
        ram_val = ram_read_main(cpu)
        complement = Bitwise.bnot(ram_val) &&& 0xF
        borrow_in = if cpu.carry, do: 0, else: 1
        result = cpu.accumulator + complement + borrow_in
        {%{cpu | accumulator: result &&& 0xF, carry: result > 0xF}, "SBM"}

      0xE9 -> # RDM: Read RAM main character
        val = ram_read_main(cpu)
        {%{cpu | accumulator: val}, "RDM"}

      0xEA -> # RDR: Read ROM I/O port
        {%{cpu | accumulator: cpu.rom_port &&& 0xF}, "RDR"}

      0xEB -> # ADM: Add RAM main character to accumulator with carry
        ram_val = ram_read_main(cpu)
        carry_in = if cpu.carry, do: 1, else: 0
        result = cpu.accumulator + ram_val + carry_in
        {%{cpu | accumulator: result &&& 0xF, carry: result > 0xF}, "ADM"}

      0xEC -> {%{cpu | accumulator: ram_read_status(cpu, 0)}, "RD0"}
      0xED -> {%{cpu | accumulator: ram_read_status(cpu, 1)}, "RD1"}
      0xEE -> {%{cpu | accumulator: ram_read_status(cpu, 2)}, "RD2"}
      0xEF -> {%{cpu | accumulator: ram_read_status(cpu, 3)}, "RD3"}

      _ -> {cpu, "UNKNOWN(0x#{Integer.to_string(raw, 16)})"}
    end
  end

  # --- Accumulator instructions (0xF0–0xFD) ---
  defp exec_accum(cpu, raw) do
    case raw do
      0xF0 -> # CLB: Clear both
        {%{cpu | accumulator: 0, carry: false}, "CLB"}

      0xF1 -> # CLC: Clear carry
        {%{cpu | carry: false}, "CLC"}

      0xF2 -> # IAC: Increment accumulator
        result = cpu.accumulator + 1
        {%{cpu | accumulator: result &&& 0xF, carry: result > 0xF}, "IAC"}

      0xF3 -> # CMC: Complement carry
        {%{cpu | carry: not cpu.carry}, "CMC"}

      0xF4 -> # CMA: Complement accumulator (4-bit NOT)
        {%{cpu | accumulator: Bitwise.bnot(cpu.accumulator) &&& 0xF}, "CMA"}

      0xF5 -> # RAL: Rotate accumulator left through carry
        # Before: [carry | A3 A2 A1 A0]
        # After:  [A3   | A2 A1 A0 old_carry]
        old_carry = if cpu.carry, do: 1, else: 0
        new_carry = (cpu.accumulator &&& 0x8) != 0
        new_a = ((cpu.accumulator <<< 1) ||| old_carry) &&& 0xF
        {%{cpu | accumulator: new_a, carry: new_carry}, "RAL"}

      0xF6 -> # RAR: Rotate accumulator right through carry
        # Before: [carry | A3 A2 A1 A0]
        # After:  [A0   | old_carry A3 A2 A1]
        old_carry = if cpu.carry, do: 1, else: 0
        new_carry = (cpu.accumulator &&& 0x1) != 0
        new_a = ((cpu.accumulator >>> 1) ||| (old_carry <<< 3)) &&& 0xF
        {%{cpu | accumulator: new_a, carry: new_carry}, "RAR"}

      0xF7 -> # TCC: Transfer carry to accumulator, clear carry
        val = if cpu.carry, do: 1, else: 0
        {%{cpu | accumulator: val, carry: false}, "TCC"}

      0xF8 -> # DAC: Decrement accumulator
        # carry = true if no borrow (A > 0 before), false if borrow (A was 0)
        new_carry = cpu.accumulator > 0
        new_a = (cpu.accumulator - 1) &&& 0xF
        {%{cpu | accumulator: new_a, carry: new_carry}, "DAC"}

      0xF9 -> # TCS: Transfer carry subtract
        # A = 10 if carry, 9 if not. Carry is always cleared.
        val = if cpu.carry, do: 10, else: 9
        {%{cpu | accumulator: val, carry: false}, "TCS"}

      0xFA -> # STC: Set carry
        {%{cpu | carry: true}, "STC"}

      0xFB -> # DAA: Decimal adjust accumulator
        # If A > 9 or carry, add 6. If overflows past 0xF, set carry.
        if cpu.accumulator > 9 or cpu.carry do
          result = cpu.accumulator + 6
          new_carry = if result > 0xF, do: true, else: cpu.carry
          {%{cpu | accumulator: result &&& 0xF, carry: new_carry}, "DAA"}
        else
          {cpu, "DAA"}
        end

      0xFC -> # KBP: Keyboard process (1-hot to binary)
        kbp_table = %{0 => 0, 1 => 1, 2 => 2, 4 => 3, 8 => 4}
        val = Map.get(kbp_table, cpu.accumulator, 15)
        {%{cpu | accumulator: val}, "KBP"}

      0xFD -> # DCL: Designate command line (select RAM bank)
        bank = cpu.accumulator &&& 0x7
        bank = if bank > 3, do: bank &&& 0x3, else: bank
        {%{cpu | ram_bank: bank}, "DCL"}

      _ -> {cpu, "UNKNOWN(0x#{Integer.to_string(raw, 16)})"}
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers — register pairs
  # ---------------------------------------------------------------------------

  defp read_pair(cpu, pair) do
    high_reg = pair * 2
    low_reg = high_reg + 1
    high = Enum.at(cpu.registers, high_reg)
    low = Enum.at(cpu.registers, low_reg)
    (high <<< 4) ||| low
  end

  defp write_pair(cpu, pair, value) do
    high_reg = pair * 2
    low_reg = high_reg + 1
    regs =
      cpu.registers
      |> List.replace_at(high_reg, (value >>> 4) &&& 0xF)
      |> List.replace_at(low_reg, value &&& 0xF)

    %{cpu | registers: regs}
  end

  # ---------------------------------------------------------------------------
  # Private helpers — stack
  # ---------------------------------------------------------------------------

  defp stack_push(cpu, addr) do
    stack = put_elem(cpu.hw_stack, cpu.stack_pointer, addr &&& 0xFFF)
    sp = rem(cpu.stack_pointer + 1, 3)
    %{cpu | hw_stack: stack, stack_pointer: sp}
  end

  defp stack_pop(cpu) do
    sp = rem(cpu.stack_pointer + 3 - 1, 3)
    addr = elem(cpu.hw_stack, sp)
    {%{cpu | stack_pointer: sp}, addr}
  end

  # ---------------------------------------------------------------------------
  # Private helpers — RAM
  # ---------------------------------------------------------------------------

  # RAM is a map: {bank, register, character} => value
  # This avoids nested list update complexity in Elixir.

  defp init_ram do
    for bank <- 0..3, reg <- 0..3, char <- 0..15, into: %{} do
      {{bank, reg, char}, 0}
    end
  end

  defp init_ram_status do
    for bank <- 0..3, reg <- 0..3, idx <- 0..3, into: %{} do
      {{bank, reg, idx}, 0}
    end
  end

  defp ram_read_main(cpu) do
    Map.get(cpu.ram, {cpu.ram_bank, cpu.ram_register, cpu.ram_character}, 0)
  end

  defp ram_write_main(cpu, value) do
    key = {cpu.ram_bank, cpu.ram_register, cpu.ram_character}
    %{cpu | ram: Map.put(cpu.ram, key, value &&& 0xF)}
  end

  defp ram_read_status(cpu, index) do
    Map.get(cpu.ram_status, {cpu.ram_bank, cpu.ram_register, index}, 0)
  end

  defp ram_write_status(cpu, index, value) do
    key = {cpu.ram_bank, cpu.ram_register, index}
    %{cpu | ram_status: Map.put(cpu.ram_status, key, value &&& 0xF)}
  end

  # ---------------------------------------------------------------------------
  # Private helpers — formatting
  # ---------------------------------------------------------------------------

  defp pad_hex(val, width) do
    val
    |> Integer.to_string(16)
    |> String.upcase()
    |> String.pad_leading(width, "0")
  end
end
