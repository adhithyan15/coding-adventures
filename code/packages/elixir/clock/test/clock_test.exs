defmodule CodingAdventures.ClockTest do
  use ExUnit.Case

  alias CodingAdventures.Clock
  alias CodingAdventures.ClockDivider
  alias CodingAdventures.MultiPhaseClock

  test "clock starts low and increments cycle on rising edges" do
    clock = Clock.new()
    assert clock.value == 0
    assert clock.cycle == 0

    {clock, rising} = Clock.tick(clock)
    assert rising.is_rising
    assert rising.cycle == 1
    assert clock.value == 1
    assert clock.cycle == 1

    {clock, falling} = Clock.tick(clock)
    assert falling.is_falling
    assert falling.cycle == 1
    assert clock.value == 0
    assert clock.total_ticks == 2
  end

  test "full_cycle and run return expected edges" do
    {clock, rising, falling} = Clock.new() |> Clock.full_cycle()
    assert rising.is_rising
    assert falling.is_falling
    assert clock.cycle == 1

    {clock, edges} = Clock.run(clock, 2)
    assert length(edges) == 4
    assert clock.cycle == 3
  end

  test "listeners can be registered and removed by index" do
    parent = self()

    listener = fn edge ->
      send(parent, {:edge, edge})
    end

    clock =
      Clock.new()
      |> Clock.register_listener(listener)

    assert Clock.listener_count(clock) == 1

    {_clock, _edge} = Clock.tick(clock)
    assert_receive {:edge, edge}
    assert edge.is_rising

    {:ok, clock} = Clock.unregister_listener(clock, 0)
    assert Clock.listener_count(clock) == 0
    assert {:error, "listener index 1 out of range"} = Clock.unregister_listener(clock, 1)
  end

  test "reset returns the initial timing state" do
    {clock, _edges} = Clock.new(2_000_000) |> Clock.run(3)
    clock = Clock.reset(clock)

    assert clock.value == 0
    assert clock.cycle == 0
    assert clock.total_ticks == 0
    assert Clock.period_ns(clock) == 500.0
  end

  test "clock divider produces one output cycle per divisor rising edges" do
    source = Clock.new(8)
    {:ok, divider} = ClockDivider.new(source, 2)

    {source, edge1} = Clock.tick(source)
    divider = ClockDivider.on_edge(divider, edge1)
    assert divider.output.cycle == 0

    {source, _} = Clock.tick(source)
    {source, edge2} = Clock.tick(source)
    divider = ClockDivider.on_edge(divider, edge2)
    assert divider.output.cycle == 1
    assert divider.output.value == 0
    assert source.cycle == 2
  end

  test "multi-phase clock rotates active phases on rising edges" do
    source = Clock.new()
    {:ok, phases} = MultiPhaseClock.new(source, 3)

    {source, edge1} = Clock.tick(source)
    phases = MultiPhaseClock.on_edge(phases, edge1)
    assert MultiPhaseClock.get_phase(phases, 0) == 1
    assert MultiPhaseClock.get_phase(phases, 1) == 0

    {source, _} = Clock.tick(source)
    {source, edge2} = Clock.tick(source)
    phases = MultiPhaseClock.on_edge(phases, edge2)
    assert MultiPhaseClock.get_phase(phases, 0) == 0
    assert MultiPhaseClock.get_phase(phases, 1) == 1
    assert source.cycle == 2
  end
end
