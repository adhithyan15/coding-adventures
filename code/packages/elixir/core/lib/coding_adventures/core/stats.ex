defmodule CodingAdventures.Core.Stats do
  @moduledoc """
  Aggregate statistics from all core sub-components.

  ## Key Metrics

  IPC (Instructions Per Cycle): the most important performance metric.

      IPC = instructions_completed / total_cycles

      IPC = 1.0: every cycle produces a result (ideal for scalar pipeline)
      IPC < 1.0: stalls and flushes are wasting cycles
      IPC > 1.0: superscalar (not modeled yet)

  CPI (Cycles Per Instruction): the inverse of IPC.

      CPI = total_cycles / instructions_completed
  """

  alias CodingAdventures.CpuPipeline.PipelineStats

  @type t :: %__MODULE__{
          instructions_completed: non_neg_integer(),
          total_cycles: non_neg_integer(),
          pipeline_stats: PipelineStats.t(),
          forward_count: non_neg_integer(),
          stall_count: non_neg_integer(),
          flush_count: non_neg_integer()
        }

  defstruct instructions_completed: 0,
            total_cycles: 0,
            pipeline_stats: %PipelineStats{},
            forward_count: 0,
            stall_count: 0,
            flush_count: 0

  @doc """
  Returns instructions per cycle.

  Returns 0.0 if no cycles have elapsed.
  """
  @spec ipc(t()) :: float()
  def ipc(%__MODULE__{total_cycles: 0}), do: 0.0
  def ipc(%__MODULE__{instructions_completed: ic, total_cycles: tc}) do
    ic / tc
  end

  @doc """
  Returns cycles per instruction.

  Returns 0.0 if no instructions have completed.
  """
  @spec cpi(t()) :: float()
  def cpi(%__MODULE__{instructions_completed: 0}), do: 0.0
  def cpi(%__MODULE__{instructions_completed: ic, total_cycles: tc}) do
    tc / ic
  end

  @doc "Returns a formatted summary of all statistics."
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{} = stats) do
    """
    Core Statistics:
      Instructions completed: #{stats.instructions_completed}
      Total cycles:           #{stats.total_cycles}
      IPC: #{:erlang.float_to_binary(ipc(stats), decimals: 3)}   CPI: #{:erlang.float_to_binary(cpi(stats), decimals: 3)}

    Pipeline:
      Stall cycles:  #{stats.pipeline_stats.stall_cycles}
      Flush cycles:  #{stats.pipeline_stats.flush_cycles}
      Bubble cycles: #{stats.pipeline_stats.bubble_cycles}

    Hazards:
      Forwards: #{stats.forward_count}
      Stalls:   #{stats.stall_count}
      Flushes:  #{stats.flush_count}
    """
  end
end
