defmodule CodingAdventures.BranchPredictor.DFAEquivalenceTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.{OneBit, TwoBit}
  alias CodingAdventures.StateMachine.DFA

  @moduledoc """
  Tests that verify the DFA definitions match the imperative transition logic.

  The one-bit and two-bit predictors use DFA transition tables as their
  single source of truth. These tests verify that:

  1. The DFA is well-formed (complete, all states reachable)
  2. The DFA transitions match what the predictor actually does
  3. Processing sequences through the DFA yields correct predictions
  """

  # ══════════════════════════════════════════════════════════════════════════
  # One-Bit DFA Equivalence
  # ══════════════════════════════════════════════════════════════════════════

  describe "one-bit DFA" do
    test "DFA is complete (every state handles every input)" do
      dfa = OneBit.dfa()
      assert DFA.complete?(dfa)
    end

    test "all states are reachable from initial" do
      dfa = OneBit.dfa()
      reachable = DFA.reachable_states(dfa)
      assert reachable == dfa.states
    end

    test "DFA validates with no warnings" do
      dfa = OneBit.dfa()
      assert DFA.validate(dfa) == []
    end

    test "DFA initial state is not_taken" do
      dfa = OneBit.dfa()
      assert dfa.initial == "not_taken"
    end

    test "DFA accepting state is taken" do
      dfa = OneBit.dfa()
      assert dfa.accepting == MapSet.new(["taken"])
    end

    test "DFA accepts? matches predictor for 'taken' sequence" do
      dfa = OneBit.dfa()
      assert DFA.accepts?(dfa, ["taken"])
    end

    test "DFA accepts? matches predictor for 'not_taken' sequence" do
      dfa = OneBit.dfa()
      refute DFA.accepts?(dfa, ["not_taken"])
    end

    test "DFA accepts? matches predictor for alternating sequence" do
      dfa = OneBit.dfa()
      refute DFA.accepts?(dfa, ["taken", "not_taken"])
      assert DFA.accepts?(dfa, ["not_taken", "taken"])
    end

    test "DFA process matches predictor update for taken" do
      dfa = OneBit.dfa()
      {:ok, dfa} = DFA.process(dfa, "taken")
      assert dfa.current == "taken"
      assert MapSet.member?(dfa.accepting, dfa.current)

      p = OneBit.new()
      p = OneBit.update(p, 0x100, true)
      {pred, _} = OneBit.predict(p, 0x100)
      assert pred.predicted_taken == true
    end

    test "DFA process matches predictor update for not_taken" do
      dfa = OneBit.dfa()
      {:ok, dfa} = DFA.process(dfa, "not_taken")
      assert dfa.current == "not_taken"
      refute MapSet.member?(dfa.accepting, dfa.current)

      p = OneBit.new()
      p = OneBit.update(p, 0x100, false)
      {pred, _} = OneBit.predict(p, 0x100)
      assert pred.predicted_taken == false
    end

    test "DFA sequence matches predictor for loop pattern" do
      dfa = OneBit.dfa()
      events = List.duplicate("taken", 9) ++ ["not_taken"]
      {:ok, dfa, _trace} = DFA.process_sequence(dfa, events)
      assert dfa.current == "not_taken"
      refute MapSet.member?(dfa.accepting, dfa.current)

      p = OneBit.new()
      p = Enum.reduce(1..9, p, fn _, acc -> OneBit.update(acc, 0x100, true) end)
      p = OneBit.update(p, 0x100, false)
      {pred, _} = OneBit.predict(p, 0x100)
      assert pred.predicted_taken == false
    end

    test "DFA trace has correct length" do
      dfa = OneBit.dfa()
      events = ["taken", "not_taken", "taken"]
      {:ok, _dfa, trace} = DFA.process_sequence(dfa, events)
      assert length(trace) == 3
    end

    test "DFA initial state matches predictor cold start" do
      dfa = OneBit.dfa()
      refute MapSet.member?(dfa.accepting, dfa.initial)

      p = OneBit.new()
      {pred, _} = OneBit.predict(p, 0x100)
      assert pred.predicted_taken == false
    end

    test "DFA accepts? matches predictor on multiple sequences" do
      sequences = [
        ["taken"],
        ["not_taken"],
        ["taken", "not_taken"],
        ["taken", "taken", "not_taken"],
        ["not_taken", "taken", "taken"]
      ]

      for seq <- sequences do
        dfa_accepts = DFA.accepts?(OneBit.dfa(), seq)

        p = OneBit.new()

        p =
          Enum.reduce(seq, p, fn event_str, acc ->
            OneBit.update(acc, 0x100, event_str == "taken")
          end)

        {pred, _} = OneBit.predict(p, 0x100)

        assert dfa_accepts == pred.predicted_taken,
               "Sequence #{inspect(seq)}: DFA=#{dfa_accepts}, predictor=#{pred.predicted_taken}"
      end
    end

    test "DFA has complete transitions (every state x event pair)" do
      dfa = OneBit.dfa()

      for state <- Enum.sort(dfa.states), event <- Enum.sort(dfa.alphabet) do
        assert Map.has_key?(dfa.transitions, {state, event}),
               "Missing transition for (#{state}, #{event})"
      end
    end
  end

  # ══════════════════════════════════════════════════════════════════════════
  # Two-Bit DFA Equivalence
  # ══════════════════════════════════════════════════════════════════════════

  describe "two-bit DFA" do
    test "DFA is complete" do
      dfa = TwoBit.dfa()
      assert DFA.complete?(dfa)
    end

    test "all states are reachable from initial" do
      dfa = TwoBit.dfa()
      reachable = DFA.reachable_states(dfa)
      assert reachable == dfa.states
    end

    test "DFA validates with no warnings" do
      dfa = TwoBit.dfa()
      assert DFA.validate(dfa) == []
    end

    test "DFA has 4 states" do
      dfa = TwoBit.dfa()
      assert MapSet.size(dfa.states) == 4
    end

    test "DFA has 2 accepting states (WT, ST)" do
      dfa = TwoBit.dfa()
      assert MapSet.size(dfa.accepting) == 2
      assert MapSet.member?(dfa.accepting, "WT")
      assert MapSet.member?(dfa.accepting, "ST")
    end

    test "DFA initial state WNT is not accepting (predicts not-taken)" do
      dfa = TwoBit.dfa()
      refute MapSet.member?(dfa.accepting, dfa.initial)
    end

    test "DFA initial state is WNT" do
      dfa = TwoBit.dfa()
      assert dfa.initial == "WNT"
    end

    test "DFA process: WNT + taken -> WT (accepting)" do
      dfa = TwoBit.dfa()
      {:ok, dfa} = DFA.process(dfa, "taken")
      assert dfa.current == "WT"
      assert MapSet.member?(dfa.accepting, dfa.current)
    end

    test "DFA process: WNT + not_taken -> SNT (not accepting)" do
      dfa = TwoBit.dfa()
      {:ok, dfa} = DFA.process(dfa, "not_taken")
      assert dfa.current == "SNT"
      refute MapSet.member?(dfa.accepting, dfa.current)
    end

    test "DFA saturation: repeated taken ends at ST" do
      dfa = TwoBit.dfa()
      events = List.duplicate("taken", 10)
      {:ok, dfa, _} = DFA.process_sequence(dfa, events)
      assert dfa.current == "ST"
    end

    test "DFA saturation: repeated not_taken ends at SNT" do
      dfa = TwoBit.dfa()
      events = List.duplicate("not_taken", 10)
      {:ok, dfa, _} = DFA.process_sequence(dfa, events)
      assert dfa.current == "SNT"
    end

    test "DFA accepts? matches predictor for single taken" do
      dfa = TwoBit.dfa()
      assert DFA.accepts?(dfa, ["taken"])

      p = TwoBit.new()
      p = TwoBit.update(p, 0x100, true)
      {pred, _} = TwoBit.predict(p, 0x100)
      assert pred.predicted_taken == true
    end

    test "DFA accepts? matches predictor for single not_taken" do
      dfa = TwoBit.dfa()
      refute DFA.accepts?(dfa, ["not_taken"])

      p = TwoBit.new()
      p = TwoBit.update(p, 0x100, false)
      {pred, _} = TwoBit.predict(p, 0x100)
      assert pred.predicted_taken == false
    end

    test "DFA hysteresis: taken, taken, not_taken -> WT (still accepting)" do
      dfa = TwoBit.dfa()
      assert DFA.accepts?(dfa, ["taken", "taken", "not_taken"])
    end

    test "DFA hysteresis: taken, taken, not_taken, not_taken -> WNT (not accepting)" do
      dfa = TwoBit.dfa()
      refute DFA.accepts?(dfa, ["taken", "taken", "not_taken", "not_taken"])
    end

    test "DFA loop pattern matches predictor" do
      dfa = TwoBit.dfa()
      events = List.duplicate("taken", 9) ++ ["not_taken"]
      {:ok, dfa_after, _} = DFA.process_sequence(dfa, events)
      assert dfa_after.current == "WT"

      p = TwoBit.new()
      p = Enum.reduce(1..9, p, fn _, acc -> TwoBit.update(acc, 0x100, true) end)
      p = TwoBit.update(p, 0x100, false)
      assert TwoBit.get_state_for_pc(p, 0x100) == "WT"
    end

    test "DFA transition table matches taken_outcome for all states" do
      for state <- ["SNT", "WNT", "WT", "ST"] do
        dfa = TwoBit.dfa()
        expected = dfa.transitions[{state, "taken"}]
        assert TwoBit.taken_outcome(state) == expected
      end
    end

    test "DFA transition table matches not_taken_outcome for all states" do
      for state <- ["SNT", "WNT", "WT", "ST"] do
        dfa = TwoBit.dfa()
        expected = dfa.transitions[{state, "not_taken"}]
        assert TwoBit.not_taken_outcome(state) == expected
      end
    end

    test "DFA accepting states match predicts_taken? for all states" do
      dfa = TwoBit.dfa()

      for state <- ["SNT", "WNT", "WT", "ST"] do
        assert TwoBit.predicts_taken?(state) == MapSet.member?(dfa.accepting, state)
      end
    end

    test "DFA accepts? matches predictor state for increasing taken count" do
      for n <- 0..4 do
        sequence = List.duplicate("taken", n)
        dfa_accepts = DFA.accepts?(TwoBit.dfa(), sequence)

        state =
          Enum.reduce(sequence, "WNT", fn _, s -> TwoBit.taken_outcome(s) end)

        assert dfa_accepts == TwoBit.predicts_taken?(state),
               "After #{n} 'taken': DFA=#{dfa_accepts}, state=#{state}"
      end
    end

    test "DFA accepts mixed sequence agrees with manual walk" do
      events = ["taken", "taken", "not_taken", "taken", "not_taken", "not_taken"]
      dfa_accepts = DFA.accepts?(TwoBit.dfa(), events)

      state =
        Enum.reduce(events, "WNT", fn event, s ->
          if event == "taken", do: TwoBit.taken_outcome(s), else: TwoBit.not_taken_outcome(s)
        end)

      assert dfa_accepts == TwoBit.predicts_taken?(state)
    end

    test "predictor update produces same results as DFA walk" do
      predictor = TwoBit.new(table_size: 4)
      pc = 0x100
      outcomes = [true, true, false, true, false, false, true]

      predictor =
        Enum.reduce(outcomes, predictor, fn taken, acc ->
          TwoBit.update(acc, pc, taken)
        end)

      final_state = TwoBit.get_state_for_pc(predictor, pc)

      dfa_state =
        Enum.reduce(outcomes, "WNT", fn taken, s ->
          event = if taken, do: "taken", else: "not_taken"
          TwoBit.dfa().transitions[{s, event}]
        end)

      assert final_state == dfa_state
    end

    test "DFA can be visualized with to_dot" do
      dfa = TwoBit.dfa()
      dot = DFA.to_dot(dfa)
      assert String.contains?(dot, "digraph DFA")
      assert String.contains?(dot, "SNT")
      assert String.contains?(dot, "ST")
    end

    test "DFA can be rendered as ASCII table" do
      dfa = TwoBit.dfa()
      ascii = DFA.to_ascii(dfa)
      assert String.contains?(ascii, "SNT")
      assert String.contains?(ascii, "ST")
    end

    test "DFA reset returns to initial state" do
      dfa = TwoBit.dfa()
      {:ok, dfa} = DFA.process(dfa, "taken")
      dfa = DFA.reset(dfa)
      assert dfa.current == "WNT"
    end

    test "DFA has complete transitions (every state x event pair)" do
      dfa = TwoBit.dfa()

      for state <- Enum.sort(dfa.states), event <- Enum.sort(dfa.alphabet) do
        assert Map.has_key?(dfa.transitions, {state, event}),
               "Missing transition for (#{state}, #{event})"
      end
    end
  end

  # ══════════════════════════════════════════════════════════════════════════
  # Cross-predictor equivalence
  # ══════════════════════════════════════════════════════════════════════════

  describe "cross-predictor" do
    test "one-bit and two-bit DFAs have different state counts" do
      assert MapSet.size(OneBit.dfa().states) == 2
      assert MapSet.size(TwoBit.dfa().states) == 4
    end

    test "both DFAs use the same alphabet" do
      assert OneBit.dfa().alphabet == TwoBit.dfa().alphabet
    end

    test "both DFAs are complete" do
      assert DFA.complete?(OneBit.dfa())
      assert DFA.complete?(TwoBit.dfa())
    end

    test "both DFAs validate cleanly" do
      assert DFA.validate(OneBit.dfa()) == []
      assert DFA.validate(TwoBit.dfa()) == []
    end
  end
end
