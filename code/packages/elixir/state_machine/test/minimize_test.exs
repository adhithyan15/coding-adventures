defmodule CodingAdventures.StateMachine.MinimizeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StateMachine.DFA
  alias CodingAdventures.StateMachine.Minimize

  describe "minimize/1 — Hopcroft's algorithm" do
    test "merges equivalent accepting states" do
      # q1 and q2 are both accepting and have identical transitions
      {:ok, big} =
        DFA.new(
          MapSet.new(["q0", "q1", "q2"]),
          MapSet.new(["a", "b"]),
          %{
            {"q0", "a"} => "q1",
            {"q0", "b"} => "q2",
            {"q1", "a"} => "q1",
            {"q1", "b"} => "q1",
            {"q2", "a"} => "q2",
            {"q2", "b"} => "q2"
          },
          "q0",
          MapSet.new(["q1", "q2"])
        )

      {:ok, small} = Minimize.minimize(big)
      assert MapSet.size(small.states) < MapSet.size(big.states)
    end

    test "minimized DFA accepts same language" do
      {:ok, big} =
        DFA.new(
          MapSet.new(["q0", "q1", "q2", "q3"]),
          MapSet.new(["a", "b"]),
          %{
            {"q0", "a"} => "q1",
            {"q0", "b"} => "q2",
            {"q1", "a"} => "q1",
            {"q1", "b"} => "q1",
            {"q2", "a"} => "q2",
            {"q2", "b"} => "q2",
            {"q3", "a"} => "q3",
            {"q3", "b"} => "q3"
          },
          "q0",
          MapSet.new(["q1", "q2"])
        )

      {:ok, small} = Minimize.minimize(big)

      # Test many strings to verify language equivalence
      test_strings = [
        [],
        ["a"],
        ["b"],
        ["a", "a"],
        ["a", "b"],
        ["b", "a"],
        ["b", "b"],
        ["a", "a", "b"],
        ["b", "a", "a"]
      ]

      for events <- test_strings do
        assert DFA.accepts?(big, events) == DFA.accepts?(small, events),
               "Mismatch for #{inspect(events)}: big=#{DFA.accepts?(big, events)}, small=#{DFA.accepts?(small, events)}"
      end
    end

    test "removes unreachable states" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1", "orphan"]),
          MapSet.new(["a"]),
          %{
            {"q0", "a"} => "q1",
            {"q1", "a"} => "q0",
            {"orphan", "a"} => "orphan"
          },
          "q0",
          MapSet.new(["q1"])
        )

      {:ok, minimized} = Minimize.minimize(dfa)

      # orphan should be gone
      refute Enum.any?(minimized.states, &(&1 =~ "orphan"))
    end

    test "already minimal DFA stays the same size" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a"]),
          %{
            {"q0", "a"} => "q1",
            {"q1", "a"} => "q0"
          },
          "q0",
          MapSet.new(["q1"])
        )

      {:ok, minimized} = Minimize.minimize(dfa)
      assert MapSet.size(minimized.states) == MapSet.size(dfa.states)
    end

    test "single state DFA" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      {:ok, minimized} = Minimize.minimize(dfa)
      assert MapSet.size(minimized.states) == 1
    end

    test "minimized DFA from NFA subset construction" do
      # Build a DFA that came from subset construction (may have redundant states)
      alias CodingAdventures.StateMachine.NFA

      {:ok, nfa} =
        NFA.new(
          MapSet.new(["q0", "q1", "q2"]),
          MapSet.new(["a", "b"]),
          %{
            {"q0", "a"} => MapSet.new(["q0", "q1"]),
            {"q0", "b"} => MapSet.new(["q0"]),
            {"q1", "b"} => MapSet.new(["q2"]),
            {"q2", "a"} => MapSet.new(["q2"]),
            {"q2", "b"} => MapSet.new(["q2"])
          },
          "q0",
          MapSet.new(["q2"])
        )

      {:ok, dfa} = NFA.to_dfa(nfa)
      {:ok, minimized} = Minimize.minimize(dfa)

      # The minimized version should accept the same strings
      assert DFA.accepts?(minimized, ["a", "b"])
      assert DFA.accepts?(minimized, ["a", "a", "b"])
      assert DFA.accepts?(minimized, ["b", "a", "b"])
      refute DFA.accepts?(minimized, ["a"])
      refute DFA.accepts?(minimized, [])

      # And should have fewer or equal states
      assert MapSet.size(minimized.states) <= MapSet.size(dfa.states)
    end

    test "DFA with non-accepting trap state" do
      # q2 is a trap/dead state — transitions to itself on all inputs, never accepts
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1", "q2"]),
          MapSet.new(["a", "b"]),
          %{
            {"q0", "a"} => "q1",
            {"q0", "b"} => "q2",
            {"q1", "a"} => "q1",
            {"q1", "b"} => "q2",
            {"q2", "a"} => "q2",
            {"q2", "b"} => "q2"
          },
          "q0",
          MapSet.new(["q1"])
        )

      {:ok, minimized} = Minimize.minimize(dfa)

      # Language should be preserved
      assert DFA.accepts?(minimized, ["a"])
      assert DFA.accepts?(minimized, ["a", "a"])
      refute DFA.accepts?(minimized, ["b"])
      refute DFA.accepts?(minimized, ["a", "b"])
    end
  end

  describe "split_group/4" do
    test "single element group never splits" do
      group = MapSet.new(["q0"])
      result = Minimize.split_group(group, ["a"], %{}, [group])
      assert result == [group]
    end

    test "two states with same transitions stay together" do
      transitions = %{
        {"q1", "a"} => "q0",
        {"q2", "a"} => "q0"
      }

      group = MapSet.new(["q1", "q2"])
      partitions = [MapSet.new(["q0"]), group]

      result = Minimize.split_group(group, ["a"], transitions, partitions)
      assert length(result) == 1
    end

    test "two states with different transitions split" do
      transitions = %{
        {"q1", "a"} => "q0",
        {"q2", "a"} => "q3"
      }

      group = MapSet.new(["q1", "q2"])
      partitions = [MapSet.new(["q0"]), MapSet.new(["q3"]), group]

      result = Minimize.split_group(group, ["a"], transitions, partitions)
      assert length(result) == 2
    end
  end
end
