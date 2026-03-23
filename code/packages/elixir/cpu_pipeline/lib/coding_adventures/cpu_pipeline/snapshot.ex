defmodule CodingAdventures.CpuPipeline.Snapshot do
  @moduledoc """
  PipelineSnapshot -- the complete state of the pipeline at one moment.

  Captures the full state of the pipeline at a single point in time
  (one clock cycle). Think of it as a photograph of the assembly line:
  you can see what instruction is at each station.

  Snapshots are used for:
    - Debugging: "What was in the EX stage at cycle 7?"
    - Visualization: drawing pipeline diagrams
    - Testing: verifying that the pipeline behaves correctly

  ## Example

      Cycle 7:
        IF:  instr@28  (fetching instruction at PC=28)
        ID:  ADD@24    (decoding an ADD instruction)
        EX:  SUB@20    (executing a SUB)
        MEM: ---       (bubble -- pipeline was stalled here)
        WB:  LDR@12    (writing back a load result)
  """

  alias CodingAdventures.CpuPipeline.Token

  @type t :: %__MODULE__{
          cycle: integer(),
          stages: %{optional(String.t()) => Token.t()},
          stalled: boolean(),
          flushing: boolean(),
          pc: integer()
        }

  defstruct cycle: 0,
            stages: %{},
            stalled: false,
            flushing: false,
            pc: 0

  @doc "Returns a compact representation of the pipeline state."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{cycle: cycle, pc: pc, stalled: stalled, flushing: flushing}) do
    "[cycle #{cycle}] PC=#{pc} stalled=#{stalled} flushing=#{flushing}"
  end
end

# =========================================================================
# PipelineStats -- execution statistics
# =========================================================================

defmodule CodingAdventures.CpuPipeline.PipelineStats do
  @moduledoc """
  Tracks performance statistics across the pipeline's execution.

  These statistics are the same ones that hardware performance counters
  measure in real CPUs. They answer the question: "How efficiently is
  the pipeline being used?"

  ## Key Metrics

  IPC (Instructions Per Cycle): The most important pipeline metric.

      IPC = instructions_completed / total_cycles

      Ideal:     IPC = 1.0 (one instruction completes every cycle)
      With stalls: IPC < 1.0 (some cycles are wasted)
      Superscalar: IPC > 1.0 (multiple instructions per cycle)

  CPI (Cycles Per Instruction): The inverse of IPC.

      CPI = total_cycles / instructions_completed

      Ideal:     CPI = 1.0
      Typical:   CPI = 1.2-2.0 for real workloads
  """

  @type t :: %__MODULE__{
          total_cycles: non_neg_integer(),
          instructions_completed: non_neg_integer(),
          stall_cycles: non_neg_integer(),
          flush_cycles: non_neg_integer(),
          bubble_cycles: non_neg_integer()
        }

  defstruct total_cycles: 0,
            instructions_completed: 0,
            stall_cycles: 0,
            flush_cycles: 0,
            bubble_cycles: 0

  @doc """
  Returns the instructions per cycle.

  IPC is the primary measure of pipeline efficiency:
    - IPC = 1.0: perfect pipeline utilization (ideal)
    - IPC < 1.0: some cycles are wasted (stalls, flushes)
    - IPC > 1.0: superscalar execution

  Returns 0.0 if no cycles have been executed.
  """
  @spec ipc(t()) :: float()
  def ipc(%__MODULE__{total_cycles: 0}), do: 0.0
  def ipc(%__MODULE__{instructions_completed: completed, total_cycles: cycles}) do
    completed / cycles
  end

  @doc """
  Returns cycles per instruction (inverse of IPC).

  Returns 0.0 if no instructions have completed.
  """
  @spec cpi(t()) :: float()
  def cpi(%__MODULE__{instructions_completed: 0}), do: 0.0
  def cpi(%__MODULE__{instructions_completed: completed, total_cycles: cycles}) do
    cycles / completed
  end

  @doc "Returns a formatted summary of pipeline statistics."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{} = stats) do
    "PipelineStats{cycles=#{stats.total_cycles}, completed=#{stats.instructions_completed}, " <>
      "IPC=#{:erlang.float_to_binary(ipc(stats), decimals: 3)}, " <>
      "CPI=#{:erlang.float_to_binary(cpi(stats), decimals: 3)}, " <>
      "stalls=#{stats.stall_cycles}, flushes=#{stats.flush_cycles}, " <>
      "bubbles=#{stats.bubble_cycles}}"
  end
end
