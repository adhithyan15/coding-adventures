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
    Options,
    Report,
    Signature,
    Verdict
  }

  defdelegate corpus_admit(existing, trace), to: Bitmap
  defdelegate calculate_energy(base, len, new, total), to: Bitmap
  defdelegate run_outcomes(initial_corpus, options, exec), to: Engine
  defdelegate tap_failures(report, target, first_test), to: Signature
  defdelegate format_tap_failure(n, target, failure_id, sig), to: Signature

  @doc """
  The outcome-level executor for `target`: `(data) -> {outcome, edges}`, one
  monitored process per call.  Used by `run/3` and by `verdict/4` to re-verify.
  """
  def executor(target, %Options{} = options) when is_function(target, 2) do
    fn data ->
      {outcome, edges, _calls} = Executor.run(target, data, options)
      {outcome, edges}
    end
  end

  @doc """
  Runs a campaign against `target` (a function `(data, handle) -> any`).  Any
  `raise`, `throw` or `exit` escaping it is a crash.  The coverage tier is
  `caller-reported` if the target called `Handle.edge/2` during the campaign and
  `black-box-uninstrumented` otherwise.
  """
  def run(initial_corpus, %Options{} = options, target) when is_function(target, 2) do
    calls = :counters.new(1, [])

    exec = fn data ->
      {outcome, edges, n} = Executor.run(target, data, options)
      :counters.add(calls, 1, n)
      {outcome, edges}
    end

    tier = fn _total ->
      if :counters.get(calls, 1) > 0, do: Report.tier_reported(), else: Report.tier_black_box()
    end

    Engine.run_outcomes(initial_corpus, options, exec, tier: tier)
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

    crashes =
      Enum.map(report.crashes, fn rec ->
        %{signature: rec.signature, kind: rec.kind, frames: rec.frames, hits: rec.hits, status: reverify(rec, exec)}
      end)

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
