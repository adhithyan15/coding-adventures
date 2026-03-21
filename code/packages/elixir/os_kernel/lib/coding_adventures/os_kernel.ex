defmodule CodingAdventures.OsKernel do
  @moduledoc "Minimal OS kernel with process management, scheduler, and syscalls."

  @sys_exit 0
  @sys_write 1
  @sys_read 2
  @sys_yield 3
  @reg_a0 10
  @reg_a1 11
  @reg_a2 12
  @reg_a7 17
  @reg_sp 2

  def sys_exit, do: @sys_exit
  def sys_write, do: @sys_write
  def sys_yield, do: @sys_yield
  def reg_a0, do: @reg_a0
  def reg_a1, do: @reg_a1
  def reg_a2, do: @reg_a2
  def reg_a7, do: @reg_a7
  def reg_sp, do: @reg_sp

  defmodule Process do
    defstruct [:pid, :state, :saved_registers, :saved_pc, :stack_pointer,
               :memory_base, :memory_size, :name, exit_code: 0]
  end

  defmodule Scheduler do
    defstruct [:process_table, current: 0]

    def new(pt), do: %__MODULE__{process_table: pt}

    def schedule(%__MODULE__{process_table: pt, current: current}) do
      n = length(pt)
      if n == 0, do: 0, else: do_schedule(pt, current, n)
    end

    defp do_schedule(pt, current, n) do
      result = Enum.find(1..n, fn i ->
        idx = rem(current + i, n)
        Enum.at(pt, idx).state == :ready
      end)
      case result do
        nil -> if Enum.at(pt, current).state == :ready, do: current, else: 0
        i -> rem(current + i, n)
      end
    end

    def context_switch(%__MODULE__{process_table: pt} = s, from, to) do
      pt = Enum.with_index(pt) |> Enum.map(fn {p, i} ->
        cond do
          i == from and p.state == :running -> %{p | state: :ready}
          i == to -> %{p | state: :running}
          true -> p
        end
      end)
      %{s | process_table: pt, current: to}
    end
  end

  defmodule MemoryManager do
    defstruct [:regions]
    def new(regions), do: %__MODULE__{regions: regions}
    def find_region(%__MODULE__{regions: regions}, address) do
      Enum.find(regions, fn r -> address >= r.base and address < r.base + r.size end)
    end
    def region_count(%__MODULE__{regions: regions}), do: length(regions)
  end

  defmodule OSKernel do
    defstruct [:config, :process_table, :current_process, :scheduler, :memory_manager,
               :display, :keyboard_buffer, :booted, next_pid: 0]

    def new(config, display) do
      %__MODULE__{config: config, process_table: [], current_process: 0,
                  display: display, keyboard_buffer: [], booted: false}
    end

    def boot(%__MODULE__{} = k) do
      idle = %Process{pid: 0, state: :ready, saved_registers: List.duplicate(0, 32),
                      saved_pc: 0x00030000, stack_pointer: 0x0003FFF0,
                      memory_base: 0x00030000, memory_size: 0x10000, name: "idle"}
      hw = %Process{pid: 1, state: :running, saved_registers: List.duplicate(0, 32),
                    saved_pc: 0x00040000, stack_pointer: 0x0004FFF0,
                    memory_base: 0x00040000, memory_size: 0x10000, name: "hello-world"}
      pt = [idle, hw]
      sched = Scheduler.new(pt)
      %{k | process_table: pt, current_process: 1, scheduler: %{sched | current: 1},
            booted: true, next_pid: 2}
    end

    def process_count(%__MODULE__{process_table: pt}), do: length(pt)

    def is_idle?(%__MODULE__{process_table: pt}) do
      Enum.all?(pt, fn p -> p.pid == 0 or p.state == :terminated end)
    end

    def get_current_pcb(%__MODULE__{process_table: pt, current_process: cp}) do
      if cp >= 0 and cp < length(pt), do: Enum.at(pt, cp), else: nil
    end
  end

  alias CodingAdventures.RiscvSimulator.Encoding

  def generate_idle_program do
    Encoding.assemble([
      Encoding.encode_addi(@reg_a7, 0, @sys_yield),
      Encoding.encode_ecall(),
      Encoding.encode_jal(0, -8)
    ])
  end

  def generate_hello_world_program(mem_base) do
    import Bitwise
    data_offset = 0x100
    data_addr = mem_base + data_offset
    message = "Hello World\n"
    msg_len = String.length(message)

    upper = data_addr >>> 12
    lower = data_addr &&& 0xFFF
    upper = if lower >= 0x800, do: upper + 1, else: upper

    instructions = [Encoding.encode_lui(@reg_a1, upper)]
    instructions = if lower != 0 do
      sl = if lower >= 0x800, do: lower - 0x1000, else: lower
      instructions ++ [Encoding.encode_addi(@reg_a1, @reg_a1, sl)]
    else
      instructions
    end

    instructions = instructions ++ [
      Encoding.encode_addi(@reg_a0, 0, 1),
      Encoding.encode_addi(@reg_a2, 0, msg_len),
      Encoding.encode_addi(@reg_a7, 0, @sys_write),
      Encoding.encode_ecall(),
      Encoding.encode_addi(@reg_a0, 0, 0),
      Encoding.encode_addi(@reg_a7, 0, @sys_exit),
      Encoding.encode_ecall()
    ]

    code = Encoding.assemble(instructions)
    msg_bytes = String.to_charlist(message)
    padding = List.duplicate(0, data_offset - length(code))
    code ++ padding ++ msg_bytes
  end
end
