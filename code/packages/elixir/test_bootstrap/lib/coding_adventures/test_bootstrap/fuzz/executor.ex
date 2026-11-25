defmodule CodingAdventures.TestBootstrap.Fuzz.Executor do
  @moduledoc """
  One target execution in its OWN monitored process (VER06n section 5).

  ## Why a process per execution

  A BEAM process is the only unit the VM can bound and kill:

    * `max_heap_size` with `kill: true` ends a process whose heap (mailbox
      included) passes the memory budget with reason `:killed`: that is the OOM
      outcome, and it is classified only from THIS execution's monitor;
    * a wall-clock `receive ... after` in the engine kills a target that never
      returns and never calls `tick`: that is the timeout outcome;
    * linked children die with it.

  Every `receive` here has an `after`: no hang can survive.

  ## The order of a verdict

  After the process is gone the engine (1) retires the handle, (2) reads the
  handle's sticky exhausted cell, and only then (3) looks at how the process ended.
  A target that swallowed the budget sentinel and went on is still a timeout or an
  oom, never a pass and never a crash.

  The outcome is `:pass`, `{:crash, kind, frames}`, `:timeout` or `:oom`.
  """
  alias CodingAdventures.TestBootstrap.Safe
  alias CodingAdventures.TestBootstrap.Fuzz.{Handle, Options, Signature}

  @stale "stale-budget-sentinel"
  # Longest the engine waits for a killed process to report its death.
  @kill_wait_ms 5_000

  @doc """
  Runs `target.(data, handle)` once.  Returns `{outcome, edges, edge_calls}`; the
  edges are those of THIS execution, read after the handle was retired.
  """
  def run(target, data, %Options{} = o) when is_function(target, 2) and is_binary(data) do
    handle = Handle.new(o.step_budget, o.mem_budget)
    tag = make_ref()
    parent = self()
    words = max(1024, div(o.mem_budget, 8))

    {pid, mon} =
      :erlang.spawn_opt(
        fn -> send(parent, {tag, in_process(target, data, handle)}) end,
        [
          :monitor,
          {:max_heap_size, %{size: words, kill: true, error_logger: false}},
          {:message_queue_data, :off_heap}
        ]
      )

    ended = await(pid, mon, o.timeout_ms)
    result = take_result(tag)
    # (1) retire, then (2) read: nothing a stale process does can change this verdict
    Handle.retire(handle)
    outcome = verdict(Handle.exhausted(handle), ended, result)
    {outcome, Handle.edges(handle), Handle.edge_calls(handle)}
  end

  # Waits for THIS monitor's :DOWN only (an unrelated :DOWN or :EXIT is left alone).
  defp await(pid, mon, timeout_ms) do
    receive do
      {:DOWN, ^mon, :process, ^pid, reason} -> {:down, reason}
    after
      timeout_ms ->
        Process.exit(pid, :kill)

        receive do
          {:DOWN, ^mon, :process, ^pid, _reason} -> :wall_clock
        after
          @kill_wait_ms ->
            Process.demonitor(mon, [:flush])
            :wall_clock
        end
    end
  end

  # The outcome message always precedes the :DOWN of the same process.
  defp take_result(tag) do
    receive do
      {^tag, outcome} -> {:ok, outcome}
    after
      0 -> :none
    end
  end

  defp verdict(:steps, _ended, _result), do: :timeout
  defp verdict(:mem, _ended, _result), do: :oom
  defp verdict(:none, :wall_clock, _result), do: :timeout
  defp verdict(:none, {:down, :killed}, _result), do: :oom
  defp verdict(:none, {:down, :normal}, {:ok, outcome}), do: outcome
  defp verdict(:none, {:down, :normal}, :none), do: {:crash, "lost-outcome", []}
  defp verdict(:none, {:down, _other}, _result), do: {:crash, "exit.signal", []}

  # Runs inside the execution process.  Only the target is inside the catch.
  defp in_process(target, data, handle) do
    case Safe.call_stacked(fn -> target.(data, handle) end) do
      {:ok, _value} -> :pass
      {:raised, kind, reason, stack} -> classify(kind, reason, stack)
    end
  end

  @doc """
  Turns a caught `kind, reason, stack` into an outcome WITHOUT reading the message
  or calling any protocol on `reason`.  A sentinel-shaped throw is the kind
  `stale-budget-sentinel` here; the engine overrides it when the handle really is
  exhausted (a genuine breach is judged by the cell, not by the throw).
  """
  def classify(:throw, {tag, _token, _which}, stack) do
    if tag == Handle.sentinel_tag(),
      do: {:crash, @stale, Signature.frames_of(stack)},
      else: {:crash, "throw", Signature.frames_of(stack)}
  end

  def classify(kind, reason, stack) do
    {:crash, Signature.kind_of(kind, reason), Signature.frames_of(stack)}
  end

  @doc "The heap limit in words an execution of `mem_budget` bytes gets."
  def heap_words(mem_budget), do: max(1024, div(mem_budget, 8))

  def kill_wait_ms, do: @kill_wait_ms
end
