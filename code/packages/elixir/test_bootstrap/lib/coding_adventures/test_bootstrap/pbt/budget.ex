defmodule CodingAdventures.TestBootstrap.Pbt.Budget do
  @moduledoc """
  The budget of a shrink, as a TREE of scopes (VER05m 3.1, ported from the Java
  engine's VER05e 4.1, C# VER05g 3.1 and Ruby VER05h 3.1).

  Why a tree?  A property may itself call `Runner.check`, a custom shrinker may
  call `Gen.shrink`, a filter predicate may do either.  If each of those got a
  fresh budget the work would multiply with the nesting depth.  Here every
  public entry point that runs user code opens a CHILD of the ambient scope with
  its own local cap, and a charge walks from the node up to the outermost root:
  every node on the path must have room, then every counter is incremented, all
  or nothing.  Whatever the nesting, the total work of one top-level check is
  capped by the outermost root.

  Two things are charged: ATTEMPTS (one per property evaluation) and WORK
  (elements copied, at least 1 per charge).

  ## The carrier (BEAM)

  Processes share nothing, so:

    * the AMBIENT node lives in the process dictionary of the process running
      the check (`:coding_adventures_pbt_budget`);
    * the COUNTERS live in `:atomics`, so any process holding a node charges the
      same tree atomically;
    * a spawned process does NOT inherit the process dictionary: a plain `spawn`
      starts with no ambient node and a check there opens a fresh root (bounded
      per process flow, pinned by a test).  Pass the handle explicitly with
      `capture/0` + `with_handle/2`, or use `spawn/1`, `spawn_link/1`, `async/1`.

  One `:atomics` cell per tree is a spin LOCK making a multi-node charge
  all-or-nothing.  The spin is bounded and failing to get the lock REFUSES the
  charge (fail closed): a holder killed inside the critical section can never
  hang the run, only starve it.  No user code runs while the lock is held.
  """

  alias CodingAdventures.TestBootstrap.Pbt.{Hooks, Limits}

  @key :coding_adventures_pbt_budget
  @stats_key {__MODULE__, :last}
  # Most nested owning scopes; deeper scopes join, keeping the walk short.
  @max_depth 64

  # counter cells
  @attempts 1
  @work 2
  @closed 3
  @refused 4

  defstruct [:counters, :lock, :parent, :root_counters, :depth, :max_attempts, :max_work]

  @type t :: %__MODULE__{}

  def max_depth, do: @max_depth

  # -- scope lifecycle ---------------------------------------------------------

  defp build(parent, max_attempts, max_work) do
    counters = :atomics.new(4, signed: true)

    {lock, root_counters, depth} =
      case parent do
        nil -> {:atomics.new(1, signed: true), counters, 0}
        p -> {p.lock, p.root_counters, p.depth + 1}
      end

    %__MODULE__{
      counters: counters,
      lock: lock,
      parent: parent,
      root_counters: root_counters,
      depth: depth,
      max_attempts: max_attempts,
      max_work: max_work
    }
  end

  @doc "The nearest open node from `node` upward whose root is open, or nil."
  def usable(nil), do: nil

  def usable(%__MODULE__{} = node) do
    cond do
      closed?(node) -> usable(node.parent)
      :atomics.get(node.root_counters, @closed) == 1 -> nil
      true -> node
    end
  end

  @doc "The ambient open budget of this process, or nil outside any shrink."
  def current, do: usable(Process.get(@key))

  @doc "Joins the ambient scope when there is one, else opens a fresh root."
  def enter(max_steps, fun) do
    case current() do
      nil ->
        run(build(nil, Limits.attempt_budget(max_steps), Hooks.get(:default_max_work)), fun)

      open ->
        fun.(open)
    end
  end

  @doc """
  The scope of a public entry point: a CHILD of the ambient scope with a fresh
  local cap, or a fresh root when none is open.  Past the depth cap it joins.
  """
  def enter_child(max_steps, max_work \\ nil, fun) do
    case acquire_child(max_steps, max_work) do
      {:joined, open} -> fun.(open)
      {:owned, scope} -> run(scope, fun)
    end
  end

  @doc "Like `enter_child/3` without a block: the caller must `close/1` the result."
  def open_child(max_steps, max_work \\ nil) do
    case acquire_child(max_steps, max_work) do
      {:joined, open} ->
        open

      {:owned, scope} ->
        Process.put(@key, scope)
        scope
    end
  end

  defp acquire_child(max_steps, max_work) do
    ambient = Process.get(@key)
    open = usable(ambient)
    work = max_work || Limits.max_shrink_work()

    cond do
      open == nil ->
        cap = if work == Limits.max_shrink_work(), do: Hooks.get(:default_max_work), else: work
        {:owned, build(nil, Limits.attempt_budget(max_steps), cap)}

      open.depth + 1 >= @max_depth ->
        {:joined, open}

      true ->
        {:owned, build(open, Limits.attempt_budget(max_steps), work)}
    end
  end

  @doc """
  The scope of a whole `Runner.check` (case loop included): a root with no open
  ambient scope, a child otherwise.  Its attempt cap is `num_tests` plus the
  shrink attempt cap.
  """
  def enter_check(num_tests, max_steps, fun) do
    case acquire_check(num_tests, max_steps) do
      {:joined, open} -> fun.(open)
      {:owned, scope} -> run(scope, fun)
    end
  end

  def open_check(num_tests, max_steps) do
    case acquire_check(num_tests, max_steps) do
      {:joined, open} ->
        open

      {:owned, scope} ->
        Process.put(@key, scope)
        scope
    end
  end

  defp acquire_check(num_tests, max_steps) do
    cap = num_tests + Limits.attempt_budget(max_steps)
    ambient = Process.get(@key)
    open = usable(ambient)

    cond do
      open == nil -> {:owned, build(nil, cap, Hooks.get(:default_max_work))}
      open.depth + 1 >= @max_depth -> {:joined, open}
      true -> {:owned, build(open, cap, Limits.max_shrink_work())}
    end
  end

  # The ambient value is set INSIDE the try that guarantees close, so an
  # exception, throw or exit in that window cannot leave an orphaned scope.
  defp run(scope, fun) do
    try do
      Process.put(@key, scope)
      fun.(scope)
    after
      close(scope)
    end
  end

  @doc """
  Ends an owning scope.  The ambient value is restored FIRST so nothing below
  can leave a stale one.  Idempotent.

  It is restored to the scope's PARENT (nil for a root).  A node never stores a
  separate "previous" value: a second reference to the same ancestor would make
  the term a tree twice as big per level (copying, comparing or hashing a depth-64
  handle would take 2^64 steps).  If the ambient value was a closed node whose
  nearest open ancestor is the parent, restoring the parent is equivalent.
  """
  def close(%__MODULE__{} = scope) do
    try do
      restore(scope.parent)
    after
      :atomics.put(scope.counters, @closed, 1)

      Process.put(
        @stats_key,
        {:atomics.get(scope.counters, @work), :atomics.get(scope.counters, @attempts)}
      )
    end

    :ok
  end

  defp restore(nil), do: Process.delete(@key)
  defp restore(previous), do: Process.put(@key, previous)

  @doc "Work of the last scope this process closed."
  def last_work, do: elem(Process.get(@stats_key, {0, 0}), 0)
  def last_attempts, do: elem(Process.get(@stats_key, {0, 0}), 1)

  # -- queries ---------------------------------------------------------------------

  def closed?(%__MODULE__{counters: c}), do: :atomics.get(c, @closed) == 1
  def nested?(%__MODULE__{parent: p}), do: p != nil
  def refused?(%__MODULE__{root_counters: r}), do: :atomics.get(r, @refused) == 1
  def attempts(%__MODULE__{counters: c}), do: :atomics.get(c, @attempts)
  def work(%__MODULE__{counters: c}), do: :atomics.get(c, @work)

  # -- charging ----------------------------------------------------------------------

  @doc """
  Charges one attempt over a candidate of `elements` elements (at least 1) to
  this node and every ancestor.  Returns false, charging nothing, when any of
  them cannot afford it: callers must check this BEFORE building the candidate.
  """
  def charge(node, elements), do: take(node, elements, 1)

  @doc "Charges a copy that is not itself a property evaluation: work only."
  def charge_copy(node, elements), do: take(node, elements, 0)

  @doc "True while at least one more attempt of one element fits everywhere on the path."
  def room?(node) do
    with_lock(node, fn ->
      node |> chain() |> Enum.all?(&room_for?(&1, 1))
    end) == true
  end

  @doc "Wraps a falsifies function so each evaluation is charged one attempt first."
  def charged(node, falsifies) do
    fn value -> charge(node, 1) and falsifies.(value) == true end
  end

  defp take(node, elements, attempt_cost) do
    cost = max(1, elements)

    ok =
      with_lock(node, fn ->
        nodes = chain(node)

        if Enum.all?(nodes, &room_for?(&1, cost)) do
          Enum.each(nodes, fn n ->
            :atomics.add(n.counters, @attempts, attempt_cost)
            :atomics.add(n.counters, @work, cost)
          end)

          true
        else
          false
        end
      end)

    if ok == true do
      true
    else
      :atomics.put(node.root_counters, @refused, 1)
      false
    end
  end

  # Overflow-safe form: work <= max_work always, so the subtraction cannot go negative.
  defp room_for?(n, cost) do
    :atomics.get(n.counters, @attempts) < n.max_attempts and
      cost <= n.max_work - :atomics.get(n.counters, @work)
  end

  defp chain(nil), do: []
  defp chain(node), do: [node | chain(node.parent)]

  defp with_lock(node, fun) do
    if acquire(node.lock, Hooks.get(:lock_spins)) do
      try do
        fun.()
      after
        :atomics.put(node.lock, 1, 0)
      end
    else
      false
    end
  end

  defp acquire(_lock, 0), do: false

  defp acquire(lock, spins) do
    case :atomics.compare_exchange(lock, 1, 0, 1) do
      :ok ->
        true

      _held ->
        :erlang.yield()
        acquire(lock, spins - 1)
    end
  end

  @doc false
  # Test seam: forces the lock cell to "held" (a killed holder).
  def stuck_lock(%__MODULE__{lock: lock}), do: :atomics.put(lock, 1, 1)

  # -- carrying a handle to another process --------------------------------------------

  @doc "The usable ambient node, to hand to a process you are about to start (or nil)."
  def capture, do: current()

  @doc "Runs `fun` with `handle` installed as this process's ambient node (nil: unchanged)."
  def with_handle(nil, fun), do: fun.()

  def with_handle(%__MODULE__{} = handle, fun) do
    previous = Process.get(@key)
    Process.put(@key, handle)

    try do
      fun.()
    after
      restore(previous)
    end
  end

  @doc "`spawn/1` that charges the spawning scope's budget tree."
  def spawn(fun) do
    handle = capture()
    Kernel.spawn(fn -> with_handle(handle, fun) end)
  end

  def spawn_link(fun) do
    handle = capture()
    Kernel.spawn_link(fn -> with_handle(handle, fun) end)
  end

  @doc "`Task.async/1` that charges the spawning scope's budget tree."
  def async(fun) do
    handle = capture()
    Task.async(fn -> with_handle(handle, fun) end)
  end
end
