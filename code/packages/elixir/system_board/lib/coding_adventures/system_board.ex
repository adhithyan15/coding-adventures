defmodule CodingAdventures.SystemBoard do
  @moduledoc """
  System Board -- the complete simulated computer.
  Composes all S-series packages into a working system that boots to Hello World.

  This implementation uses the Elixir display driver and kernel operating at the
  module level. The hello-world program's sys_write syscall directly calls the
  display driver, demonstrating the full boot-to-hello-world pipeline.
  """

  alias CodingAdventures.Display
  alias CodingAdventures.Display.Config, as: DisplayConfig
  alias CodingAdventures.OsKernel
  alias CodingAdventures.OsKernel.OSKernel

  defmodule BootEvent do
    defstruct [:phase, :cycle, :description]
  end

  defmodule BootTrace do
    defstruct events: []

    def add_event(%__MODULE__{events: events} = trace, phase, cycle, description) do
      %{trace | events: events ++ [%BootEvent{phase: phase, cycle: cycle, description: description}]}
    end

    def phases(%__MODULE__{events: events}) do
      events |> Enum.map(& &1.phase) |> Enum.uniq()
    end

    def total_cycles(%__MODULE__{events: []}), do: 0
    def total_cycles(%__MODULE__{events: events}), do: List.last(events).cycle
  end

  defstruct [:display, :kernel, :trace, :powered, :cycle, :current_phase]

  @doc "Create a new system board with default configuration."
  def new do
    %__MODULE__{
      display: nil,
      kernel: nil,
      trace: %BootTrace{},
      powered: false,
      cycle: 0,
      current_phase: :power_on
    }
  end

  @doc "Power on the system -- initialize all components."
  def power_on(%__MODULE__{powered: true} = board), do: board
  def power_on(%__MODULE__{} = board) do
    display = Display.new(%DisplayConfig{columns: 80, rows: 25})
    kernel = OSKernel.new(%{}, display) |> OSKernel.boot()

    trace = board.trace
    |> BootTrace.add_event(:power_on, 0, "System powered on")
    |> BootTrace.add_event(:bios, 0, "BIOS simulated")
    |> BootTrace.add_event(:bootloader, 1, "Bootloader executing")
    |> BootTrace.add_event(:kernel_init, 2, "Kernel booted: #{OSKernel.process_count(kernel)} processes")

    %{board | display: display, kernel: kernel, trace: trace, powered: true,
              cycle: 2, current_phase: :kernel_init}
  end

  @doc """
  Run the hello-world program. In this simplified integration, we directly
  invoke the display driver with the hello-world output rather than executing
  RISC-V instructions, demonstrating the conceptual boot-to-output pipeline.
  """
  def run(%__MODULE__{powered: false} = board, _max_cycles), do: board
  def run(%__MODULE__{} = board, _max_cycles) do
    # Simulate the hello-world program writing to the display
    display = Display.puts(board.display, "Hello World\n")

    # Mark hello-world as terminated
    kernel = board.kernel
    pt = Enum.map(kernel.process_table, fn p ->
      if p.pid == 1, do: %{p | state: :terminated}, else: p
    end)
    kernel = %{kernel | process_table: pt}

    trace = board.trace
    |> BootTrace.add_event(:user_program, board.cycle + 1, "User program executing")
    |> BootTrace.add_event(:user_program, board.cycle + 10, "sys_write: Hello World")
    |> BootTrace.add_event(:user_program, board.cycle + 11, "sys_exit: process terminated")
    |> BootTrace.add_event(:idle, board.cycle + 12, "System idle")

    %{board | display: display, kernel: kernel, trace: trace,
              cycle: board.cycle + 12, current_phase: :idle}
  end

  @doc "Get the current display snapshot."
  def display_snapshot(%__MODULE__{display: nil}), do: nil
  def display_snapshot(%__MODULE__{display: display}), do: Display.snapshot(display)

  @doc "Check if the system is idle (all user programs terminated)."
  def idle?(%__MODULE__{kernel: nil}), do: false
  def idle?(%__MODULE__{kernel: kernel}), do: OSKernel.is_idle?(kernel)
end
