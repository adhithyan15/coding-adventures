defmodule CodingAdventures.BranchPredictor.TwoBitTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.TwoBit
  alias CodingAdventures.BranchPredictor.Stats

  # ── Construction ─────────────────────────────────────────────────────────

  test "new/0 creates predictor with defaults" do
    p = TwoBit.new()
    assert p.table_size == 1024
    assert p.initial_state == "WNT"
    assert p.table == %{}
  end

  test "new/1 accepts custom table_size and initial_state" do
    p = TwoBit.new(table_size: 256, initial_state: "WT")
    assert p.table_size == 256
    assert p.initial_state == "WT"
  end

  # ── State transitions (via DFA) ─────────────────────────────────────────

  test "taken_outcome transitions SNT -> WNT" do
    assert TwoBit.taken_outcome("SNT") == "WNT"
  end

  test "taken_outcome transitions WNT -> WT" do
    assert TwoBit.taken_outcome("WNT") == "WT"
  end

  test "taken_outcome transitions WT -> ST" do
    assert TwoBit.taken_outcome("WT") == "ST"
  end

  test "taken_outcome saturates at ST" do
    assert TwoBit.taken_outcome("ST") == "ST"
  end

  test "not_taken_outcome transitions ST -> WT" do
    assert TwoBit.not_taken_outcome("ST") == "WT"
  end

  test "not_taken_outcome transitions WT -> WNT" do
    assert TwoBit.not_taken_outcome("WT") == "WNT"
  end

  test "not_taken_outcome transitions WNT -> SNT" do
    assert TwoBit.not_taken_outcome("WNT") == "SNT"
  end

  test "not_taken_outcome saturates at SNT" do
    assert TwoBit.not_taken_outcome("SNT") == "SNT"
  end

  # ── predicts_taken? ─────────────────────────────────────────────────────

  test "predicts_taken? for all states" do
    assert TwoBit.predicts_taken?("ST") == true
    assert TwoBit.predicts_taken?("WT") == true
    assert TwoBit.predicts_taken?("WNT") == false
    assert TwoBit.predicts_taken?("SNT") == false
  end

  # ── Cold start prediction ───────────────────────────────────────────────

  test "predict/2 defaults to not-taken from WNT initial state" do
    p = TwoBit.new()
    {pred, _p} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == false
    assert pred.confidence == 0.5
  end

  test "predict/2 defaults to taken from WT initial state" do
    p = TwoBit.new(initial_state: "WT")
    {pred, _p} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == true
  end

  test "predict/2 with SNT initial state has high confidence" do
    p = TwoBit.new(initial_state: "SNT")
    {pred, _p} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == false
    assert pred.confidence == 1.0
  end

  test "predict/2 with ST initial state has high confidence" do
    p = TwoBit.new(initial_state: "ST")
    {pred, _p} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == true
    assert pred.confidence == 1.0
  end

  # ── Basic predict/update cycle ──────────────────────────────────────────

  test "one taken moves WNT -> WT, predicts taken" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == true
  end

  test "two takens from WNT moves to ST" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    assert TwoBit.get_state_for_pc(p, 0x100) == "ST"
  end

  test "one not-taken from WNT moves to SNT" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, false)
    assert TwoBit.get_state_for_pc(p, 0x100) == "SNT"
  end

  test "hysteresis: one anomaly doesn't flip from ST" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    assert TwoBit.get_state_for_pc(p, 0x100) == "ST"

    p = TwoBit.update(p, 0x100, false)
    assert TwoBit.get_state_for_pc(p, 0x100) == "WT"
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == true
  end

  test "hysteresis: TWO not-takens flip from ST to WNT" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)

    p = TwoBit.update(p, 0x100, false)
    p = TwoBit.update(p, 0x100, false)
    assert TwoBit.get_state_for_pc(p, 0x100) == "WNT"
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.predicted_taken == false
  end

  # ── Loop behavior ───────────────────────────────────────────────────────

  test "loop: first invocation (10 iterations)" do
    p = TwoBit.new()
    pc = 0x100

    p = TwoBit.update(p, pc, true)
    assert TwoBit.stats(p).incorrect == 1

    p = Enum.reduce(2..9, p, fn _, acc -> TwoBit.update(acc, pc, true) end)
    assert TwoBit.stats(p).correct == 8
    assert TwoBit.stats(p).incorrect == 1

    p = TwoBit.update(p, pc, false)
    assert TwoBit.stats(p).incorrect == 2
    assert TwoBit.get_state_for_pc(p, pc) == "WT"
  end

  test "loop: second invocation starts from WT, only 1 misprediction" do
    p = TwoBit.new()
    pc = 0x100

    p = Enum.reduce(1..9, p, fn _, acc -> TwoBit.update(acc, pc, true) end)
    p = TwoBit.update(p, pc, false)

    assert TwoBit.get_state_for_pc(p, pc) == "WT"

    p2 = %{p | stats: %Stats{}}
    p2 = TwoBit.update(p2, pc, true)
    assert TwoBit.stats(p2).correct == 1

    p2 = Enum.reduce(2..9, p2, fn _, acc -> TwoBit.update(acc, pc, true) end)
    p2 = TwoBit.update(p2, pc, false)

    stats = TwoBit.stats(p2)
    assert stats.correct == 9
    assert stats.incorrect == 1
    assert Stats.accuracy(stats) == 90.0
  end

  # ── Two-bit beats one-bit ──────────────────────────────────────────────

  test "two-bit beats one-bit on repeated loops" do
    alias CodingAdventures.BranchPredictor.OneBit

    one_bit = OneBit.new()
    two_bit = TwoBit.new()
    pc = 0x100

    {one_bit, two_bit} =
      Enum.reduce(1..5, {one_bit, two_bit}, fn _, {ob, tb} ->
        {ob2, tb2} =
          Enum.reduce(1..9, {ob, tb}, fn _, {o, t} ->
            {OneBit.update(o, pc, true), TwoBit.update(t, pc, true)}
          end)

        {OneBit.update(ob2, pc, false), TwoBit.update(tb2, pc, false)}
      end)

    assert Stats.accuracy(TwoBit.stats(two_bit)) > Stats.accuracy(OneBit.stats(one_bit))
  end

  # ── Saturation behavior ─────────────────────────────────────────────────

  test "repeated taken saturates at ST" do
    p = TwoBit.new()
    p = Enum.reduce(1..10, p, fn _, acc -> TwoBit.update(acc, 0x100, true) end)
    assert TwoBit.get_state_for_pc(p, 0x100) == "ST"
  end

  test "repeated not-taken saturates at SNT" do
    p = TwoBit.new()
    p = Enum.reduce(1..10, p, fn _, acc -> TwoBit.update(acc, 0x100, false) end)
    assert TwoBit.get_state_for_pc(p, 0x100) == "SNT"
  end

  # ── Multiple branches ───────────────────────────────────────────────────

  test "independent branches tracked separately" do
    p = TwoBit.new(table_size: 1024)
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x200, false)
    p = TwoBit.update(p, 0x200, false)

    assert TwoBit.get_state_for_pc(p, 0x100) == "ST"
    assert TwoBit.get_state_for_pc(p, 0x200) == "SNT"
  end

  # ── Table aliasing ─────────────────────────────────────────────────────

  test "small table causes aliasing" do
    p = TwoBit.new(table_size: 2)
    p = Enum.reduce(1..5, p, fn _, acc -> TwoBit.update(acc, 0, true) end)
    assert TwoBit.get_state_for_pc(p, 0) == "ST"
    # PC=2 maps to same index (2 % 2 = 0)
    assert TwoBit.get_state_for_pc(p, 2) == "ST"
  end

  test "large table avoids aliasing" do
    p = TwoBit.new(table_size: 4096)
    p = Enum.reduce(1..5, p, fn _, acc -> TwoBit.update(acc, 0, true) end)
    assert TwoBit.get_state_for_pc(p, 2) == "WNT"
  end

  # ── Full state walkthrough ─────────────────────────────────────────────

  test "walk up SNT to ST and back down" do
    p = TwoBit.new(initial_state: "SNT")
    pc = 0x100

    assert TwoBit.get_state_for_pc(p, pc) == "SNT"
    p = TwoBit.update(p, pc, true)
    assert TwoBit.get_state_for_pc(p, pc) == "WNT"
    p = TwoBit.update(p, pc, true)
    assert TwoBit.get_state_for_pc(p, pc) == "WT"
    p = TwoBit.update(p, pc, true)
    assert TwoBit.get_state_for_pc(p, pc) == "ST"

    p = TwoBit.update(p, pc, false)
    assert TwoBit.get_state_for_pc(p, pc) == "WT"
    p = TwoBit.update(p, pc, false)
    assert TwoBit.get_state_for_pc(p, pc) == "WNT"
    p = TwoBit.update(p, pc, false)
    assert TwoBit.get_state_for_pc(p, pc) == "SNT"
  end

  # ── DFA ─────────────────────────────────────────────────────────────────

  test "dfa/0 returns a valid DFA struct" do
    dfa = TwoBit.dfa()
    assert dfa.states == MapSet.new(["SNT", "WNT", "WT", "ST"])
    assert dfa.alphabet == MapSet.new(["taken", "not_taken"])
    assert dfa.initial == "WNT"
    assert dfa.accepting == MapSet.new(["WT", "ST"])
  end

  test "dfa/0 has 8 transitions (4 states x 2 inputs)" do
    dfa = TwoBit.dfa()
    assert map_size(dfa.transitions) == 8
  end

  # ── Stats ───────────────────────────────────────────────────────────────

  test "stats/1 returns the stats struct" do
    p = TwoBit.new()
    assert TwoBit.stats(p) == %Stats{}
  end

  test "stats track accuracy correctly" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.update(p, 0x100, true)
    stats = TwoBit.stats(p)
    assert stats.correct == 1
    assert stats.incorrect == 1
  end

  # ── Reset ───────────────────────────────────────────────────────────────

  test "reset/1 clears table and stats" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.reset(p)
    assert p.table == %{}
    assert p.stats == %Stats{}
  end

  test "reset/1 preserves table_size and initial_state" do
    p = TwoBit.new(table_size: 512, initial_state: "WT")
    p = TwoBit.update(p, 0x100, true)
    p = TwoBit.reset(p)
    assert p.table_size == 512
    assert p.initial_state == "WT"
  end

  # ── Confidence ──────────────────────────────────────────────────────────

  test "confidence is 0.5 for weak states" do
    p = TwoBit.new()
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.confidence == 0.5

    p = TwoBit.update(p, 0x100, true)
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.confidence == 0.5
  end

  test "confidence is 1.0 for strong states" do
    p = TwoBit.new()
    p = TwoBit.update(p, 0x100, false)
    {pred, _} = TwoBit.predict(p, 0x100)
    assert pred.confidence == 1.0

    p2 = TwoBit.new()
    p2 = TwoBit.update(p2, 0x100, true)
    p2 = TwoBit.update(p2, 0x100, true)
    p2 = TwoBit.update(p2, 0x100, true)
    {pred, _} = TwoBit.predict(p2, 0x100)
    assert pred.confidence == 1.0
  end

  # ── Immutability ────────────────────────────────────────────────────────

  test "update/3 returns new struct, original unchanged" do
    original = TwoBit.new()
    _updated = TwoBit.update(original, 0x100, true)
    assert original.table == %{}
    assert original.stats.predictions == 0
  end
end
