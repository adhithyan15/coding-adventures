defmodule CodingAdventures.TestBootstrap.Fuzz.Executor do
  @moduledoc """
  One target execution in its OWN monitored process (VER06n section 5).

  ## Why a process per execution

  A BEAM process is the only unit the VM can bound and kill:

    * `max_heap_size` with `kill: true` ends a process whose heap passes the memory
      budget with reason `:killed`: that is the OOM outcome, and it is classified
      only from THIS execution's monitor.  `max_heap_size` does NOT count reference
      counted (large) binaries, so it is backed by a POLL (below);
    * every `@poll_ms` (5 ms) of waiting the engine reads `Process.info(pid, :memory)`
      (heap, stack and, because the process is NOT `:off_heap`, its mailbox); every 4th
      slice, or as soon as that is above a quarter of the budget, it also reads the
      `:binary` item and adds the sizes of the reference counted binaries the process
      references (deduplicated by binary id; more than `budget / 48` of them is over
      budget at once, as each costs the target at least 48 bytes of heap).  Past the
      budget (floored at 1 MiB) the process is killed and the outcome is `:oom`.  The
      memory budget is therefore BEST EFFORT: an allocation that completes inside one
      5 ms window is only seen on the next poll, a process that finishes between
      polls is never measured, and one non-yielding BIF that allocates before any
      poll is not stopped;
    * the same poll watches the ENGINE's own mailbox (a target can send its parent
      anything): more than 10,000 queued messages, or engine memory that grew by more
      than the budget since the execution began, kills the target as `:oom`;
    * a wall-clock deadline (`System.monotonic_time/1` plus `timeout_ms`, recomputed
      every slice) kills a target that never returns and never calls `tick`: that is
      the timeout outcome;
    * inside a campaign (`Fuzz.run/3`) the target is spawned LINKED to the engine,
      which traps exits: if the engine is killed (its caller died, or the caller's
      bound expired) the target receives the kill and dies; a target that itself traps
      exits, or an unlinked process it spawned, can survive the engine (an in-VM limit).
      A linked CHILD of the target survives the target's NORMAL exit (a link only
      propagates abnormal exits); it dies when the target is killed or crashes.
      Processes the target spawns UNLINKED are outside the engine's reach (no
      process-list diffing is attempted);
    * the engine also watches its caller: on the caller's `:DOWN` it kills the current
      target and exits.

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
  @max_queue 10_000
  @caller_key :coding_adventures_fuzz_engine
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

    # inside a campaign the engine traps exits and the target is linked to it, so an
    # engine that dies takes the target with it
    caller_mon = Process.get(@caller_key)
    check_caller(caller_mon)
    base = own_memory()
    link = if caller_mon, do: [:link], else: []
    fun = fn -> send(parent, {tag, self(), in_process(target, data, handle)}) end

    heap = {:max_heap_size, %{size: words, kill: true, error_logger: false}}
    wait = o.timeout_ms + 2 * @kill_wait_ms

    # The target's PARENT is a throwaway LAUNCHER, never the engine: whatever the
    # target sends "to its parent" lands in the launcher's mailbox, which is
    # heap-limited and dies with the execution.  The launcher is linked to the target
    # (a target that dies abnormally takes the launcher with the same reason) and,
    # inside a campaign, to the engine (so an engine that is killed takes the launcher,
    # and through it the target).
    launcher = fn ->
      {tpid, tmon} = :erlang.spawn_opt(fun, [:link, :monitor, heap])
      send(parent, {tag, :started, tpid})

      receive do
        {:DOWN, ^tmon, :process, ^tpid, _reason} -> :ok
      after
        wait -> exit(:launcher_timeout)
      end
    end

    {lpid, mon} = :erlang.spawn_opt(launcher, [:monitor] ++ link ++ [heap])

    pid =
      receive do
        {^tag, :started, tpid} -> tpid
        {:DOWN, ^mon, :process, ^lpid, _} -> nil
      after
        @kill_wait_ms -> nil
      end

    deadline = System.monotonic_time(:millisecond) + o.timeout_ms
    ended = await({lpid, pid}, mon, deadline, max(o.mem_budget, @poll_floor), base, caller_mon, 0)
    result = take_result(tag, pid)
    flush_started(tag)
    # (1) retire, then (2) read: nothing a stale process does can change this verdict
    Handle.retire(handle)
    outcome = verdict(Handle.exhausted(handle), ended, result)
    {outcome, Handle.edges(handle), Handle.edge_calls(handle)}
  end

  # The caller already died: do not even start.
  defp check_caller(nil), do: :ok

  defp check_caller(cmon) do
    receive do
      {:DOWN, ^cmon, :process, _pid, _reason} -> exit(:caller_down)
    after
      0 -> :ok
    end
  end

  defp own_memory do
    case Process.info(self(), :memory) do
      {:memory, m} -> m
      _ -> 0
    end
  end

  # Waits for THIS monitor's :DOWN only (an unrelated :DOWN or :EXIT is left alone),
  # in 5 ms slices; each slice that ends without a :DOWN polls the memory.  The
  # deadline is real time, recomputed every slice, so polling cannot stretch it.
  defp await({lpid, pid} = pids, mon, deadline, budget, base, cmon, n) do
    left = deadline - System.monotonic_time(:millisecond)

    if left <= 0 do
      kill(pids, mon, :wall_clock)
    else
      receive do
        {:DOWN, ^mon, :process, ^lpid, reason} ->
          {:down, reason}

        {:DOWN, ^cmon, :process, _cpid, _reason} ->
          kill(pids, mon, :caller_down)
          exit(:caller_down)
      after
        min(left, @poll_ms) ->
          if over_budget?(pid, budget, n) or launcher_over?(lpid, budget) or flooded?(budget, base),
            do: kill(pids, mon, :over_budget),
            else: await(pids, mon, deadline, budget, base, cmon, n + 1)
      end
    end
  end

  defp kill({lpid, pid}, mon, why) do
    if pid, do: Process.exit(pid, :kill)
    Process.exit(lpid, :kill)

    receive do
      {:DOWN, ^mon, :process, ^lpid, _reason} -> why
    after
      @kill_wait_ms ->
        Process.demonitor(mon, [:flush])
        why
    end
  end

  # Memory every slice; the expensive :binary item every 4th slice or when memory is
  # already above a quarter of the budget.
  defp over_budget?(nil, _budget, _n), do: false

  defp over_budget?(pid, budget, n) do
    case Process.info(pid, :memory) do
      {:memory, memory} ->
        cond do
          memory > budget -> true
          rem(n, 4) == 0 or memory > div(budget, 4) -> binaries_over?(pid, memory, budget)
          true -> false
        end

      _gone ->
        false
    end
  end

  defp binaries_over?(pid, memory, budget) do
    case Process.info(pid, :binary) do
      {:binary, binaries} ->
        # each ProcBin costs the target at least 48 bytes of heap
        if length(binaries) > div(budget, 48) do
          true
        else
          {_seen, sum} =
            Enum.reduce(binaries, {%{}, 0}, fn {id, size, _refc}, {seen, sum} ->
              if is_map_key(seen, id), do: {seen, sum}, else: {Map.put(seen, id, true), sum + size}
            end)

          memory + sum > budget
        end

      _gone ->
        false
    end
  end

  # The launcher is the target's parent: whatever the target sends "to its parent"
  # piles up in the launcher's mailbox, which counts in its :memory.  (A launcher
  # blocked in `receive` never garbage collects, so max_heap_size alone would not
  # stop a flood: the poll does.)
  defp launcher_over?(lpid, budget) do
    case Process.info(lpid, :memory) do
      {:memory, memory} -> memory > budget
      _gone -> false
    end
  end

  # A target can send the ENGINE anything it can reach: judge the engine's mailbox too.
  defp flooded?(budget, base) do
    case Process.info(self(), [:message_queue_len, :memory]) do
      [message_queue_len: q, memory: m] -> q > @max_queue or m - base > budget
      _ -> false
    end
  end

  # The outcome message always precedes the :DOWN of the same process.
  defp flush_started(tag) do
    receive do
      {^tag, :started, _} -> :ok
    after
      0 -> :ok
    end
  end

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
  def caller_key, do: @caller_key
end
