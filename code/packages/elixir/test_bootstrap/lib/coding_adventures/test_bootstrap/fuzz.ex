defmodule CodingAdventures.TestBootstrap.Fuzz do
  @moduledoc """
  The coverage-guided fuzz engine (VER06n): the public facade.

      alias CodingAdventures.TestBootstrap.Fuzz
      alias CodingAdventures.TestBootstrap.Fuzz.{Handle, Options}

      report =
        Fuzz.run([<<0, 0>>], Options.new(max_iterations: 500, seed: 7), fn data, h ->
          Handle.edge(h, Handle.edge_id("entered"))
          if data == <<0xFF, 0xFF>>, do: raise("boom")
          :ok
        end)

      Fuzz.Report.clean?(report)

  Every execution of `target` runs in its own monitored, heap-limited process with a
  fresh `Handle` (see `Fuzz.Executor`).  Coverage is what the target reports
  through `Handle.edge/2` (VER06n section 4.2: `:cover` is not used in the hot path).
  """
  alias CodingAdventures.TestBootstrap.Fuzz.{
    Bitmap,
    Engine,
    Executor,
    Limits,
    Options,
    Report,
    Signature,
    Verdict
  }

  @max_drain 50_000

  defdelegate corpus_admit(existing, trace), to: Bitmap
  defdelegate calculate_energy(base, len, new, total), to: Bitmap
  defdelegate run_outcomes(initial_corpus, options, exec), to: Engine
  defdelegate tap_failures(report, target, first_test), to: Signature
  defdelegate format_tap_failure(n, target, failure_id, sig), to: Signature

  @doc """
  The outcome-level executor for `target`: `(data) -> {outcome, edges}`, one
  monitored process per call.  Used by `run/3` and by `verdict/4` to re-verify.
  """
  def executor(target, %Options{} = options, flags \\ []) when is_function(target, 2) do
    drain? = Keyword.get(flags, :drain, false)

    fn data ->
      {outcome, edges, _calls} = Executor.run(target, data, options)
      if drain?, do: drain(@max_drain, Process.get(Executor.caller_key()))
      {outcome, edges}
    end
  end

  # Stray messages (a target flooding its parent) are dropped between executions,
  # at most @max_drain per drain, so the engine's mailbox cannot slow later receives.
  @doc false
  # `cmon` is the engine's monitor on its caller (nil outside an engine).  If the drain
  # swallows that :DOWN no execution is live, so the engine simply exits.
  def drain(0, _cmon), do: :ok

  def drain(n, cmon) do
    receive do
      {:DOWN, ^cmon, :process, _pid, _reason} -> exit(:caller_down)
      _stray -> drain(n - 1, cmon)
    after
      0 -> :ok
    end
  end

  # Runs `fun` in a DEDICATED engine process: the target's parent is that process,
  # so whatever a target sends "to its parent" never reaches the caller's mailbox.
  # The caller gets the result through a monitored reply with a bounded receive.
  defp in_engine(fun, bound_ms, mem_budget) do
    tag = make_ref()
    caller = self()
    # the engine's heap holds the retained corpus plus whatever a target floods it with
    words = div(Limits.max_retained_bytes() + mem_budget + 64 * 1024 * 1024, 8)

    {pid, mon} =
      :erlang.spawn_opt(
        fn ->
          # trap exits (a linked target's crash must never kill the engine) and watch the caller
          Process.flag(:trap_exit, true)
          Process.put(Executor.caller_key(), Process.monitor(caller))

          reply =
            try do
              {:ok, fun.()}
            catch
              kind, reason -> {:raised, kind, reason, __STACKTRACE__}
            end

          send(caller, {tag, reply})
        end,
        [:monitor, {:max_heap_size, %{size: words, kill: true, error_logger: false}}]
      )

    receive do
      {^tag, {:ok, value}} ->
        Process.demonitor(mon, [:flush])
        value

      {^tag, {:raised, kind, reason, stack}} ->
        Process.demonitor(mon, [:flush])
        :erlang.raise(kind, reason, stack)

      {:DOWN, ^mon, :process, ^pid, reason} ->
        raise "the fuzz engine process died: #{inspect(reason)}"
    after
      bound_ms ->
        # killing the engine also kills the target linked to it
        Process.exit(pid, :kill)
        Process.demonitor(mon, [:flush])
        raise "the fuzz engine exceeded its bound of #{bound_ms} ms"
    end
  end

  # An upper bound on one campaign: every execution is bounded by the wall clock
  # plus the kill wait.
  defp campaign_bound(initial, %Options{} = o) do
    per = 2 * (o.timeout_ms + Executor.kill_wait_ms() + 1000)
    min((length(initial) + 2 * o.max_iterations + 2) * per + 60_000, 4_294_967_295)
  end

  @doc """
  Runs a campaign against `target` (a function `(data, handle) -> any`).  Any
  `raise`, `throw` or `exit` escaping it is a crash.  The coverage tier is
  `caller-reported` if the target called `Handle.edge/2` during the campaign and
  `black-box-uninstrumented` otherwise.
  """
  def run(initial_corpus, %Options{} = options, target, flags \\ []) when is_function(target, 2) do
    unless is_list(initial_corpus), do: raise(ArgumentError, "initial_corpus must be a list of binaries")
    bound = Keyword.get(flags, :bound_ms, campaign_bound(initial_corpus, options))

    # validated BEFORE an engine is spawned: 0..2^32-1 (the range of `receive ... after`)
    unless is_integer(bound) and bound >= 0 and bound <= 4_294_967_295,
      do: raise(ArgumentError, "bound_ms must be an integer in 0..4294967295")

    in_engine(
      fn ->
        calls = :counters.new(1, [])

        exec = fn data ->
          {outcome, edges, n} = Executor.run(target, data, options)
          :counters.add(calls, 1, n)
          drain(@max_drain, Process.get(Executor.caller_key()))
          {outcome, edges}
        end

        tier = fn _total ->
          if :counters.get(calls, 1) > 0, do: Report.tier_reported(), else: Report.tier_black_box()
        end

        Engine.run_outcomes(initial_corpus, options, exec, tier: tier)
      end,
      bound,
      options.mem_budget
    )
  end

  @doc """
  The honest verdict of a report (VER06n section 6).

  Before a crash is reported its minimal input is run once more through `exec`
  (`(data) -> {outcome, edges}`): the crash is `:reproduced` if the signature comes
  back and `:flaky` if it does not (it still fails the verdict, fail closed);
  `:unverified` when no input was kept.  `passed` is true only for a clean report
  (so zero executions never pass).  With `options.ci` of `:on` or `:unset` the
  verdict carries no seed and no witness.
  """
  def verdict(%Report{} = report, %Options{} = options, exec, target_id \\ "fuzz")
      when is_function(exec, 1) and is_binary(target_id) do
    redact = options.ci != :off
    clean = Report.clean?(report)

    # re-verification runs in a dedicated engine process too (a crashing target can
    # flood its parent)
    crashes =
      in_engine(
        fn ->
          Enum.map(report.crashes, fn rec ->
            %{signature: rec.signature, kind: rec.kind, frames: rec.frames, hits: rec.hits, status: reverify(rec, exec)}
          end)
        end,
        min((length(report.crashes) + 1) * 2 * (options.timeout_ms + Executor.kill_wait_ms() + 1000) + 60_000, 4_294_967_295),
        options.mem_budget
      )

    %Verdict{
      passed: clean,
      reason: if(clean, do: nil, else: reason(report)),
      lines: Signature.tap_failures(report, target_id, 1),
      crashes: crashes,
      iterations: report.iterations,
      timeouts: report.timeouts,
      ooms: report.ooms,
      seed: if(redact, do: nil, else: report.seed),
      witnesses: if(redact, do: nil, else: for(r <- report.crashes, not r.input_dropped, do: {r.signature, r.minimal_input}))
    }
  end

  defp reverify(%{input_dropped: true}, _exec), do: :unverified

  defp reverify(rec, exec) do
    {outcome, _edges} = Engine.execute(exec, rec.minimal_input)
    if Engine.same_crash?(outcome, rec.signature), do: :reproduced, else: :flaky
  end

  defp reason(r) do
    cond do
      r.crashes != [] -> :crash
      r.timeouts > 0 -> :timeout
      r.ooms > 0 -> :oom
      r.crashes_dropped > 0 -> :crashes_dropped
      r.admissions_dropped > 0 -> :admissions_dropped
      true -> :no_iterations
    end
  end
end
