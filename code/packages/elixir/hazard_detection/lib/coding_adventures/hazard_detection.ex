defmodule CodingAdventures.HazardDetection do
  @moduledoc """
  Hazard detectors for a classic pipelined CPU.
  """

  alias CodingAdventures.HazardResult
  alias CodingAdventures.PipelineSlot

  def slot(attrs \\ []), do: struct(PipelineSlot, attrs)
  def empty_slot, do: %PipelineSlot{}

  defmodule DataHazardDetector do
    @moduledoc """
    Detects RAW hazards and resolves them by forwarding or stalling.
    """

    alias CodingAdventures.HazardResult
    alias CodingAdventures.PipelineSlot

    @spec detect(PipelineSlot.t(), PipelineSlot.t(), PipelineSlot.t()) :: HazardResult.t()
    def detect(%PipelineSlot{} = id_stage, %PipelineSlot{} = ex_stage, %PipelineSlot{} = mem_stage) do
      cond do
        not id_stage.valid ->
          %HazardResult{reason: "ID stage is empty (bubble)"}

        id_stage.source_regs == [] ->
          %HazardResult{reason: "instruction has no source registers"}

        true ->
          Enum.reduce(id_stage.source_regs, %HazardResult{reason: "no data dependencies detected"}, fn src_reg, worst ->
            pick_higher_priority(worst, check_single_register(src_reg, ex_stage, mem_stage))
          end)
      end
    end

    defp check_single_register(src_reg, %PipelineSlot{} = ex_stage, %PipelineSlot{} = mem_stage) do
      cond do
        ex_stage.valid and ex_stage.dest_reg == src_reg and ex_stage.mem_read ->
          %HazardResult{
            action: :stall,
            stall_cycles: 1,
            reason:
              "load-use hazard: R#{src_reg} is being loaded by instruction at PC=0x#{hex4(ex_stage.pc)} — must stall 1 cycle"
          }

        ex_stage.valid and ex_stage.dest_reg == src_reg ->
          %HazardResult{
            action: :forward_ex,
            forwarded_value: ex_stage.dest_value,
            forwarded_from: "EX",
            reason:
              "RAW hazard on R#{src_reg}: forwarding value #{inspect(ex_stage.dest_value)} from EX stage (instruction at PC=0x#{hex4(ex_stage.pc)})"
          }

        mem_stage.valid and mem_stage.dest_reg == src_reg ->
          %HazardResult{
            action: :forward_mem,
            forwarded_value: mem_stage.dest_value,
            forwarded_from: "MEM",
            reason:
              "RAW hazard on R#{src_reg}: forwarding value #{inspect(mem_stage.dest_value)} from MEM stage (instruction at PC=0x#{hex4(mem_stage.pc)})"
          }

        true ->
          %HazardResult{reason: "R#{src_reg} has no pending writes in EX or MEM"}
      end
    end

    defp pick_higher_priority(%HazardResult{} = a, %HazardResult{} = b) do
      if priority(b.action) > priority(a.action), do: b, else: a
    end

    defp priority(:none), do: 0
    defp priority(:forward_mem), do: 1
    defp priority(:forward_ex), do: 2
    defp priority(:stall), do: 3
    defp priority(:flush), do: 4

    defp hex4(value), do: value |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(4, "0")
  end

  defmodule ControlHazardDetector do
    @moduledoc """
    Detects branch mispredictions in EX and flushes IF/ID when needed.
    """

    alias CodingAdventures.HazardResult
    alias CodingAdventures.PipelineSlot

    @spec detect(PipelineSlot.t()) :: HazardResult.t()
    def detect(%PipelineSlot{} = ex_stage) do
      cond do
        not ex_stage.valid ->
          %HazardResult{reason: "EX stage is empty (bubble)"}

        not ex_stage.is_branch ->
          %HazardResult{reason: "EX stage instruction is not a branch"}

        ex_stage.branch_predicted_taken == ex_stage.branch_taken ->
          direction = if ex_stage.branch_taken, do: "taken", else: "not taken"

          %HazardResult{
            reason: "branch at PC=0x#{hex4(ex_stage.pc)} correctly predicted #{direction}"
          }

        true ->
          direction =
            if ex_stage.branch_taken,
              do: "predicted not-taken, actually taken",
              else: "predicted taken, actually not-taken"

          %HazardResult{
            action: :flush,
            flush_count: 2,
            reason:
              "branch misprediction at PC=0x#{hex4(ex_stage.pc)}: #{direction} — flushing IF and ID stages"
          }
      end
    end

    defp hex4(value), do: value |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(4, "0")
  end

  defmodule StructuralHazardDetector do
    @moduledoc """
    Detects execution-unit and memory-port conflicts.
    """

    alias CodingAdventures.HazardResult
    alias CodingAdventures.PipelineSlot

    defstruct num_alus: 1, num_fp_units: 1, split_caches: true

    @type t :: %__MODULE__{
            num_alus: pos_integer(),
            num_fp_units: pos_integer(),
            split_caches: boolean()
          }

    def new(attrs \\ []), do: struct(__MODULE__, attrs)

    @spec detect(t(), PipelineSlot.t(), PipelineSlot.t(), keyword()) :: HazardResult.t()
    def detect(%__MODULE__{} = detector, %PipelineSlot{} = id_stage, %PipelineSlot{} = ex_stage, opts \\ []) do
      if_stage = Keyword.get(opts, :if_stage)
      mem_stage = Keyword.get(opts, :mem_stage)

      exec_result = check_execution_unit_conflict(detector, id_stage, ex_stage)

      cond do
        exec_result.action != :none ->
          exec_result

        is_struct(if_stage, PipelineSlot) and is_struct(mem_stage, PipelineSlot) ->
          check_memory_port_conflict(detector, if_stage, mem_stage)

        true ->
          %HazardResult{reason: "no structural hazards — all resources available"}
      end
    end

    defp check_execution_unit_conflict(%__MODULE__{} = detector, %PipelineSlot{} = id_stage, %PipelineSlot{} = ex_stage) do
      cond do
        not id_stage.valid or not ex_stage.valid ->
          %HazardResult{reason: "one or both stages are empty (bubble)"}

        id_stage.uses_alu and ex_stage.uses_alu and detector.num_alus < 2 ->
          %HazardResult{
            action: :stall,
            stall_cycles: 1,
            reason:
              "structural hazard: both ID (PC=0x#{hex4(id_stage.pc)}) and EX (PC=0x#{hex4(ex_stage.pc)}) need the ALU, but only #{detector.num_alus} ALU available"
          }

        id_stage.uses_fp and ex_stage.uses_fp and detector.num_fp_units < 2 ->
          %HazardResult{
            action: :stall,
            stall_cycles: 1,
            reason:
              "structural hazard: both ID (PC=0x#{hex4(id_stage.pc)}) and EX (PC=0x#{hex4(ex_stage.pc)}) need the FP unit, but only #{detector.num_fp_units} FP unit available"
          }

        true ->
          %HazardResult{reason: "no execution unit conflict"}
      end
    end

    defp check_memory_port_conflict(%__MODULE__{split_caches: true}, _if_stage, _mem_stage) do
      %HazardResult{reason: "split caches — no memory port conflict"}
    end

    defp check_memory_port_conflict(%__MODULE__{}, %PipelineSlot{} = if_stage, %PipelineSlot{} = mem_stage) do
      cond do
        if_stage.valid and mem_stage.valid and (mem_stage.mem_read or mem_stage.mem_write) ->
          access_type = if mem_stage.mem_read, do: "load", else: "store"

          %HazardResult{
            action: :stall,
            stall_cycles: 1,
            reason:
              "structural hazard: IF (fetch at PC=0x#{hex4(if_stage.pc)}) and MEM (#{access_type} at PC=0x#{hex4(mem_stage.pc)}) both need the shared memory bus"
          }

        true ->
          %HazardResult{reason: "no memory port conflict"}
      end
    end

    defp hex4(value), do: value |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(4, "0")
  end

  defmodule HazardUnit do
    @moduledoc """
    Runs all hazard detectors each cycle and returns the highest-priority result.
    """

    alias CodingAdventures.HazardDetection.ControlHazardDetector
    alias CodingAdventures.HazardDetection.DataHazardDetector
    alias CodingAdventures.HazardDetection.StructuralHazardDetector
    alias CodingAdventures.HazardResult
    alias CodingAdventures.PipelineSlot

    defstruct data_detector: DataHazardDetector,
              control_detector: ControlHazardDetector,
              structural_detector: StructuralHazardDetector.new(),
              history: []

    @type t :: %__MODULE__{
            data_detector: module(),
            control_detector: module(),
            structural_detector: StructuralHazardDetector.t(),
            history: [HazardResult.t()]
          }

    def new(attrs \\ []) do
      structural =
        StructuralHazardDetector.new(
          num_alus: Keyword.get(attrs, :num_alus, 1),
          num_fp_units: Keyword.get(attrs, :num_fp_units, 1),
          split_caches: Keyword.get(attrs, :split_caches, true)
        )

      %__MODULE__{structural_detector: structural}
    end

    @spec check(t(), PipelineSlot.t(), PipelineSlot.t(), PipelineSlot.t(), PipelineSlot.t()) ::
            {t(), HazardResult.t()}
    def check(%__MODULE__{} = unit, %PipelineSlot{} = if_stage, %PipelineSlot{} = id_stage, %PipelineSlot{} = ex_stage, %PipelineSlot{} = mem_stage) do
      control_result = ControlHazardDetector.detect(ex_stage)
      data_result = DataHazardDetector.detect(id_stage, ex_stage, mem_stage)
      structural_result = StructuralHazardDetector.detect(unit.structural_detector, id_stage, ex_stage, if_stage: if_stage, mem_stage: mem_stage)
      final_result = pick_highest_priority([control_result, data_result, structural_result])
      {%{unit | history: unit.history ++ [final_result]}, final_result}
    end

    def stall_count(%__MODULE__{} = unit), do: Enum.reduce(unit.history, 0, &(&1.stall_cycles + &2))
    def flush_count(%__MODULE__{} = unit), do: Enum.count(unit.history, &(&1.action == :flush))
    def forward_count(%__MODULE__{} = unit), do: Enum.count(unit.history, &(&1.action in [:forward_ex, :forward_mem]))

    defp pick_highest_priority(results) do
      Enum.reduce(results, fn result, best ->
        if priority(result.action) > priority(best.action), do: result, else: best
      end)
    end

    defp priority(:none), do: 0
    defp priority(:forward_mem), do: 1
    defp priority(:forward_ex), do: 2
    defp priority(:stall), do: 3
    defp priority(:flush), do: 4
  end
end
