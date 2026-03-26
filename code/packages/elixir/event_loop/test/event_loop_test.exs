defmodule CodingAdventures.EventLoopTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.EventLoop

  # ══════════════════════════════════════════════════════════════════════════
  # Helpers — mock sources
  # ══════════════════════════════════════════════════════════════════════════

  # fixed_source/1 returns a `{poll_fn, state}` tuple where state is a list
  # of batches. Each poll call consumes the first batch and returns the rest
  # as the new state. When no batches remain, poll returns {[], []}.
  defp fixed_source(batches) do
    poll_fn = fn
      [] -> {[], []}
      [batch | rest] -> {batch, rest}
    end

    {poll_fn, batches}
  end

  # ══════════════════════════════════════════════════════════════════════════
  # Tests
  # ══════════════════════════════════════════════════════════════════════════

  test "module loads successfully" do
    assert Code.ensure_loaded?(CodingAdventures.EventLoop)
  end

  test "delivers all events to handlers" do
    source = fixed_source([[1, 2, 3], [:stop]])

    received = Agent.start_link(fn -> [] end) |> elem(1)

    handler = fn
      :stop ->
        :exit

      event ->
        Agent.update(received, fn acc -> acc ++ [event] end)
        :continue
    end

    assert :ok = EventLoop.run([source], [handler])
    assert Agent.get(received, & &1) == [1, 2, 3]
  end

  test "exit stops dispatch immediately" do
    source = fixed_source([["a", "b", "stop", "c", "d"]])

    seen = Agent.start_link(fn -> [] end) |> elem(1)

    handler = fn event ->
      Agent.update(seen, fn acc -> acc ++ [event] end)
      if event == "stop", do: :exit, else: :continue
    end

    EventLoop.run([source], [handler])

    result = Agent.get(seen, & &1)
    assert result == ["a", "b", "stop"]
    refute "c" in result
    refute "d" in result
  end

  test "multiple handlers all receive the same event" do
    source = fixed_source([[99], [:done]])

    h1_saw = Agent.start_link(fn -> nil end) |> elem(1)
    h2_saw = Agent.start_link(fn -> nil end) |> elem(1)

    h1 = fn
      99 ->
        Agent.update(h1_saw, fn _ -> 99 end)
        :continue

      :done ->
        :exit

      _ ->
        :continue
    end

    h2 = fn
      99 ->
        Agent.update(h2_saw, fn _ -> 99 end)
        :continue

      _ ->
        :continue
    end

    EventLoop.run([source], [h1, h2])

    assert Agent.get(h1_saw, & &1) == 99
    assert Agent.get(h2_saw, & &1) == 99
  end

  test "events from multiple sources are merged" do
    source_a = fixed_source([["alpha"]])
    source_b = fixed_source([["beta"]])
    source_stop = fixed_source([[], [:stop]])

    seen = Agent.start_link(fn -> [] end) |> elem(1)

    handler = fn
      :stop -> :exit
      event -> Agent.update(seen, fn acc -> acc ++ [event] end); :continue
    end

    EventLoop.run([source_a, source_b, source_stop], [handler])

    result = Agent.get(seen, & &1)
    assert length(result) == 2
    assert "alpha" in result
    assert "beta" in result
  end

  test "events within a batch arrive in order" do
    source = fixed_source([[3, 1, 4, 1, 5], [:done]])

    received = Agent.start_link(fn -> [] end) |> elem(1)

    handler = fn
      :done -> :exit
      event -> Agent.update(received, fn acc -> acc ++ [event] end); :continue
    end

    EventLoop.run([source], [handler])

    assert Agent.get(received, & &1) == [3, 1, 4, 1, 5]
  end

  test "source state evolves across iterations" do
    # Stateful countdown: emits the current count, decrements each iteration.
    countdown_fn = fn
      0 -> {[:done], 0}
      n -> {[n], n - 1}
    end

    source = {countdown_fn, 3}

    received = Agent.start_link(fn -> [] end) |> elem(1)

    handler = fn
      :done -> :exit
      n -> Agent.update(received, fn acc -> acc ++ [n] end); :continue
    end

    EventLoop.run([source], [handler])

    assert Agent.get(received, & &1) == [3, 2, 1]
  end

  test "run returns :ok on normal exit" do
    source = fixed_source([[:stop]])
    result = EventLoop.run([source], [fn :stop -> :exit end])
    assert result == :ok
  end
end
