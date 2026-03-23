defmodule CodingAdventures.BranchPredictor.StaticTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.Static.{AlwaysTaken, AlwaysNotTaken}
  alias CodingAdventures.BranchPredictor.Stats

  # ══════════════════════════════════════════════════════════════════════════
  # AlwaysTaken
  # ══════════════════════════════════════════════════════════════════════════

  describe "AlwaysTaken" do
    test "new/0 creates predictor with empty stats" do
      p = AlwaysTaken.new()
      assert AlwaysTaken.stats(p).predictions == 0
    end

    test "predict/2 always returns predicted_taken=true" do
      p = AlwaysTaken.new()
      {pred, _p} = AlwaysTaken.predict(p, 0x100)
      assert pred.predicted_taken == true
      assert pred.confidence == 0.0
    end

    test "predict/2 returns nil address" do
      p = AlwaysTaken.new()
      {pred, _p} = AlwaysTaken.predict(p, 0x100)
      assert pred.address == nil
    end

    test "predict/2 ignores the PC value" do
      p = AlwaysTaken.new()
      {pred1, _} = AlwaysTaken.predict(p, 0x000)
      {pred2, _} = AlwaysTaken.predict(p, 0xFFFF)
      {pred3, _} = AlwaysTaken.predict(p, 0xDEAD)
      {pred4, _} = AlwaysTaken.predict(p, 0xFFFF_FFFF)
      assert pred1.predicted_taken == true
      assert pred2.predicted_taken == true
      assert pred3.predicted_taken == true
      assert pred4.predicted_taken == true
    end

    test "update/3 records correct when branch was taken" do
      p = AlwaysTaken.new()
      p = AlwaysTaken.update(p, 0x100, true)
      assert AlwaysTaken.stats(p).correct == 1
      assert AlwaysTaken.stats(p).incorrect == 0
    end

    test "update/3 records incorrect when branch was not taken" do
      p = AlwaysTaken.new()
      p = AlwaysTaken.update(p, 0x100, false)
      assert AlwaysTaken.stats(p).correct == 0
      assert AlwaysTaken.stats(p).incorrect == 1
    end

    test "100% accuracy when all taken" do
      p = AlwaysTaken.new()
      p = Enum.reduce(1..100, p, fn _, acc -> AlwaysTaken.update(acc, 0x100, true) end)
      assert Stats.accuracy(AlwaysTaken.stats(p)) == 100.0
    end

    test "0% accuracy when all not taken" do
      p = AlwaysTaken.new()
      p = Enum.reduce(1..100, p, fn _, acc -> AlwaysTaken.update(acc, 0x100, false) end)
      assert Stats.accuracy(AlwaysTaken.stats(p)) == 0.0
    end

    test "accuracy on a loop (9 taken, 1 not taken) is 90%" do
      p = AlwaysTaken.new()
      p = Enum.reduce(1..9, p, fn _, acc -> AlwaysTaken.update(acc, 0x100, true) end)
      p = AlwaysTaken.update(p, 0x100, false)
      assert Stats.accuracy(AlwaysTaken.stats(p)) == 90.0
    end

    test "mixed sequence: 60% taken -> 60% accuracy" do
      p = AlwaysTaken.new()
      p = Enum.reduce(1..60, p, fn _, acc -> AlwaysTaken.update(acc, 0x100, true) end)
      p = Enum.reduce(1..40, p, fn _, acc -> AlwaysTaken.update(acc, 0x100, false) end)
      assert Stats.accuracy(AlwaysTaken.stats(p)) == 60.0
    end

    test "reset/0 clears stats" do
      p = AlwaysTaken.new()
      p = AlwaysTaken.update(p, 0x100, true)
      p = AlwaysTaken.reset(p)
      assert AlwaysTaken.stats(p).predictions == 0
    end

    test "update/4 with target argument" do
      p = AlwaysTaken.new()
      p = AlwaysTaken.update(p, 0x100, true, 0x200)
      assert AlwaysTaken.stats(p).predictions == 1
    end
  end

  # ══════════════════════════════════════════════════════════════════════════
  # AlwaysNotTaken
  # ══════════════════════════════════════════════════════════════════════════

  describe "AlwaysNotTaken" do
    test "new/0 creates predictor with empty stats" do
      p = AlwaysNotTaken.new()
      assert AlwaysNotTaken.stats(p).predictions == 0
    end

    test "predict/2 always returns predicted_taken=false" do
      p = AlwaysNotTaken.new()
      {pred, _p} = AlwaysNotTaken.predict(p, 0x100)
      assert pred.predicted_taken == false
      assert pred.confidence == 0.0
    end

    test "predict/2 ignores the PC value" do
      p = AlwaysNotTaken.new()
      {pred1, _} = AlwaysNotTaken.predict(p, 0x000)
      {pred2, _} = AlwaysNotTaken.predict(p, 0xFFFF)
      {pred3, _} = AlwaysNotTaken.predict(p, 0xDEAD)
      assert pred1.predicted_taken == false
      assert pred2.predicted_taken == false
      assert pred3.predicted_taken == false
    end

    test "update/3 records correct when branch was not taken" do
      p = AlwaysNotTaken.new()
      p = AlwaysNotTaken.update(p, 0x100, false)
      assert AlwaysNotTaken.stats(p).correct == 1
      assert AlwaysNotTaken.stats(p).incorrect == 0
    end

    test "update/3 records incorrect when branch was taken" do
      p = AlwaysNotTaken.new()
      p = AlwaysNotTaken.update(p, 0x100, true)
      assert AlwaysNotTaken.stats(p).correct == 0
      assert AlwaysNotTaken.stats(p).incorrect == 1
    end

    test "100% accuracy when all not taken" do
      p = AlwaysNotTaken.new()
      p = Enum.reduce(1..100, p, fn _, acc -> AlwaysNotTaken.update(acc, 0x100, false) end)
      assert Stats.accuracy(AlwaysNotTaken.stats(p)) == 100.0
    end

    test "0% accuracy when all taken" do
      p = AlwaysNotTaken.new()
      p = Enum.reduce(1..100, p, fn _, acc -> AlwaysNotTaken.update(acc, 0x100, true) end)
      assert Stats.accuracy(AlwaysNotTaken.stats(p)) == 0.0
    end

    test "accuracy on a loop (9 taken, 1 not taken) is 10%" do
      p = AlwaysNotTaken.new()
      p = Enum.reduce(1..9, p, fn _, acc -> AlwaysNotTaken.update(acc, 0x100, true) end)
      p = AlwaysNotTaken.update(p, 0x100, false)
      assert Stats.accuracy(AlwaysNotTaken.stats(p)) == 10.0
    end

    test "inverse of AlwaysTaken: accuracies sum to 100" do
      taken_p = AlwaysTaken.new()
      not_taken_p = AlwaysNotTaken.new()

      # 70% taken
      {taken_p, not_taken_p} =
        Enum.reduce(1..70, {taken_p, not_taken_p}, fn i, {tp, ntp} ->
          {AlwaysTaken.update(tp, i * 4, true), AlwaysNotTaken.update(ntp, i * 4, true)}
        end)

      {taken_p, not_taken_p} =
        Enum.reduce(1..30, {taken_p, not_taken_p}, fn i, {tp, ntp} ->
          {AlwaysTaken.update(tp, i * 4, false), AlwaysNotTaken.update(ntp, i * 4, false)}
        end)

      taken_acc = Stats.accuracy(AlwaysTaken.stats(taken_p))
      not_taken_acc = Stats.accuracy(AlwaysNotTaken.stats(not_taken_p))
      assert taken_acc == 70.0
      assert not_taken_acc == 30.0
      assert abs(taken_acc + not_taken_acc - 100.0) < 1.0e-10
    end

    test "reset/0 clears stats" do
      p = AlwaysNotTaken.new()
      p = AlwaysNotTaken.update(p, 0x100, false)
      p = AlwaysNotTaken.reset(p)
      assert AlwaysNotTaken.stats(p).predictions == 0
    end
  end
end
