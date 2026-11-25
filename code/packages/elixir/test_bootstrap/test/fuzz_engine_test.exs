defmodule FuzzTargets do
  @moduledoc false
  # Target code the engine runs: named functions so their frames are stable.

  def boom(_data), do: raise(ArgumentError, "secret message")
  def raise_msg(m), do: raise(ArgumentError, m)
  def boom_runtime(_data), do: raise("another")
  def deep(0), do: raise(ArgumentError, "deep")
  def deep(n), do: deep(n - 1) + 1
  def thrower(_), do: throw(:ball)
  def exiter(_), do: exit(:bye)
  def badarith(x), do: apply(:erlang, :+, [x, :atom_not_a_number])
  def badmatch(x), do: (fn {:ok, _} -> :ok end).(x)
  def erl_error(_), do: :erlang.error(:made_up_tag)
  def erl_badarg(_), do: :erlang.error(:badarg)
  def tuple_error(_), do: :erlang.error({:badmatch, :secret_value})
  def weird_error(_), do: :erlang.error(12_345)
  def nested_tuple_error(_), do: :erlang.error({"string head", 1})
  def empty_tuple_error(_), do: :erlang.error({})
end

defmodule FuzzHostileError do
  @moduledoc false
  defexception [:payload]

  # If the engine ever calls these, the probe process hears about it.
  @impl true
  def message(_e) do
    send(:persistent_term.get(:fuzz_probe), :message_called)
    "hostile"
  end
end

defmodule FuzzHostileStruct do
  @moduledoc false
  defstruct [:a]
end

defimpl Inspect, for: FuzzHostileStruct do
  def inspect(_s, _o) do
    send(:persistent_term.get(:fuzz_probe), :inspect_called)
    "hostile inspect"
  end
end

defimpl String.Chars, for: FuzzHostileStruct do
  def to_string(_s) do
    send(:persistent_term.get(:fuzz_probe), :to_string_called)
    "hostile to_string"
  end
end

defmodule FuzzEngineTest do
  use ExUnit.Case, async: false
  @moduletag timeout: 120_000

  import Bitwise
  alias CodingAdventures.TestBootstrap.Fuzz
  alias CodingAdventures.TestBootstrap.Fuzz.{Engine, Executor, Handle, Limits, Options, Report, Signature}

  defp opts(kw \\ []), do: Options.new(Keyword.merge([max_iterations: 200, seed: 42, ci: :off], kw))
  defp run_one(target, data, kw), do: Executor.run(target, data, opts(kw))
  defp outcome_of(target, data \\ <<1, 2, 3>>, kw \\ []), do: run_one(target, data, kw) |> elem(0)

  setup do
    :persistent_term.put(:fuzz_probe, self())
    on_exit(fn -> :persistent_term.erase(:fuzz_probe) end)
    :ok
  end

  # -- the executor: classification ----------------------------------------------------

  test "a returning target passes, and receives a binary copy" do
    me = self()

    assert :pass ==
             outcome_of(fn data, _h ->
               send(me, {:got, data})
               :whatever
             end)

    assert_receive {:got, <<1, 2, 3>>}, 1000
  end

  test "raise, throw and exit are all crashes, classified by kind and module only" do
    assert {:crash, "error.ArgumentError", frames} = outcome_of(fn d, _ -> FuzzTargets.boom(d) end)
    assert frames == ["FuzzTargets.boom/1"]
    assert {:crash, "error.RuntimeError", ["FuzzTargets.boom_runtime/1"]} = outcome_of(fn d, _ -> FuzzTargets.boom_runtime(d) end)
    assert {:crash, "throw", ["FuzzTargets.thrower/1"]} = outcome_of(fn d, _ -> FuzzTargets.thrower(d) end)
    assert {:crash, "exit", ["FuzzTargets.exiter/1"]} = outcome_of(fn d, _ -> FuzzTargets.exiter(d) end)
    assert {:crash, "error.badarith", _} = outcome_of(fn _d, _ -> FuzzTargets.badarith(1) end)
    assert {:crash, "error.function_clause", _} = outcome_of(fn _d, _ -> FuzzTargets.badmatch(:nope) end)
    assert {:crash, "error.badarg", _} = outcome_of(fn d, _ -> FuzzTargets.erl_badarg(d) end)
    assert {:crash, "error.badmatch", _} = outcome_of(fn d, _ -> FuzzTargets.tuple_error(d) end)
  end

  test "an Erlang error tag outside the closed set cannot mint kind text" do
    for f <- [&FuzzTargets.erl_error/1, &FuzzTargets.weird_error/1, &FuzzTargets.nested_tuple_error/1, &FuzzTargets.empty_tuple_error/1] do
      assert {:crash, "error.other", _} = outcome_of(fn d, _ -> f.(d) end)
    end
  end

  test "the message, inspect and to_string of target values are never called" do
    out = outcome_of(fn _d, _ -> raise FuzzHostileError, payload: %FuzzHostileStruct{a: 1} end)
    assert {:crash, "error.FuzzHostileError", _} = out
    assert {:crash, "throw", _} = outcome_of(fn _d, _ -> throw(%FuzzHostileStruct{a: 1}) end)
    assert {:crash, "exit", _} = outcome_of(fn _d, _ -> exit(%FuzzHostileStruct{a: 1}) end)
    refute_receive :message_called, 200
    refute_receive :inspect_called, 10
    refute_receive :to_string_called, 10
  end

  test "the message never changes the signature" do
    a = outcome_of(fn _d, _ -> FuzzTargets.raise_msg("one") end)
    b = outcome_of(fn _d, _ -> FuzzTargets.raise_msg("completely different text") end)
    assert a == b

    # the same target line with different messages dedups to one signature
    sig = fn {:crash, k, f} -> Signature.crash_signature(k, f) end
    assert sig.(a) == sig.(b)
  end

  test "frames are Module.function/arity, colon free, engine-free and collapse recursion" do
    {:crash, _, shallow} = outcome_of(fn _d, _ -> FuzzTargets.deep(3) end)
    {:crash, _, deep} = outcome_of(fn _d, _ -> FuzzTargets.deep(300) end)
    assert shallow == deep
    assert "FuzzTargets.deep/1" in deep
    all = [shallow, deep] |> List.flatten()
    refute Enum.any?(all, &String.contains?(&1, ":"))
    refute Enum.any?(all, &String.contains?(&1, "TestBootstrap.Fuzz"))
    assert Enum.all?(all, &Regex.match?(~r/\A[A-Za-z0-9._<>$\/#-]+\z/, &1))
  end

  test "frames_of is total and drops engine, Safe and malformed entries" do
    assert Signature.frames_of(:not_a_list) == []

    stack = [
      {CodingAdventures.TestBootstrap.Fuzz.Executor, :run, 3, []},
      {CodingAdventures.TestBootstrap.Safe, :call_stacked, 1, []},
      {Mod.With.Colon, :"fun:name", 2, []},
      {:lists, :map, [:a, :b, :c], []},
      {:erlang, :apply, :weird, []},
      {:fun_without_module, [], []},
      :junk
    ]

    assert Signature.frames_of(stack) == ["Mod.With.Colon.fun_name/2", "lists.map/3", "erlang.apply/_"]
    assert Signature.frames_of(Enum.map(1..10_000, fn i -> {:m, String.to_atom("f#{rem(i, 100)}"), 1, []} end)) |> length() <= 64
  end

  test "kind_of is closed and total" do
    assert Signature.kind_of(:error, %{__exception__: true, __struct__: Elixir.Foo.Bar}) == "error.Foo.Bar"
    assert Signature.kind_of(:error, %{__exception__: true, __struct__: :"weird\nname"}) == "error.weird_name"
    assert String.length(Signature.kind_of(:error, %{__exception__: true, __struct__: String.to_atom(String.duplicate("A", 250))})) <= 6 + 64
    assert Signature.kind_of(:weird, :x) == "<unreadable>"
  end

  # -- the executor: budgets, sticky exhaustion, post-run check --------------------------------

  test "a step breach is a timeout, however the target reacts to the sentinel" do
    # 1. let the sentinel through
    assert :timeout == outcome_of(fn _d, h -> Enum.each(1..50, fn _ -> Handle.tick(h) end) end, <<1>>, step_budget: 10)

    swallow = fn h ->
      try do
        Enum.each(1..50, fn _ -> Handle.tick(h) end)
      catch
        _, _ -> :swallowed
      end
    end

    # 2. swallow it and return
    assert :timeout == outcome_of(fn _d, h -> swallow.(h) end, <<1>>, step_budget: 10)
    # 3. swallow it, then call again: the breach is sticky and re-thrown, still swallowed
    assert :timeout ==
             outcome_of(
               fn _d, h ->
                 swallow.(h)
                 swallow.(h)

                 try do
                   Handle.reserve(h, 1)
                 catch
                   _, _ -> :ok
                 end
               end,
               <<1>>,
               step_budget: 10
             )

    # 4. swallow it and raise something ordinary
    assert :timeout == outcome_of(fn _d, h -> swallow.(h) && raise("ordinary") end, <<1>>, step_budget: 10)
    # 5. swallow it and exit normally
    assert :timeout == outcome_of(fn _d, h -> swallow.(h) && exit(:normal) end, <<1>>, step_budget: 10)
  end

  test "a memory breach is an oom, swallowed or not, and the first breach wins" do
    assert :oom == outcome_of(fn _d, h -> Handle.reserve(h, 1_000_000) end, <<1>>, mem_budget: 1000)

    assert :oom ==
             outcome_of(
               fn _d, h ->
                 try do
                   Handle.reserve(h, 1_000_000)
                 catch
                   _, _ -> :swallowed
                 end
               end,
               <<1>>,
               mem_budget: 1000
             )

    # first breach wins: memory first, then a tick keeps rethrowing the MEMORY sentinel
    out =
      outcome_of(
        fn _d, h ->
          try do
            Handle.reserve(h, 1_000_000)
          catch
            _, _ -> :ok
          end

          try do
            Handle.tick(h)
          catch
            _, _ -> :ok
          end
        end,
        <<1>>,
        mem_budget: 1000,
        step_budget: 1
      )

    assert out == :oom
  end

  test "the budgets are exact and a budget of zero breaches at the first call" do
    ok = fn n -> fn _d, h -> Enum.each(1..n, fn _ -> Handle.tick(h) end) end end
    assert :pass == outcome_of(ok.(10), <<1>>, step_budget: 10)
    assert :timeout == outcome_of(ok.(11), <<1>>, step_budget: 10)
    assert :timeout == outcome_of(fn _d, h -> Handle.tick(h) end, <<1>>, step_budget: 0)
    assert :pass == outcome_of(fn _d, h -> Handle.tick(h, 0) end, <<1>>, step_budget: 0)
    assert :pass == outcome_of(fn _d, h -> Handle.reserve(h, 1000) end, <<1>>, mem_budget: 1000)
    assert :oom == outcome_of(fn _d, h -> Handle.reserve(h, 1001) end, <<1>>, mem_budget: 1000)
    assert :pass == outcome_of(fn _d, h -> Handle.reserve(h, -5) end, <<1>>, mem_budget: 0)
    assert :oom == outcome_of(fn _d, h -> Handle.reserve(h, 1) end, <<1>>, mem_budget: 0)
    # an absurd tick count neither wraps the counter nor allocates
    assert :timeout == outcome_of(fn _d, h -> Handle.tick(h, 1 <<< 100) end, <<1>>, step_budget: 10)
    assert :oom == outcome_of(fn _d, h -> Handle.reserve(h, 1 <<< 100) end, <<1>>, mem_budget: 10)
  end

  test "Handle validates its arguments" do
    h = Handle.new(5, 5)
    assert_raise ArgumentError, fn -> Handle.new(-1, 5) end
    assert_raise ArgumentError, fn -> Handle.new(5, Limits.max_mem_budget() + 1) end
    assert_raise ArgumentError, fn -> Handle.edge(h, :x) end
    assert_raise ArgumentError, fn -> Handle.tick(h, 1.5) end
    assert_raise ArgumentError, fn -> Handle.reserve(h, nil) end
    assert_raise ArgumentError, fn -> Handle.tick(:not_a_handle) end
    assert Handle.edge_id("a") == CodingAdventures.TestBootstrap.Prng.fnv1a64("a")
    assert Handle.edge_id("") == 0xCBF29CE484222325
    assert inspect(h) == "#Fuzz.Handle<>"
    assert Handle.steps_used(h) == 0 and Handle.mem_used(h) == 0
  end

  test "a real heap bomb is an oom through max_heap_size, never a crash or a pass" do
    bomb = fn _d, _h ->
      Enum.reduce(1..5_000_000, [], fn i, acc -> [i | acc] end) |> length()
    end

    assert :oom == outcome_of(bomb, <<1>>, mem_budget: 1 <<< 20)
    # and an ordinary allocation within the budget is fine
    assert :pass == outcome_of(fn _d, _h -> Enum.to_list(1..1000) end, <<1>>, mem_budget: 1 <<< 20)
  end

  test "only THIS execution's :DOWN decides an oom; a stray killed :DOWN in the mailbox is ignored" do
    other = spawn(fn -> :ok end)
    send(self(), {:DOWN, make_ref(), :process, other, :killed})
    assert :pass == outcome_of(fn _d, _h -> :ok end)
    assert_received {:DOWN, _, :process, ^other, :killed}
    send(self(), {:EXIT, other, :killed})
    assert :pass == outcome_of(fn _d, _h -> :ok end)
    assert_received {:EXIT, ^other, :killed}
  end

  test "a target that kills its own execution process is classified, never a pass" do
    assert :oom == outcome_of(fn _d, _h -> Process.exit(self(), :kill) end)
    # an exit signal with another reason is a crash of the signal kind
    assert {:crash, "exit.signal", []} == outcome_of(fn _d, _h -> Process.exit(self(), :other_reason) end)
  end

  test "a target that never returns is killed by the wall clock, and no process survives" do
    me = self()

    out =
      outcome_of(
        fn _d, _h ->
          send(me, {:hanging, self()})
          Process.sleep(:infinity)
        end,
        <<1>>,
        timeout_ms: 100
      )

    assert out == :timeout
    assert_receive {:hanging, pid}, 1000
    refute Process.alive?(pid)
  end

  test "a busy loop that never ticks is killed too" do
    loop = fn loop_fun -> loop_fun.(loop_fun) end
    assert :timeout == outcome_of(fn _d, _h -> loop.(loop) end, <<1>>, timeout_ms: 100)
  end

  test "a linked child dies with the execution process" do
    me = self()

    assert :timeout ==
             outcome_of(
               fn _d, _h ->
                 child = spawn_link(fn -> Process.sleep(:infinity) end)
                 send(me, {:child, child})
                 Process.sleep(:infinity)
               end,
               <<1>>,
               timeout_ms: 100
             )

    assert_receive {:child, child}, 1000
    ref = Process.monitor(child)
    assert_receive {:DOWN, ^ref, :process, ^child, _}, 1000
  end

  test "a target running a property check inside gets its own budget root (processes do not share the ambient scope)" do
    alias CodingAdventures.TestBootstrap.Pbt.{Gens, Options, Runner}

    out =
      outcome_of(fn _d, _h ->
        v = Runner.check(1, Gens.integer(0, 10), Options.new(num_tests: 5, ci: :on), fn x -> x >= 0 end)
        true = v.passed
      end)

    assert out == :pass
  end

  # -- the executor: handle lifetime --------------------------------------------------------

  test "a retained handle is inert after the run" do
    me = self()

    {outcome, edges, calls} =
      Executor.run(
        fn _d, h ->
          Handle.edge(h, 1)
          Handle.edge(h, 65_536 + 2)
          send(me, {:handle, h})
        end,
        <<1>>,
        opts()
      )

    assert outcome == :pass
    assert edges == [1, 2]
    assert calls == 2
    assert_receive {:handle, h}, 1000
    assert Handle.retired?(h)

    # recording after the run changes nothing
    Handle.edge(h, 99)
    assert Handle.edges(h) == [1, 2]
    assert Handle.edge_calls(h) == 2
    # budgets cannot be driven any more; the sentinel is the RETIRED one
    assert {tag, token, :retired} = catch_throw(Handle.tick(h))
    assert tag == Handle.sentinel_tag()
    assert token == h.token
    assert {_, _, :retired} = catch_throw(Handle.reserve(h, 1))
    assert Handle.exhausted(h) == :none
    assert Handle.steps_used(h) == 0
    assert Handle.mem_used(h) == 0
  end

  test "a handle forwarded to another process cannot record after the run" do
    me = self()

    {:pass, edges, _} =
      Executor.run(
        fn _d, h ->
          helper =
            spawn(fn ->
              receive do
                :go ->
                  Handle.edge(h, 77)
                  r = try do Handle.tick(h) catch kind, reason -> {kind, reason} end
                  send(me, {:helper_result, r, Handle.edges(h)})
              after
                5000 -> :ok
              end
            end)

          send(me, {:helper, helper})
          # while the run is live the forwarded handle works: this edge is recorded
          Handle.edge(h, 5)
        end,
        <<1>>,
        opts()
      )

    assert edges == [5]
    assert_receive {:helper, helper}, 1000
    send(helper, :go)
    assert_receive {:helper_result, {:throw, {tag, _token, :retired}}, seen}, 2000
    assert tag == Handle.sentinel_tag()
    # the helper's edge 77 never landed
    assert seen == [5]
  end

  test "a forwarded handle used while the run is still live works (the same atomics)" do
    {:pass, edges, _} =
      Executor.run(
        fn _d, h ->
          target = self()
          _ = spawn(fn -> Handle.edge(h, 9); send(target, :helper_done) end)
          assert_receive_inline(:helper_done)
          :ok
        end,
        <<1>>,
        opts()
      )

    assert edges == [9]
  end

  # a plain receive for use inside a target (ExUnit's assert_receive is not usable there)
  defp assert_receive_inline(msg) do
    receive do
      ^msg -> :ok
    after
      2000 -> raise "inline receive timed out"
    end
  end

  test "a handle rebuilt with another token is inert (unforgeable token)" do
    {:pass, edges, _} =
      Executor.run(
        fn _d, h ->
          forged = %{h | token: make_ref()}
          Handle.edge(forged, 3)
          r = catch_throw(Handle.tick(forged))
          true = match?({_, _, :retired}, r)
          Handle.edge(h, 4)
        end,
        <<1>>,
        opts()
      )

    assert edges == [4]
  end

  test "each execution has its own handle: edges and budgets never leak into the next" do
    me = self()
    {_, _, _} = Executor.run(fn _d, h -> Handle.edge(h, 1); send(me, {:h, h}) end, <<1>>, opts())
    assert_receive {:h, first}, 1000
    {:pass, edges, _} = Executor.run(fn _d, h -> send(me, {:h, h}); :ok end, <<1>>, opts())
    assert_receive {:h, second}, 1000
    assert edges == []
    refute first.cells == second.cells
    refute first.token == second.token
  end

  test "a retained handle used in a LATER execution cannot touch that execution" do
    me = self()
    Executor.run(fn _d, h -> send(me, {:old, h}) end, <<1>>, opts())
    assert_receive {:old, old}, 1000

    {outcome, edges, _} =
      Executor.run(
        fn _d, h ->
          Handle.edge(old, 8)
          Handle.edge(h, 6)
          catch_throw(Handle.tick(old))
          :ok
        end,
        <<1>>,
        opts(step_budget: 1)
      )

    assert outcome == :pass
    assert edges == [6]
  end

  # -- sentinels -----------------------------------------------------------------------------------

  test "a forged sentinel is a crash, never a timeout" do
    forged = fn _d, _h -> throw({Handle.sentinel_tag(), make_ref(), :steps}) end
    assert {:crash, "stale-budget-sentinel", []} = outcome_of(forged) |> strip_frames()
    # even with the REAL token (read from the handle), the cell says no breach happened
    with_real = fn _d, h -> throw({Handle.sentinel_tag(), h.token, :steps}) end
    assert {:crash, "stale-budget-sentinel", []} = outcome_of(with_real) |> strip_frames()
    assert {:crash, "throw", _} = outcome_of(fn _d, _h -> throw({:other_tag, 1, 2}) end)
  end

  defp strip_frames({:crash, k, _}), do: {:crash, k, []}
  defp strip_frames(other), do: other

  test "a sentinel stored in one execution and re-thrown in another is a crash there" do
    me = self()

    assert :timeout ==
             outcome_of(
               fn _d, h ->
                 try do
                   Handle.tick(h)
                   Handle.tick(h)
                 catch
                   :throw, s -> send(me, {:stored, s})
                 end
               end,
               <<1>>,
               step_budget: 1
             )

    assert_receive {:stored, sentinel}, 1000
    assert {:crash, "stale-budget-sentinel", _} = outcome_of(fn _d, _h -> throw(sentinel) end)
  end

  # -- the campaign ------------------------------------------------------------------------------------

  defp pure(f), do: fn data -> f.(data) end
  defp passing, do: pure(fn _ -> {:pass, []} end)

  test "a campaign is a pure function of corpus, options and exec" do
    exec = pure(fn d -> {:pass, [byte_size(d) * 7, if(byte_size(d) > 2, do: 99, else: 1)]} end)
    a = Engine.run_outcomes([<<1, 2>>, <<3>>], opts(), exec)
    b = Engine.run_outcomes([<<1, 2>>, <<3>>], opts(), exec)
    assert a == b
    c = Engine.run_outcomes([<<1, 2>>, <<3>>], opts(seed: 43), exec)
    refute a.corpus == c.corpus and a.total_edges == c.total_edges and a.iterations == 0
  end

  test "the iteration budget is exact and candidates respect the length cap" do
    parent = self()

    exec =
      pure(fn d ->
        send(parent, {:len, byte_size(d)})
        {:pass, [byte_size(d)]}
      end)

    report = Engine.run_outcomes([<<1, 2, 3>>], opts(max_iterations: 500, max_input_len: 6), exec)
    assert report.iterations == 500
    lens = drain_lens([])
    assert Enum.max(lens) <= 6
    assert length(lens) == 501
  end

  defp drain_lens(acc) do
    receive do
      {:len, n} -> drain_lens([n | acc])
    after
      0 -> acc
    end
  end

  test "initial seeds run as given even above the candidate cap" do
    report = Engine.run_outcomes([:binary.copy(<<7>>, 50)], opts(max_iterations: 5, max_input_len: 4), passing())
    assert hd(report.corpus) == :binary.copy(<<7>>, 50)
    assert report.iterations == 5
  end

  test "an empty corpus is replaced by one zero seed" do
    report = Engine.run_outcomes([], opts(max_iterations: 3), passing())
    assert hd(report.corpus) == <<0>>
    assert report.admitted_seeds == 1
  end

  test "a corpus of crashers fuzzes nothing and is never clean" do
    crash = pure(fn _ -> {{:crash, "K", ["f"]}, []} end)
    report = Engine.run_outcomes([<<1>>, <<2>>], opts(), crash)
    assert report.iterations == 0
    assert report.admitted_seeds == 0
    assert length(report.crashes) == 1
    assert hd(report.crashes).hits == 2
    refute Report.clean?(report)

    timeouts = Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> {:timeout, []} end))
    assert timeouts.iterations == 0 and timeouts.timeouts == 1
    refute Report.clean?(timeouts)

    ooms = Engine.run_outcomes([<<1>>, <<2>>], opts(), pure(fn _ -> {:oom, []} end))
    assert ooms.ooms == 2
    refute Report.clean?(ooms)
  end

  test "zero executions is never a pass" do
    zero = Engine.run_outcomes([<<1>>], opts(max_iterations: 0), passing())
    assert zero.iterations == 0
    refute Report.clean?(zero)
    v = Fuzz.verdict(zero, opts(), passing())
    refute v.passed
    assert v.reason == :no_iterations
    assert v.lines == []

    live = Engine.run_outcomes([<<1>>], opts(max_iterations: 3), passing())
    assert Report.clean?(live)
    assert Fuzz.verdict(live, opts(), passing()).passed
  end

  test "the first passing seed is admitted even with no coverage (black-box tier)" do
    report = Engine.run_outcomes([<<1>>], opts(max_iterations: 20), passing())
    assert report.admitted_seeds == 1
    assert report.coverage_tier == "black-box-uninstrumented"
    assert report.total_edges == 0
  end

  test "edges are admitted only when new; the tier is caller-reported" do
    exec = pure(fn d -> {:pass, [byte_size(d)]} end)
    report = Engine.run_outcomes([<<1>>], opts(max_iterations: 300), exec)
    assert report.coverage_tier == "caller-reported"
    # one admitted seed per distinct length that was reached
    assert report.admitted_seeds == report.total_edges
    assert report.total_edges > 1
  end

  defp staircase(guided) do
    pure(fn d ->
      stage =
        cond do
          match?(<<35, 128, 255, _::binary>>, d) -> 3
          match?(<<35, 128, _::binary>>, d) -> 2
          match?(<<35, _::binary>>, d) -> 1
          true -> 0
        end

      edges = if guided, do: Enum.to_list(1..stage//1), else: []
      if stage == 3, do: {{:crash, "Staircase", ["top"]}, edges}, else: {:pass, edges}
    end)
  end

  test "coverage guidance cracks a staircase that blind mutation cannot" do
    cracked = fn guided, seeds ->
      Enum.count(seeds, fn seed ->
        r = Engine.run_outcomes([<<0, 0, 0>>], opts(seed: seed, max_iterations: 6000), staircase(guided))
        r.crashes != []
      end)
    end

    assert cracked.(true, 1..3) == 3
    assert cracked.(false, 1..2) == 0
  end

  test "crashes deduplicate by signature, counting hits" do
    exec =
      pure(fn d ->
        case d do
          <<a, _::binary>> when a < 128 -> {{:crash, "Low", ["f"]}, [1]}
          _ -> {:pass, [2]}
        end
      end)

    report = Engine.run_outcomes([<<1>>, <<2>>, <<200>>], opts(max_iterations: 300), exec)
    assert length(report.crashes) == 1
    [c] = report.crashes
    assert c.hits > 2
    assert c.signature == Signature.crash_signature("Low", ["f"])
  end

  test "different frames are different signatures" do
    exec = pure(fn d -> if d == <<1>>, do: {{:crash, "K", ["a"]}, []}, else: {{:crash, "K", ["b"]}, []} end)
    assert length(Engine.run_outcomes([<<1>>, <<2>>], opts(), exec).crashes) == 2
  end

  test "a novel crash is minimised to its essence, never trading one bug for another" do
    exec =
      pure(fn d ->
        if :binary.match(d, <<0x41>>) != :nomatch, do: {{:crash, "A", ["f"]}, []}, else: {:pass, []}
      end)

    report = Engine.run_outcomes([<<1, 2, 0x41, 3, 4, 5, 6, 7>>], opts(max_iterations: 500), exec)
    [c] = report.crashes
    assert c.minimal_input == <<0x41>>
    assert c.minimized
  end

  test "minimisation can reach the empty input and zeroes non-essential bytes" do
    always = pure(fn _ -> {{:crash, "K", ["f"]}, []} end)
    assert hd(Engine.run_outcomes([<<9>>], opts(max_iterations: 500), always).crashes).minimal_input == <<>>

    big_enough = pure(fn d -> if byte_size(d) >= 2, do: {{:crash, "K", ["f"]}, []}, else: {:pass, []} end)
    [c] = Engine.run_outcomes([<<1, 2, 3, 4>>], opts(max_iterations: 500), big_enough).crashes
    assert c.minimal_input == <<0, 0>>
  end

  test "the per-crash minimisation budget counts rejected candidates" do
    parent = self()

    exec =
      pure(fn d ->
        send(parent, :probe)
        if d == :binary.copy(<<9>>, 64), do: {{:crash, "K", ["f"]}, []}, else: {:pass, []}
      end)

    [c] = Engine.run_outcomes([:binary.copy(<<9>>, 64)], opts(max_iterations: 100, minimize_budget: 5), exec).crashes
    refute c.minimized
    assert drain_probes(0) == 1 + 5
  end

  defp drain_probes(n) do
    receive do
      :probe -> drain_probes(n + 1)
    after
      0 -> n
    end
  end

  test "the campaign-wide minimisation budget is max_iterations" do
    parent = self()

    exec =
      pure(fn d ->
        send(parent, :probe)
        # each seed is its own crash (distinct kind) and every shrink candidate passes
        if byte_size(d) == 64, do: {{:crash, "K#{:binary.first(d)}", ["f"]}, []}, else: {:pass, []}
      end)

    seeds = for i <- 1..20, do: :binary.copy(<<i>>, 64)
    report = Engine.run_outcomes(seeds, opts(max_iterations: 10, minimize_budget: 100), exec)
    assert length(report.crashes) == 20
    # 20 triage executions + at most max_iterations (10) minimisation executions,
    # then the (empty-allowance) crashes are unminimised; no mutation runs (no passing seed)
    assert drain_probes(0) <= 20 + 10
    assert Enum.count(report.crashes, & &1.minimized) < 20
  end

  test "distinct crash signatures are capped" do
    exec = pure(fn <<i::16>> -> {{:crash, "K#{i}", ["f"]}, []} end)
    seeds = for i <- 1..1100, do: <<i::16>>
    report = Engine.run_outcomes(seeds, opts(max_iterations: 0, minimize_budget: 0), exec)
    assert length(report.crashes) == 1000
    assert report.crashes_dropped == 100
    assert Report.unique_crashes(report) == 1100
  end

  test "the retained-bytes cap drops admissions and crash inputs visibly" do
    exec = pure(fn d -> {:pass, [byte_size(d)]} end)
    r = Engine.run_outcomes([<<1, 2, 3, 4>>], opts(max_iterations: 200), exec, retained_cap: 6)
    assert r.admissions_dropped > 0
    assert Enum.sum(Enum.map(r.corpus, &byte_size/1)) <= 6
    refute Report.clean?(r)

    crash = pure(fn _ -> {{:crash, "K", ["f"]}, []} end)
    [c] = Engine.run_outcomes([<<1, 2, 3, 4>>], opts(max_iterations: 0), crash, retained_cap: 0).crashes
    assert c.input_dropped
    assert c.minimal_input == <<>>
    refute c.minimized
  end

  test "a shorter crashing input replaces the retained one and adjusts the account" do
    exec =
      pure(fn d ->
        if byte_size(d) >= 2, do: {{:crash, "K", ["f"]}, []}, else: {:pass, [1]}
      end)

    # the second seed is the same signature but a different length
    report = Engine.run_outcomes([<<1, 2, 3, 4, 5, 6>>, <<1, 2>>], opts(max_iterations: 0, minimize_budget: 0), exec)
    [c] = report.crashes
    assert c.hits == 2
    assert byte_size(c.minimal_input) == 2
  end

  test "input validation" do
    assert_raise ArgumentError, fn -> Engine.run_outcomes(:x, opts(), passing()) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([1], opts(), passing()) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes(for(_ <- 1..10_001, do: <<1>>), opts(), passing()) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([:binary.copy(<<1>>, (1 <<< 20) + 1)], opts(), passing()) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> :bogus end)) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> {:weird, []} end)) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> {:pass, [:x]} end)) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> {{:crash, :k, []}, []} end)) end
    assert_raise ArgumentError, fn -> Engine.run_outcomes([<<1>>], opts(), pure(fn _ -> {{:crash, "k", [1]}, []} end)) end
  end

  test "options validation" do
    assert Options.new().max_iterations == 1000
    assert Options.new().ci == :unset

    for {k, v} <- [max_iterations: -1, max_input_len: 1 <<< 21, base_energy: -1, minimize_budget: 100_001, step_budget: 100_000_001, mem_budget: (1 <<< 30) + 1, timeout_ms: 0, timeout_ms: 60_001] do
      assert_raise ArgumentError, ~r/#{k}/, fn -> Options.new([{k, v}]) end
    end

    for k <- [:max_iterations, :seed, :max_input_len, :base_energy, :minimize_budget, :step_budget, :mem_budget, :timeout_ms] do
      assert_raise ArgumentError, fn -> Options.new([{k, 1.0}]) end
      assert_raise ArgumentError, fn -> Options.new([{k, true}]) end
    end

    assert_raise ArgumentError, fn -> Options.new(ci: :maybe) end
    assert_raise KeyError, fn -> Options.new(nonsense: 1) end
    assert Options.new(seed: -5).seed == -5
  end

  # -- end to end through the real executor ------------------------------------------------------------

  test "Fuzz.run finds a planted crash through real processes, minimised, with edges" do
    target = fn data, h ->
      Handle.edge(h, Handle.edge_id("entered"))

      case data do
        <<0xDE, 0xAD, _::binary>> ->
          Handle.edge(h, Handle.edge_id("dead"))
          raise ArgumentError, "planted"

        _ ->
          :ok
      end
    end

    # a corpus with a head start: the crash is one byte away
    report = Fuzz.run([<<0xDE, 0xAC, 1, 2, 3>>], opts(max_iterations: 1500, seed: 3), target)
    assert report.coverage_tier == "caller-reported"
    assert report.total_edges >= 1
    assert [c] = report.crashes
    assert c.kind == "error.ArgumentError"
    assert c.minimal_input == <<0xDE, 0xAD>>
    assert c.minimized
    refute Report.clean?(report)
  end

  test "Fuzz.run labels a target that reports no edges black-box" do
    r = Fuzz.run([<<1>>], opts(max_iterations: 5), fn _d, _h -> :ok end)
    assert r.coverage_tier == "black-box-uninstrumented"
    assert Report.clean?(r)
  end

  test "Fuzz.run: swallowed budgets, timeouts and ooms land in their buckets" do
    r =
      Fuzz.run([<<1>>, <<2>>, <<3>>], opts(max_iterations: 0, step_budget: 3, mem_budget: 1 <<< 20, timeout_ms: 200), fn
        <<1>>, h -> Enum.each(1..10, fn _ -> Handle.tick(h) end)
        <<2>>, h -> Handle.reserve(h, 1 <<< 30)
        <<3>>, _ -> Process.sleep(:infinity)
      end)

    assert r.timeouts == 2
    assert r.ooms == 1
    assert Signature.tap_failures(r, "t", 1) == ["not ok 1 - t [fuzz-timeout:4e61ea76]", "not ok 2 - t [fuzz-oom:7146a410]"]
  end

  test "Fuzz.run is deterministic through real processes" do
    t = fn data, h ->
      Handle.edge(h, byte_size(data))
      if byte_size(data) > 12, do: raise("big")
    end

    a = Fuzz.run([<<1>>], opts(max_iterations: 300, seed: 5), t)
    b = Fuzz.run([<<1>>], opts(max_iterations: 300, seed: 5), t)
    assert a == b
  end

  # -- verdicts: honesty, CI mode, re-verification ---------------------------------------------------------

  test "CI mode (and an unset one) carries no seed and no witness; :off carries them" do
    exec = pure(fn _ -> {{:crash, "K", ["f"]}, []} end)
    report = Engine.run_outcomes([<<1, 2>>], opts(seed: 987_654_321), exec)

    for ci <- [:on, :unset] do
      v = Fuzz.verdict(report, opts(ci: ci), exec, "tgt")
      refute v.passed
      assert v.seed == nil
      assert v.witnesses == nil
      assert v.lines == ["not ok 1 - tgt [fuzz-crash:#{Signature.crash_signature("K", ["f"])}]"]
      refute inspect(v) =~ "987654321"
      assert [%{status: :reproduced}] = v.crashes
    end

    off = Fuzz.verdict(report, opts(ci: :off), exec)
    assert off.seed == 987_654_321
    assert [{_sig, <<>>}] = off.witnesses
    # the default Options are CI
    assert Fuzz.verdict(report, Options.new(), exec).seed == nil
  end

  test "a crash that does not reproduce is flaky; still failing, never clean" do
    {:ok, calls} = Agent.start_link(fn -> 0 end)

    exec =
      pure(fn _ ->
        # crashes only on the very first execution
        n = Agent.get_and_update(calls, &{&1, &1 + 1})
        if n == 0, do: {{:crash, "K", ["f"]}, []}, else: {:pass, []}
      end)

    report = Engine.run_outcomes([<<1>>], opts(max_iterations: 0, minimize_budget: 0), exec)
    v = Fuzz.verdict(report, opts(), exec)
    refute v.passed
    assert [%{status: :flaky}] = v.crashes
    assert v.reason == :crash
  end

  test "a crash whose input was dropped is unverified" do
    crash = pure(fn _ -> {{:crash, "K", ["f"]}, []} end)
    report = Engine.run_outcomes([<<1>>], opts(max_iterations: 0), crash, retained_cap: 0)
    v = Fuzz.verdict(report, opts(ci: :off), crash)
    assert [%{status: :unverified}] = v.crashes
    assert v.witnesses == []
  end

  test "verdict reasons" do
    base = %Report{iterations: 5, seed: 1}
    exec = passing()
    assert Fuzz.verdict(%{base | timeouts: 1}, opts(), exec).reason == :timeout
    assert Fuzz.verdict(%{base | ooms: 1}, opts(), exec).reason == :oom
    assert Fuzz.verdict(%{base | crashes_dropped: 1}, opts(), exec).reason == :crashes_dropped
    assert Fuzz.verdict(%{base | admissions_dropped: 1}, opts(), exec).reason == :admissions_dropped
    assert Fuzz.verdict(base, opts(), exec).passed
    assert Fuzz.verdict(%{base | timeouts: 1}, opts(), exec).lines == ["not ok 1 - fuzz [fuzz-timeout:4e61ea76]"]
  end

  # -- reports are inert ------------------------------------------------------------------------------------

  test "reports, records, options and handles show no bytes, seed or corpus" do
    secret = "SECRETBYTES-123"

    crash = %Fuzz.CrashRecord{signature: "4eca7ae8", kind: "panic", frames: ["eval"], minimal_input: secret, hits: 1, minimized: true, input_dropped: false}
    report = %Report{iterations: 1, admitted_seeds: 1, crashes: [crash], seed: 987_654_321, corpus: [secret]}
    verdict = %Fuzz.Verdict{seed: 987_654_321, witnesses: [{"x", secret}]}
    options = Options.new(seed: 987_654_321)

    for obj <- [crash, report, verdict, options, Handle.new(1, 1)] do
      text = inspect(obj)
      refute text =~ secret
      refute text =~ "987654321"
    end

    assert inspect(report) =~ "iterations=1"
    assert inspect(crash) =~ "4eca7ae8"
  end

  # -- TAP escaping (the Canon escape set, exactly) ----------------------------------------------------------------

  # Cross-lane values: computed by running the Ruby lane's format_tap_failure
  # (code/packages/ruby/test_bootstrap, Fuzz::Signature) on the same inputs.
  @ruby_taps [
    {"a#b", "a_b"},
    {"a b\nc\rd", "a_b_c_d"},
    {"é" <> <<0x202E::utf8>> <> "x", "_____x"},
    {String.duplicate("x", 200), String.duplicate("x", 128)},
    {String.duplicate("x", 127) <> "éTAIL", String.duplicate("x", 127) <> "_"},
    {"", ""},
    {<<0x202E::utf8>>, "___"},
    {"a\tb:c", "a_b_c"}
  ]

  test "TAP target tokens equal the Ruby lane's byte-grammar reduction (VER06 9.3)" do
    for {target, token} <- @ruby_taps do
      assert Signature.format_tap_failure(1, target, "fuzz-crash", "3a8f19b2") ==
               "not ok 1 - #{token} [fuzz-crash:3a8f19b2]"
    end
  end

  test "TAP targets cannot forge, reorder or hide lines or start a directive" do
    for cp <- Enum.to_list(0..0x9F) ++ [0x2028, 0x2029, 0x061C, 0x200E, 0x202E, 0x2066, 0xFEFF, 0xE0001] do
      line = Signature.format_tap_failure(1, "a" <> <<cp::utf8>> <> "b", "fuzz-crash", "3a8f19b2")
      assert Regex.match?(~r/\Anot ok 1 - [A-Za-z0-9._\/-]+ \[fuzz-crash:3a8f19b2\]\z/, line)
    end

    refute Signature.format_tap_failure(1, "x # TODO", "fuzz-crash", "3a8f19b2") =~ "#"
  end

  test "the target is truncated before it is reduced, so the line stays bounded" do
    line = Signature.format_tap_failure(1, String.duplicate("x", 10_000_000), "fuzz-crash", "3a8f19b2")
    assert byte_size(line) < 200
    nasty = String.duplicate(<<0x202E::utf8>>, 100_000)
    assert Signature.format_tap_failure(1, nasty, "fuzz-crash", "3a8f19b2") =~ ~r/- _{128} \[/
    assert_raise ArgumentError, fn -> Signature.format_tap_failure(1, :t, "fuzz-crash", "3a8f19b2") end
  end

  test "a real campaign's TAP lines carry no input bytes" do
    secret = <<0xDE, 0xAD, 0xBE, 0xEF>>

    report =
      Fuzz.run([secret], opts(max_iterations: 0), fn d, _h -> if d == secret, do: raise("x"), else: :ok end)

    [line] = Signature.tap_failures(report, "target", 1)
    refute line =~ "deadbeef"
    refute line =~ "DEADBEEF"
    assert line =~ ~r/\Anot ok 1 - target \[fuzz-crash:[0-9a-f]{8}\]\z/
  end
end
