defmodule CodingAdventures.TestBootstrap.Pbt.Verdict do
  @moduledoc """
  The outcome of `Runner.check`.  In redacted (CI) mode `seed`, `counterexample`
  and `minimal` are nil; only the message, iteration, step count and digest remain.
  """
  defstruct passed: false,
            failure_id: nil,
            message: nil,
            cases_run: 0,
            iteration: 0,
            shrink_steps: 0,
            seed: nil,
            counterexample: nil,
            minimal: nil,
            digest: nil

  def failed?(%__MODULE__{passed: passed}), do: not passed
end

defmodule CodingAdventures.TestBootstrap.Pbt.Options do
  @moduledoc """
  Runner options.  `ci` is `:on`, `:off` or `:unset`.  `:unset` is treated as CI:
  the engine fails closed and redacts.  The library never reads the environment;
  `ci_from_lookup/1` is a pure helper (pass `&System.get_env/1` or a map).
  """
  alias CodingAdventures.TestBootstrap.Pbt.Limits

  defstruct num_tests: 100, max_shrink_steps: 1000, ci: :unset

  @doc "Validated options from a keyword list; raises `ArgumentError` on bad values."
  def new(opts \\ []) do
    o = struct!(__MODULE__, opts)

    unless o.ci in [:on, :off, :unset],
      do: raise(ArgumentError, "ci must be :on, :off or :unset")

    Limits.check_range!(o.num_tests, 0, Limits.max_cases(), "num_tests")
    Limits.check_range!(o.max_shrink_steps, 0, Limits.max_shrink_steps(), "max_shrink_steps")
    o
  end

  @doc """
  `CI` set to anything except empty, "0" or "false" (case-insensitive, trimmed)
  means CI.  `lookup` is a one-argument function or a map keyed by name.
  """
  def ci_from_lookup(lookup) do
    value = if is_function(lookup, 1), do: lookup.("CI"), else: Map.get(lookup, "CI")

    case value do
      nil ->
        :off

      v when is_binary(v) ->
        t = v |> String.trim() |> String.downcase()
        if t in ["", "0", "false"], do: :off, else: :on

      _ ->
        :on
    end
  end
end

defmodule CodingAdventures.TestBootstrap.Pbt.Runner do
  @moduledoc """
  The honest runner (VER05 7.1, VER05e 5, VER05m).

    * zero cases evaluated is `property-exhausted`, never a pass;
    * a generator failure of any kind is `property-exhausted`;
    * a property that raises, throws or exits is `property-panic`;
    * a property passes only by returning exactly `true`: `false`, `nil` and every
      other term falsify (fail closed);
    * the reported minimal witness is RE-VERIFIED: if the shrinker fails, returns
      something that no longer fails, a value the property raises on, or a
      nonsensical step count, the original candidate is reported with 0 steps;
    * redacted mode never emits the witness or the seed;
    * the call always returns a verdict (invalid arguments raise instead).

  There is no wall-clock timeout: `property-timeout` is registered and never
  produced, so a property that loops forever hangs its process.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.{Prng, Safe}
  alias CodingAdventures.TestBootstrap.CorpusJson.Canon
  alias CodingAdventures.TestBootstrap.Pbt.{Budget, ExhaustedError, FailureIds, Gen, Hooks, Options, Verdict}

  # Most elements the work estimate of one sampled value will visit.
  @walk_cap 10_000

  def walk_cap, do: @walk_cap

  @doc "Checks `prop` over the default options."
  def check(seed, gen, prop), do: check(seed, gen, Options.new(), prop, nil)
  def check(seed, gen, options, prop), do: check(seed, gen, options, prop, nil)

  @doc """
  Runs `prop` over `options.num_tests` candidates from `seed`.  `shrink`
  overrides the generator's shrinker (so a defective shrinker can be tested).
  """
  def check(seed, %Gen{} = gen, options, prop, shrink) when is_integer(seed) and is_function(prop, 1) do
    unless is_nil(shrink) or is_function(shrink, 3),
      do: raise(ArgumentError, "shrink must be a 3-arity function")

    opts = options || Options.new()
    unless is_struct(opts, Options), do: raise(ArgumentError, "options must be Options")

    seed = band(seed, Prng.mask64())
    redact = opts.ci != :off

    if opts.num_tests == 0 do
      verdict(false, FailureIds.exhausted(), "no cases were evaluated (num_tests is 0); a property that never ran is not a pass", 0, 0)
    else
      # A check nested in a property (an ambient scope exists) pays one attempt at
      # entry, one per case after the first and the size of every sampled
      # collection as work, all charged up the chain.
      outer = Budget.current()
      nested = outer != nil

      if nested and not Budget.charge(outer, 1) do
        spent(0)
      else
        # The WHOLE check runs in one scope (case loop included).
        Budget.enter_check(opts.num_tests, opts.max_shrink_steps, fn scope ->
          ctx = %{seed: seed, gen: gen, prop: prop, shrink: shrink, opts: opts, redact: redact, scope: scope, nested: nested}
          loop(1, Hooks.get(:new_rng).(seed), ctx)
        end)
      end
    end
  end

  def check(_seed, _gen, _options, _prop, _shrink),
    do: raise(ArgumentError, "check needs an integer seed, a Gen and a 1-arity property")

  defp loop(iteration, _rng, %{opts: %{num_tests: n}} = ctx) when iteration > n do
    if not ctx.nested and Budget.refused?(ctx.scope) do
      # A nested check or generator inside this run was starved, so the property
      # was not fully evaluated: never report that as a pass.
      verdict(false, FailureIds.exhausted(), "a nested check was starved by the shrink budget; the property was not fully evaluated", n, n)
    else
      verdict(true, nil, nil, n, 0)
    end
  end

  defp loop(iteration, rng, ctx) do
    if ctx.nested and iteration > 1 and not Budget.charge(ctx.scope, 1) do
      spent(iteration)
    else
      case Safe.call(fn -> Gen.sample(ctx.gen, rng) end) do
        {:raised, kind, reason} ->
          exhausted(kind, reason, iteration, ctx.redact)

        {:ok, {candidate, rng2}} ->
          if ctx.nested and not Budget.charge_copy(ctx.scope, size_of(candidate)) do
            spent(iteration)
          else
            case Safe.call(fn -> ctx.prop.(candidate) end) do
              {:ok, true} -> loop(iteration + 1, rng2, ctx)
              {:ok, _not_true} -> falsified(iteration, candidate, ctx)
              {:raised, kind, reason} -> panic(iteration, kind, reason, ctx)
            end
          end

        {:ok, _malformed} ->
          exhausted(:error, :bad_sampler_result, iteration, ctx.redact)
      end
    end
  end

  defp verdict(passed, failure_id, message, cases_run, iteration, extra \\ []) do
    %Verdict{
      passed: passed,
      failure_id: failure_id,
      message: message,
      cases_run: cases_run,
      iteration: iteration,
      shrink_steps: Keyword.get(extra, :shrink_steps, 0),
      seed: Keyword.get(extra, :seed),
      counterexample: Keyword.get(extra, :counterexample),
      minimal: Keyword.get(extra, :minimal),
      digest: Keyword.get(extra, :digest)
    }
  end

  # The enclosing shrink budget was spent: exhausted, never a pass.
  defp spent(iteration) do
    verdict(false, FailureIds.exhausted(), "the enclosing shrink budget was spent at iteration #{iteration}", max(0, iteration - 1), iteration)
  end

  @doc """
  An approximate size of a sampled value in elements (work): a binary counts its
  bytes; a list, tuple or map its length plus, one level deeper, the length of
  each element that is itself one of those.  ONE visit allowance of 10,000 is
  shared by the whole walk.  Approximate by design; a hostile term counts 0.
  """
  def size_of(value) do
    n = shallow(value)

    if n == 0 do
      0
    else
      children =
        cond do
          is_list(value) -> value
          is_map(value) -> Map.values(value)
          is_tuple(value) -> Tuple.to_list(value)
          true -> []
        end

      n + (children |> Enum.take(@walk_cap) |> Enum.reduce(0, fn e, acc -> acc + shallow(e) end))
    end
  rescue
    _ -> 0
  end

  defp shallow(v) when is_binary(v), do: byte_size(v)
  defp shallow(v) when is_list(v), do: length(v)
  defp shallow(%{__struct__: _} = v), do: map_size(v)
  defp shallow(v) when is_map(v), do: map_size(v)
  defp shallow(v) when is_tuple(v), do: tuple_size(v)
  defp shallow(_), do: 0

  defp exhausted(kind, reason, iteration, redact) do
    text =
      cond do
        kind == :error and is_struct(reason, ExhaustedError) ->
          "filter precondition rejected all candidates at iteration #{iteration}"

        redact ->
          "generator error at iteration #{iteration} (#{Safe.redacted(kind, reason)})"

        true ->
          "generator error at iteration #{iteration}: #{Safe.local(kind, reason)}"
      end

    verdict(false, FailureIds.exhausted(), text, max(0, iteration - 1), iteration)
  end

  defp panic(iteration, kind, reason, %{redact: true}) do
    verdict(false, FailureIds.panic(), "property evaluation raised at iteration #{iteration} (#{Safe.redacted(kind, reason)})", iteration, iteration)
  end

  defp panic(iteration, kind, reason, %{seed: seed}) do
    verdict(false, FailureIds.panic(), "property evaluation raised at iteration #{iteration}: #{Safe.local(kind, reason)}", iteration, iteration, seed: seed)
  end

  defp falsified(iteration, candidate, ctx) do
    %{prop: prop, opts: opts, scope: check_scope} = ctx

    # A candidate the property raises, throws or exits on is not a counterexample
    # to the claim; it is a different failure, so it is never accepted as "smaller".
    falsifies = fn v ->
      case Safe.call(fn -> prop.(v) end) do
        {:ok, true} -> false
        {:ok, _} -> true
        {:raised, _, _} -> false
      end
    end

    # Untrusted shrinker: fall back to the original.  The shrink scope is a child
    # of the check scope (itself a child of any enclosing shrink): every
    # evaluation is charged to the whole chain.
    shrunk =
      Safe.call(fn ->
        Budget.enter_child(opts.max_shrink_steps, fn scope ->
          budgeted = Budget.charged(scope, falsifies)

          if ctx.shrink,
            do: ctx.shrink.(candidate, budgeted, opts.max_shrink_steps),
            else: Gen.shrink_nested(ctx.gen, candidate, budgeted, opts.max_shrink_steps)
        end)
      end)

    # Re-verify: the witness we report must really fail the property.  One
    # attempt is charged to the check scope; if the chain refuses it the shrunk
    # value is not adopted.
    {minimal, steps} =
      case shrunk do
        {:ok, {value, used}} when is_integer(used) and used >= 0 and used <= opts.max_shrink_steps ->
          if Budget.charge(check_scope, 1) and falsifies.(value),
            do: {value, used},
            else: {candidate, 0}

        _ ->
          {candidate, 0}
      end

    report(iteration, candidate, minimal, steps, ctx)
  end

  defp report(iteration, _candidate, minimal, steps, %{redact: true}) do
    preimage = "iter:#{iteration};steps:#{steps};shrunk:#{Canon.render(minimal)}"
    digest = Prng.fnv1a64(preimage) |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(16, "0")

    verdict(false, FailureIds.counterexample(), "property falsified after #{iteration} iterations (shrunk in #{steps} steps, repro-fnv: #{digest})", iteration, iteration, shrink_steps: steps, digest: digest)
  end

  defp report(iteration, candidate, minimal, steps, %{seed: seed}) do
    verdict(
      false,
      FailureIds.counterexample(),
      "property falsified at iteration #{iteration} (seed=#{seed}): initial=#{Canon.render(candidate)}, minimal=#{Canon.render(minimal)} after #{steps} shrink steps",
      iteration,
      iteration,
      shrink_steps: steps,
      seed: seed,
      counterexample: candidate,
      minimal: minimal
    )
  end
end
