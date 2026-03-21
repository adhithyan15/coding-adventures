defmodule CodingAdventures.BranchPredictor.OneBitTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.OneBit
  alias CodingAdventures.BranchPredictor.Stats

  # ── Construction ─────────────────────────────────────────────────────────

  test "new/0 creates predictor with default table_size 1024" do
    p = OneBit.new()
    assert p.table_size == 1024
    assert p.table == %{}
    assert p.stats == %Stats{}
  end

  test "new/1 accepts custom table_size" do
    p = OneBit.new(table_size: 256)
    assert p.table_size == 256
  end

  # ── Cold start prediction ───────────────────────────────────────────────

  test "predict/2 defaults to not-taken on cold start" do
    p = OneBit.new()
    {pred, _p} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == false
    assert pred.confidence == 0.5
  end

  test "predict/2 cold start for multiple branches" do
    p = OneBit.new()
    {pred1, _} = OneBit.predict(p, 0x100)
    {pred2, _} = OneBit.predict(p, 0x200)
    {pred3, _} = OneBit.predict(p, 0x300)
    assert pred1.predicted_taken == false
    assert pred2.predicted_taken == false
    assert pred3.predicted_taken == false
  end

  # ── Basic predict/update cycle ──────────────────────────────────────────

  test "update then predict — remembers last outcome (taken)" do
    p = OneBit.new()
    p = OneBit.update(p, 0x100, true)
    {pred, _p} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == true
  end

  test "update then predict — remembers last outcome (not taken)" do
    p = OneBit.new()
    p = OneBit.update(p, 0x100, true)
    p = OneBit.update(p, 0x100, false)
    {pred, _p} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == false
  end

  test "flips on every misprediction" do
    p = OneBit.new()
    p = OneBit.update(p, 0x100, true)
    {pred, _} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == true

    p = OneBit.update(p, 0x100, false)
    {pred, _} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == false
  end

  test "confidence is always 0.5" do
    p = OneBit.new()
    {pred, _} = OneBit.predict(p, 0x100)
    assert pred.confidence == 0.5
  end

  # ── Loop pattern (double-misprediction problem) ─────────────────────────

  test "double-misprediction on loop pattern" do
    p = OneBit.new()
    pc = 0x100

    # Iter 1: cold start (not_taken), actual taken -> WRONG
    p = OneBit.update(p, pc, true)
    assert OneBit.stats(p).incorrect == 1

    # Iter 2-9: predicts taken, actual taken -> CORRECT
    p = Enum.reduce(2..9, p, fn _, acc -> OneBit.update(acc, pc, true) end)
    assert OneBit.stats(p).correct == 8
    assert OneBit.stats(p).incorrect == 1

    # Iter 10: predicts taken, actual not-taken -> WRONG
    p = OneBit.update(p, pc, false)
    assert OneBit.stats(p).incorrect == 2

    # Second invocation, iter 1: bit=false, predicts not-taken, actual taken -> WRONG
    p = OneBit.update(p, pc, true)
    assert OneBit.stats(p).incorrect == 3
  end

  test "loop accuracy — 10 iterations (9 taken, 1 not) = 80%" do
    p = OneBit.new()
    pc = 0x100

    p = Enum.reduce(1..9, p, fn _, acc -> OneBit.update(acc, pc, true) end)
    p = OneBit.update(p, pc, false)

    stats = OneBit.stats(p)
    assert stats.predictions == 10
    assert stats.correct == 8
    assert stats.incorrect == 2
    assert Stats.accuracy(stats) == 80.0
  end

  test "loop repeated invocations — 4 mispredictions across 2 runs" do
    p = OneBit.new()
    pc = 0x100

    # First invocation: 10 iterations
    p = Enum.reduce(1..9, p, fn _, acc -> OneBit.update(acc, pc, true) end)
    p = OneBit.update(p, pc, false)

    # Second invocation: 10 more iterations
    p = Enum.reduce(1..9, p, fn _, acc -> OneBit.update(acc, pc, true) end)
    p = OneBit.update(p, pc, false)

    assert OneBit.stats(p).incorrect == 4
    assert OneBit.stats(p).correct == 16
  end

  # ── Multiple branches ───────────────────────────────────────────────────

  test "independent branches don't interfere (different indices)" do
    p = OneBit.new(table_size: 1024)
    p = OneBit.update(p, 0x100, true)
    p = OneBit.update(p, 0x200, false)

    {pred1, _} = OneBit.predict(p, 0x100)
    {pred2, _} = OneBit.predict(p, 0x200)
    assert pred1.predicted_taken == true
    assert pred2.predicted_taken == false
  end

  # ── Aliasing ────────────────────────────────────────────────────────────

  test "aliasing — branches with same index conflict" do
    p = OneBit.new(table_size: 4)
    p = OneBit.update(p, 0, true)
    {pred, _} = OneBit.predict(p, 0)
    assert pred.predicted_taken == true

    # Branch at index 4 overwrites the same entry
    p = OneBit.update(p, 4, false)
    {pred, _} = OneBit.predict(p, 0)
    assert pred.predicted_taken == false
  end

  test "aliasing with small table demonstrates interference" do
    p = OneBit.new(table_size: 2)
    p = OneBit.update(p, 0, true)
    p = OneBit.update(p, 2, false)
    {pred, _} = OneBit.predict(p, 0)
    assert pred.predicted_taken == false
  end

  test "no aliasing with large table" do
    p = OneBit.new(table_size: 4096)
    p = OneBit.update(p, 0x100, true)
    p = OneBit.update(p, 0x104, false)
    {pred1, _} = OneBit.predict(p, 0x100)
    {pred2, _} = OneBit.predict(p, 0x104)
    assert pred1.predicted_taken == true
    assert pred2.predicted_taken == false
  end

  # ── Alternating pattern (worst case) ────────────────────────────────────

  test "alternating T/NT is worst case — 0% accuracy" do
    p = OneBit.new()
    pc = 0x100

    p =
      Enum.reduce(0..99, p, fn i, acc ->
        taken = rem(i, 2) == 0
        OneBit.update(acc, pc, taken)
      end)

    assert Stats.accuracy(OneBit.stats(p)) == 0.0
  end

  # ── DFA ─────────────────────────────────────────────────────────────────

  test "dfa/0 returns a valid DFA struct" do
    dfa = OneBit.dfa()
    assert dfa.states == MapSet.new(["not_taken", "taken"])
    assert dfa.alphabet == MapSet.new(["taken", "not_taken"])
    assert dfa.initial == "not_taken"
    assert dfa.accepting == MapSet.new(["taken"])
  end

  test "dfa/0 transitions are complete" do
    dfa = OneBit.dfa()
    assert Map.has_key?(dfa.transitions, {"not_taken", "taken"})
    assert Map.has_key?(dfa.transitions, {"not_taken", "not_taken"})
    assert Map.has_key?(dfa.transitions, {"taken", "taken"})
    assert Map.has_key?(dfa.transitions, {"taken", "not_taken"})
  end

  # ── Stats ───────────────────────────────────────────────────────────────

  test "stats/1 returns the stats struct" do
    p = OneBit.new()
    assert OneBit.stats(p) == %Stats{}
  end

  # ── Reset ───────────────────────────────────────────────────────────────

  test "reset/1 clears table and stats" do
    p = OneBit.new()
    p = OneBit.update(p, 0x100, true)
    p = OneBit.reset(p)
    assert p.table == %{}
    assert p.stats == %Stats{}
    {pred, _} = OneBit.predict(p, 0x100)
    assert pred.predicted_taken == false
  end

  test "reset/1 preserves table_size" do
    p = OneBit.new(table_size: 512)
    p = OneBit.update(p, 0x100, true)
    p = OneBit.reset(p)
    assert p.table_size == 512
  end

  # ── Immutability ────────────────────────────────────────────────────────

  test "predict/2 does not modify predictor state" do
    p = OneBit.new()
    p = OneBit.update(p, 0x100, true)
    {_pred, p2} = OneBit.predict(p, 0x100)
    assert p == p2
  end

  test "update/3 returns new struct, original unchanged" do
    original = OneBit.new()
    _updated = OneBit.update(original, 0x100, true)
    assert original.table == %{}
    assert original.stats.predictions == 0
  end
end
