defmodule CodingAdventures.TestBootstrap.Fuzz.Engine do
  @moduledoc """
  The deterministic, budgeted coverage-guided campaign (VER06 sections 5 and 9.1,
  VER06n sections 5 and 6).

  A campaign is a pure function of `(corpus, options, exec)`: triage every seed,
  then repeatedly pick a seed, give it an energy-scaled number of mutated
  children, run each child and either admit it (new coverage), record it (crash,
  timeout, oom) or drop it (redundant).  The engine itself reads no clock, file or
  environment; only `Executor` spawns processes.

  `exec` is a function `(data) -> {outcome, edges}` where `outcome` is `:pass`,
  `{:crash, kind, frames}`, `:timeout` or `:oom` and `edges` a list of integers
  (the trace of that execution).

  Per iteration the stream draws: `bounded(len(corpus))` for the seed; per energy
  slot `1 + bounded(4)` steps, one `next_u64` sub-seed, then, if there is more than
  one seed, `bounded(4)` and on 0 a donor index and two split draws.  That order is
  the cross-lane contract.
  """
  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Fuzz.{Bitmap, CrashRecord, Limits, Mutate, Options, Report, Signature}

  # -----------------------------------------------------------------------------
  # Minimisation (VER06 section 5.3)
  # -----------------------------------------------------------------------------

  def same_crash?({:crash, kind, frames}, sig), do: Signature.crash_signature(kind, frames) == sig
  def same_crash?(_outcome, _sig), do: false

  @doc """
  Shrinks `data` while it keeps the EXACT signature.  Returns
  `{bytes, executions_spent, finished}`.  EVERY candidate execution, accepted or
  rejected, is counted and the budget is checked before each one, so no stream of
  rejected candidates can run past it.

  Pass 1 deletes blocks, halving the block size.  It starts at `max(1, len div 2)`
  (not `len div 2`), which is what makes the empty input reachable from a 1-byte
  crasher; the chunk sizes tried for a 9-byte input are 4, 2, 1.  Pass 2 zeroes
  individual non-essential bytes.  `probe` is `(candidate) -> outcome`.
  """
  def minimize(data, sig, probe, budget) when is_binary(data) do
    if data == <<>> do
      {data, 0, true}
    else
      delete_pass(data, max(1, div(byte_size(data), 2)), 0, 0, budget, sig, probe)
    end
  end

  defp delete_pass(cur, 0, _offset, spent, budget, sig, probe),
    do: zero_pass(cur, 0, spent, budget, sig, probe)

  defp delete_pass(cur, chunk, offset, spent, budget, sig, probe) do
    size = byte_size(cur)

    cond do
      offset >= size ->
        delete_pass(cur, div(chunk, 2), 0, spent, budget, sig, probe)

      spent >= budget ->
        {cur, spent, false}

      true ->
        stop = offset + min(chunk, size - offset)
        candidate = binary_part(cur, 0, offset) <> binary_part(cur, stop, size - stop)

        if same_crash?(probe.(candidate), sig),
          # same offset again: the tail slid left
          do: delete_pass(candidate, chunk, offset, spent + 1, budget, sig, probe),
          else: delete_pass(cur, chunk, offset + chunk, spent + 1, budget, sig, probe)
    end
  end

  defp zero_pass(cur, i, spent, _budget, _sig, _probe) when i >= byte_size(cur), do: {cur, spent, true}

  defp zero_pass(cur, i, spent, budget, sig, probe) do
    original = :binary.at(cur, i)

    cond do
      original == 0 ->
        zero_pass(cur, i + 1, spent, budget, sig, probe)

      spent >= budget ->
        {cur, spent, false}

      true ->
        <<pre::binary-size(i), _::8, post::binary>> = cur
        candidate = <<pre::binary, 0, post::binary>>

        if same_crash?(probe.(candidate), sig),
          do: zero_pass(candidate, i + 1, spent + 1, budget, sig, probe),
          else: zero_pass(cur, i + 1, spent + 1, budget, sig, probe)
    end
  end

  @doc "Shrinks `data` while it keeps the exact signature; `budget` caps executions of `probe`."
  def minimize_crashing_input(data, sig, budget, probe) when is_function(probe, 1) do
    Limits.check_range!(budget, 0, Limits.max_minimize_budget(), "budget")
    unless is_binary(data), do: raise(ArgumentError, "data must be a binary")

    if byte_size(data) > Limits.max_input_bytes(),
      do: raise(ArgumentError, "data exceeds #{Limits.max_input_bytes()} bytes")

    data |> minimize(sig, probe, budget) |> elem(0)
  end

  # -----------------------------------------------------------------------------
  # The campaign
  # -----------------------------------------------------------------------------

  defp new_state(exec, %Options{} = o, retained_cap) do
    %{
      exec: exec,
      opts: o,
      rng: Prng.new(o.seed),
      cum: Bitmap.new(),
      corpus: %{},
      size: 0,
      crashes: %{},
      order: [],
      timeouts: 0,
      ooms: 0,
      dropped: 0,
      adm_dropped: 0,
      minimize_left: o.max_iterations,
      retained: 0,
      retained_cap: retained_cap
    }
  end

  @doc "Runs `exec` on one input and validates what it handed back: `{outcome, edges}`."
  def execute(exec, data) do
    case exec.(data) do
      {outcome, edges} when is_list(edges) ->
        unless valid_outcome?(outcome), do: raise(ArgumentError, "exec returned an invalid outcome")
        Enum.each(edges, &Limits.check_int!(&1, "edge"))
        {outcome, edges}

      _ ->
        raise ArgumentError, "exec must return {outcome, edges}"
    end
  end

  defp valid_outcome?(:pass), do: true
  defp valid_outcome?(:timeout), do: true
  defp valid_outcome?(:oom), do: true
  defp valid_outcome?({:crash, kind, frames}), do: is_binary(kind) and is_list(frames) and Enum.all?(frames, &is_binary/1)
  defp valid_outcome?(_), do: false

  # Puts one non-passing execution in its bucket.
  defp file(st, :timeout, _data), do: %{st | timeouts: st.timeouts + 1}
  defp file(st, :oom, _data), do: %{st | ooms: st.ooms + 1}
  defp file(st, {:crash, _, _} = out, data), do: record_crash(st, out, data)

  # Puts a passing input in the corpus iff it covered a new edge.  The first passing
  # seed is always admitted, even with no coverage, so there is something to mutate
  # in black-box mode.
  defp admit(st, data, generation, edges) do
    {has_new, count_new, cum} = Bitmap.admit(st.cum, edges)
    st = %{st | cum: cum}
    first_seed = generation == 0 and st.size == 0

    cond do
      not (has_new or first_seed) ->
        st

      st.retained + byte_size(data) > st.retained_cap ->
        %{st | adm_dropped: st.adm_dropped + 1}

      true ->
        entry = %{data: data, new_edges: count_new, generation: generation}

        %{
          st
          | corpus: Map.put(st.corpus, st.size, entry),
            size: st.size + 1,
            retained: st.retained + byte_size(data)
        }
    end
  end

  defp record_crash(st, {:crash, kind, frames}, data) do
    sig = Signature.crash_signature(kind, frames)

    case st.crashes do
      %{^sig => rec} -> repeat_crash(st, sig, rec, data)
      _ -> novel_crash(st, sig, kind, frames, data)
    end
  end

  defp repeat_crash(st, sig, rec, data) do
    if not rec.input_dropped and byte_size(data) < byte_size(rec.minimal_input) do
      saved = byte_size(rec.minimal_input) - byte_size(data)
      rec = %{rec | hits: rec.hits + 1, minimal_input: data}
      %{st | crashes: Map.put(st.crashes, sig, rec), retained: st.retained - saved}
    else
      %{st | crashes: Map.put(st.crashes, sig, %{rec | hits: rec.hits + 1})}
    end
  end

  defp novel_crash(st, sig, kind, frames, data) do
    if map_size(st.crashes) >= Limits.max_crashes() do
      %{st | dropped: st.dropped + 1}
    else
      allowance = min(st.opts.minimize_budget, st.minimize_left)
      # The minimiser's re-run: coverage is discarded and nothing is counted.
      probe = fn d -> d |> then(&execute(st.exec, &1)) |> elem(0) end
      {kept, spent, finished} = minimize(data, sig, probe, allowance)

      {kept, dropped_input, retained} =
        if st.retained + byte_size(kept) > st.retained_cap,
          do: {<<>>, true, st.retained},
          else: {kept, false, st.retained + byte_size(kept)}

      rec = %CrashRecord{
        signature: sig,
        kind: kind,
        frames: frames,
        minimal_input: kept,
        hits: 1,
        minimized: finished and not dropped_input,
        input_dropped: dropped_input
      }

      %{
        st
        | crashes: Map.put(st.crashes, sig, rec),
          order: [sig | st.order],
          minimize_left: st.minimize_left - spent,
          retained: retained
      }
    end
  end

  @doc """
  Runs a bounded coverage-guided campaign over `exec` (`(data) -> {outcome, edges}`).
  Contracts (VER06 section 9.1), each pinned by a test:

    * a pure function of (corpus, options, exec): same inputs, same report;
    * with a live corpus `iterations == max_iterations` exactly, and no mutated
      candidate exceeds `max_input_len` (initial seeds run as given);
    * an empty corpus is replaced by one `<<0>>` seed; seeds that crash, time out or
      exhaust memory are never mutated, so a corpus of crashers fuzzes nothing;
    * every execution lands in crashes, timeouts, ooms or the corpus; `clean?`
      needs at least one iteration and no failure at all;
    * one `CrashRecord` per signature; a novel crash is minimised.

  `opts`: `:retained_cap` (test seam for the 256 MiB cap) and `:tier` (a function
  `total_edges -> label`).
  """
  def run_outcomes(initial_corpus, %Options{} = o, exec, opts \\ []) when is_function(exec, 1) do
    cap = Keyword.get(opts, :retained_cap, Limits.max_retained_bytes())
    tier = Keyword.get(opts, :tier)

    unless is_list(initial_corpus) and Enum.all?(initial_corpus, &is_binary/1),
      do: raise(ArgumentError, "initial_corpus must be a list of binaries")

    if length(initial_corpus) > Limits.max_initial_corpus(),
      do: raise(ArgumentError, "initial corpus exceeds #{Limits.max_initial_corpus()} seeds")

    Enum.each(initial_corpus, fn s ->
      if byte_size(s) > Limits.max_input_bytes(),
        do: raise(ArgumentError, "an initial corpus seed exceeds #{Limits.max_input_bytes()} bytes")
    end)

    seeds = if initial_corpus == [], do: [<<0>>], else: initial_corpus

    # 1. Triage: run every seed once.
    st =
      Enum.reduce(seeds, new_state(exec, o, cap), fn data, st ->
        {out, edges} = execute(st.exec, data)
        if out == :pass, do: admit(st, data, 0, edges), else: file(st, out, data)
      end)

    # 2. The mutation loop.
    {iterations, st} = mutate_loop(st, 0)
    total = Bitmap.count(st.cum)

    %Report{
      iterations: iterations,
      admitted_seeds: st.size,
      total_edges: total,
      crashes: st.order |> Enum.reverse() |> Enum.map(&Map.fetch!(st.crashes, &1)),
      timeouts: st.timeouts,
      ooms: st.ooms,
      crashes_dropped: st.dropped,
      admissions_dropped: st.adm_dropped,
      coverage_tier: if(tier, do: tier.(total), else: default_tier(total)),
      seed: o.seed,
      corpus: for(i <- 0..(st.size - 1)//1, do: st.corpus[i].data)
    }
  end

  defp default_tier(total), do: if(total > 0, do: Report.tier_reported(), else: Report.tier_black_box())

  defp mutate_loop(st, iterations) do
    if iterations >= st.opts.max_iterations or st.size == 0 do
      {iterations, st}
    else
      {idx, rng} = Prng.bounded(st.rng, st.size)
      seed = st.corpus[idx]
      energy = Bitmap.calculate_energy(st.opts.base_energy, byte_size(seed.data), seed.new_edges, Bitmap.count(st.cum))
      {iterations, st} = slots(%{st | rng: rng}, seed, energy, 0, iterations)
      mutate_loop(st, iterations)
    end
  end

  defp slots(st, _seed, energy, slot, iterations) when slot >= energy, do: {iterations, st}

  defp slots(st, _seed, _energy, _slot, iterations) when iterations >= st.opts.max_iterations,
    do: {iterations, st}

  defp slots(st, seed, energy, slot, iterations) do
    {n, rng} = Prng.bounded(st.rng, 4)
    {sub, rng} = Prng.next_u64(rng)
    candidate = Mutate.mutate_sequence(seed.data, sub, 1 + n, Limits.max_input_bytes())
    {candidate, rng} = maybe_splice(candidate, st, rng)
    candidate = binary_part(candidate, 0, min(byte_size(candidate), st.opts.max_input_len))
    st = %{st | rng: rng}
    {out, edges} = execute(st.exec, candidate)

    st =
      if out == :pass,
        do: admit(st, candidate, seed.generation + 1, edges),
        else: file(st, out, candidate)

    slots(st, seed, energy, slot + 1, iterations + 1)
  end

  defp maybe_splice(candidate, %{size: size} = st, rng) when size > 1 do
    {pick, rng} = Prng.bounded(rng, 4)

    if pick == 0 do
      {di, rng} = Prng.bounded(rng, size)
      donor = st.corpus[di].data
      {t_split, rng} = Prng.bounded(rng, byte_size(candidate) + 1)
      {d_split, rng} = Prng.bounded(rng, byte_size(donor) + 1)
      {Mutate.splice(candidate, donor, t_split, d_split), rng}
    else
      {candidate, rng}
    end
  end

  defp maybe_splice(candidate, _st, rng), do: {candidate, rng}
end
