defmodule CodingAdventures.CpuPipeline.Pipeline do
  @moduledoc """
  A configurable N-stage instruction pipeline.

  ## How it Works

  The pipeline is a list of "slots", one per stage. Each slot holds a
  token (or nil if the stage is empty). On each clock cycle (call to
  `step/1`):

    1. Check for hazards (via hazard callback)
    2. If stalled: freeze stages before the stall point, insert bubble
    3. If flushing: replace speculative stages with bubbles
    4. Otherwise: shift all tokens one stage forward
    5. Execute stage callbacks (fetch, decode, execute, memory, writeback)
    6. Record a snapshot for tracing

  Since Elixir is functional, the pipeline takes state as input and returns
  new state -- there is no mutation. Each call to `step/1` returns an
  updated pipeline struct along with the snapshot for that cycle.

  ## Pipeline Registers

  In real hardware, pipeline registers sit BETWEEN stages and latch
  data on the clock edge. In our model, we represent this by computing
  the new state of all stages before committing any changes. The "stages"
  list IS the set of pipeline registers.
  """

  alias CodingAdventures.CpuPipeline.{
    Token,
    PipelineStage,
    PipelineConfig,
    PipelineStats,
    Snapshot,
    HazardResponse
  }

  @type fetch_fn :: (integer() -> integer())
  @type decode_fn :: (integer(), Token.t() -> Token.t())
  @type execute_fn :: (Token.t() -> Token.t())
  @type memory_fn :: (Token.t() -> Token.t())
  @type writeback_fn :: (Token.t() -> :ok)
  @type hazard_fn :: ([Token.t() | nil] -> HazardResponse.t()) | nil
  @type predict_fn :: (integer() -> integer()) | nil

  @type t :: %__MODULE__{
          config: PipelineConfig.t(),
          stages: [Token.t() | nil],
          pc: integer(),
          cycle: integer(),
          halted: boolean(),
          stats: PipelineStats.t(),
          history: [Snapshot.t()],
          fetch_fn: fetch_fn(),
          decode_fn: decode_fn(),
          execute_fn: execute_fn(),
          memory_fn: memory_fn(),
          writeback_fn: writeback_fn(),
          hazard_fn: hazard_fn(),
          predict_fn: predict_fn()
        }

  defstruct config: %PipelineConfig{},
            stages: [],
            pc: 0,
            cycle: 0,
            halted: false,
            stats: %PipelineStats{},
            history: [],
            fetch_fn: nil,
            decode_fn: nil,
            execute_fn: nil,
            memory_fn: nil,
            writeback_fn: nil,
            hazard_fn: nil,
            predict_fn: nil

  # =========================================================================
  # Configuration Presets
  # =========================================================================

  @doc """
  Returns the standard 5-stage RISC pipeline configuration.

  This is the pipeline described in every computer architecture textbook:

      IF -> ID -> EX -> MEM -> WB

  It matches the MIPS R2000 (1985) and is the foundation for understanding
  all modern CPU pipelines.
  """
  @spec classic_5_stage() :: PipelineConfig.t()
  def classic_5_stage do
    %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF", description: "Instruction Fetch", category: :fetch},
        %PipelineStage{name: "ID", description: "Instruction Decode", category: :decode},
        %PipelineStage{name: "EX", description: "Execute", category: :execute},
        %PipelineStage{name: "MEM", description: "Memory Access", category: :memory},
        %PipelineStage{name: "WB", description: "Write Back", category: :writeback}
      ],
      execution_width: 1
    }
  end

  @doc """
  Returns a 13-stage pipeline inspired by ARM Cortex-A78.

  Modern high-performance CPUs split the classic 5 stages into many
  sub-stages to enable higher clock frequencies. The tradeoff: a branch
  misprediction now costs 10+ cycles instead of 2.
  """
  @spec deep_13_stage() :: PipelineConfig.t()
  def deep_13_stage do
    %PipelineConfig{
      stages: [
        %PipelineStage{name: "IF1", description: "Fetch 1 - TLB lookup", category: :fetch},
        %PipelineStage{name: "IF2", description: "Fetch 2 - cache read", category: :fetch},
        %PipelineStage{name: "IF3", description: "Fetch 3 - align/buffer", category: :fetch},
        %PipelineStage{name: "ID1", description: "Decode 1 - pre-decode", category: :decode},
        %PipelineStage{name: "ID2", description: "Decode 2 - full decode", category: :decode},
        %PipelineStage{name: "ID3", description: "Decode 3 - register read", category: :decode},
        %PipelineStage{name: "EX1", description: "Execute 1 - ALU", category: :execute},
        %PipelineStage{name: "EX2", description: "Execute 2 - shift/multiply", category: :execute},
        %PipelineStage{name: "EX3", description: "Execute 3 - result select", category: :execute},
        %PipelineStage{name: "MEM1", description: "Memory 1 - address calc", category: :memory},
        %PipelineStage{name: "MEM2", description: "Memory 2 - cache access", category: :memory},
        %PipelineStage{name: "MEM3", description: "Memory 3 - data align", category: :memory},
        %PipelineStage{name: "WB", description: "Write Back", category: :writeback}
      ],
      execution_width: 1
    }
  end

  # =========================================================================
  # Constructor
  # =========================================================================

  @doc """
  Creates a new pipeline with the given configuration and callbacks.

  The configuration is validated before use. All five stage callbacks are
  required; hazard and predict callbacks are optional (set with
  `set_hazard_func/2` and `set_predict_func/2`).

  Returns `{:ok, pipeline}` or `{:error, reason}`.
  """
  @spec new(PipelineConfig.t(), fetch_fn(), decode_fn(), execute_fn(), memory_fn(), writeback_fn()) ::
          {:ok, t()} | {:error, String.t()}
  def new(config, fetch_fn, decode_fn, execute_fn, memory_fn, writeback_fn) do
    case PipelineConfig.validate(config) do
      :ok ->
        pipeline = %__MODULE__{
          config: config,
          stages: List.duplicate(nil, PipelineConfig.num_stages(config)),
          pc: 0,
          cycle: 0,
          halted: false,
          stats: %PipelineStats{},
          history: [],
          fetch_fn: fetch_fn,
          decode_fn: decode_fn,
          execute_fn: execute_fn,
          memory_fn: memory_fn,
          writeback_fn: writeback_fn
        }

        {:ok, pipeline}

      {:error, _} = err ->
        err
    end
  end

  @doc "Sets the optional hazard detection callback."
  @spec set_hazard_func(t(), hazard_fn()) :: t()
  def set_hazard_func(%__MODULE__{} = pipeline, func) do
    %{pipeline | hazard_fn: func}
  end

  @doc "Sets the optional branch prediction callback."
  @spec set_predict_func(t(), predict_fn()) :: t()
  def set_predict_func(%__MODULE__{} = pipeline, func) do
    %{pipeline | predict_fn: func}
  end

  @doc "Sets the program counter."
  @spec set_pc(t(), integer()) :: t()
  def set_pc(%__MODULE__{} = pipeline, pc) do
    %{pipeline | pc: pc}
  end

  @doc "Returns the current program counter."
  @spec pc(t()) :: integer()
  def pc(%__MODULE__{pc: pc}), do: pc

  @doc "Returns true if a halt instruction has reached the last stage."
  @spec halted?(t()) :: boolean()
  def halted?(%__MODULE__{halted: halted}), do: halted

  @doc "Returns the current cycle number."
  @spec cycle(t()) :: integer()
  def cycle(%__MODULE__{cycle: cycle}), do: cycle

  @doc "Returns a copy of the current execution statistics."
  @spec stats(t()) :: PipelineStats.t()
  def stats(%__MODULE__{stats: stats}), do: stats

  @doc "Returns the pipeline configuration."
  @spec config(t()) :: PipelineConfig.t()
  def config(%__MODULE__{config: config}), do: config

  @doc "Returns the complete history of pipeline snapshots."
  @spec trace(t()) :: [Snapshot.t()]
  def trace(%__MODULE__{history: history}), do: Enum.reverse(history)

  @doc """
  Returns the token currently occupying the given stage.

  Returns nil if the stage is empty or the stage name is invalid.
  """
  @spec stage_contents(t(), String.t()) :: Token.t() | nil
  def stage_contents(%__MODULE__{config: config, stages: stages}, stage_name) do
    idx = Enum.find_index(config.stages, fn s -> s.name == stage_name end)
    if idx, do: Enum.at(stages, idx), else: nil
  end

  @doc "Returns the current pipeline state without advancing the clock."
  @spec snapshot(t()) :: Snapshot.t()
  def snapshot(%__MODULE__{} = pipeline), do: take_snapshot(pipeline)

  # =========================================================================
  # Step -- advance the pipeline by one clock cycle
  # =========================================================================

  @doc """
  Advances the pipeline by one clock cycle.

  This is the heart of the pipeline simulator. Each call to `step/1`
  corresponds to one rising clock edge in hardware.

  Returns `{updated_pipeline, snapshot}`.
  """
  @spec step(t()) :: {t(), Snapshot.t()}
  def step(%__MODULE__{halted: true} = pipeline) do
    snap = take_snapshot(pipeline)
    {pipeline, snap}
  end

  def step(%__MODULE__{} = pipeline) do
    pipeline = %{pipeline | cycle: pipeline.cycle + 1}
    pipeline = update_in_stats(pipeline, :total_cycles, &(&1 + 1))

    num_stages = PipelineConfig.num_stages(pipeline.config)

    # --- Phase 1: Check for hazards ---
    hazard =
      if pipeline.hazard_fn do
        pipeline.hazard_fn.(pipeline.stages)
      else
        %HazardResponse{action: :none}
      end

    # --- Phase 2: Compute next state ---
    {next_stages, pipeline, stalled, flushing} =
      compute_next_state(pipeline, hazard, num_stages)

    # --- Phase 3: Commit the new state ---
    pipeline = %{pipeline | stages: next_stages}

    # --- Phase 4: Execute stage callbacks ---
    pipeline = execute_stage_callbacks(pipeline, num_stages)

    # --- Phase 5: Retire the instruction in the last stage ---
    pipeline = retire_last_stage(pipeline, num_stages)

    # --- Phase 5b: Count bubbles ---
    bubble_count = Enum.count(pipeline.stages, fn tok -> tok != nil and tok.is_bubble end)
    pipeline = update_in_stats(pipeline, :bubble_cycles, &(&1 + bubble_count))

    # --- Phase 6: Take snapshot ---
    snap = %Snapshot{
      cycle: pipeline.cycle,
      stages: build_stage_map(pipeline),
      stalled: stalled,
      flushing: flushing,
      pc: pipeline.pc
    }

    pipeline = %{pipeline | history: [snap | pipeline.history]}

    {pipeline, snap}
  end

  @doc """
  Runs the pipeline until a halt instruction is encountered or the
  maximum cycle count is reached.

  Returns `{updated_pipeline, final_stats}`.
  """
  @spec run(t(), integer()) :: {t(), PipelineStats.t()}
  def run(%__MODULE__{} = pipeline, max_cycles) do
    pipeline = do_run(pipeline, max_cycles)
    {pipeline, pipeline.stats}
  end

  defp do_run(%__MODULE__{cycle: cycle} = pipeline, max_cycles) when cycle >= max_cycles do
    pipeline
  end

  defp do_run(%__MODULE__{halted: true} = pipeline, _max_cycles), do: pipeline

  defp do_run(%__MODULE__{} = pipeline, max_cycles) do
    {pipeline, _snap} = step(pipeline)
    do_run(pipeline, max_cycles)
  end

  # =========================================================================
  # Internal: compute_next_state
  # =========================================================================

  defp compute_next_state(pipeline, %HazardResponse{action: :flush} = hazard, num_stages) do
    pipeline = update_in_stats(pipeline, :flush_cycles, &(&1 + 1))

    # Determine how many stages to flush (from the front).
    flush_count = determine_flush_count(hazard, pipeline.config, num_stages)

    # Build next stages: shift non-flushed stages forward, fill flushed with bubbles.
    stages_list = pipeline.config.stages
    cycle = pipeline.cycle

    next_stages =
      for i <- 0..(num_stages - 1) do
        cond do
          i < flush_count ->
            # Flushed stage: bubble
            bubble = Token.new_bubble()
            %{bubble | stage_entered: Map.put(bubble.stage_entered, Enum.at(stages_list, i).name, cycle)}

          i > 0 and (i - 1) >= flush_count ->
            # Shift from previous stage
            Enum.at(pipeline.stages, i - 1)

          i > 0 ->
            # Boundary: insert bubble
            bubble = Token.new_bubble()
            %{bubble | stage_entered: Map.put(bubble.stage_entered, Enum.at(stages_list, i).name, cycle)}

          true ->
            # i == 0, keep current
            Enum.at(pipeline.stages, i)
        end
      end

    # Redirect PC and fetch from the correct target.
    pipeline = %{pipeline | pc: hazard.redirect_pc}
    tok = fetch_new_instruction(pipeline)

    next_stages = List.replace_at(next_stages, 0, tok)

    # Advance PC after fetch
    pipeline = advance_pc(pipeline)

    {next_stages, pipeline, false, true}
  end

  defp compute_next_state(pipeline, %HazardResponse{action: :stall} = hazard, num_stages) do
    pipeline = update_in_stats(pipeline, :stall_cycles, &(&1 + 1))

    # Find the stall insertion point.
    stall_point = determine_stall_point(hazard, pipeline.config, num_stages)

    stages_list = pipeline.config.stages
    cycle = pipeline.cycle

    next_stages =
      for i <- 0..(num_stages - 1) do
        cond do
          i > stall_point ->
            # Advance normally
            Enum.at(pipeline.stages, i - 1)

          i == stall_point ->
            # Insert bubble
            bubble = Token.new_bubble()
            %{bubble | stage_entered: Map.put(bubble.stage_entered, Enum.at(stages_list, i).name, cycle)}

          true ->
            # Frozen
            Enum.at(pipeline.stages, i)
        end
      end

    # PC does NOT advance during a stall.
    {next_stages, pipeline, true, false}
  end

  defp compute_next_state(pipeline, %HazardResponse{action: action} = hazard, num_stages)
       when action in [:forward_from_ex, :forward_from_mem, :none] do
    # Handle forwarding if needed.
    pipeline =
      if action in [:forward_from_ex, :forward_from_mem] do
        apply_forwarding(pipeline, hazard)
      else
        pipeline
      end

    # Shift tokens forward (from back to front).
    next_stages =
      for i <- 0..(num_stages - 1) do
        if i > 0 do
          Enum.at(pipeline.stages, i - 1)
        else
          nil
        end
      end

    # Fetch new instruction into IF stage.
    tok = fetch_new_instruction(pipeline)
    next_stages = List.replace_at(next_stages, 0, tok)

    # Advance PC after fetch
    pipeline = advance_pc(pipeline)

    {next_stages, pipeline, false, false}
  end

  # =========================================================================
  # Internal helpers
  # =========================================================================

  defp determine_flush_count(%HazardResponse{flush_count: fc}, config, num_stages) when fc > 0 do
    min(fc, num_stages)
  end

  defp determine_flush_count(_hazard, config, num_stages) do
    idx =
      Enum.find_index(config.stages, fn s -> s.category == :execute end)

    flush_count = if idx && idx > 0, do: idx, else: 1
    min(flush_count, num_stages)
  end

  defp determine_stall_point(%HazardResponse{stall_stages: ss}, config, num_stages) when ss > 0 do
    min(ss, num_stages - 1)
  end

  defp determine_stall_point(_hazard, config, num_stages) do
    idx =
      Enum.find_index(config.stages, fn s -> s.category == :execute end)

    stall_point = if idx && idx > 0, do: idx, else: 1
    min(stall_point, num_stages - 1)
  end

  defp fetch_new_instruction(%__MODULE__{} = pipeline) do
    tok = Token.new()
    tok = %{tok | pc: pipeline.pc, raw_instruction: pipeline.fetch_fn.(pipeline.pc)}
    stage_name = Enum.at(pipeline.config.stages, 0).name
    %{tok | stage_entered: Map.put(tok.stage_entered, stage_name, pipeline.cycle)}
  end

  defp advance_pc(%__MODULE__{predict_fn: nil} = pipeline) do
    %{pipeline | pc: pipeline.pc + 4}
  end

  defp advance_pc(%__MODULE__{predict_fn: predict_fn} = pipeline) do
    %{pipeline | pc: predict_fn.(pipeline.pc)}
  end

  defp apply_forwarding(%__MODULE__{} = pipeline, hazard) do
    stages = pipeline.config.stages
    updated_pipeline_stages =
      Enum.with_index(pipeline.stages)
      |> Enum.map(fn {tok, i} ->
        stage = Enum.at(stages, i)

        if stage.category == :decode && tok != nil && !tok.is_bubble do
          %{tok | alu_result: hazard.forward_value, forwarded_from: hazard.forward_source}
        else
          tok
        end
      end)

    %{pipeline | stages: updated_pipeline_stages}
  end

  defp execute_stage_callbacks(%__MODULE__{} = pipeline, num_stages) do
    # Execute from last to first (matching Go implementation).
    Enum.reduce((num_stages - 1)..0//-1, pipeline, fn i, acc ->
      tok = Enum.at(acc.stages, i)

      if tok == nil || tok.is_bubble do
        acc
      else
        stage = Enum.at(acc.config.stages, i)

        # Record when this token entered this stage.
        tok =
          if not Map.has_key?(tok.stage_entered, stage.name) do
            %{tok | stage_entered: Map.put(tok.stage_entered, stage.name, acc.cycle)}
          else
            tok
          end

        acc = %{acc | stages: List.replace_at(acc.stages, i, tok)}

        case stage.category do
          :fetch ->
            # Already handled by fetch_new_instruction.
            acc

          :decode ->
            if tok.opcode == "" do
              decoded = acc.decode_fn.(tok.raw_instruction, tok)
              %{acc | stages: List.replace_at(acc.stages, i, decoded)}
            else
              acc
            end

          :execute ->
            if Map.get(tok.stage_entered, stage.name) == acc.cycle do
              executed = acc.execute_fn.(tok)
              %{acc | stages: List.replace_at(acc.stages, i, executed)}
            else
              acc
            end

          :memory ->
            if Map.get(tok.stage_entered, stage.name) == acc.cycle do
              result = acc.memory_fn.(tok)
              %{acc | stages: List.replace_at(acc.stages, i, result)}
            else
              acc
            end

          :writeback ->
            # Writeback handled in retire_last_stage.
            acc
        end
      end
    end)
  end

  defp retire_last_stage(%__MODULE__{} = pipeline, num_stages) do
    last_tok = Enum.at(pipeline.stages, num_stages - 1)

    if last_tok != nil and not last_tok.is_bubble do
      pipeline.writeback_fn.(last_tok)
      pipeline = update_in_stats(pipeline, :instructions_completed, &(&1 + 1))

      if last_tok.is_halt do
        %{pipeline | halted: true}
      else
        pipeline
      end
    else
      pipeline
    end
  end

  defp take_snapshot(%__MODULE__{} = pipeline) do
    %Snapshot{
      cycle: pipeline.cycle,
      stages: build_stage_map(pipeline),
      pc: pipeline.pc
    }
  end

  defp build_stage_map(%__MODULE__{config: config, stages: stages}) do
    Enum.zip(config.stages, stages)
    |> Enum.reduce(%{}, fn {stage_def, tok}, acc ->
      if tok != nil do
        Map.put(acc, stage_def.name, Token.clone(tok))
      else
        acc
      end
    end)
  end

  defp update_in_stats(%__MODULE__{stats: stats} = pipeline, field, fun) do
    %{pipeline | stats: Map.update!(stats, field, fun)}
  end
end
