defmodule CodingAdventures.TestBootstrap.Fuzz.Handle do
  @moduledoc """
  The per-execution coverage and budget handle (VER06n sections 4.2 and 5.3).

  A target receives a FRESH handle for every execution and reports coverage with
  `edge/2`, burns its step budget with `tick/2` and its memory budget with
  `reserve/2`.  All state lives in ONE `:atomics` array, so any process holding the
  handle acts on the same state atomically and no lock exists that a killed process
  could leave held:

      cell 1   state: 0 live, 1 retired
      cell 2   hash of this execution's token (`make_ref()`), checked on every call
      cell 3   step budget          cell 4   memory budget (bytes)
      cell 5   steps used           cell 6   memory reserved
      cell 7   exhausted: 0 none, 1 steps, 2 memory (STICKY: first breach wins)
      cell 8   number of `edge/2` calls
      cells 9..1032   the 65,536-bit edge map as 1,024 unsigned 64-bit words

  ## Inert after the run (and the limit of that guarantee)

  The struct carries the atomics reference and the token.  Every call first checks
  that the state is live AND that the struct's token hashes (`phash2`, 32 bits) to
  the stored token hash, so a struct rebuilt with another token is inert.  That is an
  identity check, not a cryptographic one: INSIDE ONE VM a hostile target can write
  the handle's atomics directly (clear the exhausted cell, reset a counter).  The
  engine's cross-checks (`exhausted/1` also compares the counters with the budgets)
  catch casual clearing, not a determined in-VM attacker; targets are trusted not to
  attack the harness, as in every other lane.  The engine retires the
  handle (an atomic exchange of cell 1) after the execution ends and reads the
  results AFTER that: a handle that was forwarded to another process, kept in an
  ETS table, in `:persistent_term` or in a process dictionary can neither record an
  edge nor change a budget of any later execution (each has its own atomics).

  ## Exhaustion is sticky

  The first breach compare-and-swaps the exhausted cell from 0 and throws a
  sentinel; afterwards EVERY `tick` and `reserve` throws it again.  The BEAM has no
  throwable a target cannot catch, so the guarantee is stickiness plus the post-run
  check of the executor, never the throw itself.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Fuzz.Limits

  @sentinel :coding_adventures_fuzz_budget
  @state 1
  @token 2
  @step_budget 3
  @mem_budget 4
  @steps 5
  @mem 6
  @exhausted 7
  @edge_calls 8
  @words 9
  @word_count 1024
  @cells @words + @word_count - 1
  @hash_range 1 <<< 32

  defstruct [:cells, :token]

  defimpl Inspect do
    # No counters, no edges, no token.
    def inspect(_h, _opts), do: "#Fuzz.Handle<>"
  end

  @doc "The tag of the budget sentinel `{tag, token, which}` (`which` is `:steps`, `:mem` or `:retired`)."
  def sentinel_tag, do: @sentinel

  @doc "A fresh, live handle with the given budgets (engine use)."
  def new(step_budget, mem_budget) do
    Limits.check_range!(step_budget, 0, Limits.max_step_budget(), "step_budget")
    Limits.check_range!(mem_budget, 0, Limits.max_mem_budget(), "mem_budget")
    cells = :atomics.new(@cells, signed: false)
    token = make_ref()
    :atomics.put(cells, @token, :erlang.phash2(token, @hash_range))
    :atomics.put(cells, @step_budget, step_budget)
    :atomics.put(cells, @mem_budget, mem_budget)
    %__MODULE__{cells: cells, token: token}
  end

  @doc "A stable edge id derived from a name (FNV-1a): deterministic across runs and machines."
  def edge_id(label) when is_binary(label), do: Prng.fnv1a64(label)

  # Live AND the struct's token is the one the cells were built for.
  defp live?(%__MODULE__{cells: cells, token: token}) do
    :atomics.get(cells, @state) == 0 and
      :atomics.get(cells, @token) == :erlang.phash2(token, @hash_range)
  end

  defp live?(_other), do: raise(ArgumentError, "expected a Fuzz.Handle")

  defp sentinel(%__MODULE__{token: token}, which), do: throw({@sentinel, token, which})

  defp throw_if_exhausted(h) do
    case :atomics.get(h.cells, @exhausted) do
      0 -> :ok
      1 -> sentinel(h, :steps)
      2 -> sentinel(h, :mem)
    end
  end

  @doc """
  Records that edge `id` ran (reduced modulo 65,536).  A no-op on a retired or
  forged handle.  A non-integer id is an `ArgumentError` while the handle is live.
  """
  def edge(handle, id) do
    if live?(handle) do
      e = Integer.mod(Limits.check_int!(id, "edge id"), Limits.coverage_map_size())
      :atomics.add(handle.cells, @edge_calls, 1)
      set_bit(handle.cells, @words + div(e, 64), 1 <<< rem(e, 64))
    end

    :ok
  end

  # A compare-and-swap OR: lock-free, and a failed swap means another process made progress.
  defp set_bit(cells, index, mask) do
    old = :atomics.get(cells, index)

    if band(old, mask) != 0 do
      :ok
    else
      case :atomics.compare_exchange(cells, index, old, old ||| mask) do
        :ok -> :ok
        _changed -> set_bit(cells, index, mask)
      end
    end
  end

  @doc """
  Burns `n` (default 1) steps of the step budget.  Throws the budget sentinel when
  the budget is passed, and EVERY time afterwards (sticky); throws the retired
  sentinel on a retired or forged handle.
  """
  def tick(handle, n \\ 1) do
    Limits.check_int!(n, "n")

    unless live?(handle), do: sentinel(handle, :retired)
    throw_if_exhausted(handle)

    budget = :atomics.get(handle.cells, @step_budget)
    # clamped BEFORE the add so an absurd n can neither wrap the counter nor allocate
    used = :atomics.add_get(handle.cells, @steps, n |> max(0) |> min(budget + 1))
    if used > budget, do: breach(handle, 1)
    :ok
  end

  @doc """
  Charges `n` bytes against the memory budget (a cooperative stand-in for an
  allocation the target can predict).  Non-positive reserves nothing; sticky like
  `tick/2`.
  """
  def reserve(handle, n) do
    Limits.check_int!(n, "n")

    unless live?(handle), do: sentinel(handle, :retired)
    throw_if_exhausted(handle)

    budget = :atomics.get(handle.cells, @mem_budget)
    used = :atomics.add_get(handle.cells, @mem, n |> max(0) |> min(budget + 1))
    if used > budget, do: breach(handle, 2)
    :ok
  end

  # The first breach wins; the sentinel thrown is the STORED one.
  defp breach(h, which) do
    _ = :atomics.compare_exchange(h.cells, @exhausted, 0, which)
    throw_if_exhausted(h)
  end

  # -- engine side ----------------------------------------------------------------

  @doc "Atomically retires the handle; returns whether it was live."
  def retire(%__MODULE__{cells: cells}), do: :atomics.exchange(cells, @state, 1) == 0

  def retired?(%__MODULE__{cells: cells}), do: :atomics.get(cells, @state) == 1

  @doc """
  `:none`, `:steps` or `:mem`: the sticky cell, cross-checked with the counters, so a
  target that clears the cell after a breach is still judged by `steps_used >
  step_budget` and `mem_used > mem_budget`.
  """
  def exhausted(%__MODULE__{cells: cells}) do
    cond do
      :atomics.get(cells, @exhausted) == 1 -> :steps
      :atomics.get(cells, @exhausted) == 2 -> :mem
      :atomics.get(cells, @steps) > :atomics.get(cells, @step_budget) -> :steps
      :atomics.get(cells, @mem) > :atomics.get(cells, @mem_budget) -> :mem
      true -> :none
    end
  end

  @doc "The index of the sticky exhausted cell (0 none, 1 steps, 2 memory); for tests of the cross-check."
  def exhausted_cell, do: @exhausted

  def edge_calls(%__MODULE__{cells: cells}), do: :atomics.get(cells, @edge_calls)
  def steps_used(%__MODULE__{cells: cells}), do: :atomics.get(cells, @steps)
  def mem_used(%__MODULE__{cells: cells}), do: :atomics.get(cells, @mem)

  @doc "The distinct edge ids set so far, ascending."
  def edges(%__MODULE__{cells: cells}) do
    for i <- 0..(@word_count - 1),
        w = :atomics.get(cells, @words + i),
        w != 0,
        bit <- 0..63,
        band(w, 1 <<< bit) != 0,
        do: i * 64 + bit
  end
end
