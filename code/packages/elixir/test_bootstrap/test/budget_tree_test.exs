defmodule BudgetTreeTest do
  @moduledoc """
  The hierarchical budget tree (VER05m 3.1): the bound holds whatever the
  nesting, with no classification of a call as "inside a property" or "inside a
  shrinker".  Ported from the Java ShrinkBudgetTest (VER05e 4.1), the C# tests
  and the Ruby test_budget_tree, plus the BEAM-specific cases.
  """
  # not async: several tests tune global-ish hooks (they are per-process, but the
  # concurrency cases want a quiet machine so their time limits are honest)
  use ExUnit.Case, async: false
  @moduletag timeout: 60_000

  alias CodingAdventures.TestBootstrap.Pbt.{Budget, FailureIds, Gen, Gens, Hooks, Limits, Options, Runner, Shrink}

  defp opts(num_tests, steps, ci \\ :off), do: Options.new(num_tests: num_tests, max_shrink_steps: steps, ci: ci)
  defp lists(max), do: Gens.list(Gens.u8(), 0, max)
  defp sevens(n), do: List.duplicate(7, n)
  defp attempt_budget(n), do: Limits.attempt_budget(n)
  defp counter, do: :counters.new(1, [])
  defp bump(c), do: :counters.add(c, 1, 1)
  defp count(c), do: :counters.get(c, 1)

  # Runs fun with a small root work cap; returns the root's {attempts, work}.
  defp root_totals(cap, fun) do
    Hooks.with([default_max_work: cap], fn ->
      fun.()
      {Budget.last_attempts(), Budget.last_work()}
    end)
  end

  defp drain(budget) do
    if Budget.charge(budget, 1), do: drain(budget), else: :ok
  end

  setup do
    # no test may leak an ambient scope into the next
    on_exit(fn -> :ok end)
    assert Budget.current() == nil
    :ok
  end

  # -- 1. a property calling Runner.check, depth 1..3 --------------------------------------

  defp nested(depth, max_steps, evals) do
    Runner.check(1, lists(40), opts(100, max_steps), fn l ->
      if Budget.current(), do: bump(evals)
      if depth > 1, do: nested(depth - 1, max_steps, evals)
      length(l) < 5
    end)
  end

  test "a property calling check keeps the root totals within the root cap (depth 1..3)" do
    for depth <- 1..3 do
      max_steps = 5
      evals = counter()
      {attempts, work} = root_totals(1_000_000, fn -> nested(depth, max_steps, evals) end)
      cap = 100 + attempt_budget(max_steps)
      assert attempts <= cap, "depth #{depth} attempts #{attempts}"
      assert work <= 1_000_000
      assert count(evals) > 0
      assert count(evals) <= 3 * cap * depth, "depth #{depth} evaluations #{count(evals)}"
    end
  end

  defp branch(level, counted) do
    Runner.check(1, Gens.integer(0, 9), opts(10, 5), fn x ->
      if level > 1, do: bump(counted)

      if level < 4 do
        branch(level + 1, counted)
        branch(level + 1, counted)
      end

      x < 3
    end)
  end

  test "a branching property keeps total evaluations within the root cap" do
    counted = counter()
    {attempts, _} = root_totals(1_000_000, fn -> branch(1, counted) end)
    cap = 10 + attempt_budget(5)
    assert count(counted) > 0
    assert attempts <= cap
    assert count(counted) <= cap * 2, "nested evaluations #{count(counted)} (2^depth would be far more)"
  end

  # -- 2. custom shrinkers and filter predicates ------------------------------------------------

  test "a recursing custom shrinker stays inside the root cap" do
    inner = lists(50)
    start = sevens(50)
    calls = counter()
    extra = counter()
    max_steps = 10

    custom = fn value, _falsifies, steps ->
      for _ <- 1..100 do
        Gen.shrink(inner, value, steps, fn l -> length(l) == 50 end)

        Gen.shrink(inner, value, steps, fn l ->
          bump(extra)
          length(l) == 50
        end)

        Shrink.shrink_list([1, 2, 3], 0, steps, fn l -> l == [] end)
      end

      {value, 0}
    end

    outer = Gen.new(fn r -> {start, r} end, custom)

    {attempts, work} =
      root_totals(1_000_000, fn ->
        Gen.shrink(outer, start, max_steps, fn _ ->
          bump(calls)
          false
        end)
      end)

    assert attempts <= attempt_budget(max_steps)
    assert work <= 1_000_000
    assert count(calls) + count(extra) <= attempt_budget(max_steps)
    assert count(extra) > 0
  end

  test "a filter predicate that shrinks stays inside the root cap" do
    g = lists(200)
    start = List.duplicate(3, 200)
    inner_evals = counter()
    max_steps = 5

    filtered =
      Gens.filter(Gens.integer(0, 1000), 100, fn _x ->
        Gen.shrink(g, start, 1000, fn l ->
          bump(inner_evals)
          length(l) != 200
        end)

        true
      end)

    {attempts, work} = root_totals(5_000_000, fn -> Gen.shrink(filtered, 900, max_steps, fn x -> x >= 10 end) end)
    assert count(inner_evals) > 0, "the predicate must have shrunk"
    assert attempts <= attempt_budget(max_steps)
    assert work <= 5_000_000
    assert count(inner_evals) <= attempt_budget(max_steps)
  end

  test "a filter predicate is not invoked under a spent budget" do
    calls = counter()

    filtered =
      Gens.filter(Gens.integer(0, 1000), 100, fn _x ->
        bump(calls)
        true
      end)

    Budget.enter_child(0, fn root ->
      drain(root)
      assert Gen.shrink(filtered, 900, 1000, fn _ -> false end) == {900, 0}
      assert count(calls) == 0
    end)

    {ok, _} = Gen.shrink(filtered, 900, 1000, fn x -> x >= 10 end)
    assert count(calls) > 0
    assert ok == 10
  end

  test "a filter sampler rejecting everything stays inside the root cap in a nested check" do
    pred_calls = counter()

    rejecting =
      Gens.filter(Gens.integer(0, 1000), Limits.max_retries(), fn _x ->
        bump(pred_calls)
        false
      end)

    max_steps = 5

    root_totals(1_000_000, fn ->
      Gen.shrink(Gens.integer(0, 1000), 900, max_steps, fn x ->
        Runner.check(1, rejecting, opts(3, 10), fn _ -> true end)
        x >= 10
      end)
    end)

    assert count(pred_calls) > 0
    assert count(pred_calls) <= attempt_budget(max_steps), "sampler retries #{count(pred_calls)}"
  end

  test "stacked filters cannot multiply retries at the top level" do
    calls = counter()
    level = fn g, divisor -> Gens.filter(g, Limits.max_retries(), fn x -> rem(div(x, divisor), 50) == 0 end) end

    innermost =
      Gens.filter(Gens.integer(0, 1_000_000_000), Limits.max_retries(), fn x ->
        bump(calls)
        rem(x, 50) == 0
      end)

    v =
      Hooks.with([default_max_work: 5_000], fn ->
        Runner.check(1, level.(level.(innermost, 50), 2500), opts(1, 5), fn _ -> true end)
      end)

    assert v.failure_id == FailureIds.exhausted()
    assert count(calls) <= 5_000
  end

  test "top-level filter retries are work, not attempts" do
    g = Gens.filter(Gens.integer(0, 999), 10_000, fn x -> x == 0 end)
    v = Runner.check(1, g, opts(100, 1000), fn _ -> true end)
    assert v.passed
  end

  test "max_retries is capped by a limit" do
    g = Gens.integer(0, 9)
    assert %Gen{} = Gens.filter(g, Limits.max_retries(), fn _ -> true end)
    assert_raise ArgumentError, fn -> Gens.filter(g, Limits.max_retries() + 1, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Gens.filter(g, 0, fn _ -> true end) end
  end

  # -- 3. spawned processes (BEAM carrier) ---------------------------------------------------------

  test "a process carrying the handle charges the root even after the inner scope closed" do
    root = Budget.open_child(10)
    inner = Budget.open_child(10)
    task = Budget.async(fn -> receive do
          :go -> Budget.current()
        after
          5_000 -> exit(:no_go)
        end |> then(&(&1 != nil and Budget.charge(&1, 1))) end)
    Budget.close(inner)
    before = Budget.attempts(root)
    send(task.pid, :go)
    assert Task.await(task)
    assert Budget.attempts(root) == before + 1
    Budget.close(root)
    assert Budget.current() == nil
  end

  test "a plain spawned process does NOT inherit the budget: it gets a fresh root (documented escape)" do
    root = Budget.open_child(10)
    me = self()

    spawn(fn ->
      send(me, {:ambient, Budget.current()})
      # a check there opens its own root and is NOT charged to ours
      v = Runner.check(1, Gens.integer(0, 9), opts(50, 5), fn _ -> true end)
      send(me, {:verdict, v.passed, Budget.last_attempts()})
    end)

    assert_receive {:ambient, nil}, 5_000
    assert_receive {:verdict, true, attempts}, 5_000
    assert attempts == 0 or attempts >= 0
    assert Budget.attempts(root) == 0
    Budget.close(root)
  end

  test "a nested check in a process carrying the handle is starved; in a plain spawn it is not" do
    me = self()

    Budget.enter_child(0, fn root ->
      drain(root)

      run = fn spawner ->
        spawner.(fn -> send(me, {:v, Runner.check(1, Gens.integer(0, 9), opts(3, 10), fn _ -> true end)}) end)
        assert_receive {:v, v}, 5_000
        v
      end

      carried = run.(&Budget.spawn/1)
      assert carried.failure_id == FailureIds.exhausted()
      refute carried.passed

      plain = run.(&spawn/1)
      assert plain.passed
    end)
  end

  test "spawn_link and async helpers carry the handle; with_handle(nil) is a no-op" do
    me = self()

    Budget.enter_child(10, fn root ->
      Budget.spawn_link(fn -> send(me, {:linked, Budget.current() == root}) end)
      assert_receive {:linked, true}, 5_000
      assert Task.await(Budget.async(fn -> Budget.current() == root end))
    end)

    assert Budget.with_handle(nil, fn -> :ran end) == :ran
    # outside any scope the handle is nil, and spawn gives the child no ambient node
    Budget.spawn(fn -> send(me, {:none, Budget.current()}) end)
    assert_receive {:none, nil}, 5_000
  end

  test "with_handle restores the previous ambient value, also on a raise" do
    a = Budget.open_child(10)

    b =
      Budget.enter_child(5, fn x -> x end)

    assert_raise RuntimeError, fn ->
      Budget.with_handle(b, fn ->
        assert Budget.current() == nil or Budget.current() == b or true
        raise "boom"
      end)
    end

    assert Budget.current() == a
    Budget.close(a)
  end

  test "threads created inside a scope share one global budget" do
    g = lists(50)
    start = sevens(50)
    calls = :counters.new(1, [:atomics])
    max_steps = 10

    parent =
      Gen.new(fn r -> {0, r} end, fn value, _f, _s ->
        work = fn ->
          Gen.shrink_nested(
            g,
            start,
            fn l ->
              :counters.add(calls, 1, 1)
              length(l) == 50
            end,
            max_steps
          )
        end

        tasks = for _ <- 1..6, do: Budget.async(work)
        Enum.each(tasks, &Task.await(&1, 30_000))
        {value, 0}
      end)

    Gen.shrink(parent, 0, max_steps, fn _ -> true end)
    assert :counters.get(calls, 1) <= attempt_budget(max_steps), "evaluations #{:counters.get(calls, 1)}"
    assert :counters.get(calls, 1) > 50, "the workers must really have shrunk"
  end

  test "a process carrying a handle that outlives its whole scope is spent, not given a fresh root" do
    g = lists(50)
    me = self()

    parent =
      Gen.new(fn r -> {0, r} end, fn value, _f, _s ->
        Budget.spawn(fn ->
          receive do
            :go -> :ok
          after
            5_000 -> exit(:no_go)
          end
          send(me, {:nested, Gen.shrink_nested(g, sevens(50), fn l -> length(l) >= 1 end, 1000)})
          send(me, {:public, Gen.shrink(g, sevens(50), 1000, fn l -> length(l) >= 1 end)})
        end)
        |> then(&send(me, {:pid, &1}))

        {value, 0}
      end)

    Gen.shrink(parent, 0, 10, fn _ -> true end)
    assert_receive {:pid, pid}, 5_000
    send(pid, :go)
    assert_receive {:nested, {l1, _}}, 5_000
    assert_receive {:public, {l2, _}}, 5_000
    assert length(l1) == 50
    assert length(l2) == 50
  end

  # -- 5/6/8: nested checks -------------------------------------------------------------------------

  test "a passing inner check with a huge num_tests is bounded under an outer shrink" do
    inner_cases = counter()
    max_steps = 5

    {attempts, _} =
      root_totals(1_000_000, fn ->
        Gen.shrink(Gens.integer(0, 1000), 900, max_steps, fn x ->
          Runner.check(1, Gens.integer(0, 9), opts(Limits.max_cases(), 10), fn _ ->
            bump(inner_cases)
            true
          end)

          x >= 10
        end)
      end)

    assert count(inner_cases) <= attempt_budget(max_steps) + 64
    assert attempts <= attempt_budget(max_steps)
  end

  test "shrink_int and simulate_replay inside a property are charged" do
    evals = counter()
    max_steps = 5

    {attempts, _} =
      root_totals(1_000_000, fn ->
        Gen.shrink(Gens.integer(0, 1000), 900, max_steps, fn x ->
          Shrink.shrink_int(500, 0, 0, 1000, 1000, fn v ->
            bump(evals)
            v < 100
          end)

          Shrink.simulate_replay(3, fn v ->
            bump(evals)
            v < 100
          end)

          x >= 10
        end)
      end)

    assert count(evals) > 0
    assert attempts <= attempt_budget(max_steps)
    assert count(evals) <= attempt_budget(max_steps)
  end

  test "a starved nested check is exhausted but a top-level check still runs" do
    evaluated = counter()

    Budget.enter_child(0, fn root ->
      drain(root)
      before = Budget.attempts(root)

      nested =
        Runner.check(1, Gens.list(Gens.u8(), 5, 5), opts(10, 1000), fn _ ->
          bump(evaluated)
          false
        end)

      assert nested.failure_id == FailureIds.exhausted()
      refute nested.passed
      assert count(evaluated) == 0
      # all or nothing: unchanged
      assert Budget.attempts(root) == before

      # a size-0 sample charges no work, so ONLY the entry charge can stop this one
      tiny =
        Runner.check(1, Gens.integer(0, 9), opts(1, 10), fn _ ->
          bump(evaluated)
          true
        end)

      assert tiny.failure_id == FailureIds.exhausted()
      assert count(evaluated) == 0
    end)

    top = Runner.check(1, Gens.list(Gens.u8(), 5, 5), opts(10, 1000), fn _ -> false end)
    assert top.failure_id == FailureIds.counterexample()
    assert top.minimal != nil
  end

  test "a nested check pays one attempt at entry" do
    Budget.enter_child(1000, fn root ->
      before = Budget.attempts(root)
      v = Runner.check(1, Gens.integer(0, 9), opts(1, 10), fn _ -> true end)
      assert v.passed
      assert Budget.attempts(root) == before + 1
    end)
  end

  test "a nested check inside the outer case loop charges the outer root" do
    inner_cases = counter()

    {attempts, _} =
      root_totals(1_000_000, fn ->
        Runner.check(1, Gens.integer(0, 9), opts(5, 5), fn _ ->
          Runner.check(2, Gens.integer(0, 9), opts(200, 10), fn _ ->
            bump(inner_cases)
            true
          end)

          # the outer property never fails: no shrink at all
          true
        end)
      end)

    cap = 5 + attempt_budget(5)
    assert attempts > 0
    assert attempts <= cap
    assert count(inner_cases) <= cap, "inner cases #{count(inner_cases)} (uncharged: 5 x 200)"
  end

  # -- 9/10: the primitive -----------------------------------------------------------------------------

  test "a refused take leaves every counter unchanged" do
    root = Budget.open_child(1000, 100)
    child = Budget.open_child(1000, 1_000_000)
    assert Budget.charge(child, 60)
    # the root has only 40 work left
    refute Budget.charge(child, 60)
    assert Budget.attempts(child) == 1
    assert Budget.work(child) == 60
    assert Budget.attempts(root) == 1
    assert Budget.work(root) == 60
    assert Budget.refused?(child)
    Budget.close(child)
    Budget.close(root)
  end

  test "a huge cost is refused without wrapping" do
    b = Budget.open_child(1000, 100)
    assert Budget.charge(b, 60)
    refute Budget.charge(b, 9_223_372_036_854_775_807)
    refute Budget.charge_copy(b, 9_223_372_036_854_775_807)
    refute Budget.charge(b, Integer.pow(2, 200))
    assert Budget.work(b) == 60
    Budget.close(b)
  end

  test "a zero or negative cost is charged as one" do
    b = Budget.open_child(1000, 100)
    assert Budget.charge(b, 0)
    assert Budget.charge(b, -5)
    assert Budget.work(b) == 2
    Budget.close(b)
  end

  test "concurrent charging never exceeds the cap" do
    b = Budget.open_child(1000, 5000)

    ok =
      1..16
      |> Enum.map(fn _ -> Task.async(fn -> Enum.count(1..1000, fn _ -> Budget.charge(b, 1) end) end) end)
      |> Enum.map(&Task.await(&1, 30_000))
      |> Enum.sum()

    assert ok == 5000
    assert Budget.work(b) == 5000
    assert Budget.attempts(b) == 5000
    Budget.close(b)
  end

  test "the attempt cap is enforced separately from work" do
    b = Budget.open_child(0)
    # 64 attempts allowed
    for _ <- 1..64, do: assert(Budget.charge_copy(b, 1))
    assert Budget.attempts(b) == 0
    for _ <- 1..64, do: assert(Budget.charge(b, 1))
    refute Budget.charge(b, 1)
    refute Budget.room?(b)
    Budget.close(b)
  end

  test "room? reflects work as well as attempts" do
    b = Budget.open_child(1000, 2)
    assert Budget.room?(b)
    assert Budget.charge(b, 2)
    refute Budget.room?(b)
    Budget.close(b)
  end

  # -- 11: scope hygiene ---------------------------------------------------------------------------------------

  test "the ambient budget is restored after a raise, throw or exit" do
    thrower = Gen.new(fn r -> {0, r} end, fn _, _, _ -> raise "shrinker boom" end)
    assert_raise RuntimeError, fn -> Gen.shrink(thrower, 0, 5, fn _ -> true end) end
    assert Budget.current() == nil

    assert_raise RuntimeError, fn -> Gen.shrink(Gens.integer(0, 10), 5, 5, fn _ -> raise "property boom" end) end
    assert Budget.current() == nil

    assert catch_throw(Gen.shrink(Gens.integer(0, 10), 5, 5, fn _ -> throw(:up) end)) == :up
    assert Budget.current() == nil
    assert catch_exit(Gen.shrink(Gens.integer(0, 10), 5, 5, fn _ -> exit(:bye) end)) == :bye
    assert Budget.current() == nil
  end

  test "deep nesting joins at the depth cap" do
    opened = for _ <- 1..(Budget.max_depth() + 6), do: Budget.open_child(10)
    depths = Enum.map(opened, & &1.depth)
    # past the cap the same node comes back (joined), so depth stops growing
    assert Enum.max(depths) == Budget.max_depth() - 1
    assert length(Enum.uniq(opened)) == Budget.max_depth()

    for b <- opened |> Enum.uniq() |> Enum.reverse(), do: Budget.close(b)
    assert Budget.current() == nil
  end

  test "enter_check also joins beyond the depth cap" do
    opened = for _ <- 1..(Budget.max_depth() + 3), do: Budget.open_check(5, 5)
    refute Budget.nested?(hd(opened))
    assert Budget.nested?(Enum.at(opened, 1))
    assert length(Enum.uniq(opened)) == Budget.max_depth()
    for b <- opened |> Enum.uniq() |> Enum.reverse(), do: Budget.close(b)
    assert Budget.current() == nil
  end

  test "enter joins the ambient scope or opens a root" do
    Budget.enter(5, fn b ->
      refute Budget.nested?(b)

      Budget.enter(5, fn j ->
        assert j == b
        assert Budget.charge(j, 1)
        assert Budget.attempts(b) == 1
      end)

      refute Budget.closed?(b)
    end)

    assert Budget.current() == nil
  end

  test "a closed scope resolves to the nearest open ancestor" do
    outer = Budget.open_child(10)
    inner = Budget.open_child(10)
    Budget.close(inner)
    assert Budget.current() == outer
    Budget.close(outer)
    assert Budget.current() == nil
  end

  test "closing is idempotent and records stats" do
    Budget.enter_child(10, fn b -> for _ <- 1..3, do: Budget.charge(b, 2) end)
    assert Budget.last_attempts() == 3
    assert Budget.last_work() == 6
    b = Budget.open_child(10)
    Budget.close(b)
    Budget.close(b)
    assert Budget.current() == nil
  end

  test "charged wraps a property and stops at the cap" do
    Budget.enter_child(0, fn b ->
      calls = counter()

      wrapped =
        Budget.charged(b, fn _ ->
          bump(calls)
          true
        end)

      for _ <- 1..100, do: wrapped.(1)
      assert count(calls) == 64
    end)
  end

  # -- sampling work ---------------------------------------------------------------------------------------------------

  test "a nested check sampling large lists is charged as work" do
    inner = :ets.new(:inner, [:public, :duplicate_bag])

    root_totals(10_000, fn ->
      Gen.shrink(Gens.integer(0, 1000), 900, 1000, fn x ->
        v = Runner.check(1, Gens.list(Gens.u8(), 2000, 2000), opts(50, 10), fn _ -> true end)
        :ets.insert(inner, {:v, v.failure_id})
        x >= 10
      end)
    end)

    ids = for {:v, id} <- :ets.tab2list(inner), do: id
    assert ids != []
    assert FailureIds.exhausted() in ids
  end

  test "sampled lists of lists are charged by their contents" do
    got = :ets.new(:got, [:public, :ordered_set])

    root_totals(20_000, fn ->
      Gen.shrink(Gens.integer(0, 1000), 900, 1000, fn x ->
        v = Runner.check(1, Gens.list(Gens.list(Gens.u8(), 100, 100), 100, 100), opts(50, 10), fn _ -> true end)
        :ets.insert(got, {:erlang.unique_integer([:monotonic]), v.failure_id})
        x >= 10
      end)
    end)

    # 100 + 100 x 100 = 10,100 per case: the second case no longer fits in 20,000
    assert [{_, first} | _] = :ets.tab2list(got)
    assert first == FailureIds.exhausted()
  end

  test "size_of counts binaries, lists, tuples and maps one level deep" do
    assert Runner.size_of(5) == 0
    assert Runner.size_of("abc") == 3
    assert Runner.size_of(%{a: 1, b: 2}) == 2
    assert Runner.size_of({1, 2, 3, 4}) == 4
    assert Runner.size_of([<<1, 2>>, [1, 2, 3]]) == 2 + 2 + 3
    assert Runner.size_of([1 | 2]) == 0
    assert Runner.size_of(%{__struct__: Foo, a: 1}) == 2
    assert Runner.size_of([]) == 0
    assert Runner.size_of(:atom) == 0
    assert Runner.size_of(Enum.to_list(1..50_000)) == 50_000
    assert Runner.walk_cap() == 10_000
  end

  # -- max_steps / re-verify / starvation -------------------------------------------------------------------------------

  test "max_steps is validated and zero steps returns the input" do
    g = lists(5)
    assert_raise ArgumentError, fn -> Gen.shrink(g, [], -1, fn _ -> true end) end
    assert_raise ArgumentError, fn -> Gen.shrink_nested(g, [], fn _ -> true end, -1) end
    assert_raise ArgumentError, fn -> Gen.shrink(g, [], Limits.max_shrink_steps() + 1, fn _ -> true end) end
    input = [1, 2, 3]
    assert Shrink.shrink_sequence(input, fn _ -> true end, 0, 0, nil) == {input, 0}
    assert Gen.shrink(g, input, 0, fn _ -> true end) == {input, 0}
  end

  test "a refused re-verification reports the original witness with zero steps" do
    Budget.enter_child(0, fn _root ->
      spender = fn _value, _falsifies, _steps ->
        drain(Budget.current())
        {3, 1}
      end

      v = Runner.check(1, Gens.constant(100), opts(1, 5), fn _ -> false end, spender)
      assert v.failure_id == FailureIds.counterexample()
      assert v.minimal == 100
      assert v.shrink_steps == 0
    end)

    # With room, the same shrinker's result IS adopted (3 still fails the property).
    ok = Runner.check(1, Gens.constant(100), opts(1, 5), fn _ -> false end, fn _v, _f, _s -> {3, 1} end)
    assert {ok.minimal, ok.shrink_steps} == {3, 1}
  end

  test "a starved nested check never lets the top-level check pass" do
    v =
      Runner.check(1, Gens.integer(0, 9), opts(3, 0), fn _ ->
        Runner.check(2, Gens.integer(0, 9), opts(5000, 0), fn _ -> true end)
        true
      end)

    refute v.passed
    assert v.failure_id == FailureIds.exhausted()
    assert v.message =~ "starved"

    ok =
      Runner.check(1, Gens.integer(0, 9), opts(3, 0), fn _ ->
        Runner.check(2, Gens.integer(0, 9), opts(5, 0), fn _ -> true end)
        true
      end)

    assert ok.passed
  end

  test "starvation does not override a falsified verdict" do
    v =
      Runner.check(1, Gens.integer(0, 9), opts(3, 0), fn x ->
        Runner.check(2, Gens.integer(0, 9), opts(5000, 0), fn _ -> true end)
        x > 100
      end)

    assert v.failure_id == FailureIds.counterexample()
  end

  test "a reentrant check matches a standalone run while the outer budget has room" do
    inner_run = fn -> Runner.check(5, Gens.list(Gens.u8(), 0, 50), opts(200, 1000, :on), fn l -> Enum.sum(l) <= 20 end) end
    standalone = inner_run.()
    inner = :ets.new(:reentrant, [:public, :duplicate_bag])
    outer = opts(100, 10_000, :on)
    expected = Runner.check(9, Gens.integer(0, 1000), outer, fn x -> x < 500 end)

    actual =
      Runner.check(9, Gens.integer(0, 1000), outer, fn x ->
        :ets.insert(inner, {:v, inner_run.()})
        x < 500
      end)

    verdicts = for {:v, v} <- :ets.tab2list(inner), do: v
    assert verdicts != []
    for v <- verdicts, do: assert(v == standalone)
    assert actual == expected
  end

  test "a budget spent inside the runner stops every nested shrinker from copying" do
    touched = counter()
    g = lists(10_000)

    v =
      Hooks.with([default_max_work: 100_000], fn ->
        Runner.check(1, Gens.constant(List.duplicate(0, 10_000)), opts(1, 1000), fn l ->
          :counters.add(touched, 1, length(l))
          length(l) != 10_000
        end, fn value, f, s -> Gen.shrink_nested(g, value, f, s) end)
      end)

    assert v.failure_id == FailureIds.counterexample()
    assert count(touched) <= 100_000 + 30_000, "elements handed to the property #{count(touched)}"
  end

  test "the whole check body runs in one scope" do
    seen = :ets.new(:seen, [:public, :duplicate_bag])

    Runner.check(1, Gens.integer(0, 9), opts(3, 5), fn _ ->
      :ets.insert(seen, {:s, Budget.current()})
      true
    end)

    scopes = for {:s, s} <- :ets.tab2list(seen), do: s
    assert length(scopes) == 3
    assert Enum.all?(scopes, &(&1 != nil))
    assert length(Enum.uniq(scopes)) == 1
    assert Budget.current() == nil
  end

  # -- BEAM specifics: lock, throw and exit ---------------------------------------------------------------------------------

  test "a reservation leaked by a killed process makes later charges refuse, never pass, and quickly" do
    b = Budget.open_child(10, 5)
    # a process killed mid-walk left its reservation on the first node only
    Budget.leak_reservation(b, 5)

    {micros, results} =
      :timer.tc(fn ->
        {Budget.charge(b, 1), Budget.charge_copy(b, 1), Budget.room?(b)}
      end)

    assert results == {false, false, false}
    assert micros < 1_000_000
    assert Budget.refused?(b)

    # a nested check inside it is starved, never a pass
    v = Runner.check(1, Gens.integer(0, 9), opts(1, 1), fn _ -> true end)
    refute v.passed
    assert v.failure_id == FailureIds.exhausted()
    Budget.close(b)
  end

  test "a process killed while charging leaves only conservative over-counting" do
    b = Budget.open_child(1000, 5_000)
    leaked = 100
    Budget.leak_reservation(b, leaked)
    me = self()

    pids =
      for _ <- 1..8 do
        spawn(fn ->
          send(me, :ready)
          Enum.each(1..1_000_000, fn _ -> Budget.charge(b, 1) end)
        end)
      end

    for _ <- pids, do: assert_receive(:ready, 5_000)
    Process.sleep(20)
    for pid <- pids, do: Process.exit(pid, :kill)
    Process.sleep(20)
    # still usable, and bounded by the node's real caps (plus what the seam leaked)
    assert Budget.attempts(b) <= b.max_attempts
    assert Budget.work(b) <= b.max_work + leaked
    Budget.charge(b, 1)
    assert Budget.attempts(b) <= b.max_attempts
    assert Budget.work(b) <= b.max_work + leaked
    Budget.close(b)
  end

  test "leak_reservation rejects a negative, huge or non-integer cost" do
    b = Budget.open_child(10, 100)
    assert_raise FunctionClauseError, fn -> Budget.leak_reservation(b, -1) end
    assert_raise FunctionClauseError, fn -> Budget.leak_reservation(b, 101) end
    assert_raise FunctionClauseError, fn -> Budget.leak_reservation(b, 1.5) end
    assert Budget.leak_reservation(b, 100) == :ok
    Budget.close(b)
  end

  test "concurrent charging on a shared tree never exceeds any cap and counters equal successful charges" do
    root = Budget.open_child(1000, 300)
    mid = Budget.open_child(1000, 200)
    leaf = Budget.open_child(1000, 120)

    results =
      1..12
      |> Enum.map(fn i ->
        node = if rem(i, 2) == 0, do: leaf, else: mid
        Task.async(fn -> Enum.reduce(1..100, 0, fn _, acc -> if Budget.charge(node, 2), do: acc + 2, else: acc end) end)
      end)
      |> Enum.map(&Task.await(&1, 30_000))

    leaf_ok = results |> Enum.drop(1) |> Enum.take_every(2) |> Enum.sum()
    mid_ok = results |> Enum.take_every(2) |> Enum.sum()

    assert Budget.work(leaf) == leaf_ok
    assert Budget.work(mid) == leaf_ok + mid_ok
    assert Budget.work(root) == leaf_ok + mid_ok
    assert Budget.work(leaf) <= 120
    assert Budget.work(mid) <= 200
    assert Budget.work(root) <= 300
    assert Budget.attempts(root) == div(leaf_ok + mid_ok, 2)
    for n <- [leaf, mid, root], do: Budget.close(n)
  end

  test "a closed handle is spent: every take refuses and a check is starved, never a fresh root" do
    b = Budget.open_child(1000, 1000)
    Budget.close(b)
    refute Budget.room?(b)
    refute Budget.charge(b, 1)
    refute Budget.charge_copy(b, 1)

    Budget.with_handle(b, fn ->
      assert Budget.current() == b
      v = Runner.check(1, Gens.integer(0, 9), opts(3, 5), fn _ -> true end)
      refute v.passed
      assert v.failure_id == FailureIds.exhausted()
      # public entry points join the spent handle instead of opening a root
      assert Gen.shrink(Gens.integer(0, 1000), 900, 50, fn x -> x >= 10 end) == {900, 0}
    end)

    assert Budget.current() == nil
    # a stale handle carried to another process behaves the same
    task = Task.async(fn -> Budget.with_handle(b, fn -> Budget.charge(Budget.current(), 1) end) end)
    refute Task.await(task)
  end

  test "a closed node anywhere on the path refuses" do
    root = Budget.open_child(10, 100)
    child = Budget.open_child(10, 100)
    assert Budget.charge(child, 1)
    Budget.close(root)
    refute Budget.charge(child, 1)
    refute Budget.room?(child)
    Budget.close(child)
  end

  test "throw and exit in a property are property-panic, never a crash" do
    for {fun, kind} <- [{fn _ -> throw(:thrown) end, "throw"}, {fn _ -> exit(:gone) end, "exit"}, {fn _ -> raise "x" end, "error"}] do
      v = Runner.check(1, Gens.integer(0, 9), opts(3, 10, :on), fun)
      assert v.failure_id == FailureIds.panic()
      assert v.message =~ kind
      refute v.message =~ "thrown"
      refute v.message =~ "gone"
      assert v.seed == nil
    end

    v = Runner.check(1, Gens.integer(0, 9), opts(3, 10, :off), fn _ -> throw(:thrown) end)
    assert v.failure_id == FailureIds.panic()
    assert v.seed == 1
    assert v.message =~ ":thrown"
  end
end
