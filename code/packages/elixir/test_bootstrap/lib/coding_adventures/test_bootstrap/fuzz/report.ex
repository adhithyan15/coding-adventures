defmodule CodingAdventures.TestBootstrap.Fuzz.CrashRecord do
  @moduledoc """
  One deduplicated crash.  `minimal_input` is the shortest input known to
  reproduce the signature; it is PRIVATE (VER06 section 6.3): never in `inspect`,
  and public output goes through `Signature.format_tap_failure/4` only.
  `minimized` is false when minimisation was skipped or cut short by a budget;
  `input_dropped` is true when the retained-bytes cap stopped the input being kept.
  """
  defstruct [:signature, :kind, :frames, :minimal_input, :hits, :minimized, :input_dropped]

  defimpl Inspect do
    def inspect(r, _opts), do: "#Fuzz.CrashRecord<#{r.signature} #{r.kind} frames=#{length(r.frames)} hits=#{r.hits}>"
  end
end

defmodule CodingAdventures.TestBootstrap.Fuzz.Report do
  @moduledoc """
  What a campaign found.  Every execution lands in exactly one bucket:
  `iterations` counts MUTATION executions; crashes, timeouts and ooms count triage
  and mutation executions; anything that passed is in the corpus or was redundant.
  The seed and corpus never appear in `inspect` (VER06 section 6).
  """
  @tier_reported "caller-reported"
  @tier_black_box "black-box-uninstrumented"

  defstruct iterations: 0,
            admitted_seeds: 0,
            total_edges: 0,
            crashes: [],
            timeouts: 0,
            ooms: 0,
            crashes_dropped: 0,
            admissions_dropped: 0,
            coverage_tier: @tier_black_box,
            seed: nil,
            corpus: []

  def tier_reported, do: @tier_reported
  def tier_black_box, do: @tier_black_box

  defimpl Inspect do
    def inspect(r, _opts) do
      "#Fuzz.Report<iterations=#{r.iterations} seeds=#{r.admitted_seeds} edges=#{r.total_edges} " <>
        "crashes=#{length(r.crashes) + r.crashes_dropped} timeouts=#{r.timeouts} ooms=#{r.ooms} tier=#{r.coverage_tier}>"
    end
  end

  @doc """
  Can this be read as "the campaign found nothing"?  Only if at least one mutation
  iteration ran and nothing crashed, timed out, ran out of memory or was dropped.
  ZERO EXECUTIONS IS NEVER A PASS.
  """
  def clean?(%__MODULE__{} = r) do
    r.iterations > 0 and r.crashes == [] and r.timeouts == 0 and r.ooms == 0 and
      r.crashes_dropped == 0 and r.admissions_dropped == 0
  end

  def unique_crashes(%__MODULE__{} = r), do: length(r.crashes) + r.crashes_dropped
end

defmodule CodingAdventures.TestBootstrap.Fuzz.Verdict do
  @moduledoc """
  The honest, CI-safe summary of a report (VER06n section 6).  `passed` is true only
  for a clean report.  In CI mode (`ci: :on` or `:unset`) `seed` and `witnesses` are
  nil and `lines` carry only the closed failure id and the 8-hex signature.
  `crashes` holds `%{signature, kind, frames, hits, status}` where `status` is
  `:reproduced`, `:flaky` (the minimal input did not crash again with the same
  signature) or `:unverified` (no input was kept).
  """
  defstruct passed: false,
            reason: nil,
            lines: [],
            crashes: [],
            iterations: 0,
            timeouts: 0,
            ooms: 0,
            seed: nil,
            witnesses: nil

  defimpl Inspect do
    def inspect(v, _opts), do: "#Fuzz.Verdict<passed=#{v.passed} reason=#{inspect(v.reason)} lines=#{length(v.lines)}>"
  end
end
