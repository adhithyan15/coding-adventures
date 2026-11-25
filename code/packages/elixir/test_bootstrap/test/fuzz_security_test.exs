defmodule FuzzSecurityTest do
  use ExUnit.Case, async: false
  import Bitwise
  @moduletag timeout: 120_000

  alias CodingAdventures.TestBootstrap.Fuzz
  alias CodingAdventures.TestBootstrap.Fuzz.{Executor, Handle, Options}

  defp opts(kw \\ []), do: Options.new(Keyword.merge([max_iterations: 5, ci: :off], kw))
  defp outcome_of(target, kw), do: Executor.run(target, <<1>>, opts(kw)) |> elem(0)

  # -- sticky exhaustion: the two throw_if_exhausted lines must be load-bearing ----------

  test "after a steps breach, reserve(0) and reserve(1) throw the STEPS sentinel" do
    h = Handle.new(1, 1_000_000)
    assert {_, _, :steps} = catch_throw(Handle.tick(h, 2))
    assert {tag, token, :steps} = catch_throw(Handle.reserve(h, 0))
    assert tag == Handle.sentinel_tag() and token == h.token
    assert {_, _, :steps} = catch_throw(Handle.reserve(h, 1))
    assert {_, _, :steps} = catch_throw(Handle.tick(h, 0))
  end

  test "after a memory breach, tick(0) and tick(1) throw the MEMORY sentinel" do
    h = Handle.new(1_000_000, 10)
    assert {_, _, :mem} = catch_throw(Handle.reserve(h, 11))
    assert {_, _, :mem} = catch_throw(Handle.tick(h, 0))
    assert {_, _, :mem} = catch_throw(Handle.tick(h, 1))
    assert {_, _, :mem} = catch_throw(Handle.reserve(h, 0))
  end

  # -- F3: the counters cross-check the exhausted cell --------------------------------------

  test "a target that clears the exhausted cell after a breach is still a timeout / oom" do
    clear = fn h -> :atomics.put(h.cells, Handle.exhausted_cell(), 0) end

    assert :timeout ==
             outcome_of(
               fn _d, h ->
                 try do
                   Handle.tick(h, 100)
                 catch
                   _, _ -> clear.(h)
                 end
               end,
               step_budget: 10
             )

    assert :oom ==
             outcome_of(
               fn _d, h ->
                 try do
                   Handle.reserve(h, 1 <<< 40)
                 catch
                   _, _ -> clear.(h)
                 end
               end,
               mem_budget: 1000
             )
  end

  # -- F1: refc binaries and the mailbox are bounded by the poll --------------------------------

  test "a target holding more refc binary memory than the budget is an oom, long before the wall clock" do
    target = fn _d, _h ->
      # 8 x 32 MiB = 256 MiB attempted; the 64 MiB budget kills it after a few chunks
      Enum.reduce(1..8, [], fn _, acc ->
        Process.sleep(10)
        [:binary.copy(<<0>>, 32 * 1024 * 1024) | acc]
      end)

      Process.sleep(:infinity)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 30_000_000
  end

  test "a single 128 MiB binary.copy held by the target is an oom, not a pass" do
    target = fn _d, _h ->
      bin = :binary.copy(<<0>>, 128 * 1024 * 1024)
      Process.sleep(2000)
      byte_size(bin)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 30_000_000
  end

  test "a self-sent message flood is an oom (the mailbox counts)" do
    target = fn _d, _h ->
      me = self()

      Enum.each(1..100_000, fn i ->
        send(me, :binary.copy(<<1>>, 1024))
        if rem(i, 1000) == 0, do: Process.sleep(1)
      end)

      Process.sleep(:infinity)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 30_000_000
  end

  # -- F2: result messages carry the pid -------------------------------------------------------------

  test "a forged result message to the parent is ignored" do
    out =
      outcome_of(
        fn _d, _h ->
          {:parent, p} = :erlang.process_info(self(), :parent)
          send(p, {make_ref(), self(), :pass})
          send(p, {make_ref(), :pass})
          raise "real crash"
        end,
        []
      )

    assert {:crash, "error.RuntimeError", _} = out
  end

  # -- F4: a dedicated engine process --------------------------------------------------------------------

  test "a target flooding its parent neither slows later executions nor touches the caller's mailbox" do
    target = fn d, _h ->
      {:parent, p} = :erlang.process_info(self(), :parent)
      Enum.each(1..20_000, fn _ -> send(p, :junk) end)
      if d == <<1>>, do: :ok, else: :ok
    end

    {us, report} = :timer.tc(fn -> Fuzz.run([<<1>>], opts(max_iterations: 8), target) end)
    assert report.iterations == 8
    assert Fuzz.Report.clean?(report)
    assert us < 30_000_000
    assert Process.info(self(), :message_queue_len) == {:message_queue_len, 0}
  end

  test "an exception in the engine process is re-raised in the caller" do
    assert_raise ArgumentError, fn -> Fuzz.run(:nope, opts(), fn _, _ -> :ok end) end
    assert_raise ArgumentError, fn -> Fuzz.run([1], opts(), fn _, _ -> :ok end) end
  end

  # -- R2-1: no orphaned engine or target -----------------------------------------------------

  # Reports the three processes of one execution: the target, its launcher (its
  # parent) and the ENGINE (the other link of the launcher).
  defp busy_loop_target(me) do
    fn _d, _h ->
      {:parent, launcher} = :erlang.process_info(self(), :parent)
      {:links, links} = :erlang.process_info(launcher, :links)
      [engine] = links -- [self()]
      send(me, {:pids, self(), launcher, engine})
      loop = fn f -> f.(f) end
      loop.(loop)
    end
  end

  defp assert_all_dead(target, launcher, engine) do
    for pid <- [engine, launcher, target] do
      ref = Process.monitor(pid)
      assert_receive {:DOWN, ^ref, :process, ^pid, _}, 10_000
    end
  end

  test "killing the caller mid-campaign kills the engine, the launcher and the infinite-loop target" do
    me = self()
    caller = spawn(fn -> Fuzz.run([<<1>>], opts(max_iterations: 5, timeout_ms: 60_000), busy_loop_target(me)) end)
    assert_receive {:pids, target, launcher, engine}, 10_000
    assert Enum.all?([target, launcher, engine], &Process.alive?/1)
    assert engine != launcher and engine != caller
    Process.exit(caller, :kill)
    assert_all_dead(target, launcher, engine)
  end

  test "a caller whose bound expires leaves no live engine, launcher or target" do
    me = self()

    assert_raise RuntimeError, ~r/exceeded its bound/, fn ->
      Fuzz.run([<<1>>], opts(max_iterations: 5, timeout_ms: 60_000), busy_loop_target(me), bound_ms: 300)
    end

    assert_receive {:pids, target, launcher, engine}, 10_000
    assert_all_dead(target, launcher, engine)
  end

  test "bound_ms is validated before an engine is spawned" do
    before = length(Process.list())
    target = fn _, _ -> :ok end

    for bad <- [-1, 1.5, :infinity, nil, "5", 4_294_967_296] do
      assert_raise ArgumentError, ~r/bound_ms/, fn -> Fuzz.run([<<1>>], opts(), target, bound_ms: bad) end
    end

    assert length(Process.list()) <= before + 2
    assert Fuzz.run([<<1>>], opts(max_iterations: 1), target, bound_ms: 4_294_967_295).iterations == 1
    assert Fuzz.run([<<1>>], opts(max_iterations: 1), target, bound_ms: 60_000).iterations == 1
  end

  test "a caller :DOWN swallowed by the drain makes the engine exit" do
    cmon = make_ref()

    {pid, mon} =
      spawn_monitor(fn ->
        send(self(), :stray)
        send(self(), {:DOWN, make_ref(), :process, self(), :other})
        send(self(), {:DOWN, cmon, :process, self(), :killed})
        send(self(), :never_reached)
        Fuzz.drain(100, cmon)
        # only reached if the drain did not notice
        exit(:drain_ignored_the_caller)
      end)

    assert_receive {:DOWN, ^mon, :process, ^pid, :caller_down}, 5_000
  end

  test "the drain drops strays and, outside an engine, never exits" do
    send(self(), :a)
    send(self(), :b)
    assert Fuzz.drain(100, nil) == :ok
    refute_received :a
    send(self(), :c)
    send(self(), :d)
    assert Fuzz.drain(1, nil) == :ok
    assert_received :d
    assert Fuzz.drain(0, nil) == :ok
  end

  test "a target crash never kills the engine (it traps exits)" do
    r = Fuzz.run([<<1>>], opts(max_iterations: 20), fn _d, _h -> exit(:abnormal) end)
    assert r.iterations == 0 and length(r.crashes) == 1
  end

  # -- R2-2: a target flooding its parent with big terms ---------------------------------------------

  test "a target sending its parent 1 MiB terms in a loop is an oom and the engine survives" do
    target = fn _d, _h ->
      {:parent, engine} = :erlang.process_info(self(), :parent)
      term = :erlang.make_tuple(131_072, 0)
      Enum.each(1..2_000, fn _ -> send(engine, term) end)
      Process.sleep(:infinity)
    end

    {us, r} = :timer.tc(fn -> Fuzz.run([<<1>>], opts(max_iterations: 0, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000), target) end)
    assert r.ooms == 1
    assert us < 30_000_000
  end

  test "a flood of tiny messages to the parent is an oom once their memory passes the budget" do
    target = fn _d, _h ->
      {:parent, engine} = :erlang.process_info(self(), :parent)
      Enum.each(1..100_000, fn _ -> send(engine, :x) end)
      Process.sleep(:infinity)
    end

    {us, r} = :timer.tc(fn -> Fuzz.run([<<1>>], opts(max_iterations: 0, mem_budget: 2 * 1024 * 1024, timeout_ms: 30_000), target) end)
    assert r.ooms == 1
    assert us < 20_000_000
  end

  # -- R2-3: a real deadline --------------------------------------------------------------------------

  test "the wall clock is a real deadline" do
    {us, out} = :timer.tc(fn -> outcome_of(fn _d, _h -> Process.sleep(:infinity) end, timeout_ms: 300) end)
    assert out == :timeout
    assert us >= 280_000 and us < 5_000_000
  end

  # -- R2-4: a binary-reference flood is judged by count -------------------------------------------------

  test "a target holding millions of tiny binary references is over budget by count" do
    target = fn _d, _h ->
      refs = for i <- 1..200_000, do: :binary.copy(<<i::32, 0::size(640)>>)
      Process.sleep(:infinity)
      length(refs)
    end

    assert :oom == outcome_of(target, mem_budget: 4 * 1024 * 1024, timeout_ms: 30_000)
  end

  # -- the ENGINE's own mailbox (flooded?/2) -------------------------------------------------------

  test "a target that finds the engine pid and floods it directly is an oom; later executions are unaffected" do
    target = fn d, _h ->
      if d == <<9, 9, 9>> do
        {:parent, launcher} = :erlang.process_info(self(), :parent)
        {:links, links} = :erlang.process_info(launcher, :links)
        [engine] = links -- [self()]
        Enum.each(1..30_000, fn _ -> send(engine, :flood) end)
        Process.sleep(:infinity)
      else
        :ok
      end
    end

    {us, r} =
      :timer.tc(fn -> Fuzz.run([<<9, 9, 9>>, <<2>>], opts(max_iterations: 3, timeout_ms: 60_000), target) end)

    assert r.ooms == 1
    assert r.iterations == 3
    assert us < 30_000_000
  end
end
