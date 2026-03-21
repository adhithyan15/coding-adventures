defmodule CodingAdventures.Core.MultiCore do
  @moduledoc """
  Connects multiple processor cores to shared resources.

  Each core has a private register file and pipeline. All cores share
  main memory via a MemoryController, and an InterruptController routes
  interrupts to specific cores.

  ## Execution Model

  All cores run on the same clock. Each call to `step/1` advances every core
  by one cycle. Cores are independent -- they do not share register files
  or pipeline state. They only interact through shared memory.

  ## Architecture Diagram

      Core 0: Pipeline + RegisterFile (private)
      Core 1: Pipeline + RegisterFile (private)
              |    |
         ==============================
         Memory Controller (serializes requests)
              |
         Shared Main Memory (DRAM)
  """

  alias CodingAdventures.Core.{MultiCoreConfig, InterruptController, MemoryController, Stats}
  alias CodingAdventures.Core.Core, as: CoreModule

  @type t :: %__MODULE__{
          config: MultiCoreConfig.t(),
          cores: [{CoreModule.t(), pid()}],
          shared_mem_ctrl: MemoryController.t(),
          interrupt_ctrl: InterruptController.t(),
          cycle: non_neg_integer()
        }

  defstruct config: nil,
            cores: [],
            shared_mem_ctrl: nil,
            interrupt_ctrl: nil,
            cycle: 0

  @doc """
  Creates a multi-core processor.

  All cores share the same main memory. Each core gets its own ISA decoder
  module (from the decoder_modules list). If length(decoder_modules) < num_cores,
  the last module is reused.

  Returns `{:ok, multi_core}` or `{:error, reason}`.
  """
  @spec new(MultiCoreConfig.t(), [module()]) :: {:ok, t()} | {:error, String.t()}
  def new(%MultiCoreConfig{} = config, decoder_modules) do
    num_cores = max(config.num_cores, 1)
    mem_size = if config.memory_size > 0, do: config.memory_size, else: 1_048_576
    mem_latency = if config.memory_latency > 0, do: config.memory_latency, else: 100

    # Create shared memory controller
    shared_mem = MemoryController.new(mem_size, mem_latency)

    # Create cores
    cores_result =
      Enum.reduce_while(0..(num_cores - 1), {:ok, []}, fn i, {:ok, acc} ->
        decoder = Enum.at(decoder_modules, i, List.last(decoder_modules))
        core_cfg = %{config.core_config | memory_size: mem_size, memory_latency: mem_latency}

        case CoreModule.new(core_cfg, decoder) do
          {:ok, core_tuple} ->
            {:cont, {:ok, acc ++ [core_tuple]}}
          {:error, reason} ->
            # Stop any already-created agents
            Enum.each(acc, fn ct -> CoreModule.stop(ct) end)
            {:halt, {:error, reason}}
        end
      end)

    case cores_result do
      {:ok, cores} ->
        mc = %__MODULE__{
          config: config,
          cores: cores,
          shared_mem_ctrl: shared_mem,
          interrupt_ctrl: InterruptController.new(num_cores),
          cycle: 0
        }
        {:ok, mc}

      {:error, _} = err ->
        err
    end
  end

  @doc """
  Loads a program into shared memory for a specific core.

  The program is written to the shared memory controller at the given address.
  The specified core's PC is set to start_address.
  """
  @spec load_program(t(), non_neg_integer(), [byte()], non_neg_integer()) :: t()
  def load_program(%__MODULE__{} = mc, core_id, program_bytes, start_address) do
    if core_id < 0 or core_id >= length(mc.cores) do
      mc
    else
      # Write to shared memory
      shared = MemoryController.load_program(mc.shared_mem_ctrl, program_bytes, start_address)

      # Also write to the core's own memory so it can read instructions
      core_tuple = Enum.at(mc.cores, core_id)
      core_tuple = CoreModule.load_program(core_tuple, program_bytes, start_address)

      cores = List.replace_at(mc.cores, core_id, core_tuple)
      %{mc | cores: cores, shared_mem_ctrl: shared}
    end
  end

  @doc """
  Advances all cores by one clock cycle.

  Returns `{updated_multi_core, snapshots}`.
  """
  @spec step(t()) :: {t(), [CodingAdventures.CpuPipeline.Snapshot.t()]}
  def step(%__MODULE__{} = mc) do
    mc = %{mc | cycle: mc.cycle + 1}

    {cores, snapshots} =
      Enum.map(mc.cores, fn core_tuple ->
        {core_tuple, snap} = CoreModule.step(core_tuple)
        {core_tuple, snap}
      end)
      |> Enum.unzip()

    # Tick shared memory controller
    {shared, _completed} = MemoryController.tick(mc.shared_mem_ctrl)

    %{mc | cores: cores, shared_mem_ctrl: shared}
    |> then(fn mc -> {mc, snapshots} end)
  end

  @doc """
  Runs all cores until all have halted or max_cycles is reached.

  Returns `{updated_multi_core, per_core_stats}`.
  """
  @spec run(t(), pos_integer()) :: {t(), [Stats.t()]}
  def run(%__MODULE__{} = mc, max_cycles) do
    mc = do_run(mc, max_cycles)
    {mc, stats(mc)}
  end

  defp do_run(%__MODULE__{cycle: cycle} = mc, max_cycles) when cycle >= max_cycles, do: mc

  defp do_run(%__MODULE__{} = mc, max_cycles) do
    if all_halted?(mc) do
      mc
    else
      {mc, _snaps} = step(mc)
      do_run(mc, max_cycles)
    end
  end

  @doc "Returns per-core statistics."
  @spec stats(t()) :: [Stats.t()]
  def stats(%__MODULE__{cores: cores}) do
    Enum.map(cores, fn {core, _agent} -> CoreModule.stats(core) end)
  end

  @doc "Returns true if every core has halted."
  @spec all_halted?(t()) :: boolean()
  def all_halted?(%__MODULE__{cores: cores}) do
    Enum.all?(cores, fn ct -> CoreModule.halted?(ct) end)
  end

  @doc "Returns the array of core tuples."
  @spec cores(t()) :: [{CoreModule.t(), pid()}]
  def cores(%__MODULE__{cores: cores}), do: cores

  @doc "Returns the interrupt controller."
  @spec interrupt_controller(t()) :: InterruptController.t()
  def interrupt_controller(%__MODULE__{interrupt_ctrl: ic}), do: ic

  @doc "Returns the shared memory controller."
  @spec shared_memory_controller(t()) :: MemoryController.t()
  def shared_memory_controller(%__MODULE__{shared_mem_ctrl: mc}), do: mc

  @doc "Returns the global cycle count."
  @spec cycle(t()) :: non_neg_integer()
  def cycle(%__MODULE__{cycle: cycle}), do: cycle

  @doc "Stops all core agents."
  @spec stop(t()) :: :ok
  def stop(%__MODULE__{cores: cores}) do
    Enum.each(cores, fn ct -> CoreModule.stop(ct) end)
    :ok
  end
end
