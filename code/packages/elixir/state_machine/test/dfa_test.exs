defmodule CodingAdventures.StateMachine.DFATest do
  use ExUnit.Case, async: true

  alias CodingAdventures.StateMachine.DFA
  # === Fixture helpers ===
  # These build common DFAs used across many tests, following the literate
  # programming principle of making test code self-documenting.

  # Build the classic turnstile DFA.
  #
  # States: locked, unlocked
  # Alphabet: coin, push
  # Initial: locked
  # Accepting: unlocked
  #
  # This is the canonical example from automata theory textbooks.
  defp turnstile do
    DFA.new(
      MapSet.new(["locked", "unlocked"]),
      MapSet.new(["coin", "push"]),
      %{
        {"locked", "coin"} => "unlocked",
        {"locked", "push"} => "locked",
        {"unlocked", "coin"} => "unlocked",
        {"unlocked", "push"} => "locked"
      },
      "locked",
      MapSet.new(["unlocked"])
    )
  end

  # Build a simple two-state toggle DFA.
  #
  # States: a, b
  # Alphabet: x
  # Initial: a
  # Accepting: b
  defp toggle do
    DFA.new(
      MapSet.new(["a", "b"]),
      MapSet.new(["x"]),
      %{
        {"a", "x"} => "b",
        {"b", "x"} => "a"
      },
      "a",
      MapSet.new(["b"])
    )
  end

  # Build a DFA that accepts strings ending in "ab".
  #
  # States: q0, q1, q2
  # Alphabet: a, b
  # Initial: q0
  # Accepting: q2
  defp ends_with_ab do
    DFA.new(
      MapSet.new(["q0", "q1", "q2"]),
      MapSet.new(["a", "b"]),
      %{
        {"q0", "a"} => "q1",
        {"q0", "b"} => "q0",
        {"q1", "a"} => "q1",
        {"q1", "b"} => "q2",
        {"q2", "a"} => "q1",
        {"q2", "b"} => "q0"
      },
      "q0",
      MapSet.new(["q2"])
    )
  end

  # === Construction tests ===

  describe "new/5 — construction and validation" do
    test "creates a valid DFA" do
      assert {:ok, dfa} = turnstile()
      assert dfa.current == "locked"
      assert dfa.initial == "locked"
      assert MapSet.member?(dfa.states, "locked")
      assert MapSet.member?(dfa.states, "unlocked")
    end

    test "rejects empty states" do
      assert {:error, msg} =
               DFA.new(MapSet.new(), MapSet.new(["a"]), %{}, "q0", MapSet.new())

      assert msg =~ "non-empty"
    end

    test "rejects initial state not in states" do
      assert {:error, msg} =
               DFA.new(
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
               DFA.new(
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
               DFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q99", "a"} => "q0"},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end

    test "rejects transition with unknown event" do
      assert {:error, msg} =
               DFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q0", "z"} => "q0"},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "z"
    end

    test "rejects transition with unknown target" do
      assert {:error, msg} =
               DFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q0", "a"} => "q99"},
                 "q0",
                 MapSet.new()
               )

      assert msg =~ "q99"
    end

    test "starts with empty trace" do
      {:ok, dfa} = turnstile()
      assert dfa.trace == []
    end

    test "accepts empty accepting set" do
      assert {:ok, dfa} =
               DFA.new(
                 MapSet.new(["q0"]),
                 MapSet.new(["a"]),
                 %{{"q0", "a"} => "q0"},
                 "q0",
                 MapSet.new()
               )

      assert MapSet.size(dfa.accepting) == 0
    end
  end

  # === Processing tests ===

  describe "process/2 — single event processing" do
    test "moves to correct state" do
      {:ok, dfa} = turnstile()
      assert {:ok, dfa} = DFA.process(dfa, "coin")
      assert dfa.current == "unlocked"
    end

    test "chains multiple transitions" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      {:ok, dfa} = DFA.process(dfa, "push")
      assert dfa.current == "locked"
    end

    test "records trace" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      assert length(dfa.trace) == 1

      [record] = dfa.trace
      assert record.source == "locked"
      assert record.event == "coin"
      assert record.target == "unlocked"
    end

    test "accumulates trace across multiple transitions" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      {:ok, dfa} = DFA.process(dfa, "push")
      {:ok, dfa} = DFA.process(dfa, "coin")
      assert length(dfa.trace) == 3
    end

    test "rejects unknown event" do
      {:ok, dfa} = turnstile()
      assert {:error, msg} = DFA.process(dfa, "kick")
      assert msg =~ "kick"
      assert msg =~ "alphabet"
    end

    test "rejects missing transition" do
      # Build a DFA with incomplete transitions
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a", "b"]),
          %{{"q0", "a"} => "q1"},
          "q0",
          MapSet.new(["q1"])
        )

      assert {:error, msg} = DFA.process(dfa, "b")
      assert msg =~ "No transition"
    end

    test "self-loops work correctly" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "push")
      assert dfa.current == "locked"
    end

    test "toggle back and forth" do
      {:ok, dfa} = toggle()
      {:ok, dfa} = DFA.process(dfa, "x")
      assert dfa.current == "b"
      {:ok, dfa} = DFA.process(dfa, "x")
      assert dfa.current == "a"
      {:ok, dfa} = DFA.process(dfa, "x")
      assert dfa.current == "b"
    end
  end

  # === Sequence processing tests ===

  describe "process_sequence/2" do
    test "processes all events in order" do
      {:ok, dfa} = turnstile()
      {:ok, dfa, trace} = DFA.process_sequence(dfa, ["coin", "push", "coin"])
      assert dfa.current == "unlocked"
      assert length(trace) == 3
    end

    test "returns empty trace for empty sequence" do
      {:ok, dfa} = turnstile()
      {:ok, dfa, trace} = DFA.process_sequence(dfa, [])
      assert dfa.current == "locked"
      assert trace == []
    end

    test "trace records match expectations" do
      {:ok, dfa} = toggle()
      {:ok, _dfa, trace} = DFA.process_sequence(dfa, ["x", "x", "x"])

      assert Enum.map(trace, fn r -> {r.source, r.target} end) == [
               {"a", "b"},
               {"b", "a"},
               {"a", "b"}
             ]
    end

    test "fails on invalid event mid-sequence" do
      {:ok, dfa} = turnstile()
      assert {:error, _msg} = DFA.process_sequence(dfa, ["coin", "kick"])
    end

    test "long sequence processes correctly" do
      {:ok, dfa} = ends_with_ab()
      events = List.duplicate("a", 50) ++ ["b"]
      {:ok, dfa, _trace} = DFA.process_sequence(dfa, events)
      assert dfa.current == "q2"
    end
  end

  # === Acceptance tests ===

  describe "accepts?/2" do
    test "turnstile accepts coin" do
      {:ok, dfa} = turnstile()
      assert DFA.accepts?(dfa, ["coin"])
    end

    test "turnstile rejects coin then push" do
      {:ok, dfa} = turnstile()
      refute DFA.accepts?(dfa, ["coin", "push"])
    end

    test "turnstile rejects empty input" do
      {:ok, dfa} = turnstile()
      refute DFA.accepts?(dfa, [])
    end

    test "accepts? does not modify the DFA" do
      {:ok, dfa} = turnstile()
      DFA.accepts?(dfa, ["coin", "push", "coin"])
      assert dfa.current == "locked"
      assert dfa.trace == []
    end

    test "toggle accepts odd number of x's" do
      {:ok, dfa} = toggle()
      assert DFA.accepts?(dfa, ["x"])
      refute DFA.accepts?(dfa, ["x", "x"])
      assert DFA.accepts?(dfa, ["x", "x", "x"])
    end

    test "ends_with_ab accepts strings ending in ab" do
      {:ok, dfa} = ends_with_ab()
      assert DFA.accepts?(dfa, ["a", "b"])
      assert DFA.accepts?(dfa, ["b", "a", "b"])
      assert DFA.accepts?(dfa, ["a", "a", "b"])
    end

    test "ends_with_ab rejects strings not ending in ab" do
      {:ok, dfa} = ends_with_ab()
      refute DFA.accepts?(dfa, ["a"])
      refute DFA.accepts?(dfa, ["b"])
      refute DFA.accepts?(dfa, ["a", "b", "a"])
    end

    test "returns false for missing transition (does not crash)" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a", "b"]),
          %{{"q0", "a"} => "q1"},
          "q0",
          MapSet.new(["q1"])
        )

      refute DFA.accepts?(dfa, ["b"])
    end

    test "raises on invalid event" do
      {:ok, dfa} = turnstile()

      assert_raise ArgumentError, fn ->
        DFA.accepts?(dfa, ["kick"])
      end
    end

    test "accepts initial state if it is accepting" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      assert DFA.accepts?(dfa, [])
    end
  end

  # === Reset tests ===

  describe "reset/1" do
    test "resets to initial state" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      assert dfa.current == "unlocked"

      dfa = DFA.reset(dfa)
      assert dfa.current == "locked"
    end

    test "clears trace" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      assert length(dfa.trace) == 1

      dfa = DFA.reset(dfa)
      assert dfa.trace == []
    end

    test "preserves structure" do
      {:ok, dfa} = turnstile()
      {:ok, dfa} = DFA.process(dfa, "coin")
      dfa = DFA.reset(dfa)

      assert dfa.initial == "locked"
      assert MapSet.size(dfa.states) == 2
      assert map_size(dfa.transitions) == 4
    end
  end

  # === Introspection tests ===

  describe "reachable_states/1" do
    test "all states reachable in turnstile" do
      {:ok, dfa} = turnstile()
      reachable = DFA.reachable_states(dfa)
      assert MapSet.equal?(reachable, MapSet.new(["locked", "unlocked"]))
    end

    test "detects unreachable states" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1", "orphan"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q1", {"q1", "a"} => "q0", {"orphan", "a"} => "orphan"},
          "q0",
          MapSet.new(["q1"])
        )

      reachable = DFA.reachable_states(dfa)
      assert MapSet.member?(reachable, "q0")
      assert MapSet.member?(reachable, "q1")
      refute MapSet.member?(reachable, "orphan")
    end

    test "single state is reachable" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      assert DFA.reachable_states(dfa) == MapSet.new(["q0"])
    end
  end

  describe "complete?/1" do
    test "turnstile is complete" do
      {:ok, dfa} = turnstile()
      assert DFA.complete?(dfa)
    end

    test "incomplete DFA detected" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a", "b"]),
          %{{"q0", "a"} => "q1"},
          "q0",
          MapSet.new(["q1"])
        )

      refute DFA.complete?(dfa)
    end
  end

  describe "validate/1" do
    test "no warnings for a well-formed DFA" do
      {:ok, dfa} = turnstile()
      assert DFA.validate(dfa) == []
    end

    test "warns about unreachable states" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1", "orphan"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q1", {"q1", "a"} => "q0", {"orphan", "a"} => "orphan"},
          "q0",
          MapSet.new(["q1"])
        )

      warnings = DFA.validate(dfa)
      assert Enum.any?(warnings, &(&1 =~ "Unreachable"))
      assert Enum.any?(warnings, &(&1 =~ "orphan"))
    end

    test "warns about missing transitions" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a", "b"]),
          %{{"q0", "a"} => "q1"},
          "q0",
          MapSet.new(["q1"])
        )

      warnings = DFA.validate(dfa)
      assert Enum.any?(warnings, &(&1 =~ "Missing transitions"))
    end

    test "warns about unreachable accepting states" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "orphan"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0", {"orphan", "a"} => "orphan"},
          "q0",
          MapSet.new(["orphan"])
        )

      warnings = DFA.validate(dfa)
      assert Enum.any?(warnings, &(&1 =~ "Unreachable accepting"))
    end
  end

  # === Visualization tests ===

  describe "to_dot/1" do
    test "generates valid DOT output" do
      {:ok, dfa} = turnstile()
      dot = DFA.to_dot(dfa)

      assert dot =~ "digraph DFA"
      assert dot =~ "rankdir=LR"
      assert dot =~ "__start"
      assert dot =~ "doublecircle"
      assert dot =~ "locked"
      assert dot =~ "unlocked"
    end

    test "marks accepting states as doublecircle" do
      {:ok, dfa} = turnstile()
      dot = DFA.to_dot(dfa)
      assert dot =~ "\"unlocked\" [shape=doublecircle]"
      assert dot =~ "\"locked\" [shape=circle]"
    end

    test "includes transition labels" do
      {:ok, dfa} = turnstile()
      dot = DFA.to_dot(dfa)
      assert dot =~ "coin"
      assert dot =~ "push"
    end
  end

  describe "to_ascii/1" do
    test "generates readable table" do
      {:ok, dfa} = turnstile()
      ascii = DFA.to_ascii(dfa)

      assert ascii =~ "coin"
      assert ascii =~ "push"
      assert ascii =~ "locked"
      assert ascii =~ "unlocked"
    end

    test "marks initial state with >" do
      {:ok, dfa} = turnstile()
      ascii = DFA.to_ascii(dfa)
      assert ascii =~ "> locked"
    end

    test "marks accepting state with *" do
      {:ok, dfa} = turnstile()
      ascii = DFA.to_ascii(dfa)
      assert ascii =~ "* unlocked"
    end
  end

  describe "to_table/1" do
    test "generates correct header" do
      {:ok, dfa} = turnstile()
      [header | _rows] = DFA.to_table(dfa)
      assert hd(header) == "State"
      assert "coin" in header
      assert "push" in header
    end

    test "generates correct data rows" do
      {:ok, dfa} = turnstile()
      [_header | rows] = DFA.to_table(dfa)
      assert length(rows) == 2

      locked_row = Enum.find(rows, fn [s | _] -> s == "locked" end)
      assert locked_row != nil
    end

    test "marks missing transitions" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0", "q1"]),
          MapSet.new(["a", "b"]),
          %{{"q0", "a"} => "q1"},
          "q0",
          MapSet.new(["q1"])
        )

      table = DFA.to_table(dfa)
      flat = List.flatten(table)
      assert "—" in flat
    end
  end

  # === Edge case tests ===

  describe "edge cases" do
    test "single state, single event, self-loop" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new(["q0"])
        )

      assert DFA.accepts?(dfa, [])
      assert DFA.accepts?(dfa, ["a"])
      assert DFA.accepts?(dfa, ["a", "a", "a"])
    end

    test "DFA with no accepting states never accepts" do
      {:ok, dfa} =
        DFA.new(
          MapSet.new(["q0"]),
          MapSet.new(["a"]),
          %{{"q0", "a"} => "q0"},
          "q0",
          MapSet.new()
        )

      refute DFA.accepts?(dfa, [])
      refute DFA.accepts?(dfa, ["a"])
    end

    test "large alphabet" do
      states = MapSet.new(["q0"])
      alphabet = MapSet.new(Enum.map(1..26, &"#{<<(&1 + 96)>>}"))

      # Just test that construction with many alphabet symbols works
      {:ok, dfa} =
        DFA.new(
          states,
          alphabet,
          Enum.map(alphabet, fn e -> {{"q0", e}, "q0"} end) |> Map.new(),
          "q0",
          MapSet.new(["q0"])
        )

      assert MapSet.size(dfa.alphabet) == 26
    end
  end
end
