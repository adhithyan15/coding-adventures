defmodule RunnerTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  alias CodingAdventures.TestBootstrap.Pbt.{Budget, FailureIds, Gens, Options, Runner, Verdict}

  defp opts(n \\ 100, steps \\ 1000, ci \\ :off), do: Options.new(num_tests: n, max_shrink_steps: steps, ci: ci)

  test "a property passes only by returning exactly true" do
    v = Runner.check(1, Gens.u8(), opts(), fn _ -> true end)
    assert v.passed
    assert v.cases_run == 100
    refute Verdict.failed?(v)

    for not_true <- [false, nil, :ok, 1, "yes", {:ok, true}, []] do
      v = Runner.check(1, Gens.u8(), opts(5, 100), fn _ -> not_true end)
      assert v.failure_id == FailureIds.counterexample(), "#{inspect(not_true)} must falsify"
      assert Verdict.failed?(v)
    end
  end

  test "zero cases is never a pass" do
    v = Runner.check(1, Gens.u8(), opts(0, 10), fn _ -> true end)
    refute v.passed
    assert v.failure_id == FailureIds.exhausted()
    assert v.message =~ "never ran"
  end

  test "a falsified property shrinks to the minimal witness" do
    v = Runner.check(42, Gens.integer(0, 100_000), opts(100, 10_000), fn x -> x < 5000 end)
    assert v.failure_id == FailureIds.counterexample()
    assert v.minimal == 5000
    assert v.counterexample >= 5000
    assert v.shrink_steps > 0
    assert v.seed == 42
    assert v.message =~ "seed=42"
    assert v.message =~ "minimal=5000"
  end

  test "check/3 uses default options, which redact (ci is unset)" do
    v = Runner.check(1, Gens.u8(), fn _ -> false end)
    assert v.failure_id == FailureIds.counterexample()
    assert v.seed == nil and v.minimal == nil and v.counterexample == nil
    assert v.digest =~ ~r/\A[0-9a-f]{16}\z/
  end

  test "CI redaction carries no seed, witness or minimal anywhere" do
    for ci <- [:on, :unset] do
      v = Runner.check(777_123, Gens.integer(0, 1_000_000), opts(100, 1000, ci), fn x -> x < 424_242 end)
      assert v.failure_id == FailureIds.counterexample()
      assert v.seed == nil and v.minimal == nil and v.counterexample == nil
      refute v.message =~ "424242"
      refute v.message =~ "777123"
      assert v.digest =~ ~r/\A[0-9a-f]{16}\z/
      assert v.message =~ v.digest
    end
  end

  test "the digest is deterministic" do
    run = fn -> Runner.check(5, Gens.integer(0, 1000), opts(100, 1000, :on), fn x -> x < 500 end) end
    assert run.().digest == run.().digest
  end

  test "raise, throw and exit in a property are property-panic" do
    for f <- [fn _ -> raise "boom" end, fn _ -> throw(:t) end, fn _ -> exit(:e) end, fn _ -> :erlang.error(:badarg) end, fn x -> x / 0 end] do
      v = Runner.check(3, Gens.u8(), opts(), f)
      assert v.failure_id == FailureIds.panic()
      assert v.iteration == 1
      assert v.seed == 3
    end
  end

  test "redacted panic names the kind and exception module only" do
    v = Runner.check(3, Gens.u8(), opts(5, 5, :on), fn _ -> raise ArgumentError, "secret-value" end)
    assert v.failure_id == FailureIds.panic()
    assert v.message =~ "ArgumentError"
    refute v.message =~ "secret-value"
    assert v.seed == nil

    v = Runner.check(3, Gens.u8(), opts(5, 5, :on), fn _ -> :erlang.error(:badarg) end)
    assert v.message =~ "error"
    v = Runner.check(3, Gens.u8(), opts(5, 5, :on), fn _ -> throw({:secret, 1}) end)
    refute v.message =~ "secret"
  end

  test "a generator failing in any way is property-exhausted" do
    for f <- [fn _ -> raise "gen" end, fn _ -> throw(:gen) end, fn _ -> exit(:gen) end, fn _ -> :not_a_pair end] do
      gen = %CodingAdventures.TestBootstrap.Pbt.Gen{sampler: f}
      v = Runner.check(1, gen, opts(3, 5, :off), fn _ -> true end)
      assert v.failure_id == FailureIds.exhausted()
      v = Runner.check(1, gen, opts(3, 5, :on), fn _ -> true end)
      assert v.failure_id == FailureIds.exhausted()
      refute v.message =~ "gen\""
    end
  end

  test "an unsatisfiable filter is exhausted with the filter message" do
    gen = Gens.filter(Gens.integer(0, 10), 7, fn _ -> false end)
    v = Runner.check(1, gen, opts(1, 1, :on), fn _ -> true end)
    assert v.failure_id == FailureIds.exhausted()
    assert v.message =~ "filter precondition"
  end

  test "the witness is re-verified: bad shrinkers fall back to the original with 0 steps" do
    gen = Gens.constant(100)
    prop = fn x -> x < 50 end

    for shrinker <- [
          fn _v, _f, _s -> {1, 1} end,
          fn _v, _f, _s -> raise "bad shrinker" end,
          fn _v, _f, _s -> throw(:bad) end,
          fn _v, _f, _s -> :garbage end,
          fn _v, _f, _s -> {200, -1} end,
          fn _v, _f, _s -> {200, 1_000_000} end,
          fn _v, _f, _s -> {200, :x} end,
          # a candidate the property raises on is never accepted as smaller
          fn _v, f, _s -> if f.(5), do: {5, 1}, else: {100, 0} end
        ] do
      v = Runner.check(1, gen, opts(1, 100, :off), prop, shrinker)
      assert v.failure_id == FailureIds.counterexample()
      assert v.minimal == 100
      assert v.shrink_steps == 0
    end

    boom_on_small = fn x -> if x < 50, do: raise("small"), else: false end
    v = Runner.check(1, gen, opts(1, 100, :off), boom_on_small, fn _v, f, _s -> if f.(5), do: {5, 1}, else: {100, 0} end)
    assert v.minimal == 100
    assert v.shrink_steps == 0
  end

  test "throw and exit while shrinking do not make a smaller witness" do
    prop = fn x -> if x < 10, do: throw(:small), else: x < 5 end
    v = Runner.check(1, Gens.constant(500), opts(1, 100, :off), prop)
    assert v.failure_id == FailureIds.counterexample()
    assert v.minimal >= 10
    prop2 = fn x -> if x < 10, do: exit(:small), else: x < 5 end
    v = Runner.check(1, Gens.constant(500), opts(1, 100, :off), prop2)
    assert v.minimal >= 10
  end

  test "check validates its arguments" do
    assert_raise ArgumentError, fn -> Runner.check(1.5, Gens.u8(), opts(), fn _ -> true end) end
    assert_raise ArgumentError, fn -> Runner.check(1, :not_gen, opts(), fn _ -> true end) end
    assert_raise ArgumentError, fn -> Runner.check(1, Gens.u8(), opts(), fn _, _ -> true end) end
    assert_raise ArgumentError, fn -> Runner.check(1, Gens.u8(), opts(), fn _ -> true end, :x) end
    assert_raise ArgumentError, fn -> Runner.check(1, Gens.u8(), %{}, fn _ -> true end) end
    assert %Verdict{} = Runner.check(1, Gens.u8(), nil, fn _ -> true end, nil)
  end

  test "negative seeds wrap modulo 2^64" do
    a = Runner.check(-1, Gens.u64(), opts(3, 5), fn _ -> false end)
    b = Runner.check(0xFFFF_FFFF_FFFF_FFFF, Gens.u64(), opts(3, 5), fn _ -> false end)
    assert a.counterexample == b.counterexample
  end

  test "options are validated" do
    for bad <- [[num_tests: -1], [num_tests: 100_001], [max_shrink_steps: 10_001], [max_shrink_steps: -1], [ci: :maybe], [num_tests: 1.5]] do
      assert_raise ArgumentError, fn -> Options.new(bad) end
    end

    assert_raise KeyError, fn -> Options.new(nope: 1) end
  end

  test "ci_from_lookup" do
    assert Options.ci_from_lookup(%{}) == :off
    for v <- ["", "0", "false", " FALSE ", "False"], do: assert(Options.ci_from_lookup(%{"CI" => v}) == :off)
    for v <- ["1", "true", "yes", "x"], do: assert(Options.ci_from_lookup(%{"CI" => v}) == :on)
    assert Options.ci_from_lookup(fn "CI" -> "true" end) == :on
    assert Options.ci_from_lookup(fn _ -> nil end) == :off
    assert Options.ci_from_lookup(%{"CI" => 1}) == :on
  end

  test "failure id vocabulary is closed" do
    assert FailureIds.all() == ["property-counterexample", "property-exhausted", "property-timeout", "property-panic"]
    refute FailureIds.valid?("unregistered-id")
    refute FailureIds.valid?(:"property-panic")
    assert FailureIds.valid?(FailureIds.timeout())
  end

  test "a property that mutates nothing is deterministic" do
    run = fn -> Runner.check(9, Gens.list(Gens.u8(), 0, 20), opts(), fn l -> Enum.sum(l) < 300 end) end
    assert run.() == run.()
  end

  test "the runner leaves no ambient scope behind" do
    Runner.check(1, Gens.u8(), opts(), fn _ -> false end)
    Runner.check(1, Gens.u8(), opts(), fn _ -> raise "x" end)
    assert Budget.current() == nil
  end
end
