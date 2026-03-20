defmodule CodingAdventures.BranchPredictor.StatsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BranchPredictor.Stats

  # ── Construction ─────────────────────────────────────────────────────────

  test "new/0 creates stats with all zeroes" do
    stats = Stats.new()
    assert stats.predictions == 0
    assert stats.correct == 0
    assert stats.incorrect == 0
  end

  # ── Recording outcomes ──────────────────────────────────────────────────

  test "record/2 increments predictions and correct on true" do
    stats = Stats.new() |> Stats.record(true)
    assert stats.predictions == 1
    assert stats.correct == 1
    assert stats.incorrect == 0
  end

  test "record/2 increments predictions and incorrect on false" do
    stats = Stats.new() |> Stats.record(false)
    assert stats.predictions == 1
    assert stats.correct == 0
    assert stats.incorrect == 1
  end

  test "record/2 accumulates multiple outcomes" do
    stats =
      Stats.new()
      |> Stats.record(true)
      |> Stats.record(true)
      |> Stats.record(false)
      |> Stats.record(true)

    assert stats.predictions == 4
    assert stats.correct == 3
    assert stats.incorrect == 1
  end

  test "record/2 handles long sequences" do
    stats = Enum.reduce(1..100, Stats.new(), fn _, acc -> Stats.record(acc, true) end)
    assert stats.predictions == 100
    assert stats.correct == 100
    assert stats.incorrect == 0
  end

  test "record/2 with all incorrect" do
    stats = Enum.reduce(1..50, Stats.new(), fn _, acc -> Stats.record(acc, false) end)
    assert stats.predictions == 50
    assert stats.correct == 0
    assert stats.incorrect == 50
  end

  test "record/2 with mixed sequence counts correctly" do
    outcomes = [true, true, false, true, false, true, true, true, true, false]

    stats =
      Enum.reduce(outcomes, Stats.new(), fn correct?, acc -> Stats.record(acc, correct?) end)

    assert stats.predictions == 10
    assert stats.correct == 7
    assert stats.incorrect == 3
    assert Stats.accuracy(stats) == 70.0
  end

  # ── Accuracy ────────────────────────────────────────────────────────────

  test "accuracy/1 returns 0.0 with no predictions" do
    assert Stats.accuracy(Stats.new()) == 0.0
  end

  test "accuracy/1 returns 100.0 when all correct" do
    stats =
      Stats.new()
      |> Stats.record(true)
      |> Stats.record(true)

    assert Stats.accuracy(stats) == 100.0
  end

  test "accuracy/1 returns 0.0 when all incorrect" do
    stats =
      Stats.new()
      |> Stats.record(false)
      |> Stats.record(false)

    assert Stats.accuracy(stats) == 0.0
  end

  test "accuracy/1 returns correct percentage" do
    stats = %Stats{predictions: 100, correct: 87, incorrect: 13}
    assert Stats.accuracy(stats) == 87.0
  end

  test "accuracy/1 with 2 out of 3 correct" do
    stats =
      Stats.new()
      |> Stats.record(true)
      |> Stats.record(true)
      |> Stats.record(false)

    assert_in_delta Stats.accuracy(stats), 66.67, 0.01
  end

  test "accuracy/1 returns float" do
    stats = %Stats{predictions: 3, correct: 1, incorrect: 2}
    assert_in_delta Stats.accuracy(stats), 33.33, 0.01
  end

  test "accuracy/1 with 75 percent" do
    stats = %Stats{predictions: 200, correct: 150, incorrect: 50}
    assert Stats.accuracy(stats) == 75.0
  end

  # ── Misprediction rate ──────────────────────────────────────────────────

  test "misprediction_rate/1 returns 0.0 with no predictions" do
    assert Stats.misprediction_rate(Stats.new()) == 0.0
  end

  test "misprediction_rate/1 returns 0.0 when all correct" do
    stats = Stats.new() |> Stats.record(true) |> Stats.record(true)
    assert Stats.misprediction_rate(stats) == 0.0
  end

  test "misprediction_rate/1 returns 100.0 when all incorrect" do
    stats = Stats.new() |> Stats.record(false)
    assert Stats.misprediction_rate(stats) == 100.0
  end

  test "misprediction_rate/1 is complement of accuracy" do
    stats = %Stats{predictions: 100, correct: 87, incorrect: 13}
    assert Stats.accuracy(stats) + Stats.misprediction_rate(stats) == 100.0
  end

  test "misprediction_rate/1 returns correct percentage" do
    stats = %Stats{predictions: 100, correct: 87, incorrect: 13}
    assert Stats.misprediction_rate(stats) == 13.0
  end

  test "accuracy + misprediction sum to 100 for arbitrary values" do
    stats = %Stats{predictions: 37, correct: 23, incorrect: 14}
    assert abs(Stats.accuracy(stats) + Stats.misprediction_rate(stats) - 100.0) < 1.0e-10
  end

  # ── Reset ───────────────────────────────────────────────────────────────

  test "reset/1 clears all counters" do
    stats = %Stats{predictions: 50, correct: 40, incorrect: 10}
    reset = Stats.reset(stats)
    assert reset.predictions == 0
    assert reset.correct == 0
    assert reset.incorrect == 0
  end

  test "reset/1 results in 0.0 accuracy" do
    stats = %Stats{predictions: 50, correct: 40, incorrect: 10}
    reset = Stats.reset(stats)
    assert Stats.accuracy(reset) == 0.0
  end

  test "reset/1 results in 0.0 misprediction_rate" do
    stats = %Stats{predictions: 50, correct: 40, incorrect: 10}
    reset = Stats.reset(stats)
    assert Stats.misprediction_rate(reset) == 0.0
  end

  test "record after reset works correctly" do
    stats =
      Stats.new()
      |> Stats.record(true)
      |> Stats.record(false)
      |> Stats.reset()
      |> Stats.record(true)

    assert stats.predictions == 1
    assert stats.correct == 1
    assert Stats.accuracy(stats) == 100.0
  end

  # ── Immutability ────────────────────────────────────────────────────────

  test "record/2 does not mutate original struct" do
    original = Stats.new()
    _updated = Stats.record(original, true)
    assert original.predictions == 0
    assert original.correct == 0
  end

  test "reset/1 does not mutate original struct" do
    original = %Stats{predictions: 10, correct: 8, incorrect: 2}
    _reset = Stats.reset(original)
    assert original.predictions == 10
  end
end
