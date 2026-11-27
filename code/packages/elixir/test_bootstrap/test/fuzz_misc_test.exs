defmodule FuzzMiscTest do
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  import Bitwise

  alias CodingAdventures.TestBootstrap.Fuzz
  alias CodingAdventures.TestBootstrap.Fuzz.{Executor, FailureIds, Limits, Options}

  test "the failure-id constructors are the closed vocabulary" do
    assert [FailureIds.crash(), FailureIds.timeout(), FailureIds.oom(), FailureIds.leak(), FailureIds.panic()] == FailureIds.all()
  end

  test "limits: the ceilings the spec names" do
    assert Limits.max_retained_bytes() == 256 * 1024 * 1024
    assert Limits.corpus_sequence_work() == 1 <<< 24 and Limits.max_sequence_work() == 1 <<< 28
    assert Limits.max_input_bytes() == 1_048_576
    assert Limits.max_initial_corpus() == 10_000
    assert Limits.default_minimize_budget() == 2000 and Limits.default_step_budget() == 200_000
    assert Limits.default_mem_budget() == 64 * 1024 * 1024 and Limits.default_timeout_ms() == 5000
    assert Limits.coverage_map_size() * 1 == Limits.coverage_bytes() * 8
    assert Limits.max_crashes() == 1000
    assert_raise ArgumentError, ~r/x must be an integer, got a float/, fn -> Limits.check_int!(1.5, "x") end
    assert_raise ArgumentError, ~r/an atom/, fn -> Limits.check_int!(:a, "x") end
    assert_raise ArgumentError, ~r/a binary/, fn -> Limits.check_int!("a", "x") end
    assert_raise ArgumentError, ~r/another term/, fn -> Limits.check_int!({}, "x") end
  end

  test "the heap limit is the budget in words, never below 1,024 words" do
    assert Executor.heap_words(64 * 1024 * 1024) == 8 * 1024 * 1024
    assert Executor.heap_words(0) == 1024
    assert Executor.kill_wait_ms() > 0
  end

  test "the facade delegates to the engine" do
    assert Fuzz.corpus_admit([1], [2]) == {true, 1}
    assert Fuzz.calculate_energy(10, 4, 3, 6) == 8
    assert Fuzz.format_tap_failure(1, "t", "fuzz-crash", "3a8f19b2") == "not ok 1 - t [fuzz-crash:3a8f19b2]"
    report = Fuzz.run_outcomes([<<1>>], Options.new(max_iterations: 3), fn _ -> {:pass, []} end)
    assert report.iterations == 3
    assert Fuzz.tap_failures(report, "t", 1) == []
    exec = Fuzz.executor(fn _d, _h -> :ok end, Options.new())
    assert {:pass, []} = exec.(<<1>>)
  end

  test "classify handles every kind without reading the value" do
    assert {:crash, "throw", []} = Executor.classify(:throw, {:not_a_sentinel, 1, 2}, [])
    assert {:crash, "exit", []} = Executor.classify(:exit, :whatever, [])
    assert {:crash, "error.badarg", []} = Executor.classify(:error, :badarg, [])
  end
end
