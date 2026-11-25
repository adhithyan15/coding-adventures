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
      # 16 x 32 MiB = 512 MiB attempted; the 64 MiB budget kills it after a few chunks
      Enum.reduce(1..16, [], fn _, acc ->
        Process.sleep(10)
        [:binary.copy(<<0>>, 32 * 1024 * 1024) | acc]
      end)

      Process.sleep(:infinity)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 10_000_000
  end

  test "a single 300 MB binary.copy held by the target is an oom, not a pass" do
    target = fn _d, _h ->
      bin = :binary.copy(<<0>>, 300_000_000)
      Process.sleep(2000)
      byte_size(bin)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 8_000_000
  end

  test "a self-sent message flood is an oom (the mailbox counts)" do
    target = fn _d, _h ->
      me = self()

      Enum.each(1..500_000, fn i ->
        send(me, :binary.copy(<<1>>, 1024))
        if rem(i, 1000) == 0, do: Process.sleep(1)
      end)

      Process.sleep(:infinity)
    end

    {us, out} = :timer.tc(fn -> outcome_of(target, mem_budget: 64 * 1024 * 1024, timeout_ms: 30_000) end)
    assert out == :oom
    assert us < 15_000_000
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
    assert us < 20_000_000
    assert Process.info(self(), :message_queue_len) == {:message_queue_len, 0}
  end

  test "an exception in the engine process is re-raised in the caller" do
    assert_raise ArgumentError, fn -> Fuzz.run(:nope, opts(), fn _, _ -> :ok end) end
    assert_raise ArgumentError, fn -> Fuzz.run([1], opts(), fn _, _ -> :ok end) end
  end
end
