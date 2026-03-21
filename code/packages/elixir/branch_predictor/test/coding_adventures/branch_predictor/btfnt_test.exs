defmodule CodingAdventures.BranchPredictor.BTFNTTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.Static.BTFNT

  # ── Construction ─────────────────────────────────────────────────────────

  test "new/0 creates predictor with empty state" do
    p = BTFNT.new()
    assert BTFNT.stats(p).predictions == 0
    assert p.targets == %{}
  end

  # ── Cold start behavior ──────────────────────────────────────────────────

  test "predict/2 defaults to not-taken on cold start" do
    p = BTFNT.new()
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.predicted_taken == false
    assert pred.confidence == 0.0
  end

  test "cold start prediction has nil address" do
    p = BTFNT.new()
    {pred, _p} = BTFNT.predict(p, 0x100)
    assert pred.address == nil
  end

  # ── Backward branches (loop back-edges) ──────────────────────────────────

  test "predict/2 returns taken for backward branch after learning target" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.predicted_taken == true
    assert pred.confidence == 0.5
    assert pred.address == 0x100
  end

  # ── Forward branches (if-else) ──────────────────────────────────────────

  test "predict/2 returns not-taken for forward branch after learning target" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x200, false, 0x20C)
    {pred, _p} = BTFNT.predict(p, 0x200)
    assert pred.predicted_taken == false
    assert pred.address == 0x20C
  end

  # ── Equal target (degenerate case) ──────────────────────────────────────

  test "predict/2 returns taken when target equals pc" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x100, true, 0x100)
    {pred, _p} = BTFNT.predict(p, 0x100)
    assert pred.predicted_taken == true
  end

  # ── Target learning ─────────────────────────────────────────────────────

  test "update/4 stores target for future predictions" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    assert Map.get(p.targets, 0x108) == 0x100
  end

  test "update/3 with nil target does not corrupt stored targets" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    p = BTFNT.update(p, 0x108, true)
    assert Map.get(p.targets, 0x108) == 0x100
  end

  # ── Accuracy on loop patterns ──────────────────────────────────────────

  test "backward branch loop: 9 taken + 1 not-taken" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    p = Enum.reduce(1..8, p, fn _, acc -> BTFNT.update(acc, 0x108, true, 0x100) end)
    p = BTFNT.update(p, 0x108, false, 0x100)

    stats = BTFNT.stats(p)
    assert stats.predictions == 10
    assert stats.correct == 9
    assert stats.incorrect == 1
  end

  test "forward branch accuracy: 8 not-taken + 2 taken" do
    p = BTFNT.new()
    pc = 0x200
    target = 0x20C

    # First update: no prior target known. After storing target, forward => predicted not-taken,
    # actual not-taken => correct
    p = BTFNT.update(p, pc, false, target)

    # 7 more not-taken: forward => predicted not-taken => correct
    p = Enum.reduce(1..7, p, fn _, acc -> BTFNT.update(acc, pc, false, target) end)

    # 2 taken: forward => predicted not-taken, actual taken => wrong
    p = BTFNT.update(p, pc, true, target)
    p = BTFNT.update(p, pc, true, target)

    stats = BTFNT.stats(p)
    assert stats.correct == 8
    assert stats.incorrect == 2
  end

  # ── Confidence ──────────────────────────────────────────────────────────

  test "confidence on known branch is 0.5" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.confidence == 0.5
  end

  test "confidence on unknown branch is 0.0" do
    p = BTFNT.new()
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.confidence == 0.0
  end

  # ── Address in prediction ──────────────────────────────────────────────

  test "known branches include address in prediction" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.address == 0x100
  end

  # ── Reset ──────────────────────────────────────────────────────────────

  test "reset/0 clears targets and stats" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    p = BTFNT.reset(p)
    assert BTFNT.stats(p).predictions == 0
    assert p.targets == %{}
  end

  test "after reset, cold start behavior resumes" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x108, true, 0x100)
    p = BTFNT.reset(p)
    {pred, _p} = BTFNT.predict(p, 0x108)
    assert pred.predicted_taken == false
  end

  # ── Multiple branches ──────────────────────────────────────────────────

  test "multiple branches tracked independently" do
    p = BTFNT.new()
    p = BTFNT.update(p, 0x100, true, 0x080)
    p = BTFNT.update(p, 0x200, false, 0x300)

    {pred1, _} = BTFNT.predict(p, 0x100)
    {pred2, _} = BTFNT.predict(p, 0x200)

    # 0x080 < 0x100 => backward => taken
    assert pred1.predicted_taken == true
    # 0x300 > 0x200 => forward => not taken
    assert pred2.predicted_taken == false
  end

  test "different branches with different directions" do
    p = BTFNT.new()
    # Backward branch (loop)
    p = BTFNT.update(p, 0x108, true, 0x100)
    # Forward branch (if-else)
    p = BTFNT.update(p, 0x200, false, 0x20C)
    # Self-loop
    p = BTFNT.update(p, 0x300, true, 0x300)

    {pred1, _} = BTFNT.predict(p, 0x108)
    {pred2, _} = BTFNT.predict(p, 0x200)
    {pred3, _} = BTFNT.predict(p, 0x300)

    assert pred1.predicted_taken == true
    assert pred2.predicted_taken == false
    assert pred3.predicted_taken == true
  end
end
