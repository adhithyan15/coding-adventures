defmodule CodingAdventures.StateMachine.NFATest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StateMachine.NFA
  alias CodingAdventures.StateMachine.DFA

  # === Fixture helpers ===

  # NFA that accepts strings containing the substring "ab".
  #
  # This is non-deterministic because in q0, on input "a", the machine
  # can either stay in q0 (guessing 'ab' hasn't started) or move to q1
  # (guessing this 'a' starts the 'ab' pattern).
  defp contains_ab do
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
  end

  # NFA with epsilon transitions: accepts "a" or "b" (union).
  #
  # q0 --epsilon--> q1 --a--> q3 (accept)
  # q0 --epsilon--> q2 --b--> q3 (accept)
  defp epsilon_union do
    NFA.new(
      MapSet.new(["q0", "q1", "q2", "q3"]),
      MapSet.new(["a", "b"]),
      %{
        {"q0", ""} => MapSet.new(["q1", "q2"]),
        {"q1", "a"} => MapSet.new(["q3"]),
        {"q2", "b"} => MapSet.new(["q3"])
      },
      "q0",
      MapSet.new(["q3"])
    )
  end

  # Simple NFA that accepts "a" then "b" with no non-determinism.
  # Used to test basic functionality.
  defp simple_ab do
    NFA.new(
      MapSet.new(["q0", "q1", "q2"]),
      MapSet.new(["a", "b"]),
      %{
        {"q0", "a"} => MapSet.new(["q1"]),
        {"q1", "b"} => MapSet.new(["q2"])
      },
      "q0",
      MapSet.new(["q2"])
    )
  end

  # NFA with chained epsilon transitions: q0 --eps--> q1 --eps--> q2 --a--> q3.
  defp chained_epsilon do
    NFA.new(
      MapSet.new(["q0", "q1", "q2", "q3"]),
      MapSet.new(["a"]),
      %{
        {"q0", ""} => MapSet.new(["q1"]),
        {"q1", ""} => MapSet.new(["q2"]),
        {"q2", "a"} => MapSet.new(["q3"])
      },
      "q0",
      MapSet.new(["q3"])
    )
  end

  # === Construction tests ===

  describe "new/5 — construction and validation" do
    test "creates a valid NFA" do
      assert {:ok, nfa} = contains_ab()
      assert nfa.initial == "q0"
      assert MapSet.member?(nfa.current, "q0")
    end

    test "initial current includes epsilon closure" do
      {:ok, nfa} = epsilon_union()
      # q0 has epsilon transitions to q1 and q2
      assert MapSet.member?(nfa.current, "q0")
      assert MapSet.member?(nfa.current, "q1")
      assert MapSet.member?(nfa.current, "q2")
    end

    test "rejects empty states" do
      assert {:error, msg} =
               NFA.new(MapSet.new(), MapSet.new(["a"]), %{}, "q0", MapSet.new())

      assert msg =~ "non-empty"
    end

    test "rejects epsilon in alphabet" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a", ""]),
                 %{},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "epsilon"
    end

    test "rejects initial state not in states" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{},
                 "q99",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end

    test "rejects accepting states not in states" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{},
                 "q0",
                 MapSet.new(["q99"])
               )

      assert msg =~ "q99"
    end

    test "rejects transition with unknown source" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q99", "a"} => MapSet.new(["q0"])},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end

    test "rejects transition with unknown event" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q0", "z"} => MapSet.new(["q0"])},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "z"
    end

    test "rejects transition with unknown target" do
      assert {:error, msg} =
               NFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q0", "a"} => MapSet.new(["q99"])},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end
  end

  # === Epsilon closure tests ===

  describe "epsilon_closure/2" do
    test "closure of state with no epsilon transitions is itself" do
      {:ok, nfa} = contains_ab()
      closure = NFA.epsilon_closure(nfa, MapSet.new(["q0"]))
      assert closure == MapSet.new(["q0"])
    end

    test "follows single epsilon transition" do
      {:ok, nfa} = epsilon_union()
      closure = NFA.epsilon_closure(nfa, MapSet.new(["q0"]))
      assert MapSet.equal?(closure, MapSet.new(["q0", "q1", "q2"]))
    end

    test "follows chained epsilon transitions" do
      {:ok, nfa} = chained_epsilon()
      closure = NFA.epsilon_closure(nfa, MapSet.new(["q0"]))
      assert MapSet.equal?(closure, MapSet.new(["q0", "q1", "q2"]))
    end

    test "closure of empty set is empty" do
      {:ok, nfa} = contains_ab()
      assert NFA.epsilon_closure(nfa, MapSet.new()) == MapSet.new()
    end

    test "closure of multiple states is union" do
      {:ok, nfa} = epsilon_union()
      closure = NFA.epsilon_closure(nfa, MapSet.new(["q1", "q2"]))
      assert MapSet.equal?(closure, MapSet.new(["q1", "q2"]))
    end
  end

  # === Processing tests ===

  describe "process/2" do
    test "processes single event" do
      {:ok, nfa} = contains_ab()
      {:ok, nfa} = NFA.process(nfa, "a")
      # In q0, "a" goes to {q0, q1}
      assert MapSet.member?(nfa.current, "q0")
      assert MapSet.member?(nfa.current, "q1")
    end

    test "non-deterministic branching" do
      {:ok, nfa} = contains_ab()
      {:ok, nfa} = NFA.process(nfa, "a")
      {:ok, nfa} = NFA.process(nfa, "b")
      # q0 on b -> q0, q1 on b -> q2
      assert MapSet.member?(nfa.current, "q0")
      assert MapSet.member?(nfa.current, "q2")
    end

    test "dead branches are pruned" do
      {:ok, nfa} = simple_ab()
      {:ok, nfa} = NFA.process(nfa, "a")
      {:ok, nfa} = NFA.process(nfa, "a")
      # q1 has no transition on "a", only q0->a transitions work
      # But there is no transition for q1 on "a", so that branch dies
      # and q0 has no transition on "a" either (only q0->a->q1)
      # Wait — q0 on "a" -> q1, q1 on "a" -> nothing
      # After first "a": current = {q1}
      # After second "a": current = {} (dead)
      assert MapSet.size(nfa.current) == 0
    end

    test "rejects unknown event" do
      {:ok, nfa} = contains_ab()
      assert {:error, msg} = NFA.process(nfa, "z")
      assert msg =~ "z"
    end

    test "epsilon transitions applied after event" do
      {:ok, nfa} = epsilon_union()
      {:ok, nfa} = NFA.process(nfa, "a")
      assert MapSet.member?(nfa.current, "q3")
    end
  end

  # === Acceptance tests ===

  describe "accepts?/2" do
    test "contains_ab accepts 'ab'" do
      {:ok, nfa} = contains_ab()
      assert NFA.accepts?(nfa, ["a", "b"])
    end

    test "contains_ab accepts 'aab'" do
      {:ok, nfa} = contains_ab()
      assert NFA.accepts?(nfa, ["a", "a", "b"])
    end

    test "contains_ab accepts 'abb'" do
      {:ok, nfa} = contains_ab()
      assert NFA.accepts?(nfa, ["a", "b", "b"])
    end

    test "contains_ab accepts 'bab'" do
      {:ok, nfa} = contains_ab()
      assert NFA.accepts?(nfa, ["b", "a", "b"])
    end

    test "contains_ab rejects 'ba'" do
      {:ok, nfa} = contains_ab()
      refute NFA.accepts?(nfa, ["b", "a"])
    end

    test "contains_ab rejects 'a'" do
      {:ok, nfa} = contains_ab()
      refute NFA.accepts?(nfa, ["a"])
    end

    test "contains_ab rejects empty" do
      {:ok, nfa} = contains_ab()
      refute NFA.accepts?(nfa, [])
    end

    test "epsilon_union accepts 'a'" do
      {:ok, nfa} = epsilon_union()
      assert NFA.accepts?(nfa, ["a"])
    end

    test "epsilon_union accepts 'b'" do
      {:ok, nfa} = epsilon_union()
      assert NFA.accepts?(nfa, ["b"])
    end

    test "epsilon_union rejects 'ab'" do
      {:ok, nfa} = epsilon_union()
      refute NFA.accepts?(nfa, ["a", "b"])
    end

    test "epsilon_union rejects empty" do
      {:ok, nfa} = epsilon_union()
      refute NFA.accepts?(nfa, [])
    end

    test "chained epsilon NFA accepts 'a'" do
      {:ok, nfa} = chained_epsilon()
      assert NFA.accepts?(nfa, ["a"])
    end

    test "chained epsilon NFA rejects empty" do
      {:ok, nfa} = chained_epsilon()
      refute NFA.accepts?(nfa, [])
    end

    test "accepts? does not modify the NFA" do
      {:ok, nfa} = contains_ab()
      original_current = nfa.current
      NFA.accepts?(nfa, ["a", "b"])
      assert nfa.current == original_current
    end

    test "simple_ab accepts exactly 'ab'" do
      {:ok, nfa} = simple_ab()
      assert NFA.accepts?(nfa, ["a", "b"])
      refute NFA.accepts?(nfa, ["a"])
      refute NFA.accepts?(nfa, ["b"])
      refute NFA.accepts?(nfa, ["b", "a"])
      refute NFA.accepts?(nfa, ["a", "b", "a"])
    end

    test "raises on invalid event" do
      {:ok, nfa} = contains_ab()

      assert_raise ArgumentError, fn ->
        NFA.accepts?(nfa, ["z"])
      end
    end
  end

  # === Reset tests ===

  describe "reset/1" do
    test "resets to initial epsilon closure" do
      {:ok, nfa} = epsilon_union()
      {:ok, nfa} = NFA.process(nfa, "a")
      nfa = NFA.reset(nfa)

      assert MapSet.member?(nfa.current, "q0")
      assert MapSet.member?(nfa.current, "q1")
      assert MapSet.member?(nfa.current, "q2")
    end

    test "resets after processing" do
      {:ok, nfa} = contains_ab()
      {:ok, nfa} = NFA.process(nfa, "a")
      {:ok, nfa} = NFA.process(nfa, "b")
      nfa = NFA.reset(nfa)
      assert nfa.current == MapSet.new(["q0"])
    end
  end

  # === Conversion to DFA tests ===

  describe "to_dfa/1" do
    test "converted DFA accepts same language as NFA — contains_ab" do
      {:ok, nfa} = contains_ab()
      {:ok, dfa} = NFA.to_dfa(nfa)

      # Test several strings
      assert DFA.accepts?(dfa, ["a", "b"])
      assert DFA.accepts?(dfa, ["a", "a", "b"])
      assert DFA.accepts?(dfa, ["b", "a", "b"])
      refute DFA.accepts?(dfa, ["b", "a"])
      refute DFA.accepts?(dfa, ["a"])
      refute DFA.accepts?(dfa, [])
    end

    test "converted DFA accepts same language — epsilon_union" do
      {:ok, nfa} = epsilon_union()
      {:ok, dfa} = NFA.to_dfa(nfa)

      assert DFA.accepts?(dfa, ["a"])
      assert DFA.accepts?(dfa, ["b"])
      refute DFA.accepts?(dfa, ["a", "b"])
      refute DFA.accepts?(dfa, [])
    end

    test "converted DFA accepts same language — chained_epsilon" do
      {:ok, nfa} = chained_epsilon()
      {:ok, dfa} = NFA.to_dfa(nfa)

      assert DFA.accepts?(dfa, ["a"])
      refute DFA.accepts?(dfa, [])
    end

    test "converted DFA has valid structure" do
      {:ok, nfa} = contains_ab()
      {:ok, dfa} = NFA.to_dfa(nfa)

      assert MapSet.size(dfa.states) > 0
      assert MapSet.size(dfa.alphabet) > 0
      assert map_size(dfa.transitions) > 0
    end

    test "DFA state names are deterministic" do
      {:ok, nfa} = contains_ab()
      {:ok, dfa1} = NFA.to_dfa(nfa)
      {:ok, dfa2} = NFA.to_dfa(nfa)

      assert dfa1.states == dfa2.states
      assert dfa1.initial == dfa2.initial
    end
  end

  # === Visualization tests ===

  describe "to_dot/1" do
    test "generates valid DOT" do
      {:ok, nfa} = contains_ab()
      dot = NFA.to_dot(nfa)

      assert dot =~ "digraph NFA"
      assert dot =~ "rankdir=LR"
      assert dot =~ "__start"
    end

    test "marks accepting states" do
      {:ok, nfa} = contains_ab()
      dot = NFA.to_dot(nfa)
      assert dot =~ "doublecircle"
    end

    test "shows epsilon transitions" do
      {:ok, nfa} = epsilon_union()
      dot = NFA.to_dot(nfa)
      # Should contain the epsilon character
      assert dot =~ "\u03b5"
    end
  end

  # === state_set_name tests ===

  describe "state_set_name/1" do
    test "single state" do
      assert NFA.state_set_name(MapSet.new(["q0"])) == "{q0}"
    end

    test "multiple states sorted" do
      assert NFA.state_set_name(MapSet.new(["q2", "q0", "q1"])) == "{q0,q1,q2}"
    end

    test "empty set" do
      assert NFA.state_set_name(MapSet.new()) == "{}"
    end
  end

  # === Edge cases ===

  describe "edge cases" do
    test "NFA with only epsilon transitions" do
      {:ok, nfa} =
        NFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a"]),
          %{{"q0", ""} => MapSet.new(["q1"])},
          "q0",
          MapSet.new(["q1"])
        )

      # Should accept empty string because initial epsilon closure reaches q1
      assert NFA.accepts?(nfa, [])
    end

    test "NFA with no transitions" do
      {:ok, nfa} =
        NFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{},
          "q0",
          MapSet.new(["q0"])
        )

      assert NFA.accepts?(nfa, [])
      refute NFA.accepts?(nfa, ["a"])
    end

    test "NFA with self-loop epsilon does not infinite loop" do
      # This shouldn't happen in practice, but epsilon_closure must handle it
      {:ok, nfa} =
        NFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", ""} => MapSet.new(["q0"])},
          "q0",
          MapSet.new(["q0"])
        )

      closure = NFA.epsilon_closure(nfa, MapSet.new(["q0"]))
      assert closure == MapSet.new(["q0"])
    end
  end
end
