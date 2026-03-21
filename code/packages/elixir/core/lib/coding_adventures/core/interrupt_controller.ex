defmodule CodingAdventures.Core.InterruptController do
  @moduledoc """
  Manages interrupt routing in a multi-core system.

  ## What are Interrupts?

  An interrupt is a signal that temporarily diverts the CPU from its current
  work to handle an urgent event. Examples:

    - Timer interrupt: "100ms have passed, let the OS scheduler run"
    - I/O interrupt: "keyboard key was pressed"
    - Inter-processor interrupt (IPI): "Core 0 needs Core 1 to flush its TLB"
    - Software interrupt: "this program wants to make a system call"

  ## How the Controller Works

    1. An external device (or another core) raises an interrupt.
    2. The controller queues it and decides which core should handle it.
    3. On the next cycle, the controller signals the target core.
    4. The core acknowledges the interrupt and begins handling it.

  This implementation is a simplified shell -- it queues interrupts and
  routes them, but does not model priorities or masking.
  """

  @type pending_interrupt :: %{interrupt_id: integer(), target_core: integer()}
  @type acknowledged_interrupt :: %{core_id: integer(), interrupt_id: integer()}

  @type t :: %__MODULE__{
          pending: [pending_interrupt()],
          acknowledged: [acknowledged_interrupt()],
          num_cores: pos_integer()
        }

  defstruct pending: [], acknowledged: [], num_cores: 1

  @doc "Creates an interrupt controller for the given number of cores."
  @spec new(pos_integer()) :: t()
  def new(num_cores) do
    %__MODULE__{num_cores: num_cores}
  end

  @doc """
  Queues an interrupt for delivery.

  If target_core is -1, the interrupt will be routed to core 0
  (simplest routing policy).
  """
  @spec raise_interrupt(t(), integer(), integer()) :: t()
  def raise_interrupt(%__MODULE__{} = ic, interrupt_id, target_core) do
    target =
      cond do
        target_core == -1 -> 0
        target_core >= ic.num_cores -> 0
        true -> target_core
      end

    interrupt = %{interrupt_id: interrupt_id, target_core: target}
    %{ic | pending: ic.pending ++ [interrupt]}
  end

  @doc """
  Records that a core has begun handling an interrupt.

  Removes the interrupt from the pending list and adds it to acknowledged.
  """
  @spec acknowledge(t(), integer(), integer()) :: t()
  def acknowledge(%__MODULE__{} = ic, core_id, interrupt_id) do
    ack = %{core_id: core_id, interrupt_id: interrupt_id}

    # Remove first matching pending interrupt.
    {remaining, _removed} =
      Enum.reduce(ic.pending, {[], false}, fn p, {acc, removed} ->
        if not removed and p.interrupt_id == interrupt_id and p.target_core == core_id do
          {acc, true}
        else
          {acc ++ [p], removed}
        end
      end)

    %{ic | pending: remaining, acknowledged: ic.acknowledged ++ [ack]}
  end

  @doc "Returns all pending interrupts targeted at a specific core."
  @spec pending_for_core(t(), integer()) :: [pending_interrupt()]
  def pending_for_core(%__MODULE__{pending: pending}, core_id) do
    Enum.filter(pending, fn p -> p.target_core == core_id end)
  end

  @doc "Returns the total number of pending (unacknowledged) interrupts."
  @spec pending_count(t()) :: non_neg_integer()
  def pending_count(%__MODULE__{pending: pending}), do: length(pending)

  @doc "Returns the total number of acknowledged interrupts."
  @spec acknowledged_count(t()) :: non_neg_integer()
  def acknowledged_count(%__MODULE__{acknowledged: acked}), do: length(acked)

  @doc "Clears all pending and acknowledged interrupts."
  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = ic) do
    %{ic | pending: [], acknowledged: []}
  end
end
