defmodule CodingAdventures.TestBootstrap.Fuzz.Executor do
  @moduledoc """
  One target execution in its OWN monitored process (VER06n section 5).

  ## Why a process per execution

  A BEAM process is the only unit the VM can bound and kill:

    * `max_heap_size` with `kill: true` ends a process whose heap passes the memory
      budget with reason `:killed`: that is the OOM outcome, and it is classified
      only from THIS execution's monitor.  `max_heap_size` does NOT count reference
      counted (large) binaries, so it is backed by a POLL (below);
    * every `@poll_ms` (5 ms) while it waits, the engine sums
      `Process.info(pid, [:memory, :binary])`: the process's own memory (heap,
      stack and, because the process is NOT `:off_heap`, its mailbox) plus the sizes
      of the reference counted binaries it references (deduplicated by binary id).
      Past the budget (floored at 1 MiB) the process is killed and the outcome is
      `:oom`.  The memory budget is therefore BEST EFFORT: an allocation that
      completes inside one 5 ms window is only seen on the next poll, and a process
      that finishes between polls is never measured;
    * a wall-clock bound in the same loop kills a target that never returns and
      never calls `tick`: that is the timeout outcome (the loop counts its 5 ms
      slices, so it never reads a clock and the real bound is at least
      `timeout_ms`);
    * linked children die with it.  Processes the target spawns UNLINKED are outside
      the engine's reach (no process-list diffing is attempted).

  The `message_queue_data: :off_heap` option is deliberately NOT used: off-heap
  messages are invisible to `max_heap_size`, so a self-sent flood would grow
  unbounded; on-heap, the mailbox counts in the heap and in `:memory`.

  ## Results and forgery

  The execution reports `{tag, pid, outcome}` and the engine matches the pid too.
  A deliberately hostile target can recover the tag from its own backtrace or
  process dictionary and forge a result: targets are trusted not to attack the
  harness inside the VM, the same limit as every other lane.

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

  @poll_ms 5
  # The poll never judges a budget below 1 MiB (a bare process is already tens of KiB).
  @poll_floor 1_048_576

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
    words = heap_words(o.mem_budget)

    {pid, mon} =
      :erlang.spawn_opt(
        fn -> send(parent, {tag, self(), in_process(target, data, handle)}) end,
        [:monitor, {:max_heap_size, %{size: words, kill: true, error_logger: false}}]
      )

    ended = await(pid, mon, o.timeout_ms, max(o.mem_budget, @poll_floor))
    result = take_result(tag, pid)
    # (1) retire, then (2) read: nothing a stale process does can change this verdict
    Handle.retire(handle)
    outcome = verdict(Handle.exhausted(handle), ended, result)
    {outcome, Handle.edges(handle), Handle.edge_calls(handle)}
  end

  # Waits for THIS monitor's :DOWN only (an unrelated :DOWN or :EXIT is left alone),
  # in 5 ms slices; each slice that ends without a :DOWN polls the memory.
  defp await(pid, mon, left, budget) do
    slice = min(left, @poll_ms)

    receive do
      {:DOWN, ^mon, :process, ^pid, reason} -> {:down, reason}
    after
      slice ->
        cond do
          over_budget?(pid, budget) -> kill(pid, mon, :over_budget)
          left - slice <= 0 -> kill(pid, mon, :wall_clock)
          true -> await(pid, mon, left - slice, budget)
        end
    end
  end

  defp kill(pid, mon, why) do
    Process.exit(pid, :kill)

    receive do
      {:DOWN, ^mon, :process, ^pid, _reason} -> why
    after
      @kill_wait_ms ->
        Process.demonitor(mon, [:flush])
        why
    end
  end

  # Heap and stack, the on-heap mailbox, plus every referenced refc binary once.
  defp over_budget?(pid, budget) do
    case Process.info(pid, [:memory, :binary]) do
      [memory: memory, binary: binaries] ->
        total = Enum.reduce(binaries, %{}, fn {id, size, _refc}, acc -> Map.put(acc, id, size) end)
        memory + Enum.sum(Map.values(total)) > budget

      _gone ->
        false
    end
  end

  # The outcome message always precedes the :DOWN of the same process.
  defp take_result(tag, pid) do
    receive do
      {^tag, ^pid, outcome} -> {:ok, outcome}
    after
      0 -> :none
    end
  end

  defp verdict(:steps, _ended, _result), do: :timeout
  defp verdict(:mem, _ended, _result), do: :oom
  defp verdict(:none, :wall_clock, _result), do: :timeout
  defp verdict(:none, :over_budget, _result), do: :oom
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
