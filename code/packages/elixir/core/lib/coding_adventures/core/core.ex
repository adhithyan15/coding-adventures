defmodule CodingAdventures.Core.Core do
  @moduledoc """
  A complete processor core that composes all D-series sub-components.

  The Core wires together:
    - Pipeline (D04): manages instruction flow through stages
    - Register File: fast operand storage
    - Memory Controller: access to backing memory
    - ISA Decoder: instruction semantics (injected via behaviour)

  The Core provides callback functions to the pipeline. When the pipeline
  needs to fetch an instruction, it calls the Core's fetch callback, which
  reads from memory. When it needs to decode, it calls the ISA decoder.

  ## Functional Design

  Since Elixir is functional, the Core is immutable. Each call to `step/1`
  returns a new Core struct with updated state.
  """

  alias CodingAdventures.CpuPipeline.{Pipeline, PipelineConfig, PipelineStats}
  alias CodingAdventures.Core.{Config, RegisterFile, MemoryController, Stats}

  @type t :: %__MODULE__{
          config: Config.t(),
          decoder_module: module(),
          pipeline: Pipeline.t(),
          reg_file: RegisterFile.t(),
          mem_ctrl: MemoryController.t(),
          halted: boolean(),
          cycle: non_neg_integer(),
          instructions_completed: non_neg_integer()
        }

  defstruct config: nil,
            decoder_module: nil,
            pipeline: nil,
            reg_file: nil,
            mem_ctrl: nil,
            halted: false,
            cycle: 0,
            instructions_completed: 0

  @doc """
  Creates a fully-wired processor core from the given configuration and
  ISA decoder module.

  The decoder_module must implement the `CodingAdventures.Core.Decoder` behaviour.

  Returns `{:ok, core}` or `{:error, reason}`.
  """
  @spec new(Config.t(), module()) :: {:ok, t()} | {:error, String.t()}
  def new(%Config{} = config, decoder_module) do
    # 1. Register File
    reg_file = RegisterFile.new(config.register_file)

    # 2. Memory
    mem_size = if config.memory_size > 0, do: config.memory_size, else: 65536
    mem_latency = if config.memory_latency > 0, do: config.memory_latency, else: 100
    mem_ctrl = MemoryController.new(mem_size, mem_latency)

    # 3. Pipeline config
    pipeline_config = config.pipeline || Pipeline.classic_5_stage()

    # Build a core struct first (without pipeline) so callbacks can reference it.
    core = %__MODULE__{
      config: config,
      decoder_module: decoder_module,
      reg_file: reg_file,
      mem_ctrl: mem_ctrl
    }

    # 4. Create pipeline with callbacks that close over the core's mutable state.
    # Since Elixir is functional, we use Agents or pass state through.
    # For this simulator, callbacks capture the core's Agent PID for state access.
    #
    # DESIGN CHOICE: We use an Agent to hold the core state so that pipeline
    # callbacks can read/write the register file and memory. The pipeline calls
    # step(), which updates the core state inside the Agent.
    {:ok, agent} = Agent.start_link(fn -> core end)

    fetch_fn = fn pc ->
      Agent.get(agent, fn c -> MemoryController.read_word(c.mem_ctrl, pc) end)
    end

    decode_fn = fn raw, token ->
      decoder_module.decode(raw, token)
    end

    execute_fn = fn token ->
      Agent.get(agent, fn c ->
        decoder_module.execute(token, c.reg_file)
      end)
    end

    memory_fn = fn token ->
      if token.mem_read do
        mem_data = Agent.get(agent, fn c -> MemoryController.read_word(c.mem_ctrl, token.alu_result) end)
        %{token | mem_data: mem_data, write_data: mem_data}
      else
        if token.mem_write do
          Agent.update(agent, fn c ->
            %{c | mem_ctrl: MemoryController.write_word(c.mem_ctrl, token.alu_result, token.write_data)}
          end)
        end
        token
      end
    end

    writeback_fn = fn token ->
      if token.reg_write and token.rd >= 0 do
        Agent.update(agent, fn c ->
          %{c | reg_file: RegisterFile.write(c.reg_file, token.rd, token.write_data)}
        end)
      end
      :ok
    end

    case Pipeline.new(pipeline_config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn) do
      {:ok, pipeline} ->
        # Set predict callback (default: PC + instruction_size).
        pipeline = Pipeline.set_predict_func(pipeline, fn pc ->
          pc + decoder_module.instruction_size()
        end)

        core = %{core | pipeline: pipeline}
        Agent.update(agent, fn _ -> core end)

        # Store the agent PID in the core so we can access it later.
        core = %{core | pipeline: pipeline}
        {:ok, {core, agent}}

      {:error, _} = err ->
        Agent.stop(agent)
        err
    end
  end

  @doc """
  Loads machine code into memory starting at the given address.
  Sets the PC to start_address.
  """
  @spec load_program({t(), pid()}, [byte()], non_neg_integer()) :: {t(), pid()}
  def load_program({core, agent}, program_bytes, start_address) do
    Agent.update(agent, fn c ->
      %{c | mem_ctrl: MemoryController.load_program(c.mem_ctrl, program_bytes, start_address)}
    end)

    pipeline = Pipeline.set_pc(core.pipeline, start_address)
    core = %{core | pipeline: pipeline}
    Agent.update(agent, fn c -> %{c | pipeline: pipeline} end)

    {core, agent}
  end

  @doc """
  Executes one clock cycle.

  Returns `{updated_core_tuple, pipeline_snapshot}`.
  """
  @spec step({t(), pid()}) :: {{t(), pid()}, CodingAdventures.CpuPipeline.Snapshot.t()}
  def step({%__MODULE__{halted: true} = core, agent}) do
    snap = Pipeline.snapshot(core.pipeline)
    {{core, agent}, snap}
  end

  def step({%__MODULE__{} = core, agent}) do
    core = %{core | cycle: core.cycle + 1}

    {pipeline, snap} = Pipeline.step(core.pipeline)
    core = %{core | pipeline: pipeline}

    if Pipeline.halted?(pipeline) do
      core = %{core | halted: true}
    end

    core = %{core |
      halted: Pipeline.halted?(pipeline),
      instructions_completed: Pipeline.stats(pipeline).instructions_completed
    }

    Agent.update(agent, fn c -> %{c | pipeline: pipeline} end)

    {{core, agent}, snap}
  end

  @doc """
  Runs the core until it halts or max_cycles is reached.

  Returns `{updated_core_tuple, core_stats}`.
  """
  @spec run({t(), pid()}, pos_integer()) :: {{t(), pid()}, Stats.t()}
  def run(core_tuple, max_cycles) do
    core_tuple = do_run(core_tuple, max_cycles)
    {core, _agent} = core_tuple
    {core_tuple, stats(core)}
  end

  defp do_run({%__MODULE__{cycle: cycle}, _} = ct, max_cycles) when cycle >= max_cycles, do: ct
  defp do_run({%__MODULE__{halted: true}, _} = ct, _max_cycles), do: ct
  defp do_run(core_tuple, max_cycles) do
    {core_tuple, _snap} = step(core_tuple)
    do_run(core_tuple, max_cycles)
  end

  @doc "Returns aggregate statistics."
  @spec stats(t()) :: Stats.t()
  def stats(%__MODULE__{} = core) do
    p_stats = Pipeline.stats(core.pipeline)

    %Stats{
      instructions_completed: p_stats.instructions_completed,
      total_cycles: p_stats.total_cycles,
      pipeline_stats: p_stats
    }
  end

  @doc "Returns true if a halt instruction has completed."
  @spec halted?(t() | {t(), pid()}) :: boolean()
  def halted?({%__MODULE__{halted: h}, _}), do: h
  def halted?(%__MODULE__{halted: h}), do: h

  @doc "Returns the current cycle number."
  @spec cycle(t() | {t(), pid()}) :: non_neg_integer()
  def cycle({%__MODULE__{cycle: c}, _}), do: c
  def cycle(%__MODULE__{cycle: c}), do: c

  @doc "Reads a general-purpose register."
  @spec read_register({t(), pid()}, integer()) :: integer()
  def read_register({_core, agent}, index) do
    Agent.get(agent, fn c -> RegisterFile.read(c.reg_file, index) end)
  end

  @doc "Writes a general-purpose register."
  @spec write_register({t(), pid()}, integer(), integer()) :: {t(), pid()}
  def write_register({core, agent}, index, value) do
    Agent.update(agent, fn c ->
      %{c | reg_file: RegisterFile.write(c.reg_file, index, value)}
    end)
    {core, agent}
  end

  @doc "Returns the core configuration."
  @spec config(t()) :: Config.t()
  def config(%__MODULE__{config: config}), do: config

  @doc "Returns the memory controller."
  @spec memory_controller({t(), pid()}) :: MemoryController.t()
  def memory_controller({_core, agent}) do
    Agent.get(agent, fn c -> c.mem_ctrl end)
  end

  @doc "Stops the agent backing this core."
  @spec stop({t(), pid()}) :: :ok
  def stop({_core, agent}), do: Agent.stop(agent)
end
